use clap::{Parser, Subcommand, ValueEnum};
use neo_catalogue::Catalogue;
use neo_core::{MissionPlan, UserDepth, UserIntent};
use neo_device::DeviceRecord;
use neo_match::{match_device, MatchContext};
use neo_probe::scan_current_machine;
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
    /// Show the current implementation boundary.
    Status,
}

#[derive(Debug, Subcommand)]
enum CatalogueCommand {
    /// Validate a Neo catalogue JSON file without installing or downloading anything.
    Validate {
        path: PathBuf,
        /// Emit the normalized catalogue as JSON.
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
                let layout = VaultLayout::new(mode, &app_root).map_err(|error| error.to_string())?;
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
            VaultCommand::Audit {
                app_root,
                portable,
            } => {
                let mode = if portable {
                    VaultMode::Portable
                } else {
                    VaultMode::Installed
                };
                let layout = VaultLayout::new(mode, &app_root).map_err(|error| error.to_string())?;
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
            println!("Neo Driver implementation phase: managed package vault foundation");
            println!("Public machine mutation: intentionally disabled");
            println!("Network package acquisition: intentionally disabled");
            println!("Managed vault root: Builder/portable root + NeoData");
            println!("Model dependency: none");
            Ok(())
        }
    }
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
