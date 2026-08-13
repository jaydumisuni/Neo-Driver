use neo_probe::{CommandEvidence, CommandRunner};
use neo_state_plan::{
    CapturedState, CapturedStates, ObservedState, ReaderId, RegistryHive, RegistryValueKind,
    RegistryView, StatePlanError, TweakValue, WindowsReadSource, WindowsReaderSources,
};

pub fn capture_with_runner<R: CommandRunner>(
    sources: &WindowsReaderSources,
    runner: &R,
) -> Result<CapturedStates, StatePlanError> {
    sources.validate()?;
    let mut values = Vec::with_capacity(sources.readers.len());
    for item in &sources.readers {
        values.push(CapturedState {
            reader: ReaderId::new(item.reader.as_str())?,
            state: capture_source(&item.source, runner),
            source: format!("reader:{}", item.reader.as_str()),
        });
    }
    CapturedStates::new(values)
}

fn capture_source<R: CommandRunner>(source: &WindowsReadSource, runner: &R) -> ObservedState {
    match source {
        WindowsReadSource::RegistryValue {
            hive,
            subkey,
            value_name,
            value_kind,
            view,
        } => capture_registry(*hive, subkey, value_name, *value_kind, *view, runner),
        WindowsReadSource::ServiceStartType { service_name } => {
            capture_service_start(service_name, runner)
        }
        WindowsReadSource::ServiceState { service_name } => capture_service_state(service_name, runner),
        WindowsReadSource::OptionalFeature { feature_name } => {
            capture_feature(feature_name, runner)
        }
        WindowsReadSource::AppxCurrentUser {
            package_family_name,
        } => capture_appx(package_family_name, runner),
    }
}

fn capture_registry<R: CommandRunner>(
    hive: RegistryHive,
    subkey: &str,
    value_name: &str,
    value_kind: RegistryValueKind,
    view: RegistryView,
    runner: &R,
) -> ObservedState {
    let root = match hive {
        RegistryHive::LocalMachine => "HKLM",
        RegistryHive::CurrentUser => "HKCU",
    };
    let key = format!(r"{root}\{subkey}");
    let view_arg = match view {
        RegistryView::Default => None,
        RegistryView::Registry32 => Some("/reg:32"),
        RegistryView::Registry64 => Some("/reg:64"),
    };

    let mut value_args = vec!["query", key.as_str()];
    if value_name.is_empty() {
        value_args.push("/ve");
    } else {
        value_args.extend(["/v", value_name]);
    }
    if let Some(view_arg) = view_arg {
        value_args.push(view_arg);
    }
    let value_evidence = run_evidence(runner, "reg.exe", &value_args);
    if value_evidence.succeeded() {
        return parse_registry_value(&value_evidence.stdout, value_name, value_kind)
            .unwrap_or_else(unparseable);
    }

    let mut key_args = vec!["query", key.as_str()];
    if let Some(view_arg) = view_arg {
        key_args.push(view_arg);
    }
    let key_evidence = run_evidence(runner, "reg.exe", &key_args);
    if key_evidence.succeeded() {
        ObservedState::Absent
    } else {
        unavailable("registry read did not complete successfully")
    }
}

fn capture_service_start<R: CommandRunner>(service_name: &str, runner: &R) -> ObservedState {
    let key = format!(r"HKLM\SYSTEM\CurrentControlSet\Services\{service_name}");
    let evidence = run_evidence(runner, "reg.exe", &["query", key.as_str(), "/v", "Start"]);
    if !evidence.succeeded() {
        return unavailable("service start type was not readable");
    }
    match parse_registry_value(&evidence.stdout, "Start", RegistryValueKind::U32) {
        Some(ObservedState::Present {
            value: TweakValue::U32(value),
        }) => {
            let label = match value {
                0 => "boot",
                1 => "system",
                2 => "automatic",
                3 => "manual",
                4 => "disabled",
                _ => return unavailable("service start type was outside the documented range"),
            };
            ObservedState::Present {
                value: TweakValue::Text(label.to_string()),
            }
        }
        _ => unparseable(),
    }
}

fn capture_service_state<R: CommandRunner>(service_name: &str, runner: &R) -> ObservedState {
    let evidence = run_evidence(runner, "sc.exe", &["query", service_name]);
    if !evidence.succeeded() {
        return unavailable("service state was not readable");
    }
    let state = evidence.stdout.lines().find_map(|line| {
        let upper = line.to_ascii_uppercase();
        let marker = upper.find("STATE")?;
        let tail = &line[marker + "STATE".len()..];
        let (_, tail) = tail.split_once(':')?;
        tail.split_whitespace().next()?.parse::<u32>().ok()
    });
    let label = match state {
        Some(1) => "stopped",
        Some(2) => "start_pending",
        Some(3) => "stop_pending",
        Some(4) => "running",
        Some(5) => "continue_pending",
        Some(6) => "pause_pending",
        Some(7) => "paused",
        _ => return unparseable(),
    };
    ObservedState::Present {
        value: TweakValue::Text(label.to_string()),
    }
}

