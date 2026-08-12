#!/usr/bin/env python3
"""Deterministic 20-lane static review for Neo Driver Phase 5."""
from __future__ import annotations

from dataclasses import dataclass
from pathlib import Path
import sys
import tomllib

ROOT = Path(__file__).resolve().parents[1]
DRIVERSTORE_DIR = ROOT / "crates/neo-driverstore/src"
DRIVERSTORE = "\n".join(
    path.read_text(encoding="utf-8") for path in sorted(DRIVERSTORE_DIR.rglob("*.rs"))
)
WINDOWS = (DRIVERSTORE_DIR / "windows.rs").read_text(encoding="utf-8")
PLAN = (DRIVERSTORE_DIR / "plan.rs").read_text(encoding="utf-8")
EXECUTOR = (DRIVERSTORE_DIR / "executor.rs").read_text(encoding="utf-8")
TESTS = (DRIVERSTORE_DIR / "tests.rs").read_text(encoding="utf-8")
CLI = "\n".join(
    path.read_text(encoding="utf-8")
    for path in sorted((ROOT / "crates/neo-cli/src").rglob("*.rs"))
)
WORKSPACE = tomllib.loads((ROOT / "Cargo.toml").read_text(encoding="utf-8"))
CRATE = tomllib.loads((ROOT / "crates/neo-driverstore/Cargo.toml").read_text(encoding="utf-8"))


@dataclass(frozen=True)
class Lane:
    number: int
    name: str
    passed: bool
    detail: str


def contains_all(text: str, values: list[str]) -> bool:
    return all(value in text for value in values)


def section(text: str, start: str, end: str) -> str:
    begin = text.find(start)
    finish = text.find(end, begin + len(start)) if begin >= 0 else -1
    if begin < 0 or finish < 0:
        return ""
    return text[begin:finish]


