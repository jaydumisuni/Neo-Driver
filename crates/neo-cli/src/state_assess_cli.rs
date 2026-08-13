#[path = "state_readback_windows.rs"]
mod state_readback_windows;

use clap::{Parser, Subcommand};
use neo_state_plan::{
    assess_tweaks, resolve_selected_evidence, StateBindings, TweakCatalogue, TweakEvidence,
};
use std::error::Error;
use std::path::PathBuf;

#[derive(Debug, Parser)]
#[command(name = "neo-state-assess")]
pub struct Args {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    Validate {
        catalogue: PathBuf,
        #[arg(long)]
        json: bool,
    },
    Assess {
        #[arg(long)]
        catalogue: PathBuf,
        #[arg(long)]
        evidence: PathBuf,
        #[arg(long = "select", required = true)]
        select: Vec<String>,
        #[arg(long, default_value = "NEO-STATE-ASSESSMENT")]
        mission_id: String,
        #[arg(long)]
        json: bool,
    },
    Live {
        #[arg(long)]
        catalogue: PathBuf,
        #[arg(long)]
        bindings: PathBuf,
        #[arg(long = "select", required = true)]
        select: Vec<String>,
        #[arg(long, default_value = "NEO-LIVE-STATE-ASSESSMENT")]
        mission_id: String,
        #[arg(long)]
        json: bool,
    },
}

pub fn run() -> Result<(), Box<dyn Error>> {
    match Args::parse().command {
        Command::Validate { catalogue, json } => {
            let catalogue = TweakCatalogue::read_json(catalogue)?;
            if json {
                println!("{}", serde_json::to_string_pretty(&catalogue)?);
            } else {
                println!("Phase 9 state catalogue: PASS");
                println!("Entries: {}", catalogue.tweaks.len());
                println!("Machine changes: none");
            }
        }
        Command::Assess {
            catalogue,
            evidence,
            select,
            mission_id,
            json,
        } => {
            let catalogue = TweakCatalogue::read_json(catalogue)?;
            let evidence = TweakEvidence::read_json(evidence)?;
            let report = assess_tweaks(&catalogue, &evidence, &select, mission_id)?;
            print_report("Phase 9 state assessment", &report, json)?;
        }
        Command::Live {
            catalogue,
            bindings,
            select,
            mission_id,
            json,
        } => {
            let catalogue = TweakCatalogue::read_json(catalogue)?;
            let bindings: StateBindings =
                serde_json::from_str(&std::fs::read_to_string(bindings)?)?;

            #[cfg(windows)]
            let captured = state_readback_windows::capture_live(&bindings)?;
            #[cfg(not(windows))]
            let captured = {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::Unsupported,
                    "live state assessment requires Windows",
                )
                .into());
            };

            let evidence = resolve_selected_evidence(&catalogue, &bindings, &captured, &select)?;
            let report = assess_tweaks(&catalogue, &evidence, &select, mission_id)?;
            if json {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&serde_json::json!({
                        "captured": captured,
                        "assessment": report,
                        "machine_changes": "none"
                    }))?
                );
            } else {
                print_report("Phase 10 live state assessment", &report, false)?;
            }
        }
    }
    Ok(())
}

fn print_report(
    label: &str,
    report: &neo_state_plan::TweakAssessment,
    json: bool,
) -> Result<(), Box<dyn Error>> {
    if json {
        println!("{}", serde_json::to_string_pretty(report)?);
    } else {
        println!("{label}: PASS");
        println!("Items: {}", report.items.len());
        let satisfied = report
            .items
            .iter()
            .filter(|item| item.already_satisfied)
            .count();
        println!("Already satisfied: {satisfied}");
        println!("Machine changes: none");
    }
    Ok(())
}
