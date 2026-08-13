#[path = "../state_assess_cli.rs"]
mod state_assess_cli;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    state_assess_cli::run()
}
