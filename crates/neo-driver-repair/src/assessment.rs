#[cfg(any(windows, test))]
use neo_driverstore::DriverHost;

use crate::model::CM_PROB_DISABLED_CODE;
use crate::{
    DriverRepairAssessment, DriverRepairAssessmentReport, DriverRepairDeviceEvidence,
    DriverRepairError, DriverRepairEvidence, DriverRepairRoute, DriverRepairState,
    PnpStatusEvidence,
};

#[cfg(any(windows, test))]
pub(crate) fn capture_and_assess_with_host<H: DriverHost>(
    host: &H,
) -> Result<DriverRepairAssessmentReport, DriverRepairError> {
    let inventory = host.inventory()?;
    let mut devices = Vec::with_capacity(inventory.devices.len());

    for device in inventory.devices {
        let pnp_status = PnpStatusEvidence::from_device(&device)?;
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
            pnp_status,
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
            .then_with(|| {
                left.device
                    .instance_id
                    .as_str()
                    .cmp(right.device.instance_id.as_str())
            })
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
    let disabled = device.disabled == Some(true)
        || matches!(
            evidence.pnp_status,
            PnpStatusEvidence::Problem {
                code: CM_PROB_DISABLED_CODE
            }
        );

    let (state, route, detail) = if disabled {
        (
            DriverRepairState::Disabled,
            DriverRepairRoute::ManualInvestigation,
            "Windows reports the device disabled. Phase 22 records this state but has no enable or re-enumeration authority.".to_string(),
        )
    } else {
        match evidence.pnp_status {
            PnpStatusEvidence::NoProblem if !binding_present => (
                DriverRepairState::EvidenceUnavailable,
                DriverRepairRoute::ManualInvestigation,
                "PnP reports no device problem, but no active driver binding exists. Neo will not infer that driver selection or repair is required without an actual PnP problem.".to_string(),
            ),
            PnpStatusEvidence::NoProblem if !published_valid => (
                DriverRepairState::EvidenceUnavailable,
                DriverRepairRoute::ManualInvestigation,
                "PnP reports no device problem, but the active binding does not expose a valid published INF identity, so exact Driver Store continuity cannot be proven.".to_string(),
            ),
            PnpStatusEvidence::NoProblem if evidence.current_package.is_none() => (
                DriverRepairState::EvidenceUnavailable,
                DriverRepairRoute::ManualInvestigation,
                "PnP reports no device problem, but the exact active published INF could not be resolved to its Driver Store package; repair readiness is therefore unproven.".to_string(),
            ),
            PnpStatusEvidence::NoProblem => (
                DriverRepairState::Healthy,
                DriverRepairRoute::NoAction,
                "PnP reports no device problem and the active published INF resolves to the exact Driver Store package. Neo recommends no repair.".to_string(),
            ),
            PnpStatusEvidence::Problem { code } if !binding_present => (
                DriverRepairState::MissingDriverBinding,
                DriverRepairRoute::DriverSelectionRequired,
                format!("PnP reports problem code {code} and no active driver binding. Candidate selection must occur through the existing matcher/catalogue authority."),
            ),
            PnpStatusEvidence::Problem { code } if !published_valid => (
                DriverRepairState::EvidenceUnavailable,
                DriverRepairRoute::ManualInvestigation,
                format!("PnP reports problem code {code}, but the active binding lacks a valid published INF identity; an exact repair baseline cannot be proven."),
            ),
            PnpStatusEvidence::Problem { code } if evidence.current_package.is_some() => (
                DriverRepairState::PnpProblem,
                DriverRepairRoute::CurrentExactDriverReinstallCandidate,
                format!("PnP reports problem code {code}. The current published INF and exact Driver Store package are both proven, so a future authority phase may evaluate an exact-current-driver reinstall."),
            ),
            PnpStatusEvidence::Problem { code } => (
                DriverRepairState::PnpProblem,
                DriverRepairRoute::ManualInvestigation,
                format!("PnP reports problem code {code}, but the active published INF cannot be resolved to an exact Driver Store package. Neo will not claim reversible repair readiness."),
            ),
        }
    };

    DriverRepairAssessment {
        instance_id: device.instance_id.to_string(),
        description: device.description.clone(),
        pnp_status: evidence.pnp_status,
        problem_code: evidence.pnp_status.problem_code(),
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
