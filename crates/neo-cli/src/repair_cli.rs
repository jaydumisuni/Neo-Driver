use clap::Subcommand;
use neo_repair::inspect_windows;

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
}

pub(crate) fn run(command: RepairCommand) -> Result<(), String> {
    let report = inspect_windows().map_err(|error| error.to_string())?;
    match command {
        RepairCommand::Inspect { json } => {
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
            if json {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&report.features)
                        .map_err(|error| error.to_string())?
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
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cli_surface_contains_only_read_only_commands() {
        let inspect = RepairCommand::Inspect { json: false };
        let features = RepairCommand::Features { json: true };
        assert!(matches!(inspect, RepairCommand::Inspect { .. }));
        assert!(matches!(features, RepairCommand::Features { .. }));
    }
}
