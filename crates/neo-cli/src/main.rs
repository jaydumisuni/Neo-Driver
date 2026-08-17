mod repair_cli;

use clap::{Parser, Subcommand, ValueEnum};
use neo_catalogue::Catalogue;
use neo_core::{MissionPlan, UserDepth, UserIntent};
use neo_device::DeviceRecord;
use neo_match::{match_device, MatchContext};
use neo_probe::scan_current_machine;
use neo_runtime::{
    assess_runtime_profile, component_label, RuntimeInventory, RuntimePolicy, RuntimeProfile,
};
use neo_runtime_executor::RuntimeExecutionPlan;
use neo_runtime_probe::scan_current_runtime_inventory;
use neo_transaction::{TransactionCheckpoint, TransactionPlan};
use neo_vault::{DriverSourceMap, VaultLayout, VaultMode, VaultStore};
use std::path::PathBuf;
use std::process::ExitCode;

#[derive(Debug, Parser)]
#[command(
    name = "neo",
    version,
    about = "Neo Driver model-free Windows setup and repair core"
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Read-only machine scan. No driver, runtime, tweak, or security state is changed.
    Scan {
        /// Emit machine-readable JSON.
        #[arg(long)]
        json: bool,
    },
    /// Read-only Windows runtime System X-Ray using documented evidence paths.
    RuntimeScan {
        /// Emit machine-readable JSON including raw command evidence.
        #[arg(long)]
        json: bool,
    },
    /// Create an empty authority-safe mission envelope for an intended workflow.
    Plan {
        #[arg(value_enum)]
        intent: CliIntent,
        #[arg(long, value_enum, default_value_t = CliDepth::Standard)]
        depth: CliDepth,
        /// Emit machine-readable JSON.
        #[arg(long)]
        json: bool,
    },
    /// Read-only catalogue inspection and validation.
    Catalogue {
        #[command(subcommand)]
        command: CatalogueCommand,
    },
    /// Read-only driver candidate matching. This never installs or stages a driver.
    Match {
        /// Validated Neo device JSON.
        #[arg(long)]
        device: PathBuf,
        /// Validated Neo catalogue JSON.
        #[arg(long)]
        catalogue: PathBuf,
        /// Windows architecture such as x64 or arm64.
        #[arg(long)]
        architecture: String,
        /// Windows build number.
        #[arg(long)]
        build: u32,
        /// Emit machine-readable JSON.
        #[arg(long)]
        json: bool,
    },
    /// Read-only runtime profile assessment. Actions are reviewable plans only.
    Runtimes {
        /// Normalized runtime evidence JSON.
        #[arg(long)]
        evidence: PathBuf,
        /// Validated Neo package catalogue JSON.
        #[arg(long)]
        catalogue: PathBuf,
        /// Runtime package-to-component policy JSON.
        #[arg(long)]
        policy: PathBuf,
        /// Profile to assess.
        #[arg(long, value_enum, default_value_t = CliRuntimeProfile::FreshWindows)]
        profile: CliRuntimeProfile,
        /// Emit machine-readable JSON.
        #[arg(long)]
        json: bool,
    },
    /// Read-only gaming readiness assessment using the same runtime engine.
    Gaming {
        /// Normalized runtime evidence JSON.
        #[arg(long)]
        evidence: PathBuf,
        /// Validated Neo package catalogue JSON.
        #[arg(long)]
        catalogue: PathBuf,
        /// Runtime package-to-component policy JSON.
        #[arg(long)]
        policy: PathBuf,
        /// Emit machine-readable JSON.
        #[arg(long)]
        json: bool,
    },
    /// Validate an exact Phase 8 runtime execution plan without executing it.
    RuntimeExecutorValidatePlan {
        /// Persisted Phase 8 runtime execution plan JSON.
        #[arg(long)]
        plan: PathBuf,
        /// Emit the normalized validated plan as JSON.
        #[arg(long)]
        json: bool,
    },
    /// Read-only transaction/checkpoint validation. No machine change is executed.
    Transaction {
        #[command(subcommand)]
        command: TransactionCommand,
    },
    /// Read-only managed package-vault inspection and source-map validation.
    Vault {
        #[command(subcommand)]
        command: VaultCommand,
    },
    /// Read-only Repair & Windows Features inspection. No machine mutation is exposed.
    Repair {
        #[command(subcommand)]
        command: repair_cli::RepairCommand,
    },
    /// Show the current implementation boundary.
    Status,
}

