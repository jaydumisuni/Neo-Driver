use crate::{FeatureDesiredState, RepairOperation, SupportedWindowsFeature, WindowsFeatureState};

#[test]
fn feature_identity_is_closed_over_the_frozen_catalogue() {
    assert!(SupportedWindowsFeature::parse_id("NetFx3").is_none());
    assert!(SupportedWindowsFeature::parse_id("netfx3").is_some());
    assert!(SupportedWindowsFeature::parse_id("telnet_client").is_none());
    assert_eq!(SupportedWindowsFeature::all().len(), 6);
}

#[test]
fn feature_operation_contains_typed_identity_not_raw_command_text() {
    let operation = RepairOperation::SetWindowsFeature {
        feature: SupportedWindowsFeature::VirtualMachinePlatform,
        desired: FeatureDesiredState::Enabled,
    };
    let json = serde_json::to_string(&operation).unwrap();
    assert!(json.contains("virtual_machine_platform"));
    assert!(!json.contains("/Enable-Feature"));
    assert!(!json.contains("dism.exe"));
}

#[test]
fn pending_states_are_not_stable_transaction_baselines() {
    assert!(!WindowsFeatureState::EnablePending.is_stable());
    assert!(!WindowsFeatureState::DisablePending.is_stable());
    assert!(!WindowsFeatureState::Removed.is_stable());
    assert!(!WindowsFeatureState::Unavailable.is_stable());
}
