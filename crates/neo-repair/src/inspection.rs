use crate::error::RepairError;
use crate::host::RepairHost;
use crate::model::{
    RepairHealthInspectionReport, RepairInspectionReport, SupportedWindowsFeature,
    WindowsFeaturesInspectionReport,
};

pub(crate) fn inspect_repair_health_with_host<H: RepairHost>(
    host: &H,
) -> Result<RepairHealthInspectionReport, RepairError> {
    Ok(RepairHealthInspectionReport {
        component_store: host.observe_component_store()?,
        system_files: host.observe_system_files()?,
        machine_changes: false,
    })
}

pub(crate) fn inspect_features_with_host<H: RepairHost>(
    host: &H,
) -> Result<WindowsFeaturesInspectionReport, RepairError> {
    let mut features = Vec::with_capacity(SupportedWindowsFeature::all().len());
    for feature in SupportedWindowsFeature::all().iter().copied() {
        features.push(host.observe_feature(feature)?);
    }
    Ok(WindowsFeaturesInspectionReport {
        features,
        machine_changes: false,
    })
}

pub(crate) fn inspect_with_host<H: RepairHost>(
    host: &H,
) -> Result<RepairInspectionReport, RepairError> {
    let health = inspect_repair_health_with_host(host)?;
    let features = inspect_features_with_host(host)?;
    Ok(RepairInspectionReport {
        component_store: health.component_store,
        system_files: health.system_files,
        features: features.features,
        machine_changes: false,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::host::testsupport::FakeRepairHost;
    use crate::model::{ComponentStoreState, SystemFileState};

    #[test]
    fn inspection_reads_every_fixed_surface_without_execution() {
        let host = FakeRepairHost::new(ComponentStoreState::Healthy, SystemFileState::Healthy);
        let report = inspect_with_host(&host).unwrap();
        assert!(!report.machine_changes);
        assert_eq!(report.features.len(), SupportedWindowsFeature::all().len());
        assert_eq!(host.executed.borrow().len(), 0);
        assert_eq!(
            host.observed.borrow().len(),
            2 + SupportedWindowsFeature::all().len()
        );
    }

    #[test]
    fn health_inspection_does_not_probe_optional_features() {
        let host = FakeRepairHost::new(ComponentStoreState::Healthy, SystemFileState::Healthy);
        let report = inspect_repair_health_with_host(&host).unwrap();
        assert!(!report.machine_changes);
        assert_eq!(
            host.observed.borrow().as_slice(),
            &["component_store".to_string(), "system_files".to_string()]
        );
        assert!(host.executed.borrow().is_empty());
    }

    #[test]
    fn feature_inspection_does_not_probe_component_store_or_sfc() {
        let host = FakeRepairHost::new(ComponentStoreState::Healthy, SystemFileState::Healthy);
        let report = inspect_features_with_host(&host).unwrap();
        assert!(!report.machine_changes);
        assert_eq!(report.features.len(), SupportedWindowsFeature::all().len());
        assert_eq!(
            host.observed.borrow().len(),
            SupportedWindowsFeature::all().len()
        );
        assert!(host
            .observed
            .borrow()
            .iter()
            .all(|entry| entry.starts_with("feature:")));
        assert!(host.executed.borrow().is_empty());
    }
}
