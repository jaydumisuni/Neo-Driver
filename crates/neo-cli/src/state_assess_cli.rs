use clap::{Parser, Subcommand};
use neo_state_plan::{assess_tweaks, TweakCatalogue, TweakEvidence};
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
            if json {
                println!("{}", serde_json::to_string_pretty(&report)?);
            } else {
                println!("Phase 9 state assessment: PASS");
                println!("Items: {}", report.items.len());
                let satisfied = report.items.iter().filter(|item| item.already_satisfied).count();
                println!("Already satisfied: {satisfied}");
                println!("Machine changes: none");
            }
        }
    }
    Ok(())
}
