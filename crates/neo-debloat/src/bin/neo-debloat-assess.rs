use neo_debloat::{assess_debloat, DebloatCatalogue, DebloatEvidence, DebloatProfile};
use std::error::Error;
use std::fs;
use std::io;
use std::path::PathBuf;
use std::str::FromStr;

fn main() {
    if let Err(error) = run() {
        eprintln!("neo-debloat-assess: {error}");
        std::process::exit(2);
    }
}

fn run() -> Result<(), Box<dyn Error>> {
    let mut args = std::env::args().skip(1).collect::<Vec<_>>();
    let json = if let Some(position) = args.iter().position(|value| value == "--json") {
        args.remove(position);
        true
    } else {
        false
    };

    if args.len() != 4 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "usage: neo-debloat-assess <catalogue.json> <evidence.json> <profile> <id[,id...]> [--json]",
        )
        .into());
    }

    let catalogue_path = PathBuf::from(&args[0]);
    let evidence_path = PathBuf::from(&args[1]);
    let profile = DebloatProfile::from_str(&args[2])?;
    let selected_ids = args[3]
        .split(',')
        .map(ToOwned::to_owned)
        .collect::<Vec<_>>();

    let catalogue: DebloatCatalogue = serde_json::from_str(&fs::read_to_string(catalogue_path)?)?;
    let evidence: DebloatEvidence = serde_json::from_str(&fs::read_to_string(evidence_path)?)?;
    let assessment = assess_debloat(&catalogue, &evidence, profile, &selected_ids)?;

    if json {
        println!("{}", serde_json::to_string_pretty(&assessment)?);
    } else {
        println!("Neo Driver — Debloat assessment");
        println!("Profile: {:?}", assessment.profile);
        println!("Machine changes: none");
        for item in &assessment.items {
            println!(
                "{} | {} | {:?} | installed={:?} | provisioned={:?}",
                item.id, item.package_id, item.disposition, item.installed, item.provisioned
            );
            for reason in &item.reasons {
                println!("  - {reason}");
            }
        }
    }

    Ok(())
}
