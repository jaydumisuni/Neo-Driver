#!/usr/bin/env python3
from __future__ import annotations

from pathlib import Path


def replace_once(path: str, old: str, new: str) -> None:
    target = Path(path)
    text = target.read_text(encoding="utf-8")
    count = text.count(old)
    if count != 1:
        raise SystemExit(f"{path}: expected one replacement, found {count}")
    target.write_text(text.replace(old, new), encoding="utf-8")


# 1. Centralize all executable Windows servicing argv behind a typed authority.
command_path = Path("crates/neo-repair/src/command.rs")
if command_path.exists():
    raise SystemExit("command.rs already exists; refusing ambiguous patch")
command_path.write_text(
    r'''use crate::model::{FeatureDesiredState, SupportedWindowsFeature};
use crate::operation::RepairOperation;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum TrustedProgram {
    Dism,
    Sfc,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct TrustedCommand {
    pub(crate) program: TrustedProgram,
    pub(crate) args: Vec<String>,
}

fn args(values: &[&str]) -> Vec<String> {
    values.iter().map(|value| (*value).to_string()).collect()
}

pub(crate) fn component_store_inspection_command() -> TrustedCommand {
    TrustedCommand {
        program: TrustedProgram::Dism,
        args: args(&["/Online", "/Cleanup-Image", "/CheckHealth", "/English"]),
    }
}

pub(crate) fn system_files_inspection_command() -> TrustedCommand {
    TrustedCommand {
        program: TrustedProgram::Sfc,
        args: args(&["/verifyonly"]),
    }
}

pub(crate) fn feature_inspection_command(feature: SupportedWindowsFeature) -> TrustedCommand {
    TrustedCommand {
        program: TrustedProgram::Dism,
        args: vec![
            "/Online".to_string(),
            "/Get-FeatureInfo".to_string(),
            format!("/FeatureName:{}", feature.dism_name()),
            "/English".to_string(),
        ],
    }
}

pub(crate) fn operation_command(operation: RepairOperation) -> TrustedCommand {
    match operation {
        RepairOperation::RestoreComponentStore => TrustedCommand {
            program: TrustedProgram::Dism,
            args: args(&[
                "/Online",
                "/NoRestart",
                "/Cleanup-Image",
                "/RestoreHealth",
                "/English",
            ]),
        },
        RepairOperation::RepairSystemFiles => TrustedCommand {
            program: TrustedProgram::Sfc,
            args: args(&["/scannow"]),
        },
        RepairOperation::SetWindowsFeature { feature, desired } => TrustedCommand {
            program: TrustedProgram::Dism,
            args: vec![
                "/Online".to_string(),
                "/NoRestart".to_string(),
                match desired {
                    FeatureDesiredState::Enabled => "/Enable-Feature".to_string(),
                    FeatureDesiredState::Disabled => "/Disable-Feature".to_string(),
                },
                format!("/FeatureName:{}", feature.dism_name()),
                "/English".to_string(),
            ],
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn exact(program: TrustedProgram, values: &[&str]) -> TrustedCommand {
        TrustedCommand {
            program,
            args: args(values),
        }
    }

    #[test]
    fn trusted_command_contract_is_exact() {
        assert_eq!(
            component_store_inspection_command(),
            exact(
                TrustedProgram::Dism,
                &["/Online", "/Cleanup-Image", "/CheckHealth", "/English"],
            )
        );
        assert_eq!(
            system_files_inspection_command(),
            exact(TrustedProgram::Sfc, &["/verifyonly"])
        );
        assert_eq!(
            operation_command(RepairOperation::RestoreComponentStore),
            exact(
                TrustedProgram::Dism,
                &[
                    "/Online",
                    "/NoRestart",
                    "/Cleanup-Image",
                    "/RestoreHealth",
                    "/English",
                ],
            )
        );
        assert_eq!(
            operation_command(RepairOperation::RepairSystemFiles),
            exact(TrustedProgram::Sfc, &["/scannow"])
        );

        assert_eq!(
            SupportedWindowsFeature::all(),
            &[
                SupportedWindowsFeature::NetFx3,
                SupportedWindowsFeature::DirectPlay,
                SupportedWindowsFeature::HyperV,
                SupportedWindowsFeature::WindowsSubsystemLinux,
                SupportedWindowsFeature::VirtualMachinePlatform,
                SupportedWindowsFeature::WindowsSandbox,
            ]
        );

        for feature in SupportedWindowsFeature::all().iter().copied() {
            assert_eq!(
                feature_inspection_command(feature),
                TrustedCommand {
                    program: TrustedProgram::Dism,
                    args: vec![
                        "/Online".to_string(),
                        "/Get-FeatureInfo".to_string(),
                        format!("/FeatureName:{}", feature.dism_name()),
                        "/English".to_string(),
                    ],
                }
            );
            for desired in [FeatureDesiredState::Enabled, FeatureDesiredState::Disabled] {
                let command = operation_command(RepairOperation::SetWindowsFeature {
                    feature,
                    desired,
                });
                assert_eq!(command.program, TrustedProgram::Dism);
                assert_eq!(
                    command.args,
                    vec![
                        "/Online".to_string(),
                        "/NoRestart".to_string(),
                        match desired {
                            FeatureDesiredState::Enabled => "/Enable-Feature".to_string(),
                            FeatureDesiredState::Disabled => "/Disable-Feature".to_string(),
                        },
                        format!("/FeatureName:{}", feature.dism_name()),
                        "/English".to_string(),
                    ]
                );
                assert!(!command.args.iter().any(|arg| {
                    matches!(arg.as_str(), "/Remove" | "/Source" | "/LimitAccess")
                }));
            }
        }
    }
}
''',
    encoding="utf-8",
)

