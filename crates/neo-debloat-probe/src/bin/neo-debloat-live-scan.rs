use neo_debloat::DebloatCatalogue;
use neo_debloat_probe::scan_current_debloat_evidence;
use std::error::Error;
use std::fs;
use std::io;
use std::path::PathBuf;

fn main() {
    if let Err(error) = run() {
        eprintln!("neo-debloat-live-scan: {error}");
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
    if args.len() != 1 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "usage: neo-debloat-live-scan <catalogue.json> [--json]",
        )
        .into());
    }

    let catalogue_path = PathBuf::from(&args[0]);
    let catalogue: DebloatCatalogue = serde_json::from_str(&fs::read_to_string(catalogue_path)?)?;
    let report = scan_current_debloat_evidence(&catalogue)?;

    if json {
        println!("{}", serde_json::to_string_pretty(&report)?);
    } else {
        println!("Neo Driver — Live debloat inventory");
        println!("Machine changes: none");
        for observation in report.evidence.observations() {
            println!(
                "{} | installed={:?} | provisioned={:?} | version={}",
                observation.package_id,
                observation.installed,
                observation.provisioned,
                observation.version.as_deref().unwrap_or("unknown")
            );
        }
        for warning in &report.warnings {
            println!("warning: {warning}");
        }
    }
    Ok(())
}