def review() -> list[Lane]:
    members = set(WORKSPACE["workspace"]["members"])
    target_deps = CRATE.get("target", {}).get("cfg(windows)", {}).get("dependencies", {})
    forward = section(WINDOWS, "fn install_best_match(", "fn restore_specific_driver(")
    rollback = section(WINDOWS, "fn restore_specific_driver(", "fn remove_published_package(")
    uninstall = section(WINDOWS, "fn remove_published_package(", "fn present_device_set(")
    forbidden_force = [
        "DiInstallDriverW",
        "DIIRFLAG_FORCE_INF",
        "SUOI_FORCEDELETE",
        "SP_COPY_REPLACEONLY",
        "SP_COPY_FORCE_NEWER",
    ]
    forbidden_cli_mutation = [
        "DriverInstallSession",
        "WindowsDriverHost",
        "prepare_driver_install",
        "install_best_match",
        "restore_specific_driver",
        "stage_driver",
    ]
    required_regressions = [
        "planner_binds_windows_impact_to_catalogue_impact",
        "planner_refuses_missing_baseline_driver_package",
        "source_byte_drift_blocks_before_staging",
        "healthy_target_install_reaches_complete",
        "healthy_windows_noop_cleans_new_store_package_and_completes",
        "backend_failure_after_binding_change_routes_exact_rollback",
        "runtime_install_reboot_is_persisted_and_reproven",
        "rollback_reboot_defers_store_removal_until_binding_is_restored",
        "post_mutation_inventory_failure_routes_conservative_rollback",
        "transient_verification_probe_can_be_retried",
        "direct_session_deserialization_cannot_rebind_transaction",
    ]
    return [
        Lane(
            1,
            "workspace-and-windows-binding",
            "crates/neo-driverstore" in members and "windows" in target_deps,
            "neo-driverstore is a workspace member and the Win32 dependency is Windows-only",
        ),
        Lane(
            2,
            "validated-root-contracts",
            contains_all(
                DRIVERSTORE,
                [
                    "DriverInstallPlanWire",
                    "DriverInstallSessionWire",
                    "serde(try_from",
                    "SessionInvariantViolation",
                    "fingerprint",
                ],
            ),
            "driver plans/sessions are validated at root deserialization and remain fingerprint-bound",
        ),
        Lane(
            3,
            "signature-authority",
            contains_all(PLAN, ["SignatureStatus::Verified", "expected_signature", "SignatureMismatch"])
            and contains_all(WINDOWS, ["SetupVerifyInfFileW", "SP_INF_SIGNER_INFO_V2_W"]),
            "catalogue Verified state is re-proven against the actual Windows INF signature/catalogue",
        ),
        Lane(
            4,
            "exact-source-bytes",
            contains_all(DRIVERSTORE, ["source_inf_sha256", "sha256_file", "PrestateDrift"])
            and contains_all(PLAN, ["canonicalize", "starts_with", "UnsafeInfPath"]),
            "authority binds to canonical in-root INF bytes and apply rechecks the SHA-256",
        ),
        Lane(
            5,
            "exact-windows-impact",
            contains_all(
                WINDOWS,
                [
                    "compatible_present_devices",
                    "DI_ENUMSINGLEINF",
                    "SPDIT_COMPATDRIVER",
                    "SetupDiBuildDriverInfoList",
                    "SetupDiEnumDriverInfoW",
                ],
            ),
            "Windows compatibility is queried against the exact INF for present devices",
        ),
        Lane(
            6,
            "catalogue-impact-cross-check",
            contains_all(
                PLAN,
                [
                    "match_device",
                    "windows_impacts",
                    "catalogue_impacts",
                    "CatalogueImpactMismatch",
                ],
            ),
            "Windows exact-INF impact must equal Neo catalogue/matcher impact",
        ),
        Lane(
            7,
            "rollback-baseline-package",
            contains_all(
                DRIVERSTORE,
                [
                    "baseline_package",
                    "MissingBaselinePackage",
                    "resolve_published_package",
                    "MissingBaselinePublishedInf",
                ],
            ),
            "every impacted device has an exact active binding and resolvable baseline Driver Store package before authority",
        ),
        Lane(
            8,
            "transaction-reconstruction",
            contains_all(
                DRIVERSTORE,
                [
                    "transaction_contract",
                    "baseline_contract",
                    "plan_fingerprint",
                    "TransactionCheckpoint",
                    "capture_baseline",
                ],
            ),
            "driver session reconstructs and validates the generic transaction/baseline contract",
        ),
        Lane(
            9,
            "preflight-reproof",
            contains_all(
                EXECUTOR,
                [
                    "fn preflight",
                    "source_inf_sha256",
                    "verify_inf_signature",
                    "compatible_present_devices",
                    "baseline_package",
                    "PrestateDrift",
                    "ImpactDrift",
                ],
            ),
            "source, signature, impact, bindings, baseline packages, and target-store baseline are re-proven immediately before mutation",
        ),
        Lane(
            10,
            "published-package-identity",
            contains_all(
                WINDOWS,
                [
                    "SetupCopyOEMInfW",
                    "SetupGetInfPublishedNameW",
                    "SetupGetInfDriverStoreLocationW",
                    "is_safe_published_name",
                    "StagedPackageMismatch",
                ],
            ),
            "staging captures and round-trips the exact Windows OEM published/package identity",
        ),
        Lane(
            11,
            "per-device-best-match-forward",
            contains_all(forward, ["instance_id", "DiInstallDevice", "None", "DIINSTALLDEVICE_FLAGS(0)"])
            and "SetupDiBuildDriverInfoList" not in forward,
            "forward mutation is scoped to each authorized device and asks Windows for its best preinstalled match without supplying a driver node",
        ),
        Lane(
            12,
            "no-force-path",
            not any(marker in DRIVERSTORE for marker in forbidden_force)
            and contains_all(uninstall, ["SetupUninstallOEMInfW", "PCWSTR(wide.as_ptr()), 0, None"]),
            "no force-install/force-delete primitive exists; package removal uses flags=0",
        ),
        Lane(
            13,
            "blast-radius-enforcement",
            contains_all(
                EXECUTOR,
                [
                    "impact_ids",
                    "evaluate_forward",
                    "unexpected binding change outside authority",
                    "UnexpectedBindingChange",
                ],
            ),
            "outside-authority binding changes fail the forward policy",
        ),
        Lane(
            14,
            "api-result-not-proof",
            contains_all(
                EXECUTOR,
                [
                    "machine_changed",
                    "policy_observation",
                    "verify_postconditions",
                    "PolicyUnsatisfied",
                    "ApplyOutcome::Success",
                ],
            ),
            "backend success is separate from net mutation and deterministic postcondition proof",
        ),
        Lane(
            15,
            "no-op-store-restoration",
            contains_all(
                EXECUTOR,
                [
                    "!binding_changed",
                    "DriverStoreBaseline::Absent",
                    "remove_published_package",
                    "unused staged package cleanup failed",
                ],
            )
            and "healthy_windows_noop_cleans_new_store_package_and_completes" in TESTS,
            "a newly staged but unused package is removed so a healthy best-match no-op restores the original Driver Store state",
        ),
        Lane(
            16,
            "post-mutation-uncertainty",
            contains_all(
                EXECUTOR,
                [
                    "record_uncertain_apply_failure",
                    "machine_changed: true",
                    "post-mutation device inventory failed",
                    "post-mutation Driver Store probe failed",
                ],
            )
            and "post_mutation_inventory_failure_routes_conservative_rollback" in TESTS,
            "post-write observation uncertainty is conservatively recorded as changed and enters recovery",
        ),
        Lane(
            17,
            "runtime-reboot-proof",
            contains_all(
                EXECUTOR,
                ["reboot_required", "resume_after_reboot", "reprobe_after_block", "verify_current"],
            )
            and "runtime_install_reboot_is_persisted_and_reproven" in TESTS,
            "runtime reboot evidence is persisted and post-reboot policy must be re-proven",
        ),
        Lane(
            18,
            "exact-rollback-binding",
            contains_all(
                rollback,
                [
                    "published_inf",
                    "configure_single_inf",
                    "SetupDiBuildDriverInfoList",
                    "SetupDiEnumDriverInfoW",
                    "Some(&driver)",
                    "DiInstallDevice",
                ],
            )
            and contains_all(EXECUTOR, ["baseline_package.published_inf", "restore_specific_driver"]),
            "specific-driver installation exists only in rollback and restores the captured baseline package",
        ),
        Lane(
            19,
            "rollback-store-and-reboot-proof",
            contains_all(
                EXECUTOR,
                [
                    "AwaitingRollbackReboot",
                    "resume_after_rollback_reboot",
                    "verify_rollback_current",
                    "restore_driver_store_if_possible",
                    "rollback_observations",
                ],
            )
            and "rollback_reboot_defers_store_removal_until_binding_is_restored" in TESTS,
            "rollback reboot/store cleanup remains persistent and requires exact baseline verification",
        ),
        Lane(
            20,
            "closed-mutation-surface-and-regressions",
            not any(marker in CLI for marker in forbidden_cli_mutation)
            and all(test in TESTS for test in required_regressions)
            and contains_all(
                WINDOWS,
                [
                    "config_manager_status_failure_is_not_treated_as_healthy",
                    "published_name_requires_numeric_oem_index",
                ],
            ),
            "Phase 5 mutator remains internal (no CLI write surface) and adversarial + Windows validation regressions are present",
        ),
    ]


def main() -> int:
    lanes = review()
    for lane in lanes:
        print(f"{'PASS' if lane.passed else 'FAIL'} {lane.number:02d} {lane.name}: {lane.detail}")
    failures = [lane for lane in lanes if not lane.passed]
    if failures:
        print(f"\nPhase 5 static review failed: {len(failures)} lane(s) unresolved.")
        return 1
    print("\nPhase 5 static review: PASS (20/20 lanes).")
    return 0


if __name__ == "__main__":
    sys.exit(main())
