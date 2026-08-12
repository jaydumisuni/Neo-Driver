//! Deterministic, read-only driver candidate matching for Neo Driver.
//!
//! This crate preserves Windows identifier-score semantics without pretending to
//! compute the full Windows driver rank. Exact signature-score and FeatureScore
//! values are not yet present in the catalogue, so Neo exposes those gaps rather
//! than inventing them.

use neo_catalogue::{
    Catalogue, DriverArtifact, InfModelEntry, PackageKind, PackageManifest, SignatureStatus,
};
use neo_core::EvidenceVerdict;
use neo_device::{DeviceRecord, OpaqueDeviceId};
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;
use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MatchContext {
    pub architecture: String,
    pub windows_build: u32,
}

impl MatchContext {
    pub fn validate(&self) -> Result<(), MatchError> {
        if self.architecture.trim().is_empty() {
            return Err(MatchError::EmptyArchitecture);
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum IdentifierMatchType {
    DeviceHardwareToInfHardware,
    DeviceHardwareToInfCompatible,
    DeviceCompatibleToInfHardware,
    DeviceCompatibleToInfCompatible,
}

impl IdentifierMatchType {
    pub fn type_score(self) -> u32 {
        match self {
            Self::DeviceHardwareToInfHardware => 0x0000,
            Self::DeviceHardwareToInfCompatible => 0x1000,
            Self::DeviceCompatibleToInfHardware => 0x2000,
            Self::DeviceCompatibleToInfCompatible => 0x3000,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct IdentifierMatchEvidence {
    pub match_type: IdentifierMatchType,
    pub device_id: String,
    pub inf_id: String,
    pub device_position: usize,
    pub model_position: usize,
    pub inf_position: usize,
    pub identifier_score: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RejectionReason {
    ArchitectureMetadataMissing,
    ArchitectureMismatch,
    WindowsBuildTooOld,
    WindowsBuildTooNew,
    NoIdentifierMatch,
    IdentifierScoreOutOfRange,
    InvalidSignature,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CandidateMatch {
    pub package_id: String,
    pub package_name: String,
    pub vendor: String,
    pub inf_path: String,
    pub verdict: EvidenceVerdict,
    pub signature_status: SignatureStatus,
    pub identifier: Option<IdentifierMatchEvidence>,
    pub rejection_reasons: Vec<RejectionReason>,
    pub driver_date: Option<String>,
    pub driver_version: Option<String>,
    pub full_windows_rank_available: bool,
}

impl CandidateMatch {
    pub fn is_rejected(&self) -> bool {
        self.verdict == EvidenceVerdict::Rejected
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MatchReport {
    pub device_instance_id: String,
    pub context: MatchContext,
    pub candidates: Vec<CandidateMatch>,
    pub best_candidate: Option<CandidateIdentity>,
    pub ranking_complete: bool,
    pub notes: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CandidateIdentity {
    pub package_id: String,
    pub inf_path: String,
}

pub fn match_device(
    device: &DeviceRecord,
    catalogue: &Catalogue,
    context: &MatchContext,
) -> Result<MatchReport, MatchError> {
    device
        .validate()
        .map_err(|error| MatchError::Device(error.to_string()))?;
    catalogue
        .validate()
        .map_err(|error| MatchError::Catalogue(error.to_string()))?;
    context.validate()?;

    let mut candidates = Vec::new();
    for package in &catalogue.packages {
        if package.kind != PackageKind::InfDriverBundle {
            continue;
        }
        for artifact in &package.driver_artifacts {
            candidates.push(match_artifact(device, package, artifact, context));
        }
    }

    candidates.sort_by(compare_candidates);
    let best_candidate = determine_unique_best(&candidates);
    let non_rejected: Vec<_> = candidates
        .iter()
        .filter(|candidate| !candidate.is_rejected())
        .collect();
    let ranking_complete = !non_rejected.is_empty()
        && non_rejected
            .iter()
            .all(|candidate| candidate.full_windows_rank_available);

    Ok(MatchReport {
        device_instance_id: device.instance_id.to_string(),
        context: context.clone(),
        candidates,
        best_candidate,
        ranking_complete,
        notes: vec![
            "Identifier scores follow Windows device/INF match classes and ordered ID positions."
                .to_string(),
            "Neo does not claim the full Windows rank until exact signature-score and FeatureScore evidence is available."
                .to_string(),
            "Verified signature state is a Neo safety gate; unknown or unsigned candidates require investigation."
                .to_string(),
            "Driver date/version only break ties after equal Neo safety state and identifier score."
                .to_string(),
        ],
    })
}

fn match_artifact(
    device: &DeviceRecord,
    package: &PackageManifest,
    artifact: &DriverArtifact,
    context: &MatchContext,
) -> CandidateMatch {
    let mut rejection_reasons = Vec::new();

    if package.windows.architectures.is_empty() {
        rejection_reasons.push(RejectionReason::ArchitectureMetadataMissing);
    } else if !package
        .windows
        .architectures
        .iter()
        .any(|value| value.eq_ignore_ascii_case(context.architecture.trim()))
    {
        rejection_reasons.push(RejectionReason::ArchitectureMismatch);
    }

    if let Some(minimum) = package.windows.minimum_build {
        if context.windows_build < minimum {
            rejection_reasons.push(RejectionReason::WindowsBuildTooOld);
        }
    }
    if let Some(maximum) = package.windows.maximum_build {
        if context.windows_build > maximum {
            rejection_reasons.push(RejectionReason::WindowsBuildTooNew);
        }
    }

    let identifier_search = identifier_match_search(device, artifact);
    let identifier = identifier_search.best;
    if identifier.is_none() {
        if identifier_search.out_of_range_match {
            rejection_reasons.push(RejectionReason::IdentifierScoreOutOfRange);
        } else {
            rejection_reasons.push(RejectionReason::NoIdentifierMatch);
        }
    }

    if artifact.signature.status == SignatureStatus::Invalid {
        rejection_reasons.push(RejectionReason::InvalidSignature);
    }

    let verdict = if !rejection_reasons.is_empty() {
        EvidenceVerdict::Rejected
    } else {
        match artifact.signature.status {
            SignatureStatus::Verified => EvidenceVerdict::Certified,
            SignatureStatus::Unknown | SignatureStatus::Unsigned => EvidenceVerdict::Investigate,
            SignatureStatus::Invalid => EvidenceVerdict::Rejected,
        }
    };

    CandidateMatch {
        package_id: package.package_id.clone(),
        package_name: package.name.clone(),
        vendor: package.vendor.clone(),
        inf_path: artifact.inf_path.clone(),
        verdict,
        signature_status: artifact.signature.status,
        identifier,
        rejection_reasons,
        driver_date: artifact.driver_date.clone(),
        driver_version: artifact.driver_version.clone(),
        full_windows_rank_available: false,
    }
}

#[derive(Debug, Default)]
struct IdentifierMatchSearch {
    best: Option<IdentifierMatchEvidence>,
    out_of_range_match: bool,
}

struct MatchCoordinates<'a> {
    device_id: &'a OpaqueDeviceId,
    inf_id: &'a OpaqueDeviceId,
    device_position: usize,
    model_position: usize,
    inf_position: usize,
}

pub fn best_identifier_match(
    device: &DeviceRecord,
    artifact: &DriverArtifact,
) -> Option<IdentifierMatchEvidence> {
    identifier_match_search(device, artifact).best
}

fn identifier_match_search(
    device: &DeviceRecord,
    artifact: &DriverArtifact,
) -> IdentifierMatchSearch {
    let mut matches = Vec::new();
    let mut out_of_range_match = false;

    for (model_position, model) in artifact.models.iter().enumerate() {
        collect_model_matches(
            &mut matches,
            &mut out_of_range_match,
            device,
            model,
            model_position,
        );
    }

    let best = matches.into_iter().min_by(|left, right| {
        left.identifier_score
            .cmp(&right.identifier_score)
            .then_with(|| left.model_position.cmp(&right.model_position))
            .then_with(|| left.inf_position.cmp(&right.inf_position))
            .then_with(|| left.device_id.cmp(&right.device_id))
            .then_with(|| left.inf_id.cmp(&right.inf_id))
    });

    IdentifierMatchSearch {
        best,
        out_of_range_match,
    }
}

fn collect_model_matches(
    output: &mut Vec<IdentifierMatchEvidence>,
    out_of_range_match: &mut bool,
    device: &DeviceRecord,
    model: &InfModelEntry,
    model_position: usize,
) {
    for (device_position, device_id) in device.ids.hardware_ids.iter().enumerate() {
        if let Some(hardware_id) = &model.hardware_id {
            if device_id
                .as_str()
                .eq_ignore_ascii_case(hardware_id.as_str())
            {
                push_scored_match(
                    output,
                    out_of_range_match,
                    IdentifierMatchType::DeviceHardwareToInfHardware,
                    MatchCoordinates {
                        device_id,
                        inf_id: hardware_id,
                        device_position,
                        model_position,
                        inf_position: 0,
                    },
                );
            }
        }
        for (inf_position, inf_id) in model.compatible_ids.iter().enumerate() {
            if device_id.as_str().eq_ignore_ascii_case(inf_id.as_str()) {
                push_scored_match(
                    output,
                    out_of_range_match,
                    IdentifierMatchType::DeviceHardwareToInfCompatible,
                    MatchCoordinates {
                        device_id,
                        inf_id,
                        device_position,
                        model_position,
                        inf_position,
                    },
                );
            }
        }
    }

    for (device_position, device_id) in device.ids.compatible_ids.iter().enumerate() {
        if let Some(hardware_id) = &model.hardware_id {
            if device_id
                .as_str()
                .eq_ignore_ascii_case(hardware_id.as_str())
            {
                push_scored_match(
                    output,
                    out_of_range_match,
                    IdentifierMatchType::DeviceCompatibleToInfHardware,
                    MatchCoordinates {
                        device_id,
                        inf_id: hardware_id,
                        device_position,
                        model_position,
                        inf_position: 0,
                    },
                );
            }
        }
        for (inf_position, inf_id) in model.compatible_ids.iter().enumerate() {
            if device_id.as_str().eq_ignore_ascii_case(inf_id.as_str()) {
                push_scored_match(
                    output,
                    out_of_range_match,
                    IdentifierMatchType::DeviceCompatibleToInfCompatible,
                    MatchCoordinates {
                        device_id,
                        inf_id,
                        device_position,
                        model_position,
                        inf_position,
                    },
                );
            }
        }
    }
}

fn push_scored_match(
    output: &mut Vec<IdentifierMatchEvidence>,
    out_of_range_match: &mut bool,
    match_type: IdentifierMatchType,
    coordinates: MatchCoordinates<'_>,
) {
    if let Some(identifier_score) = identifier_score(
        match_type,
        coordinates.device_position,
        coordinates.inf_position,
    ) {
        output.push(IdentifierMatchEvidence {
            match_type,
            device_id: coordinates.device_id.to_string(),
            inf_id: coordinates.inf_id.to_string(),
            device_position: coordinates.device_position,
            model_position: coordinates.model_position,
            inf_position: coordinates.inf_position,
            identifier_score,
        });
    } else {
        *out_of_range_match = true;
    }
}

/// Computes the documented Windows identifier score when it fits the THHH field.
///
/// `None` means the supplied list positions cannot be represented inside the
/// documented `0x0000..=0x3fff` identifier-score range. Neo fails closed rather
/// than wrapping, saturating, or inventing a rank value.
pub fn identifier_score(
    match_type: IdentifierMatchType,
    device_position: usize,
    inf_position: usize,
) -> Option<u32> {
    let device_position = u32::try_from(device_position).ok()?;
    let inf_position = u32::try_from(inf_position).ok()?;
    let position_score = match match_type {
        IdentifierMatchType::DeviceHardwareToInfHardware
        | IdentifierMatchType::DeviceHardwareToInfCompatible
        | IdentifierMatchType::DeviceCompatibleToInfHardware => device_position,
        IdentifierMatchType::DeviceCompatibleToInfCompatible => {
            device_position.checked_add(inf_position.checked_mul(0x100)?)?
        }
    };
    if position_score > 0x0fff {
        return None;
    }
    match_type.type_score().checked_add(position_score)
}

fn compare_candidates(left: &CandidateMatch, right: &CandidateMatch) -> Ordering {
    candidate_class(left)
        .cmp(&candidate_class(right))
        .then_with(|| {
            signature_class(left.signature_status).cmp(&signature_class(right.signature_status))
        })
        .then_with(|| identifier_value(left).cmp(&identifier_value(right)))
        .then_with(|| compare_known_windows_tiebreaks(left, right))
        .then_with(|| left.package_id.cmp(&right.package_id))
        .then_with(|| left.inf_path.cmp(&right.inf_path))
}

fn compare_known_windows_tiebreaks(left: &CandidateMatch, right: &CandidateMatch) -> Ordering {
    let (Some(left_date), Some(right_date)) = (
        parsed_date(&left.driver_date),
        parsed_date(&right.driver_date),
    ) else {
        return Ordering::Equal;
    };

    match right_date.cmp(&left_date) {
        Ordering::Equal => {
            let (Some(left_version), Some(right_version)) = (
                parsed_version(&left.driver_version),
                parsed_version(&right.driver_version),
            ) else {
                return Ordering::Equal;
            };
            right_version.cmp(&left_version)
        }
        ordering => ordering,
    }
}

fn candidate_class(candidate: &CandidateMatch) -> u8 {
    match candidate.verdict {
        EvidenceVerdict::Certified => 0,
        EvidenceVerdict::Provisional => 1,
        EvidenceVerdict::Investigate => 2,
        EvidenceVerdict::Rejected => 3,
    }
}

fn signature_class(status: SignatureStatus) -> u8 {
    match status {
        SignatureStatus::Verified => 0,
        SignatureStatus::Unknown | SignatureStatus::Unsigned => 1,
        SignatureStatus::Invalid => 2,
    }
}

fn identifier_value(candidate: &CandidateMatch) -> u32 {
    candidate
        .identifier
        .as_ref()
        .map(|value| value.identifier_score)
        .unwrap_or(u32::MAX)
}

fn determine_unique_best(candidates: &[CandidateMatch]) -> Option<CandidateIdentity> {
    let first = candidates.first()?;
    if first.is_rejected() {
        return None;
    }
    if let Some(second) = candidates.get(1) {
        if equivalent_selection_key(first, second) {
            return None;
        }
    }
    Some(CandidateIdentity {
        package_id: first.package_id.clone(),
        inf_path: first.inf_path.clone(),
    })
}

fn equivalent_selection_key(left: &CandidateMatch, right: &CandidateMatch) -> bool {
    if candidate_class(left) != candidate_class(right)
        || signature_class(left.signature_status) != signature_class(right.signature_status)
        || identifier_value(left) != identifier_value(right)
    {
        return false;
    }

    let (Some(left_date), Some(right_date)) = (
        parsed_date(&left.driver_date),
        parsed_date(&right.driver_date),
    ) else {
        return true;
    };
    if left_date != right_date {
        return false;
    }

    let (Some(left_version), Some(right_version)) = (
        parsed_version(&left.driver_version),
        parsed_version(&right.driver_version),
    ) else {
        return true;
    };
    left_version == right_version
}

fn parsed_date(value: &Option<String>) -> Option<(u16, u8, u8)> {
    let value = value.as_deref()?.trim();
    if let Some(parsed) = parse_ymd(value) {
        return Some(parsed);
    }
    parse_mdy(value)
}

fn parse_ymd(value: &str) -> Option<(u16, u8, u8)> {
    let mut parts = value.split('-');
    let year = parts.next()?.parse::<u16>().ok()?;
    let month = parts.next()?.parse::<u8>().ok()?;
    let day = parts.next()?.parse::<u8>().ok()?;
    if parts.next().is_some() || !valid_calendar_date(year, month, day) {
        return None;
    }
    Some((year, month, day))
}

fn parse_mdy(value: &str) -> Option<(u16, u8, u8)> {
    let mut parts = value.split('/');
    let month = parts.next()?.parse::<u8>().ok()?;
    let day = parts.next()?.parse::<u8>().ok()?;
    let year = parts.next()?.parse::<u16>().ok()?;
    if parts.next().is_some() || !valid_calendar_date(year, month, day) {
        return None;
    }
    Some((year, month, day))
}

fn valid_calendar_date(year: u16, month: u8, day: u8) -> bool {
    if !(1..=12).contains(&month) || day == 0 {
        return false;
    }
    let leap_year =
        year.is_multiple_of(4) && (!year.is_multiple_of(100) || year.is_multiple_of(400));
    let maximum = match month {
        2 if leap_year => 29,
        2 => 28,
        4 | 6 | 9 | 11 => 30,
        _ => 31,
    };
    day <= maximum
}

fn parsed_version(value: &Option<String>) -> Option<[u64; 4]> {
    let value = value.as_deref()?.trim();
    let parts: Vec<_> = value.split('.').collect();
    if parts.is_empty() || parts.len() > 4 {
        return None;
    }
    let mut result = [0_u64; 4];
    for (index, part) in parts.iter().enumerate() {
        result[index] = part.parse::<u64>().ok()?;
    }
    Some(result)
}

#[derive(Debug, Error)]
pub enum MatchError {
    #[error("architecture cannot be empty")]
    EmptyArchitecture,
    #[error("device evidence invalid: {0}")]
    Device(String),
    #[error("catalogue invalid: {0}")]
    Catalogue(String),
}

#[cfg(test)]
mod tests;