replace_once(
    "crates/neo-repair/src/lib.rs",
    "mod error;\n#[cfg(any(windows, test))]\nmod executor;",
    "#[cfg(any(windows, test))]\nmod command;\nmod error;\n#[cfg(any(windows, test))]\nmod executor;",
)

host = Path("crates/neo-repair/src/host.rs")
host_text = host.read_text(encoding="utf-8")
import_anchor = "use crate::error::RepairError;\n"
command_import = '''#[cfg(windows)]
use crate::command::{
    component_store_inspection_command, feature_inspection_command, operation_command,
    system_files_inspection_command, TrustedCommand, TrustedProgram,
};
'''
if host_text.count(import_anchor) != 1:
    raise SystemExit("host.rs import anchor changed")
host_text = host_text.replace(import_anchor, command_import + import_anchor, 1)
start = host_text.index("#[cfg(windows)]\nimpl WindowsRepairHost {")
end = host_text.index("\n#[cfg(windows)]\nfn path_text", start)
new_host_impl = r'''#[cfg(windows)]
impl WindowsRepairHost {
    pub(crate) fn new() -> Result<Self, RepairError> {
        let windows = trusted_windows_directory()?;
        let system32 = windows.join("System32");
        let dism = system32.join("dism.exe");
        let sfc = system32.join("sfc.exe");
        Ok(Self {
            runner: SystemCommandRunner,
            dism: path_text(&dism)?,
            sfc: path_text(&sfc)?,
        })
    }

    fn capture(&self, program: &str, args: &[&str]) -> BoundedCommandEvidence {
        let evidence = match self.runner.run(program, args) {
            Ok(value) => value,
            Err(error) => CommandEvidence::failed_to_start(program, args, &error),
        };
        BoundedCommandEvidence::from_command(evidence)
    }

    fn capture_trusted(&self, command: TrustedCommand) -> BoundedCommandEvidence {
        let program = match command.program {
            TrustedProgram::Dism => &self.dism,
            TrustedProgram::Sfc => &self.sfc,
        };
        let args: Vec<&str> = command.args.iter().map(String::as_str).collect();
        self.capture(program, &args)
    }

    fn feature_info(&self, feature: SupportedWindowsFeature) -> WindowsFeatureObservation {
        feature_observation(
            feature,
            self.capture_trusted(feature_inspection_command(feature)),
        )
    }
}

#[cfg(windows)]
impl RepairHost for WindowsRepairHost {
    fn observe_component_store(&self) -> Result<ComponentStoreObservation, RepairError> {
        Ok(component_store_observation(
            self.capture_trusted(component_store_inspection_command()),
        ))
    }

    fn observe_system_files(&self) -> Result<SystemFileObservation, RepairError> {
        Ok(system_file_observation(
            self.capture_trusted(system_files_inspection_command()),
        ))
    }

    fn observe_feature(
        &self,
        feature: SupportedWindowsFeature,
    ) -> Result<WindowsFeatureObservation, RepairError> {
        Ok(self.feature_info(feature))
    }

    fn execute(&self, operation: RepairOperation) -> Result<BoundedCommandEvidence, RepairError> {
        Ok(self.capture_trusted(operation_command(operation)))
    }
}
'''
host.write_text(host_text[:start] + new_host_impl + host_text[end:], encoding="utf-8")

