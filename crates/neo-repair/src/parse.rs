use crate::model::{
    BoundedCommandEvidence, ComponentStoreObservation, ComponentStoreState,
    SupportedWindowsFeature, SystemFileObservation, SystemFileState, WindowsFeatureObservation,
    WindowsFeatureState,
};

const ELEVATION_EXIT_CODE: i32 = 740;

pub(crate) fn component_store_observation(
    evidence: BoundedCommandEvidence,
) -> ComponentStoreObservation {
    if let Some(reason) = unavailable_reason(&evidence) {
        return ComponentStoreObservation {
            state: ComponentStoreState::Unavailable,
            detail: reason,
            evidence,
        };
    }
    let text = normalized_text(&evidence);
    let (state, detail) = if text.contains("no component store corruption detected") {
        (
            ComponentStoreState::Healthy,
            "DISM reports no component store corruption.".to_string(),
        )
    } else if text.contains("component store is repairable") {
        (
            ComponentStoreState::Repairable,
            "DISM reports component store corruption that is repairable.".to_string(),
        )
    } else if text.contains("component store cannot be repaired")
        || text.contains("component store is non-repairable")
    {
        (
            ComponentStoreState::Unrepairable,
            "DISM reports component store corruption that is not repairable by this route."
                .to_string(),
        )
    } else {
        (
            ComponentStoreState::Unavailable,
            "DISM output did not match a frozen Phase 21 component-store state.".to_string(),
        )
    };
    ComponentStoreObservation {
        state,
        detail,
        evidence,
    }
}

pub(crate) fn system_file_observation(evidence: BoundedCommandEvidence) -> SystemFileObservation {
    if let Some(reason) = unavailable_reason(&evidence) {
        return SystemFileObservation {
            state: SystemFileState::Unavailable,
            detail: reason,
            evidence,
        };
    }
    let text = normalized_text(&evidence);
    let (state, detail) = if text
        .contains("windows resource protection did not find any integrity violations")
    {
        (
            SystemFileState::Healthy,
            "SFC reports no protected-system-file integrity violations.".to_string(),
        )
    } else if text.contains("windows resource protection found integrity violations")
        || text.contains("windows resource protection found corrupt files")
    {
        (
            SystemFileState::IntegrityViolations,
            "SFC reports protected-system-file integrity violations.".to_string(),
        )
    } else if text.contains("windows resource protection could not perform the requested operation")
    {
        (
            SystemFileState::Unavailable,
            "SFC could not perform the requested verification operation.".to_string(),
        )
    } else {
        (
            SystemFileState::Unavailable,
            "SFC output did not match a frozen Phase 21 system-file state.".to_string(),
        )
    };
    SystemFileObservation {
        state,
        detail,
        evidence,
    }
}

pub(crate) fn feature_observation(
    feature: SupportedWindowsFeature,
    evidence: BoundedCommandEvidence,
) -> WindowsFeatureObservation {
    if let Some(reason) = unavailable_reason(&evidence) {
        return WindowsFeatureObservation {
            feature,
            state: WindowsFeatureState::Unavailable,
            detail: reason,
            evidence,
        };
    }
    let text = normalized_text(&evidence);
    if text.contains("0x800f080c") || text.contains("feature name") && text.contains("is unknown") {
        return WindowsFeatureObservation {
            feature,
            state: WindowsFeatureState::Unavailable,
            detail: "The feature is not available on this Windows edition/build.".to_string(),
            evidence,
        };
    }
    let state = parse_feature_state(&text);
    let detail = match state {
        WindowsFeatureState::Enabled => "Windows reports the feature as enabled.",
        WindowsFeatureState::Disabled => "Windows reports the feature as disabled.",
        WindowsFeatureState::EnablePending => {
            "Windows reports feature enablement pending a servicing transition/reboot."
        }
        WindowsFeatureState::DisablePending => {
            "Windows reports feature disablement pending a servicing transition/reboot."
        }
        WindowsFeatureState::Removed => {
            "Windows reports the feature payload as removed; normal reversible Phase 21 mutation is blocked."
        }
        WindowsFeatureState::Unavailable => {
            "DISM output did not match a frozen Phase 21 feature state."
        }
    };
    WindowsFeatureObservation {
        feature,
        state,
        detail: detail.to_string(),
        evidence,
    }
}

fn parse_feature_state(text: &str) -> WindowsFeatureState {
    if contains_state(text, "disabled with payload removed") || contains_state(text, "removed") {
        WindowsFeatureState::Removed
    } else if contains_state(text, "enable pending") || contains_state(text, "enablepending") {
        WindowsFeatureState::EnablePending
    } else if contains_state(text, "disable pending") || contains_state(text, "disablepending") {
        WindowsFeatureState::DisablePending
    } else if contains_state(text, "enabled") {
        WindowsFeatureState::Enabled
    } else if contains_state(text, "disabled") {
        WindowsFeatureState::Disabled
    } else {
        WindowsFeatureState::Unavailable
    }
}

