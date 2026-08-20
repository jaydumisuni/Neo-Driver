use clap::Subcommand;
use neo_driver_repair::{
    assess_driver_repair_evidence, inspect_windows_driver_repair, DriverRepairEvidence,
};
use neo_repair::{inspect_windows_features, inspect_windows_repair_health};
use std::path::PathBuf;

#[derive(Debug, Subcommand)]
pub(crate) enum RepairCommand {
    /// Inspect component-store and protected-system-file health without repairing anything.
    Inspect {
        /// Emit machine-readable JSON including bounded command evidence.
        #[arg(long)]
        json: bool,
    },
    /// Inspect the fixed Phase 21 Windows optional-feature catalogue without changing features.
    Features {
        /// Emit machine-readable JSON including bounded command evidence.
        #[arg(long)]
        json: bool,
    },
    /// Assess present-device Driver Store / PnP repair readiness without changing any device.
    Drivers {
        /// Optional normalized Phase 22 evidence JSON. Omit on Windows to inspect the live host.
        #[arg(long)]
        evidence: Option<PathBuf>,
        /// Emit machine-readable JSON including exact current binding/package evidence.
        #[arg(long)]
        json: bool,
    },
}

pub(crate) fn run(command: RepairCommand) -> Result<(), String> {
    match command {
        RepairCommand::Inspect { json } => {
            let report = inspect_windows_repair_health().map_err(|error| error.to_string())?;
            if json {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&report).map_err(|error| error.to_string())?
                );
            } else {
                println!("Neo read-only Windows repair inspection");
                println!("---------------------------------------");
                println!("Component store: {:?}", report.component_store.state);
                println!("  {}", report.component_store.detail);
                println!("Protected system files: {:?}", report.system_files.state);
                println!("  {}", report.system_files.detail);
                println!("Machine changes: none");
            }
        }
        RepairCommand::Features { json } => {
            let report = inspect_windows_features().map_err(|error| error.to_string())?;
            if json {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&report).map_err(|error| error.to_string())?
                );
            } else {
                println!("Neo read-only Windows optional-feature inspection");
                println!("-------------------------------------------------");
                for feature in &report.features {
                    println!("- {}: {:?}", feature.feature.title(), feature.state);
                    println!("  {}", feature.detail);
                }
                println!("Machine changes: none");
            }
        }
        RepairCommand::Drivers { evidence, json } => {
            let report = match evidence {
                Some(path) => {
                    let raw = std::fs::read_to_string(&path).map_err(|error| error.to_string())?;
                    let evidence = DriverRepairEvidence::from_json_str(&raw)
                        .map_err(|error| error.to_string())?;
                    assess_driver_repair_evidence(evidence).map_err(|error| error.to_string())?
                }
                None => inspect_windows_driver_repair().map_err(|error| error.to_string())?,
            };
            if json {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&report).map_err(|error| error.to_string())?
                );
            } else {
                println!("Neo read-only Driver Store / PnP repair assessment");
                println!("---------------------------------------------------");
                for item in &report.assessments {
                    println!(
                        "- {}: {:?} -> {:?}",
                        item.instance_id, item.state, item.route
                    );
                    println!("  {}", item.detail);
                }
                println!("Evidence SHA-256: {}", report.source_evidence_sha256);
                println!("Machine changes: none");
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::CommandFactory;

    #[derive(Debug, clap::Parser)]
    struct Probe {
        #[command(subcommand)]
        command: RepairCommand,
    }

    #[test]
    fn cli_surface_contains_only_read_only_commands() {
        let mut names: Vec<String> = Probe::command()
            .get_subcommands()
            .map(|subcommand| subcommand.get_name().to_string())
            .collect();
        names.sort();
        assert_eq!(
            names,
            vec![
                "drivers".to_string(),
                "features".to_string(),
                "inspect".to_string(),
            ]
        );
    }
}