# 2. Preserve primary execution truth if post-mutation persistence also fails.
rpc = Path("crates/neo-repair/src/rpc.rs")
rpc_text = rpc.read_text(encoding="utf-8")
payload_old = '''pub struct RepairRpcErrorPayload {
    pub schema: String,
    pub code: String,
    pub message: String,
    pub retryable: bool,
}'''
payload_new = '''pub struct RepairRpcErrorPayload {
    pub schema: String,
    pub code: String,
    pub message: String,
    pub retryable: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub resume_version: Option<u64>,
}'''
if rpc_text.count(payload_old) != 1:
    raise SystemExit("rpc payload anchor changed")
rpc_text = rpc_text.replace(payload_old, payload_new, 1)

enum_old = '''    #[error("Phase 21 repair service failure")]
    Repair(#[from] RepairError),'''
enum_new = '''    #[error("Phase 21 repair service failure with durable resume state")]
    RepairWithResume {
        #[source]
        source: RepairError,
        persistence: Option<RepairError>,
        persisted_version: u64,
    },
    #[error("Phase 21 repair service failure")]
    Repair(#[from] RepairError),'''
if rpc_text.count(enum_old) != 1:
    raise SystemExit("rpc enum anchor changed")
rpc_text = rpc_text.replace(enum_old, enum_new, 1)

impl_start = rpc_text.index("impl RepairRpcError {")
impl_end = rpc_text.index("\nstruct PendingRepairRpcSession", impl_start)
new_error_impl = r'''impl RepairRpcError {
    pub fn code(&self) -> &'static str {
        match self {
            Self::InvalidRequest(_) => "invalid_request",
            Self::UnauthorizedCaller => "unauthorized_caller",
            Self::PermissionDenied(_) => "permission_denied",
            Self::ConfirmationRequired => "confirmation_required",
            Self::IrreversibleAcknowledgementRequired => "irreversible_ack_required",
            Self::SessionNotFound => "session_not_found",
            Self::CallerMismatch => "caller_mismatch",
            Self::AuthorityMismatch => "authority_mismatch",
            Self::PlanMismatch => "plan_mismatch",
            Self::ApprovalMismatch => "approval_mismatch",
            Self::VersionMismatch => "version_mismatch",
            Self::SessionNotResumable => "session_not_resumable",
            Self::SequenceExhausted => "sequence_exhausted",
            Self::Repair(error) | Self::RepairWithResume { source: error, .. } => {
                repair_error_code(error)
            }
        }
    }

    pub fn payload(&self) -> RepairRpcErrorPayload {
        let resume_version = match self {
            Self::RepairWithResume {
                persisted_version, ..
            } => Some(*persisted_version),
            _ => None,
        };
        let (message, retryable) = match self {
            Self::InvalidRequest(_) => ("The repair request is invalid.", false),
            Self::UnauthorizedCaller => {
                ("This caller is not authorized for repair service.", false)
            }
            Self::PermissionDenied(_) => ("The required repair permission is missing.", false),
            Self::ConfirmationRequired => ("Explicit repair confirmation is required.", false),
            Self::IrreversibleAcknowledgementRequired => (
                "Explicit irreversible-repair acknowledgement is required.",
                false,
            ),
            Self::SessionNotFound => ("The prepared repair session was not found.", false),
            Self::CallerMismatch => ("The repair session belongs to another caller.", false),
            Self::AuthorityMismatch => (
                "The repair authority class does not match the session.",
                false,
            ),
            Self::PlanMismatch => ("The repair plan fingerprint does not match.", false),
            Self::ApprovalMismatch => ("The approved repair action does not match.", false),
            Self::VersionMismatch => ("A newer repair session state already exists.", false),
            Self::SessionNotResumable => ("The repair session is already terminal.", false),
            Self::SequenceExhausted => ("The repair service session sequence is exhausted.", false),
            Self::Repair(error) | Self::RepairWithResume { source: error, .. } => {
                repair_error_message(error)
            }
        };
        RepairRpcErrorPayload {
            schema: REPAIR_RPC_SCHEMA.to_string(),
            code: self.code().to_string(),
            message: message.to_string(),
            retryable,
            resume_version,
        }
    }
}

fn repair_error_code(error: &RepairError) -> &'static str {
    match error {
        RepairError::UnsupportedPlatform => "unsupported_platform",
        RepairError::ElevationRequired => "elevation_required",
        RepairError::NothingToRepair(_) | RepairError::NothingToChange(_) => "nothing_to_do",
        RepairError::StateUnavailable(_) | RepairError::FeatureNotReversible(_) => {
            "state_unavailable"
        }
        RepairError::SessionStore(_) | RepairError::InvalidPersistedSession(_) => {
            "session_store_failed"
        }
        RepairError::BaselineDrift(_) => "baseline_drift",
        _ => "execution_failed",
    }
}

fn repair_error_message(error: &RepairError) -> (&'static str, bool) {
    match error {
        RepairError::UnsupportedPlatform => (
            "Windows repair authority is unavailable on this platform.",
            false,
        ),
        RepairError::ElevationRequired => {
            ("Elevated Windows servicing authority is required.", true)
        }
        RepairError::NothingToRepair(_) | RepairError::NothingToChange(_) => {
            ("The selected operation is already satisfied.", false)
        }
        RepairError::StateUnavailable(_) | RepairError::FeatureNotReversible(_) => (
            "The required Windows state is unavailable for this operation.",
            true,
        ),
        RepairError::SessionStore(_) | RepairError::InvalidPersistedSession(_) => (
            "The trusted repair session store is unavailable or invalid.",
            true,
        ),
        RepairError::BaselineDrift(_) => (
            "Windows state changed after repair preparation; prepare again.",
            true,
        ),
        _ => ("The repair operation could not be completed safely.", true),
    }
}
'''
rpc_text = rpc_text[:impl_start] + new_error_impl + rpc_text[impl_end:]

