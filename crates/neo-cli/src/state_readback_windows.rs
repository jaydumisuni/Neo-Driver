use neo_probe::scan_current_machine;
use neo_state_plan::{
    CapturedState, CapturedStates, ObservedState, ReaderId, StateBindings, StatePlanError,
    TweakValue,
};
use std::collections::BTreeSet;

pub fn capture_live(bindings: &StateBindings) -> Result<CapturedStates, StatePlanError> {
    bindings.validate()?;
    let report = scan_current_machine().map_err(|_| StatePlanError::UnavailableObservation {
        tweak_id: "windows.readback".to_string(),
        reason: "Windows System X-Ray did not complete".to_string(),
    })?;

    let mut seen = BTreeSet::new();
    let mut values = Vec::new();
    for binding in &bindings.bindings {
        if seen.insert(binding.reader.clone()) {
            values.push(CapturedState {
                reader: ReaderId::new(binding.reader.as_str())?,
                state: state_for_reader(&binding.reader, &report.profile),
                source: format!("neo-probe:{}", binding.reader.as_str()),
            });
        }
    }
    CapturedStates::new(values)
}

fn state_for_reader(reader: &ReaderId, profile: &neo_core::MachineProfile) -> ObservedState {
    match reader.as_str() {
        "windows.os.product_name" => text_state(profile.os.product_name.as_deref()),
        "windows.os.display_version" => text_state(profile.os.display_version.as_deref()),
        "windows.os.current_build" => text_state(profile.os.build_number.as_deref()),
        "windows.os.architecture" => text_state(profile.os.architecture.as_deref()),
        "windows.security.test_signing" => bool_state(profile.security.test_signing),
        "windows.security.no_integrity_checks" => {
            bool_state(profile.security.no_integrity_checks)
        }
        "windows.security.secure_boot" => bool_state(profile.security.secure_boot),
        "windows.security.memory_integrity" => bool_state(profile.security.memory_integrity),
        "windows.security.pending_reboot" => bool_state(profile.security.pending_reboot),
        _ => unavailable("reader is not registered in the Windows readback catalogue"),
    }
}

fn text_state(value: Option<&str>) -> ObservedState {
    match value {
        Some(value) if !value.trim().is_empty() => ObservedState::Present {
            value: TweakValue::Text(value.to_string()),
        },
        _ => unavailable("requested Windows state was not available"),
    }
}

fn bool_state(value: Option<bool>) -> ObservedState {
    match value {
        Some(value) => ObservedState::Present {
            value: TweakValue::U32(u32::from(value)),
        },
        None => unavailable("requested Windows state was not available"),
    }
}

fn unavailable(reason: &str) -> ObservedState {
    ObservedState::Unavailable {
        reason: reason.to_string(),
    }
}