#[derive(Debug, Subcommand)]
enum CatalogueCommand {
    /// Validate a Neo catalogue JSON file without installing or downloading anything.
    Validate {
        path: PathBuf,
        /// Emit the normalized catalogue as JSON after validation.
        #[arg(long)]
        json: bool,
    },
}

#[derive(Debug, Subcommand)]
enum TransactionCommand {
    /// Validate an exact Neo transaction plan and its fingerprint.
    ValidatePlan {
        path: PathBuf,
        /// Emit machine-readable JSON.
        #[arg(long)]
        json: bool,
    },
    /// Emit a new Planned checkpoint bound to the exact validated plan.
    CheckpointTemplate { path: PathBuf },
    /// Validate a persisted checkpoint without advancing it.
    ValidateCheckpoint {
        path: PathBuf,
        /// Emit machine-readable JSON.
        #[arg(long)]
        json: bool,
    },
}

#[derive(Debug, Subcommand)]
enum VaultCommand {
    /// Describe the NeoData layout beneath the application root supplied by Builder.
    Describe {
        #[arg(long)]
        app_root: PathBuf,
        /// Treat app_root as a portable Neo folder instead of an installed app root.
        #[arg(long)]
        portable: bool,
        /// Emit machine-readable JSON.
        #[arg(long)]
        json: bool,
    },
    /// Validate a Neo driver-source map without downloading any package.
    ValidateSources {
        path: PathBuf,
        /// Emit the normalized source map as JSON.
        #[arg(long)]
        json: bool,
    },
    /// Audit an existing NeoData tree for unsafe link/reparse paths without changing it.
    Audit {
        #[arg(long)]
        app_root: PathBuf,
        /// Treat app_root as a portable Neo folder instead of an installed app root.
        #[arg(long)]
        portable: bool,
    },
}

#[derive(Debug, Clone, Copy, ValueEnum)]
enum CliDepth {
    Beginner,
    Standard,
    Expert,
}

impl From<CliDepth> for UserDepth {
    fn from(value: CliDepth) -> Self {
        match value {
            CliDepth::Beginner => Self::Beginner,
            CliDepth::Standard => Self::Standard,
            CliDepth::Expert => Self::Expert,
        }
    }
}

#[derive(Debug, Clone, Copy, ValueEnum)]
enum CliIntent {
    SetupPc,
    FixProblem,
    InstallDrivers,
    PrepareGaming,
    PrepareTechnician,
    ImproveWindows,
    DebloatWindows,
    RepairDevices,
    Advanced,
}

impl From<CliIntent> for UserIntent {
    fn from(value: CliIntent) -> Self {
        match value {
            CliIntent::SetupPc => Self::SetupPc,
            CliIntent::FixProblem => Self::FixProblem,
            CliIntent::InstallDrivers => Self::InstallDrivers,
            CliIntent::PrepareGaming => Self::PrepareGaming,
            CliIntent::PrepareTechnician => Self::PrepareTechnician,
            CliIntent::ImproveWindows => Self::ImproveWindows,
            CliIntent::DebloatWindows => Self::DebloatWindows,
            CliIntent::RepairDevices => Self::RepairDevices,
            CliIntent::Advanced => Self::Advanced,
        }
    }
}

#[derive(Debug, Clone, Copy, ValueEnum)]
enum CliRuntimeProfile {
    FreshWindows,
    Gaming,
    Technician,
    Developer,
}

impl From<CliRuntimeProfile> for RuntimeProfile {
    fn from(value: CliRuntimeProfile) -> Self {
        match value {
            CliRuntimeProfile::FreshWindows => Self::FreshWindows,
            CliRuntimeProfile::Gaming => Self::Gaming,
            CliRuntimeProfile::Technician => Self::Technician,
            CliRuntimeProfile::Developer => Self::Developer,
        }
    }
}

fn main() -> ExitCode {
    match run(Cli::parse()) {
        Ok(()) => ExitCode::SUCCESS,
        Err(message) => {
            eprintln!("neo: {message}");
            ExitCode::FAILURE
        }
    }
}