apply_old = '''        let execution_result = pending
            .session
            .execute_applying_with_host(&capability, host);
        let persisted =
            self.store
                .persist(&request.session_id, &pending.owner, &pending.session)?;
        if let Err(error) = execution_result {
            return Err(error.into());
        }
        if persisted.version < write_ahead.version {'''
apply_new = '''        let execution_result = pending
            .session
            .execute_applying_with_host(&capability, host);
        let persist_result =
            self.store
                .persist(&request.session_id, &pending.owner, &pending.session);
        let persisted = resolve_post_mutation_persist(
            execution_result,
            persist_result,
            write_ahead.version,
        )?;
        if persisted.version < write_ahead.version {'''
if rpc_text.count(apply_old) != 1:
    raise SystemExit("rpc apply persist block changed")
rpc_text = rpc_text.replace(apply_old, apply_new, 1)

resume_old = '''        let action_id = stored.session.plan().action_id();
        let capability = RepairExecutorCapability::for_rpc();
        let resume_result = stored.session.resume_with_host(&capability, host);
        let persisted = self
            .store
            .persist(&request.session_id, &stored.owner, &stored.session)?;
        if let Err(error) = resume_result {
            return Err(error.into());
        }
        Ok(execution_receipt('''
resume_new = '''        let action_id = stored.session.plan().action_id();
        let last_known_version = stored.version;
        let capability = RepairExecutorCapability::for_rpc();
        let resume_result = stored.session.resume_with_host(&capability, host);
        let persist_result = self
            .store
            .persist(&request.session_id, &stored.owner, &stored.session);
        let persisted = resolve_post_mutation_persist(
            resume_result,
            persist_result,
            last_known_version,
        )?;
        Ok(execution_receipt('''
if rpc_text.count(resume_old) != 1:
    raise SystemExit("rpc resume persist block changed")
rpc_text = rpc_text.replace(resume_old, resume_new, 1)

helper_anchor = '''#[expect(
    clippy::too_many_arguments,'''
helper = r'''fn resolve_post_mutation_persist<T>(
    operation_result: Result<(), RepairError>,
    persist_result: Result<T, RepairError>,
    last_known_version: u64,
) -> Result<T, RepairRpcError> {
    match (operation_result, persist_result) {
        (Err(operation_error), Ok(_)) => Err(operation_error.into()),
        (Err(operation_error), Err(persist_error)) => Err(RepairRpcError::RepairWithResume {
            source: operation_error,
            persistence: Some(persist_error),
            persisted_version: last_known_version,
        }),
        (Ok(()), Ok(persisted)) => Ok(persisted),
        (Ok(()), Err(persist_error)) => Err(RepairRpcError::RepairWithResume {
            source: persist_error,
            persistence: None,
            persisted_version: last_known_version,
        }),
    }
}

'''
if rpc_text.count(helper_anchor) != 1:
    raise SystemExit("rpc helper anchor changed")
