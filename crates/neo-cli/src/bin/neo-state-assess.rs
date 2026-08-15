#[path = "../state_assess_v2.rs"]
mod state_assess_cli;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    state_assess_cli::run()
}