fn capture_feature<R: CommandRunner>(feature_name: &str, runner: &R) -> ObservedState {
    let feature_arg = format!("/FeatureName:{feature_name}");
    let evidence = run_evidence(
        runner,
        "dism.exe",
        &["/Online", "/English", "/Get-FeatureInfo", feature_arg.as_str()],
    );
    if !evidence.succeeded() {
        return unavailable("optional feature state was not readable");
    }
    let state = evidence.stdout.lines().find_map(|line| {
        let (label, value) = line.split_once(':')?;
        if label.trim().eq_ignore_ascii_case("State") {
            Some(value.trim())
        } else {
            None
        }
    });
    match state {
        Some(value) if !value.is_empty() => ObservedState::Present {
            value: TweakValue::Text(normalize_label(value)),
        },
        _ => unparseable(),
    }
}

fn capture_appx<R: CommandRunner>(package_family_name: &str, runner: &R) -> ObservedState {
    const SCRIPT: &str =
        "Get-AppxPackage | Select-Object -ExpandProperty PackageFamilyName";
    let evidence = run_evidence(
        runner,
        "powershell.exe",
        &["-NoProfile", "-NonInteractive", "-Command", SCRIPT],
    );
    if !evidence.succeeded() {
        return unavailable("current-user package state was not readable");
    }
    let present = evidence
        .stdout
        .lines()
        .map(str::trim)
        .any(|line| line.eq_ignore_ascii_case(package_family_name));
    if present {
        ObservedState::Present {
            value: TweakValue::Text(package_family_name.to_string()),
        }
    } else {
        ObservedState::Absent
    }
}

fn parse_registry_value(
    stdout: &str,
    value_name: &str,
    kind: RegistryValueKind,
) -> Option<ObservedState> {
    for line in stdout.lines() {
        let trimmed = line.trim();
        let remaining = if value_name.is_empty() {
            trimmed.strip_prefix("(Default)")?
        } else if trimmed.len() >= value_name.len()
            && trimmed[..value_name.len()].eq_ignore_ascii_case(value_name)
        {
            &trimmed[value_name.len()..]
        } else {
            continue;
        };
        let remaining = remaining.trim_start();
        let (type_name, raw) = split_type_and_value(remaining)?;
        let value = match kind {
            RegistryValueKind::Text
                if type_name.eq_ignore_ascii_case("REG_SZ")
                    || type_name.eq_ignore_ascii_case("REG_EXPAND_SZ") =>
            {
                TweakValue::Text(raw.to_string())
            }
            RegistryValueKind::U32 if type_name.eq_ignore_ascii_case("REG_DWORD") => {
                TweakValue::U32(parse_integer(raw)? as u32)
            }
            RegistryValueKind::U64 if type_name.eq_ignore_ascii_case("REG_QWORD") => {
                TweakValue::U64(parse_integer(raw)?)
            }
            _ => return None,
        };
        return Some(ObservedState::Present { value });
    }
    None
}

fn split_type_and_value(input: &str) -> Option<(&str, &str)> {
    let type_end = input.find(char::is_whitespace)?;
    let type_name = &input[..type_end];
    let raw = input[type_end..].trim_start();
    Some((type_name, raw))
}

fn parse_integer(value: &str) -> Option<u64> {
    let value = value.trim();
    if let Some(hex) = value.strip_prefix("0x").or_else(|| value.strip_prefix("0X")) {
        u64::from_str_radix(hex, 16).ok()
    } else {
        value.parse().ok()
    }
}

fn normalize_label(value: &str) -> String {
    value
        .trim()
        .to_ascii_lowercase()
        .chars()
        .map(|character| if character.is_ascii_alphanumeric() { character } else { '_' })
        .collect::<String>()
        .split('_')
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>()
        .join("_")
}

fn run_evidence<R: CommandRunner>(runner: &R, program: &str, args: &[&str]) -> CommandEvidence {
    match runner.run(program, args) {
        Ok(evidence) => evidence,
        Err(error) => CommandEvidence::failed_to_start(program, args, &error),
    }
}

fn unavailable(reason: &str) -> ObservedState {
    ObservedState::Unavailable {
        reason: reason.to_string(),
    }
}

fn unparseable() -> ObservedState {
    unavailable("read-only command output could not be normalized")
}