rpc_text = rpc_text.replace(helper_anchor, helper + helper_anchor, 1)

test_anchor = '''    #[test]
    fn caller_safe_error_payload_does_not_leak_internal_session_path() {'''
tests = r'''    #[test]
    fn dual_failure_preserves_execution_truth_and_last_durable_version() {
        let error = resolve_post_mutation_persist::<()>(
            Err(RepairError::CommandFailed("primary execution failure".to_string())),
            Err(RepairError::SessionStore("secondary persistence failure".to_string())),
            7,
        )
        .unwrap_err();
        assert_eq!(error.code(), "execution_failed");
        let payload = error.payload();
        assert_eq!(payload.resume_version, Some(7));
        assert!(!payload.message.contains("primary execution failure"));
        assert!(!payload.message.contains("secondary persistence failure"));
        assert!(matches!(
            error,
            RepairRpcError::RepairWithResume {
                persistence: Some(_),
                persisted_version: 7,
                ..
            }
        ));
    }

    #[test]
    fn post_mutation_persist_failure_exposes_last_durable_version() {
        let error = resolve_post_mutation_persist::<()>(
            Ok(()),
            Err(RepairError::SessionStore("persistence failure".to_string())),
            11,
        )
        .unwrap_err();
        assert_eq!(error.code(), "session_store_failed");
        assert_eq!(error.payload().resume_version, Some(11));
    }

'''
if rpc_text.count(test_anchor) != 1:
    raise SystemExit("rpc test anchor changed")
rpc_text = rpc_text.replace(test_anchor, tests + test_anchor, 1)
rpc.write_text(rpc_text, encoding="utf-8")

# 3. Make Phase 21 static proof inspect typed command authority and active CI structure.
review = Path("tools/phase21_static_review.py")
review_text = review.read_text(encoding="utf-8")
source_anchor = 'HOST = (SRC / "host.rs").read_text(encoding="utf-8")\n'
if review_text.count(source_anchor) != 1:
    raise SystemExit("static review source anchor changed")
review_text = review_text.replace(
    source_anchor,
    source_anchor + 'COMMAND = (SRC / "command.rs").read_text(encoding="utf-8")\n',
    1,
)

helper_anchor = '''def has_all(text: str, values: tuple[str, ...]) -> bool:
    return all(value in text for value in values)


'''
parser_helpers = r'''def has_all(text: str, values: tuple[str, ...]) -> bool:
    return all(value in text for value in values)


def _unquote(value: str) -> str:
    value = value.strip()
    if len(value) >= 2 and value[0] == value[-1] and value[0] in {"'", '"'}:
        return value[1:-1]
    return value


def parse_ci_job(text: str, job_name: str) -> tuple[dict[str, str], list[dict[str, str]]]:
    job: dict[str, str] = {}
    steps: list[dict[str, str]] = []
    in_jobs = False
    in_job = False
    in_steps = False
    current: dict[str, str] | None = None
    section: str | None = None

    for raw in text.splitlines():
        stripped = raw.strip()
        if not stripped or stripped.startswith("#"):
            continue
        indent = len(raw) - len(raw.lstrip(" "))
        if indent == 0:
            if current is not None:
                steps.append(current)
                current = None
            in_jobs = stripped == "jobs:"
            in_job = False
            in_steps = False
            section = None
            continue
        if not in_jobs:
            continue
        if indent == 2 and stripped.endswith(":"):
            if current is not None:
                steps.append(current)
                current = None
            in_job = stripped[:-1] == job_name
            in_steps = False
            section = None
            continue
        if not in_job:
            continue
        if indent == 4:
            if stripped == "steps:":
                in_steps = True
                continue
            key, sep, value = stripped.partition(":")
            if sep:
                job[key] = _unquote(value)
            continue
        if in_steps and indent == 6 and stripped.startswith("- "):
            if current is not None:
                steps.append(current)
            current = {}
            section = None
            key, sep, value = stripped[2:].partition(":")
            if sep:
                current[key] = _unquote(value)
            continue
        if in_steps and current is not None and indent == 8:
            key, sep, value = stripped.partition(":")
            if sep:
                value = _unquote(value)
                current[key] = value
                section = key if not value else None
            continue
        if in_steps and current is not None and indent == 10 and section:
            key, sep, value = stripped.partition(":")
            if sep:
                current[f"{section}.{key}"] = _unquote(value)

    if current is not None:
        steps.append(current)
    return job, steps


def ci_step_matches(name: str, **expected: str) -> bool:
    matches = [step for step in CI_STEPS if step.get("name") == name]
    return len(matches) == 1 and all(matches[0].get(key) == value for key, value in expected.items())


'''
if review_text.count(helper_anchor) != 1:
    raise SystemExit("static review helper anchor changed")
