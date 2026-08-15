use neo_debloat::{DebloatCatalogue, DebloatEvidence, DebloatProfile};
use neo_debloat_plan::{prepare_debloat_transaction_from_evidence, ExactAppxInventory};
use std::env;
use std::fs;
use std::str::FromStr;

const USAGE: &str = "usage: neo-debloat-prepare <catalogue.json> <evidence.json> <exact-inventory.json> <profile> <selected-id> <mission-id> [--json]";

fn json_output_requested(args: &[String]) -> Result<bool, String> {
    match args.len() {
        7 => Ok(false),
        8 if args[7] == "--json" => Ok(true),
        8 => Err(format!("unexpected argument: {}\n{USAGE}", args[7])),
        _ => Err(USAGE.to_string()),
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = env::args().collect::<Vec<_>>();
    let json = json_output_requested(&args)?;
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
    if json {
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

#[cfg(test)]
mod tests {
    use super::json_output_requested;

    fn args(extra: &[&str]) -> Vec<String> {
        [
            "neo-debloat-prepare",
            "catalogue.json",
            "evidence.json",
            "inventory.json",
            "safe-cleanup",
            "appx.contoso.phase15",
            "mission-phase15",
        ]
        .into_iter()
        .chain(extra.iter().copied())
        .map(str::to_string)
        .collect()
    }

    #[test]
    fn text_output_accepts_six_required_positional_arguments() {
        assert!(!json_output_requested(&args(&[])).expect("text mode must be accepted"));
    }

    #[test]
    fn json_output_requires_exact_json_flag() {
        assert!(json_output_requested(&args(&["--json"])).expect("JSON mode must be accepted"));
        assert!(json_output_requested(&args(&["--yaml"])).is_err());
        assert!(json_output_requested(&args(&["--json", "extra"])).is_err());
    }
}
