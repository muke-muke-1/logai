pub mod aggregator;
pub mod ai;
pub mod cli;
pub mod errors;
pub mod parser;
pub mod renderer;
pub mod renderer_html;
pub mod tui;
pub mod types;
pub mod watcher;

#[tokio::main]
async fn main() {
    if let Err(e) = cli::run().await {
        eprintln!("❌ Error: {}", e);
        std::process::exit(1);
    }
}