review_text = review_text.replace(helper_anchor, parser_helpers, 1)
members_anchor = 'members = set(WORKSPACE["workspace"]["members"])\n'
if review_text.count(members_anchor) != 1:
    raise SystemExit("static review members anchor changed")
review_text = review_text.replace(
    members_anchor,
    'CI_JOB, CI_STEPS = parse_ci_job(CI, "engineering-proof")\n' + members_anchor,
    1,
)

fixed_start = review_text.index('    (\n        "fixed-command-surface",')
fixed_end = review_text.index('    (\n        "elevation-truth",', fixed_start)
fixed_block = '''    (
        "fixed-command-surface",
        has_all(
            HOST,
            (
                "component_store_inspection_command()",
                "system_files_inspection_command()",
                "feature_inspection_command(feature)",
                "operation_command(operation)",
                "capture_trusted",
            ),
        )
        and HOST.count("self.capture(") == 1
        and "std::process::Command" not in HOST
        and has_all(
            COMMAND,
            (
                "pub(crate) fn component_store_inspection_command",
                "pub(crate) fn system_files_inspection_command",
                "pub(crate) fn feature_inspection_command",
                "pub(crate) fn operation_command",
                "trusted_command_contract_is_exact",
            ),
        )
        and "pub fn operation_command" not in COMMAND
        and "pub fn feature_inspection_command" not in COMMAND,
    ),
'''
review_text = review_text[:fixed_start] + fixed_block + review_text[fixed_end:]

regression_start = review_text.index('    (\n        "regression-and-ci-continuity",')
regression_end = review_text.index('    (\n        "adversarial-source-first-acceptance",', regression_start)
regression_block = '''    (
        "regression-and-ci-continuity",
        CI_JOB.get("runs-on") == "${{ matrix.os }}"
        and ci_step_matches(
            "Set up Python 3.11",
            uses="actions/setup-python@v5",
            **{"with.python-version": "3.11"},
        )
        and ci_step_matches(
            "Phase 20 twenty-lane static review",
            run="python -W error tools/phase20_static_review.py",
        )
        and ci_step_matches(
            "Phase 21 twenty-lane static review",
            run="python -W error tools/phase21_static_review.py",
        )
        and ci_step_matches(
            "Phase 21 Repair & Windows Features proof",
            run="cargo test --locked -p neo-repair",
        )
        and ci_step_matches(
            "Phase 21 trusted Windows command contract proof",
            run="cargo test --locked -p neo-repair command::tests::trusted_command_contract_is_exact",
        )
        and ci_step_matches(
            "Phase 21 read-only Windows repair source proof",
            **{
                "if": "runner.os == 'Windows'",
                "timeout-minutes": "20",
                "run": "cargo run --locked -p neo-cli -- repair inspect --json",
            },
        )
        and ci_step_matches(
            "Phase 21 read-only Windows feature source proof",
            **{
                "if": "runner.os == 'Windows'",
                "timeout-minutes": "20",
                "run": "cargo run --locked -p neo-cli -- repair features --json",
            },
        ),
    ),
'''
review_text = review_text[:regression_start] + regression_block + review_text[regression_end:]
review.write_text(review_text, encoding="utf-8")

ci = Path(".github/workflows/ci.yml")
ci_text = ci.read_text(encoding="utf-8")
ci_anchor = '''      - name: Phase 21 Repair & Windows Features proof
        run: cargo test --locked -p neo-repair

'''
ci_replacement = '''      - name: Phase 21 Repair & Windows Features proof
        run: cargo test --locked -p neo-repair

      - name: Phase 21 trusted Windows command contract proof
        run: cargo test --locked -p neo-repair command::tests::trusted_command_contract_is_exact

'''
if ci_text.count(ci_anchor) != 1:
    raise SystemExit("CI Phase 21 proof anchor changed")
ci.write_text(ci_text.replace(ci_anchor, ci_replacement, 1), encoding="utf-8")

# No temporary patch machinery may survive the correction commit.
Path(".github/workflows/phase21-review-fix.yml").unlink(missing_ok=True)
Path("tools/phase21_apply_review_fix.py").unlink(missing_ok=True)
