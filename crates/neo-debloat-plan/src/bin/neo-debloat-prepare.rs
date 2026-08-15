use neo_debloat::{DebloatCatalogue, DebloatEvidence, DebloatProfile};
use neo_debloat_plan::{prepare_debloat_transaction_from_evidence, ExactAppxInventory};
use std::env;
use std::fs;
use std::str::FromStr;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = env::args().collect::<Vec<_>>();
    if args.len() < 8 {
        return Err("usage: neo-debloat-prepare <catalogue.json> <evidence.json> <exact-inventory.json> <profile> <selected-id> <mission-id> <--json>".into());
    }
    let catalogue: DebloatCatalogue = serde_json::from_str(&fs::read_to_string(&args[1])?)?;
    let evidence: DebloatEvidence = serde_json::from_str(&fs::read_to_string(&args[2])?)?;
    let inventory: ExactAppxInventory = serde_json::from_str(&fs::read_to_string(&args[3])?)?;
    let profile = DebloatProfile::from_str(&args[4])?;
    let selected = vec![args[5].clone()];
    let prepared = prepare_debloat_transaction_from_evidence(
        &catalogue,
        &evidence,
        &inventory,
        profile,
        &selected,
        args[6].clone(),
    )?;
    if args[7] == "--json" {
        println!("{}", serde_json::to_string_pretty(&prepared)?);
    } else {
        println!(
            "Prepared transaction: {}",
            prepared.transaction().transaction_id()
        );
        println!("Plan fingerprint: {}", prepared.plan_fingerprint());
        println!("Machine changes: none");
    }
    Ok(())
}
