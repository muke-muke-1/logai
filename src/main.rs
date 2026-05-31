pub mod types;
pub mod parser;
pub mod aggregator;
pub mod ai;
pub mod renderer;
pub mod cli;

#[tokio::main]
async fn main() {
    if let Err(e) = cli::run().await {
        eprintln!("❌ Error: {}", e);
        std::process::exit(1);
    }
}