fn run(cli: Cli) -> Result<(), String> {
    match cli.command {
        Command::Scan { json } => {
            let report = scan_current_machine().map_err(|error| error.to_string())?;
            if json {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&report).map_err(|error| error.to_string())?
                );
            } else {
                println!("Neo read-only scan");
                println!("------------------");
                println!(
                    "Windows: {}",
                    report
                        .profile
                        .os
                        .product_name
                        .as_deref()
                        .unwrap_or("unknown")
                );
                println!(
                    "Build: {}",
                    report
                        .profile
                        .os
                        .build_number
                        .as_deref()
                        .unwrap_or("unknown")
                );
                println!(
                    "Test signing: {}",
                    format_optional_bool(report.profile.security.test_signing)
                );
                println!(
                    "Integrity checks disabled: {}",
                    format_optional_bool(report.profile.security.no_integrity_checks)
                );
                println!(
                    "Secure Boot: {}",
                    format_optional_bool(report.profile.security.secure_boot)
                );
                println!(
                    "Memory Integrity: {}",
                    format_optional_bool(report.profile.security.memory_integrity)
                );
                println!(
                    "Pending reboot: {}",
                    format_optional_presence(report.profile.security.pending_reboot)
                );
                println!("Evidence lanes: {}", report.command_evidence.len());
                if !report.profile.warnings.is_empty() {
                    println!("Warnings:");
                    for warning in &report.profile.warnings {
                        println!("  - {warning}");
                    }
                }
            }
            Ok(())
        }
        Command::Repair { command } => repair_cli::run(command),
        Command::RuntimeScan { json } => {
            let report = scan_current_runtime_inventory().map_err(|error| error.to_string())?;
            if json {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&report).map_err(|error| error.to_string())?
                );
            } else {
                println!("Neo read-only runtime System X-Ray");
                println!("--------------------------------");
                println!("Build: {}", report.inventory.windows_build);
                println!("Architecture: {}", report.inventory.architecture);
                for item in &report.inventory.observations {
                    println!(
                        "- {}: {:?}{}",
                        component_label(item.component),
                        item.state,
                        item.detected_version
                            .as_deref()
                            .map(|version| format!(" ({version})"))
                            .unwrap_or_default()
                    );
                }
                println!("Raw command evidence: {}", report.command_evidence.len());
                if !report.warnings.is_empty() {
                    println!("Warnings:");
                    for warning in &report.warnings {
                        println!("  - {warning}");
                    }
                }
                println!("Machine changes: none");
            }
            Ok(())
        }
        Command::Plan {
            intent,
            depth,
            json,
        } => {
            let plan = MissionPlan::new("NEO-UNSCHEDULED", intent.into(), depth.into());
            plan.validate().map_err(|error| error.to_string())?;
            if json {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&plan).map_err(|error| error.to_string())?
                );
            } else {
                println!("Neo mission envelope");
                println!("Intent: {}", plan.intent);
                println!("Depth: {}", plan.user_depth);
                println!("Actions: 0 (planning engine not yet connected)");
                println!("Machine changes: none");
            }
            Ok(())
        }
        Command::Catalogue { command } => match command {
            CatalogueCommand::Validate { path, json } => {
                let catalogue = Catalogue::read_json(&path).map_err(|error| error.to_string())?;
                if json {
                    println!(
                        "{}",
                        serde_json::to_string_pretty(&catalogue)
                            .map_err(|error| error.to_string())?
                    );
                } else {
                    println!("Neo catalogue validation: PASS");
                    println!("File: {}", path.display());
                    println!("Packages: {}", catalogue.packages.len());
                    println!("Machine changes: none");
                }
                Ok(())
            }
        },
        Command::Match {
            device,
            catalogue,
            architecture,
            build,
            json,
        } => {
            let device_json =
                std::fs::read_to_string(&device).map_err(|error| error.to_string())?;
            let device_record: DeviceRecord =
                serde_json::from_str(&device_json).map_err(|error| error.to_string())?;
            let catalogue = Catalogue::read_json(&catalogue).map_err(|error| error.to_string())?;
            let context = MatchContext {
                architecture,
                windows_build: build,
            };
            let report = match_device(&device_record, &catalogue, &context)
                .map_err(|error| error.to_string())?;

            if json {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&report).map_err(|error| error.to_string())?
                );
            } else {
                println!("Neo read-only driver match");
                println!("Device: {}", report.device_instance_id);
                println!("Candidates: {}", report.candidates.len());
                match &report.best_candidate {
                    Some(best) => {
                        println!("Best available-evidence candidate: {}", best.package_id);
                        println!("INF: {}", best.inf_path);
                    }
                    None => println!("Best available-evidence candidate: none / ambiguous"),
                }
                println!("Full Windows rank available: {}", report.ranking_complete);
                println!("Machine changes: none");
            }
            Ok(())
        }
        Command::Runtimes {
            evidence,
            catalogue,
            policy,
            profile,
            json,
        } => run_runtime_assessment(evidence, catalogue, policy, profile.into(), json),
        Command::Gaming {
            evidence,
            catalogue,
            policy,
            json,
        } => run_runtime_assessment(evidence, catalogue, policy, RuntimeProfile::Gaming, json),
        Command::RuntimeExecutorValidatePlan { plan, json } => {
            let input = std::fs::read_to_string(&plan).map_err(|error| error.to_string())?;
            let execution_plan =
                RuntimeExecutionPlan::from_json_str(&input).map_err(|error| error.to_string())?;
            if json {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&execution_plan)
                        .map_err(|error| error.to_string())?
                );
            } else {
                println!("Neo Phase 8 runtime execution-plan validation: PASS");
                println!("File: {}", plan.display());
                println!("Mission: {}", execution_plan.mission_id);
                println!("Transaction: {}", execution_plan.transaction_id);
                println!("Component: {:?}", execution_plan.component);
                println!("Operation: {:?}", execution_plan.operation);
                println!("Package: {}", execution_plan.package_id);
                println!(
                    "Payload: {}",
                    execution_plan
                        .payload_path()
                        .map_err(|error| error.to_string())?
                        .display()
                );
                println!("Execution: disabled from CLI");
                println!("Machine changes: none");
            }
            Ok(())
        }
        Command::Transaction { command } => match command {
            TransactionCommand::ValidatePlan { path, json } => {
                let input = std::fs::read_to_string(&path).map_err(|error| error.to_string())?;
                let plan =
                    TransactionPlan::from_json_str(&input).map_err(|error| error.to_string())?;
                let fingerprint = plan.fingerprint().map_err(|error| error.to_string())?;
                if json {
                    println!(
                        "{}",
                        serde_json::json!({
                            "transaction_id": plan.transaction_id(),
                            "revision": plan.revision(),
                            "mission_id": plan.mission_id(),
                            "fingerprint": fingerprint,
                            "actions": plan.actions().len(),
                            "machine_changes": "none"
                        })
                    );
                } else {
                    println!("Neo transaction plan validation: PASS");
                    println!("File: {}", path.display());
                    println!("Transaction: {}", plan.transaction_id());
                    println!("Revision: {}", plan.revision());
                    println!("Fingerprint: {fingerprint}");
                    println!("Actions: {}", plan.actions().len());
                    println!("Machine changes: none");
                }
                Ok(())
            }
            TransactionCommand::CheckpointTemplate { path } => {
                let input = std::fs::read_to_string(&path).map_err(|error| error.to_string())?;
                let plan =
                    TransactionPlan::from_json_str(&input).map_err(|error| error.to_string())?;
                let checkpoint =
                    TransactionCheckpoint::new(plan).map_err(|error| error.to_string())?;
                println!(
                    "{}",
                    serde_json::to_string_pretty(&checkpoint).map_err(|error| error.to_string())?
                );
                Ok(())
            }
            TransactionCommand::ValidateCheckpoint { path, json } => {
                let input = std::fs::read_to_string(&path).map_err(|error| error.to_string())?;
                let checkpoint = TransactionCheckpoint::from_json_str(&input)
                    .map_err(|error| error.to_string())?;
                if json {
                    println!(
                        "{}",
                        serde_json::json!({
                            "transaction_id": checkpoint.plan().transaction_id(),
                            "stage": checkpoint.stage(),
                            "fingerprint": checkpoint.plan_fingerprint(),
                            "machine_changes": "none"
                        })
                    );
                } else {
                    println!("Neo transaction checkpoint validation: PASS");
                    println!("File: {}", path.display());
                    println!("Transaction: {}", checkpoint.plan().transaction_id());
                    println!("Stage: {:?}", checkpoint.stage());
                    println!("Fingerprint: {}", checkpoint.plan_fingerprint());
                    println!("Machine changes: none");
                }
                Ok(())
            }
        },
        Command::Vault { command } => match command {
            VaultCommand::Describe {
                app_root,
                portable,
                json,
            } => {
                let mode = if portable {
                    VaultMode::Portable
                } else {
                    VaultMode::Installed
                };
                let layout =
                    VaultLayout::new(mode, &app_root).map_err(|error| error.to_string())?;
                if json {
                    println!(
                        "{}",
                        serde_json::json!({
                            "mode": layout.mode(),
                            "application_root": layout.application_root(),
                            "managed_root": layout.managed_root(),
                            "catalogue": layout.catalogue(),
                            "driver_packs": layout.driver_packs(),
                            "packages": layout.packages(),
                            "runtimes": layout.runtimes(),
                            "staging": layout.staging(),
                            "sessions": layout.sessions(),
                            "backups": layout.backups(),
                            "logs": layout.logs(),
                            "cache": layout.cache(),
                            "machine_changes": "none"
                        })
                    );
                } else {
                    println!("Neo managed vault layout");
                    println!("Mode: {:?}", layout.mode());
                    println!("Application root: {}", layout.application_root().display());
                    println!("Managed root: {}", layout.managed_root().display());
                    println!("Driver packs: {}", layout.driver_packs().display());
                    println!("Runtimes: {}", layout.runtimes().display());
                    println!("Staging: {}", layout.staging().display());
                    println!("Machine changes: none");
                }
                Ok(())
            }
            VaultCommand::ValidateSources { path, json } => {
                let input = std::fs::read_to_string(&path).map_err(|error| error.to_string())?;
                let source_map =
                    DriverSourceMap::from_json_str(&input).map_err(|error| error.to_string())?;
                if json {
                    println!(
                        "{}",
                        serde_json::to_string_pretty(&source_map)
                            .map_err(|error| error.to_string())?
                    );
                } else {
                    println!("Neo driver source-map validation: PASS");
                    println!("File: {}", path.display());
                    println!("Sources: {}", source_map.sources.len());
                    println!("Downloads: none");
                    println!("Machine changes: none");
                }
                Ok(())
            }
            VaultCommand::Audit { app_root, portable } => {
                let mode = if portable {
                    VaultMode::Portable
                } else {
                    VaultMode::Installed
                };
                let layout =
                    VaultLayout::new(mode, &app_root).map_err(|error| error.to_string())?;
                let store = VaultStore::new(layout);
                store
                    .audit_existing_tree()
                    .map_err(|error| error.to_string())?;
                println!("Neo managed vault audit: PASS");
                println!("Application root: {}", app_root.display());
                println!("Machine changes: none");
                Ok(())
            }
        },
        Command::Status => {
            println!("Neo Driver implementation phase: Phase 8 internal runtime executor proof");
            println!("Driver mutation backend: internal pending live attached-device proof");
            println!("Phase 6 runtime System X-Ray: read-only documented evidence paths");
            println!("Runtime/gaming assessment: read-only and user-selectable");
            println!("Phase 8 runtime executor: internal single-file EXE/MSI boundary");
            println!("Managed vault root: Builder/portable root + NeoData");
            println!("Network package acquisition: intentionally disabled at this gate");
            println!("Runtime downloads/installations: intentionally disabled on public CLI at this gate");
            println!("Archive extraction and Windows-feature mutation: intentionally disabled");
            println!("Runtime execution from CLI: intentionally disabled");
            println!("Transaction advancement from CLI: intentionally disabled");
            println!("Model dependency: none");
            Ok(())
        }
    }
}