fn contains_state(text: &str, expected: &str) -> bool {
    text.lines().any(|line| {
        let line = line.trim();
        line.strip_prefix("state")
            .and_then(|rest| rest.trim_start().strip_prefix(':'))
            .is_some_and(|value| value.trim() == expected)
    })
}

fn unavailable_reason(evidence: &BoundedCommandEvidence) -> Option<String> {
    if evidence.truncated() {
        return Some("Windows command output exceeded the Phase 21 evidence bound.".to_string());
    }
    if let Some(error) = &evidence.start_error {
        return Some(format!("Windows command could not start: {error}"));
    }
    let text = normalized_text(evidence);
    if evidence.exit_code == Some(ELEVATION_EXIT_CODE)
        || text.contains("elevated permissions are required")
        || text.contains("you must be an administrator running a console session")
    {
        return Some("Elevated Windows servicing read authority is required.".to_string());
    }
    if evidence.exit_code.is_none() {
        return Some("Windows command exit status is unavailable.".to_string());
    }
    if evidence.exit_code != Some(0) {
        return Some(format!(
            "Windows command failed with exit code {}.",
            evidence.exit_code.unwrap_or_default()
        ));
    }
    None
}

fn normalized_text(evidence: &BoundedCommandEvidence) -> String {
    // SFC emits UTF-16LE console text on supported Windows builds. The shared
    // command evidence boundary preserves those bytes as NUL-separated scalar
    // text, so remove NUL separators before deterministic English-token parsing.
    evidence.combined_text().replace('\0', "").to_ascii_lowercase()
}

#[cfg(test)]
mod tests {
    use super::*;
    use neo_probe::CommandEvidence;

    fn evidence(exit_code: i32, stdout: &str) -> BoundedCommandEvidence {
        BoundedCommandEvidence::from_command(CommandEvidence {
            program: r"C:\Windows\System32\dism.exe".to_string(),
            args: Vec::new(),
            exit_code: Some(exit_code),
            stdout: stdout.to_string(),
            stderr: String::new(),
            start_error: None,
        })
    }

    #[test]
    fn dism_health_states_are_fail_closed() {
        assert_eq!(
            component_store_observation(evidence(0, "No component store corruption detected."))
                .state,
            ComponentStoreState::Healthy
        );
        assert_eq!(
            component_store_observation(evidence(0, "The component store is repairable.")).state,
            ComponentStoreState::Repairable
        );
        assert_eq!(
            component_store_observation(evidence(0, "unexpected success text")).state,
            ComponentStoreState::Unavailable
        );
    }

    #[test]
    fn elevation_failure_never_becomes_a_state_claim() {
        let observed = component_store_observation(evidence(
            740,
            "Elevated permissions are required to run DISM.",
        ));
        assert_eq!(observed.state, ComponentStoreState::Unavailable);
        assert!(observed.detail.to_ascii_lowercase().contains("elevated"));
    }

    #[test]
    fn nul_separated_sfc_admin_failure_is_elevation_required() {
        let text = "Y\0o\0u\0 \0m\0u\0s\0t\0 \0b\0e\0 \0a\0n\0 \0a\0d\0m\0i\0n\0i\0s\0t\0r\0a\0t\0o\0r\0 \0r\0u\0n\0n\0i\0n\0g\0 \0a\0 \0c\0o\0n\0s\0o\0l\0e\0 \0s\0e\0s\0s\0i\0o\0n\0 \0i\0n\0 \0o\0r\0d\0e\0r\0 \0t\0o\0 \0u\0s\0e\0 \0t\0h\0e\0 \0S\0F\0C\0 \0u\0t\0i\0l\0i\0t\0y\0.\0";
        let observed = system_file_observation(evidence(1, text));
        assert_eq!(observed.state, SystemFileState::Unavailable);
        assert!(observed.detail.to_ascii_lowercase().contains("elevated"));
    }

    #[test]
    fn sfc_verifyonly_states_are_distinct() {
        assert_eq!(
            system_file_observation(evidence(
                0,
                "Windows Resource Protection did not find any integrity violations."
            ))
            .state,
            SystemFileState::Healthy
        );
        assert_eq!(
            system_file_observation(evidence(
                0,
                "Windows Resource Protection found integrity violations."
            ))
            .state,
            SystemFileState::IntegrityViolations
        );
    }

    #[test]
    fn feature_states_require_explicit_state_line() {
        let feature = SupportedWindowsFeature::NetFx3;
        assert_eq!(
            feature_observation(feature, evidence(0, "State : Enabled\r\n")).state,
            WindowsFeatureState::Enabled
        );
        assert_eq!(
            feature_observation(feature, evidence(0, "State : Enable Pending\r\n")).state,
            WindowsFeatureState::EnablePending
        );
        assert_eq!(
            feature_observation(feature, evidence(0, "Operation completed successfully.")).state,
            WindowsFeatureState::Unavailable
        );
    }
}
