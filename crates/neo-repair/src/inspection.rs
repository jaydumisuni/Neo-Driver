use crate::error::RepairError;
use crate::host::RepairHost;
use crate::model::{RepairInspectionReport, SupportedWindowsFeature};

pub(crate) fn inspect_with_host<H: RepairHost>(
    host: &H,
) -> Result<RepairInspectionReport, RepairError> {
    let component_store = host.observe_component_store()?;
    let system_files = host.observe_system_files()?;
    let mut features = Vec::with_capacity(SupportedWindowsFeature::all().len());
    for feature in SupportedWindowsFeature::all().iter().copied() {
        features.push(host.observe_feature(feature)?);
    }
    Ok(RepairInspectionReport {
        component_store,
        system_files,
        features,
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
}