fn run_runtime_assessment(
    evidence: PathBuf,
    catalogue_path: PathBuf,
    policy_path: PathBuf,
    profile: RuntimeProfile,
    json: bool,
) -> Result<(), String> {
    let catalogue = Catalogue::read_json(&catalogue_path).map_err(|error| error.to_string())?;
    let inventory = RuntimeInventory::read_json(&evidence).map_err(|error| error.to_string())?;
    let policy =
        RuntimePolicy::read_json(&policy_path, &catalogue).map_err(|error| error.to_string())?;
    let assessment = assess_runtime_profile(profile, &inventory, &catalogue, &policy)
        .map_err(|error| error.to_string())?;

    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(&assessment).map_err(|error| error.to_string())?
        );
    } else {
        println!("Neo read-only runtime assessment");
        println!("Profile: {:?}", assessment.profile);
        println!("Baseline ready: {}", assessment.ready);
        for item in &assessment.recommendations {
            let selection = item
                .action
                .as_ref()
                .map(|action| {
                    if action.selected_by_default {
                        "preselected / user may deselect"
                    } else {
                        "not selected"
                    }
                })
                .unwrap_or("no action authority");
            println!(
                "- {}: {:?} -> {:?} ({selection})",
                component_label(item.component),
                item.state,
                item.recommendation
            );
            for warning in &item.warnings {
                println!("    warning: {warning}");
            }
        }
        println!("Machine changes: none");
    }
    Ok(())
}

fn format_optional_bool(value: Option<bool>) -> &'static str {
    match value {
        Some(true) => "enabled",
        Some(false) => "disabled",
        None => "unknown",
    }
}

fn format_optional_presence(value: Option<bool>) -> &'static str {
    match value {
        Some(true) => "yes",
        Some(false) => "no",
        None => "unknown",
    }
}
