use neo_driverstore::DriverHost;

use crate::{
    DriverRepairAssessment, DriverRepairAssessmentReport, DriverRepairDeviceEvidence,
    DriverRepairError, DriverRepairEvidence, DriverRepairRoute, DriverRepairState,
};

pub(crate) fn capture_and_assess_with_host<H: DriverHost>(
    host: &H,
) -> Result<DriverRepairAssessmentReport, DriverRepairError> {
    let inventory = host.inventory()?;
    let mut devices = Vec::with_capacity(inventory.devices.len());

    for device in inventory.devices {
        let published = device
            .active_driver
            .as_ref()
            .and_then(|binding| binding.published_name.as_deref())
            .map(str::trim)
            .filter(|value| !value.is_empty());
        let current_package = match published {
            Some(value) if value.to_ascii_lowercase().ends_with(".inf") => {
                host.resolve_published_package(value)?
            }
            _ => None,
        };
        devices.push(DriverRepairDeviceEvidence {
            device,
            current_package,
        });
    }

    assess(DriverRepairEvidence { devices })
}

pub(crate) fn assess(
    mut evidence: DriverRepairEvidence,
) -> Result<DriverRepairAssessmentReport, DriverRepairError> {
    evidence.validate()?;
    evidence.devices.sort_by(|left, right| {
        left.device
            .instance_id
            .as_str()
            .to_ascii_lowercase()
            .cmp(&right.device.instance_id.as_str().to_ascii_lowercase())
            .then_with(|| left.device.instance_id.as_str().cmp(right.device.instance_id.as_str()))
    });
    let source_evidence_sha256 = evidence.digest()?;
    let assessments = evidence.devices.iter().map(assess_device).collect();
    Ok(DriverRepairAssessmentReport {
        source_evidence_sha256,
        assessments,
        machine_changes: false,
    })
}

fn assess_device(evidence: &DriverRepairDeviceEvidence) -> DriverRepairAssessment {
    let device = &evidence.device;
    let published = evidence.active_published_inf();
    let binding_present = device.active_driver.is_some();
    let published_valid = published
        .map(|value| value.to_ascii_lowercase().ends_with(".inf"))
        .unwrap_or(false);

    let (state, route, detail) = if device.disabled == Some(true) {
        (
            DriverRepairState::Disabled,
            DriverRepairRoute::ManualInvestigation,
            "The device is reported disabled. Phase 22 records this state but has no enable or re-enumeration authority.".to_string(),
        )
    } else {
        match device.problem_code {
            None => (
                DriverRepairState::EvidenceUnavailable,
                DriverRepairRoute::ManualInvestigation,
                "PnP problem-code evidence is unavailable; Neo will not infer device health or a repair route.".to_string(),
            ),
            Some(0) if !binding_present => (
                DriverRepairState::MissingDriverBinding,
                DriverRepairRoute::DriverSelectionRequired,
                "The device has no active driver binding; a separate matcher/catalogue decision is required before any future mutation authority.".to_string(),
            ),
            Some(0) if !published_valid => (
                DriverRepairState::EvidenceUnavailable,
                DriverRepairRoute::ManualInvestigation,
                "The active binding does not expose a valid published INF identity, so exact Driver Store continuity cannot be proven.".to_string(),
            ),
            Some(0) if evidence.current_package.is_none() => (
                DriverRepairState::EvidenceUnavailable,
                DriverRepairRoute::ManualInvestigation,
                "PnP reports no problem, but the exact active published INF could not be resolved to its Driver Store package; repair readiness is therefore unproven.".to_string(),
            ),
            Some(0) => (
                DriverRepairState::Healthy,
                DriverRepairRoute::NoAction,
                "PnP reports no problem and the active published INF resolves to the exact Driver Store package. Neo recommends no repair.".to_string(),
            ),
            Some(code) if !binding_present => (
                DriverRepairState::MissingDriverBinding,
                DriverRepairRoute::DriverSelectionRequired,
                format!("PnP reports problem code {code} and no active driver binding. Candidate selection must occur through the existing matcher/catalogue authority."),
            ),
            Some(code) if !published_valid => (
                DriverRepairState::EvidenceUnavailable,
                DriverRepairRoute::ManualInvestigation,
                format!("PnP reports problem code {code}, but the active binding lacks a valid published INF identity; an exact repair baseline cannot be proven."),
            ),
            Some(code) if evidence.current_package.is_some() => (
                DriverRepairState::PnpProblem,
                DriverRepairRoute::CurrentExactDriverReinstallCandidate,
                format!("PnP reports problem code {code}. The current published INF and exact Driver Store package are both proven, so a future authority phase may evaluate an exact-current-driver reinstall."),
            ),
            Some(code) => (
                DriverRepairState::PnpProblem,
                DriverRepairRoute::ManualInvestigation,
                format!("PnP reports problem code {code}, but the active published INF cannot be resolved to an exact Driver Store package. Neo will not claim reversible repair readiness."),
            ),
        }
    };

    DriverRepairAssessment {
        instance_id: device.instance_id.to_string(),
        description: device.description.clone(),
        problem_code: device.problem_code,
        disabled: device.disabled,
        active_published_inf: published.map(ToOwned::to_owned),
        exact_driver_store_package: evidence.current_package.clone(),
        upper_filters: device.upper_filters.clone(),
        lower_filters: device.lower_filters.clone(),
        state,
        route,
        detail,
    }
}
