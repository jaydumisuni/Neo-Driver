//! Phase 13 read-only AppX/debloat assessment foundation.
//!
//! This crate models catalogue classification, installed/provisioned evidence,
//! preservation policy, restore metadata, explicit selection, and deterministic
//! assessment. It intentionally contains no Windows API, process execution,
//! package removal, provisioning mutation, transaction, or capability issuance.

use neo_core::{EvidenceVerdict, RecommendationState, RiskLevel};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::str::FromStr;
use thiserror::Error;

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum DebloatError {
    #[error("{0} is required")]
    EmptyField(&'static str),
    #[error("invalid Neo debloat id: {0}")]
    InvalidId(String),
    #[error("invalid AppX package id: {0}")]
    InvalidPackageId(String),
    #[error("invalid Store id: {0}")]
    InvalidStoreId(String),
    #[error("debloat catalogue cannot be empty")]
    EmptyCatalogue,
    #[error("duplicate Neo debloat id: {0}")]
    DuplicateId(String),
    #[error("duplicate AppX package identity: {0}")]
    DuplicatePackageId(String),
    #[error("duplicate preserved profile on {0}")]
    DuplicatePreservedProfile(String),
    #[error("non-safe debloat class requires side-effect/dependency notes: {0}")]
    MissingSideEffectNotes(String),
    #[error("only SAFE OPTIONAL items may be selected by default: {0}")]
    UnsafeDefaultClass(String),
    #[error("only LOW risk items may be selected by default: {0}")]
    UnsafeDefaultRisk(String),
    #[error("default-selected item must be CERTIFIED: {0}")]
    NonCertifiedDefault(String),
    #[error("unsafe recommendation cannot be selected by default: {0}")]
    UnsafeRecommendationDefault(String),
    #[error("default-selected item must have a restore path: {0}")]
    DefaultWithoutRestore(String),
    #[error("Safe Cleanup cannot default-select an item it preserves: {0}")]
    DefaultPreservedBySafeCleanup(String),
    #[error("duplicate package observation: {0}")]
    DuplicateObservation(String),
    #[error("debloat selection cannot be empty")]
    EmptySelection,
    #[error("duplicate selected debloat id: {0}")]
    DuplicateSelection(String),
    #[error("unknown selected debloat id: {0}")]
    UnknownSelection(String),
    #[error("missing package observation for: {0}")]
    MissingObservation(String),
    #[error("package observation is unavailable for: {0}")]
    UnavailableObservation(String),
    #[error("unknown debloat profile: {0}")]
    UnknownProfile(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DebloatClass {
    SafeOptional,
    FeatureDependent,
    DependencySensitive,
    ProtectedManualOnly,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DebloatProfile {
    SafeCleanup,
    Gaming,
    Technician,
    Developer,
    Custom,
}

impl FromStr for DebloatProfile {
    type Err = DebloatError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "safe-cleanup" | "safe_cleanup" => Ok(Self::SafeCleanup),
            "gaming" => Ok(Self::Gaming),
            "technician" => Ok(Self::Technician),
            "developer" => Ok(Self::Developer),
            "custom" => Ok(Self::Custom),
            _ => Err(DebloatError::UnknownProfile(value.to_string())),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DebloatScope {
    CurrentUser,
    Provisioned,
    CurrentUserAndProvisioned,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum RestoreMethod {
    Store { store_id: String },
    ProvisionedImage,
    Vendor { source: String },
    None,
}

impl RestoreMethod {
    pub fn available(&self) -> bool {
        !matches!(self, Self::None)
    }

    fn validate(&self) -> Result<(), DebloatError> {
        match self {
            Self::Store { store_id } => validate_store_id(store_id),
            Self::Vendor { source } => require_text("restore vendor source", source),
            Self::ProvisionedImage | Self::None => Ok(()),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DebloatDefinition {
    pub id: String,
    pub package_id: String,
    pub title: String,
    pub category: String,
    pub description: String,
    pub class: DebloatClass,
    pub scope: DebloatScope,
    pub risk: RiskLevel,
    pub recommendation: RecommendationState,
    pub verdict: EvidenceVerdict,
    pub selected_by_default: bool,
    pub restore: RestoreMethod,
    #[serde(default)]
    pub side_effects: Vec<String>,
    #[serde(default)]
    pub preserve_in_profiles: Vec<DebloatProfile>,
}

impl DebloatDefinition {
    pub fn validate(&self) -> Result<(), DebloatError> {
        validate_id(&self.id)?;
        validate_package_id(&self.package_id)?;
        require_text("debloat title", &self.title)?;
        require_text("debloat category", &self.category)?;
        require_text("debloat description", &self.description)?;
        self.restore.validate()?;

        for side_effect in &self.side_effects {
            require_text("debloat side effect", side_effect)?;
        }
        if self.class != DebloatClass::SafeOptional && self.side_effects.is_empty() {
            return Err(DebloatError::MissingSideEffectNotes(self.id.clone()));
        }

        let mut profiles = BTreeSet::new();
        for profile in &self.preserve_in_profiles {
            if !profiles.insert(*profile) {
                return Err(DebloatError::DuplicatePreservedProfile(self.id.clone()));
            }
        }

        if self.selected_by_default {
            if self.class != DebloatClass::SafeOptional {
                return Err(DebloatError::UnsafeDefaultClass(self.id.clone()));
            }
            if self.risk != RiskLevel::Low {
                return Err(DebloatError::UnsafeDefaultRisk(self.id.clone()));
            }
            if self.verdict != EvidenceVerdict::Certified {
                return Err(DebloatError::NonCertifiedDefault(self.id.clone()));
            }
            if !recommendation_allows_removal(self.recommendation) {
                return Err(DebloatError::UnsafeRecommendationDefault(self.id.clone()));
            }
            if !self.restore.available() {
                return Err(DebloatError::DefaultWithoutRestore(self.id.clone()));
            }
            if self
                .preserve_in_profiles
                .contains(&DebloatProfile::SafeCleanup)
            {
                return Err(DebloatError::DefaultPreservedBySafeCleanup(self.id.clone()));
            }
        }

        Ok(())
    }

    pub fn canonical_package_id(&self) -> String {
        canonical_package_id(&self.package_id)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct DebloatCatalogue {
    items: Vec<DebloatDefinition>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
struct DebloatCatalogueWire {
    items: Vec<DebloatDefinition>,
}

impl TryFrom<DebloatCatalogueWire> for DebloatCatalogue {
    type Error = DebloatError;

    fn try_from(wire: DebloatCatalogueWire) -> Result<Self, Self::Error> {
        Self::new(wire.items)
    }
}

impl<'de> Deserialize<'de> for DebloatCatalogue {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let wire = DebloatCatalogueWire::deserialize(deserializer)?;
        Self::try_from(wire).map_err(serde::de::Error::custom)
    }
}

impl DebloatCatalogue {
    pub fn new(items: Vec<DebloatDefinition>) -> Result<Self, DebloatError> {
        if items.is_empty() {
            return Err(DebloatError::EmptyCatalogue);
        }

        let mut ids = BTreeSet::new();
        let mut packages = BTreeSet::new();
        for item in &items {
            item.validate()?;
            if !ids.insert(item.id.clone()) {
                return Err(DebloatError::DuplicateId(item.id.clone()));
            }
            let package_key = item.canonical_package_id();
            if !packages.insert(package_key) {
                return Err(DebloatError::DuplicatePackageId(item.package_id.clone()));
            }
        }
        Ok(Self { items })
    }

    pub fn items(&self) -> &[DebloatDefinition] {
        &self.items
    }

    pub fn get(&self, id: &str) -> Option<&DebloatDefinition> {
        self.items.iter().find(|item| item.id == id)
    }

    pub fn default_selection(&self, profile: DebloatProfile) -> Vec<String> {
        if profile == DebloatProfile::Custom {
            return Vec::new();
        }
        self.items
            .iter()
            .filter(|item| item.selected_by_default)
            .filter(|item| !item.preserve_in_profiles.contains(&profile))
            .map(|item| item.id.clone())
            .collect()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ObservedPresence {
    Present,
    Absent,
    Unavailable,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DebloatObservation {
    pub package_id: String,
    pub installed: ObservedPresence,
    pub provisioned: ObservedPresence,
    pub version: Option<String>,
    pub source: String,
}

impl DebloatObservation {
    pub fn validate(&self) -> Result<(), DebloatError> {
        validate_package_id(&self.package_id)?;
        require_text("debloat observation source", &self.source)?;
        if let Some(version) = &self.version {
            require_text("debloat observation version", version)?;
        }
        Ok(())
    }

    pub fn canonical_package_id(&self) -> String {
        canonical_package_id(&self.package_id)
    }

    fn fully_available(&self) -> bool {
        self.installed != ObservedPresence::Unavailable
            && self.provisioned != ObservedPresence::Unavailable
    }

    fn absent_for_scope(&self, scope: DebloatScope) -> bool {
        match scope {
            DebloatScope::CurrentUser => self.installed == ObservedPresence::Absent,
            DebloatScope::Provisioned => self.provisioned == ObservedPresence::Absent,
            DebloatScope::CurrentUserAndProvisioned => {
                self.installed == ObservedPresence::Absent
                    && self.provisioned == ObservedPresence::Absent
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct DebloatEvidence {
    observations: Vec<DebloatObservation>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
struct DebloatEvidenceWire {
    observations: Vec<DebloatObservation>,
}

impl TryFrom<DebloatEvidenceWire> for DebloatEvidence {
    type Error = DebloatError;

    fn try_from(wire: DebloatEvidenceWire) -> Result<Self, Self::Error> {
        Self::new(wire.observations)
    }
}

impl<'de> Deserialize<'de> for DebloatEvidence {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let wire = DebloatEvidenceWire::deserialize(deserializer)?;
        Self::try_from(wire).map_err(serde::de::Error::custom)
    }
}

impl DebloatEvidence {
    pub fn new(observations: Vec<DebloatObservation>) -> Result<Self, DebloatError> {
        let mut seen = BTreeSet::new();
        for observation in &observations {
            observation.validate()?;
            let key = observation.canonical_package_id();
            if !seen.insert(key) {
                return Err(DebloatError::DuplicateObservation(
                    observation.package_id.clone(),
                ));
            }
        }
        Ok(Self { observations })
    }

    pub fn observations(&self) -> &[DebloatObservation] {
        &self.observations
    }

    pub fn get_by_package_id(&self, package_id: &str) -> Option<&DebloatObservation> {
        let key = canonical_package_id(package_id);
        self.observations
            .iter()
            .find(|observation| observation.canonical_package_id() == key)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DebloatDisposition {
    RemovalCandidate,
    AlreadyAbsent,
    NeedsReview,
    BlockedByProfile,
    BlockedProtected,
    BlockedPolicy,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct DebloatAssessmentItem {
    pub id: String,
    pub package_id: String,
    pub title: String,
    pub class: DebloatClass,
    pub scope: DebloatScope,
    pub installed: ObservedPresence,
    pub provisioned: ObservedPresence,
    pub version: Option<String>,
    pub restore_available: bool,
    pub disposition: DebloatDisposition,
    pub reasons: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct DebloatAssessment {
    pub profile: DebloatProfile,
    pub items: Vec<DebloatAssessmentItem>,
    pub machine_changes: bool,
}

pub fn assess_debloat(
    catalogue: &DebloatCatalogue,
    evidence: &DebloatEvidence,
    profile: DebloatProfile,
    selected_ids: &[String],
) -> Result<DebloatAssessment, DebloatError> {
    if selected_ids.is_empty() {
        return Err(DebloatError::EmptySelection);
    }

    let mut seen = BTreeSet::new();
    let mut selected = Vec::with_capacity(selected_ids.len());
    for id in selected_ids {
        validate_id(id)?;
        if !seen.insert(id.clone()) {
            return Err(DebloatError::DuplicateSelection(id.clone()));
        }
        let definition = catalogue
            .get(id)
            .ok_or_else(|| DebloatError::UnknownSelection(id.clone()))?;
        selected.push(definition);
    }

    let mut items = Vec::with_capacity(selected.len());
    for definition in selected {
        let observation = evidence
            .get_by_package_id(&definition.package_id)
            .ok_or_else(|| DebloatError::MissingObservation(definition.package_id.clone()))?;
        if !observation.fully_available() {
            return Err(DebloatError::UnavailableObservation(
                definition.package_id.clone(),
            ));
        }

        let mut reasons = Vec::new();
        let disposition = if observation.absent_for_scope(definition.scope) {
            reasons.push("package is already absent for the requested scope".to_string());
            DebloatDisposition::AlreadyAbsent
        } else if definition.class == DebloatClass::ProtectedManualOnly {
            reasons.push(
                "protected/manual-only package cannot become normal debloat authority".to_string(),
            );
            DebloatDisposition::BlockedProtected
        } else if definition.preserve_in_profiles.contains(&profile) {
            reasons.push(format!("package is preserved by the {profile:?} profile"));
            DebloatDisposition::BlockedByProfile
        } else if definition.verdict == EvidenceVerdict::Rejected
            || matches!(
                definition.recommendation,
                RecommendationState::Conflict
                    | RecommendationState::Unsupported
                    | RecommendationState::DoNotTouch
            )
        {
            reasons.push(
                "catalogue policy blocks this package from normal debloat candidacy".to_string(),
            );
            DebloatDisposition::BlockedPolicy
        } else if !candidate_policy_allows(definition) {
            reasons.push(
                "package does not satisfy Neo's low-risk certified removal-candidate policy"
                    .to_string(),
            );
            DebloatDisposition::NeedsReview
        } else {
            match definition.class {
                DebloatClass::SafeOptional if definition.restore.available() => {
                    reasons.push(
                        "SAFE OPTIONAL item with a declared restore path; removal may be reviewed"
                            .to_string(),
                    );
                    DebloatDisposition::RemovalCandidate
                }
                DebloatClass::SafeOptional => {
                    reasons.push(
                        "SAFE OPTIONAL item lacks a declared restore path and requires review"
                            .to_string(),
                    );
                    DebloatDisposition::NeedsReview
                }
                DebloatClass::FeatureDependent => {
                    reasons.push(
                        "feature-dependent package requires side-effect/dependency review"
                            .to_string(),
                    );
                    DebloatDisposition::NeedsReview
                }
                DebloatClass::DependencySensitive => {
                    reasons.push(
                        "dependency-sensitive package requires explicit dependency review"
                            .to_string(),
                    );
                    DebloatDisposition::NeedsReview
                }
                DebloatClass::ProtectedManualOnly => DebloatDisposition::BlockedProtected,
            }
        };

        if !definition.side_effects.is_empty() {
            reasons.extend(
                definition
                    .side_effects
                    .iter()
                    .map(|value| format!("side effect: {value}")),
            );
        }

        items.push(DebloatAssessmentItem {
            id: definition.id.clone(),
            package_id: definition.package_id.clone(),
            title: definition.title.clone(),
            class: definition.class,
            scope: definition.scope,
            installed: observation.installed,
            provisioned: observation.provisioned,
            version: observation.version.clone(),
            restore_available: definition.restore.available(),
            disposition,
            reasons,
        });
    }

    Ok(DebloatAssessment {
        profile,
        items,
        machine_changes: false,
    })
}

pub fn assessment_index(assessment: &DebloatAssessment) -> BTreeMap<&str, &DebloatAssessmentItem> {
    assessment
        .items
        .iter()
        .map(|item| (item.id.as_str(), item))
        .collect()
}

fn recommendation_allows_removal(value: RecommendationState) -> bool {
    matches!(
        value,
        RecommendationState::Recommended | RecommendationState::OptionalComponent
    )
}

fn candidate_policy_allows(definition: &DebloatDefinition) -> bool {
    definition.class == DebloatClass::SafeOptional
        && definition.risk == RiskLevel::Low
        && definition.verdict == EvidenceVerdict::Certified
        && recommendation_allows_removal(definition.recommendation)
        && definition.restore.available()
}

fn validate_id(value: &str) -> Result<(), DebloatError> {
    let valid = value.bytes().all(|byte| {
        byte.is_ascii_lowercase() || byte.is_ascii_digit() || matches!(byte, b'.' | b'_' | b'-')
    });
    if value.is_empty() || value != value.to_ascii_lowercase() || !valid {
        return Err(DebloatError::InvalidId(value.to_string()));
    }
    Ok(())
}

fn validate_package_id(value: &str) -> Result<(), DebloatError> {
    let valid = !value.is_empty()
        && value.len() <= 200
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b'-'));
    if !valid {
        return Err(DebloatError::InvalidPackageId(value.to_string()));
    }
    Ok(())
}

fn canonical_package_id(value: &str) -> String {
    value.to_ascii_lowercase()
}

fn validate_store_id(value: &str) -> Result<(), DebloatError> {
    let valid = !value.is_empty()
        && value.len() <= 64
        && value.bytes().all(|byte| byte.is_ascii_alphanumeric());
    if !valid {
        return Err(DebloatError::InvalidStoreId(value.to_string()));
    }
    Ok(())
}

fn require_text(label: &'static str, value: &str) -> Result<(), DebloatError> {
    if value.trim().is_empty() {
        return Err(DebloatError::EmptyField(label));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn definition(id: &str, package_id: &str) -> DebloatDefinition {
        DebloatDefinition {
            id: id.to_string(),
            package_id: package_id.to_string(),
            title: "Fixture package".to_string(),
            category: "Fixture".to_string(),
            description: "Synthetic package used only for deterministic proof.".to_string(),
            class: DebloatClass::SafeOptional,
            scope: DebloatScope::CurrentUserAndProvisioned,
            risk: RiskLevel::Low,
            recommendation: RecommendationState::OptionalComponent,
            verdict: EvidenceVerdict::Certified,
            selected_by_default: true,
            restore: RestoreMethod::Store {
                store_id: "9FIXTURE123".to_string(),
            },
            side_effects: vec![],
            preserve_in_profiles: vec![],
        }
    }

    fn observation(package_id: &str) -> DebloatObservation {
        DebloatObservation {
            package_id: package_id.to_string(),
            installed: ObservedPresence::Present,
            provisioned: ObservedPresence::Present,
            version: Some("1.0.0".to_string()),
            source: "synthetic-unit-proof".to_string(),
        }
    }

    #[test]
    fn catalogue_rejects_case_insensitive_duplicate_package_identity() {
        let a = definition("appx.fixture.one", "Contoso.Fixture");
        let b = definition("appx.fixture.two", "contoso.fixture");
        assert!(matches!(
            DebloatCatalogue::new(vec![a, b]),
            Err(DebloatError::DuplicatePackageId(_))
        ));
    }

    #[test]
    fn non_safe_item_cannot_be_default_selected() {
        let mut item = definition("appx.fixture", "Contoso.Fixture");
        item.class = DebloatClass::FeatureDependent;
        item.side_effects = vec!["synthetic feature consequence".to_string()];
        assert!(matches!(
            item.validate(),
            Err(DebloatError::UnsafeDefaultClass(_))
        ));
    }

    #[test]
    fn non_low_item_cannot_be_default_selected() {
        let mut item = definition("appx.fixture", "Contoso.Fixture");
        item.risk = RiskLevel::Normal;
        assert!(matches!(
            item.validate(),
            Err(DebloatError::UnsafeDefaultRisk(_))
        ));
    }

    #[test]
    fn non_certified_item_cannot_be_default_selected() {
        let mut item = definition("appx.fixture", "Contoso.Fixture");
        item.verdict = EvidenceVerdict::Provisional;
        assert!(matches!(
            item.validate(),
            Err(DebloatError::NonCertifiedDefault(_))
        ));
    }

    #[test]
    fn unsafe_recommendation_cannot_be_default_selected() {
        let mut item = definition("appx.fixture", "Contoso.Fixture");
        item.recommendation = RecommendationState::Unknown;
        assert!(matches!(
            item.validate(),
            Err(DebloatError::UnsafeRecommendationDefault(_))
        ));
    }

    #[test]
    fn default_selected_item_requires_restore_path() {
        let mut item = definition("appx.fixture", "Contoso.Fixture");
        item.restore = RestoreMethod::None;
        assert!(matches!(
            item.validate(),
            Err(DebloatError::DefaultWithoutRestore(_))
        ));
    }

    #[test]
    fn custom_profile_never_receives_hidden_defaults() {
        let catalogue =
            DebloatCatalogue::new(vec![definition("appx.fixture", "Contoso.Fixture")]).unwrap();
        assert!(catalogue
            .default_selection(DebloatProfile::Custom)
            .is_empty());
    }

    #[test]
    fn preserved_profile_filters_default_selection() {
        let mut item = definition("appx.fixture", "Contoso.Fixture");
        item.preserve_in_profiles.push(DebloatProfile::Gaming);
        let catalogue = DebloatCatalogue::new(vec![item]).unwrap();
        assert!(catalogue
            .default_selection(DebloatProfile::Gaming)
            .is_empty());
        assert_eq!(
            catalogue.default_selection(DebloatProfile::SafeCleanup),
            vec!["appx.fixture".to_string()]
        );
    }

    #[test]
    fn assessment_requires_explicit_selection() {
        let catalogue =
            DebloatCatalogue::new(vec![definition("appx.fixture", "Contoso.Fixture")]).unwrap();
        let evidence = DebloatEvidence::new(vec![observation("Contoso.Fixture")]).unwrap();
        assert_eq!(
            assess_debloat(&catalogue, &evidence, DebloatProfile::SafeCleanup, &[]),
            Err(DebloatError::EmptySelection)
        );
    }

    #[test]
    fn duplicate_selection_fails_closed() {
        let catalogue =
            DebloatCatalogue::new(vec![definition("appx.fixture", "Contoso.Fixture")]).unwrap();
        let evidence = DebloatEvidence::new(vec![observation("Contoso.Fixture")]).unwrap();
        let selected = vec!["appx.fixture".to_string(), "appx.fixture".to_string()];
        assert!(matches!(
            assess_debloat(
                &catalogue,
                &evidence,
                DebloatProfile::SafeCleanup,
                &selected
            ),
            Err(DebloatError::DuplicateSelection(_))
        ));
    }

    #[test]
    fn unknown_selection_fails_closed() {
        let catalogue =
            DebloatCatalogue::new(vec![definition("appx.fixture", "Contoso.Fixture")]).unwrap();
        let evidence = DebloatEvidence::new(vec![observation("Contoso.Fixture")]).unwrap();
        let selected = vec!["appx.unknown".to_string()];
        assert!(matches!(
            assess_debloat(
                &catalogue,
                &evidence,
                DebloatProfile::SafeCleanup,
                &selected
            ),
            Err(DebloatError::UnknownSelection(_))
        ));
    }

    #[test]
    fn duplicate_observation_fails_closed() {
        assert!(matches!(
            DebloatEvidence::new(vec![
                observation("Contoso.Fixture"),
                observation("contoso.fixture")
            ]),
            Err(DebloatError::DuplicateObservation(_))
        ));
    }

    #[test]
    fn missing_observation_fails_closed() {
        let catalogue =
            DebloatCatalogue::new(vec![definition("appx.fixture", "Contoso.Fixture")]).unwrap();
        let evidence = DebloatEvidence::new(vec![]).unwrap();
        let selected = vec!["appx.fixture".to_string()];
        assert!(matches!(
            assess_debloat(
                &catalogue,
                &evidence,
                DebloatProfile::SafeCleanup,
                &selected
            ),
            Err(DebloatError::MissingObservation(_))
        ));
    }

    #[test]
    fn unavailable_observation_fails_closed() {
        let catalogue =
            DebloatCatalogue::new(vec![definition("appx.fixture", "Contoso.Fixture")]).unwrap();
        let mut observed = observation("Contoso.Fixture");
        observed.provisioned = ObservedPresence::Unavailable;
        let evidence = DebloatEvidence::new(vec![observed]).unwrap();
        let selected = vec!["appx.fixture".to_string()];
        assert!(matches!(
            assess_debloat(
                &catalogue,
                &evidence,
                DebloatProfile::SafeCleanup,
                &selected
            ),
            Err(DebloatError::UnavailableObservation(_))
        ));
    }

    #[test]
    fn absent_package_is_reported_as_already_absent() {
        let catalogue =
            DebloatCatalogue::new(vec![definition("appx.fixture", "Contoso.Fixture")]).unwrap();
        let mut observed = observation("Contoso.Fixture");
        observed.installed = ObservedPresence::Absent;
        observed.provisioned = ObservedPresence::Absent;
        let evidence = DebloatEvidence::new(vec![observed]).unwrap();
        let selected = vec!["appx.fixture".to_string()];
        let assessment = assess_debloat(
            &catalogue,
            &evidence,
            DebloatProfile::SafeCleanup,
            &selected,
        )
        .unwrap();
        assert_eq!(
            assessment.items[0].disposition,
            DebloatDisposition::AlreadyAbsent
        );
        assert!(!assessment.machine_changes);
    }

    #[test]
    fn safe_optional_with_restore_is_candidate_only() {
        let catalogue =
            DebloatCatalogue::new(vec![definition("appx.fixture", "Contoso.Fixture")]).unwrap();
        let evidence = DebloatEvidence::new(vec![observation("Contoso.Fixture")]).unwrap();
        let selected = vec!["appx.fixture".to_string()];
        let assessment = assess_debloat(
            &catalogue,
            &evidence,
            DebloatProfile::SafeCleanup,
            &selected,
        )
        .unwrap();
        assert_eq!(
            assessment.items[0].disposition,
            DebloatDisposition::RemovalCandidate
        );
        assert!(!assessment.machine_changes);
    }

    #[test]
    fn feature_dependent_item_needs_review() {
        let mut item = definition("appx.fixture", "Contoso.Fixture");
        item.selected_by_default = false;
        item.class = DebloatClass::FeatureDependent;
        item.side_effects = vec!["synthetic feature consequence".to_string()];
        let catalogue = DebloatCatalogue::new(vec![item]).unwrap();
        let evidence = DebloatEvidence::new(vec![observation("Contoso.Fixture")]).unwrap();
        let selected = vec!["appx.fixture".to_string()];
        let assessment = assess_debloat(
            &catalogue,
            &evidence,
            DebloatProfile::SafeCleanup,
            &selected,
        )
        .unwrap();
        assert_eq!(
            assessment.items[0].disposition,
            DebloatDisposition::NeedsReview
        );
    }

    #[test]
    fn dependency_sensitive_item_needs_review() {
        let mut item = definition("appx.fixture", "Contoso.Fixture");
        item.selected_by_default = false;
        item.class = DebloatClass::DependencySensitive;
        item.side_effects = vec!["synthetic dependency consequence".to_string()];
        let catalogue = DebloatCatalogue::new(vec![item]).unwrap();
        let evidence = DebloatEvidence::new(vec![observation("Contoso.Fixture")]).unwrap();
        let selected = vec!["appx.fixture".to_string()];
        let assessment = assess_debloat(
            &catalogue,
            &evidence,
            DebloatProfile::SafeCleanup,
            &selected,
        )
        .unwrap();
        assert_eq!(
            assessment.items[0].disposition,
            DebloatDisposition::NeedsReview
        );
    }

    #[test]
    fn protected_item_is_blocked() {
        let mut item = definition("appx.fixture", "Contoso.Fixture");
        item.selected_by_default = false;
        item.class = DebloatClass::ProtectedManualOnly;
        item.side_effects = vec!["synthetic protected consequence".to_string()];
        item.risk = RiskLevel::High;
        item.recommendation = RecommendationState::DoNotTouch;
        item.restore = RestoreMethod::None;
        let catalogue = DebloatCatalogue::new(vec![item]).unwrap();
        let evidence = DebloatEvidence::new(vec![observation("Contoso.Fixture")]).unwrap();
        let selected = vec!["appx.fixture".to_string()];
        let assessment = assess_debloat(
            &catalogue,
            &evidence,
            DebloatProfile::SafeCleanup,
            &selected,
        )
        .unwrap();
        assert_eq!(
            assessment.items[0].disposition,
            DebloatDisposition::BlockedProtected
        );
    }

    #[test]
    fn profile_preservation_blocks_candidate() {
        let mut item = definition("appx.fixture", "Contoso.Fixture");
        item.preserve_in_profiles.push(DebloatProfile::Technician);
        let catalogue = DebloatCatalogue::new(vec![item]).unwrap();
        let evidence = DebloatEvidence::new(vec![observation("Contoso.Fixture")]).unwrap();
        let selected = vec!["appx.fixture".to_string()];
        let assessment =
            assess_debloat(&catalogue, &evidence, DebloatProfile::Technician, &selected).unwrap();
        assert_eq!(
            assessment.items[0].disposition,
            DebloatDisposition::BlockedByProfile
        );
    }

    #[test]
    fn serde_catalogue_validation_rejects_invalid_default() {
        let json = r#"{
            "items": [{
                "id": "appx.fixture",
                "package_id": "Contoso.Fixture",
                "title": "Fixture",
                "category": "Fixture",
                "description": "Fixture",
                "class": "protected_manual_only",
                "scope": "current_user",
                "risk": "high",
                "recommendation": "do_not_touch",
                "verdict": "certified",
                "selected_by_default": true,
                "restore": {"kind": "none"}
            }]
        }"#;
        assert!(serde_json::from_str::<DebloatCatalogue>(json).is_err());
    }

    #[test]
    fn serde_evidence_validation_rejects_duplicate_package_identity() {
        let json = r#"{
            "observations": [
                {"package_id":"Contoso.Fixture","installed":"present","provisioned":"present","version":"1","source":"fixture"},
                {"package_id":"contoso.fixture","installed":"present","provisioned":"present","version":"1","source":"fixture"}
            ]
        }"#;
        assert!(serde_json::from_str::<DebloatEvidence>(json).is_err());
    }
}
