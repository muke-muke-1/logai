use crate::ai::create_backend;
use crate::aggregator::aggregate;
use crate::parser::parse_log_file;
use crate::renderer::render_report;
use crate::types::Model;
use clap::{Parser, ValueEnum};
use std::path::PathBuf;
use std::time::Instant;

#[derive(Parser)]
#[command(name = "logai", about = "AI-powered log analysis CLI")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(clap::Subcommand)]
pub enum Command {
    /// Analyze a log file
    Analyze(AnalyzeArgs),
}

#[derive(clap::Args)]
pub struct AnalyzeArgs {
    /// Log file path
    pub file: PathBuf,

    /// AI model backend (auto-detect by default)
    #[arg(short, long, default_value = "auto")]
    pub model: ModelArg,

    /// Use deep/stronger model for analysis
    #[arg(long, default_value_t = false)]
    pub deep: bool,

    /// Force log format
    #[arg(short, long, default_value = "auto")]
    pub format: FormatArg,

    /// Minimum log level to include
    #[arg(long, default_value = "info")]
    pub min_level: LevelArg,
}

#[derive(Clone, ValueEnum)]
pub enum ModelArg {
    #[value(name = "claude")]
    Claude,
    #[value(name = "openai")]
    OpenAI,
    #[value(name = "deepseek")]
    DeepSeek,
    #[value(name = "ollama")]
    Ollama,
    #[value(name = "auto")]
    Auto,
}

#[derive(Clone, ValueEnum)]
pub enum FormatArg {
    Json,
    Text,
    Auto,
}

impl FormatArg {
    fn to_format(self) -> Option<crate::types::Format> {
        match self {
            FormatArg::Json => Some(crate::types::Format::Json),
            FormatArg::Text => Some(crate::types::Format::PlainText),
            FormatArg::Auto => None,
        }
    }
}

#[derive(Clone, ValueEnum)]
pub enum LevelArg {
    Error,
    Warn,
    Info,
    Debug,
}

impl LevelArg {
    fn to_level(self) -> crate::types::Level {
        match self {
            LevelArg::Error => crate::types::Level::Error,
            LevelArg::Warn => crate::types::Level::Warn,
            LevelArg::Info => crate::types::Level::Info,
            LevelArg::Debug => crate::types::Level::Debug,
        }
    }
}

impl From<ModelArg> for Model {
    fn from(arg: ModelArg) -> Self {
        match arg {
            ModelArg::Claude => Model::Claude,
            ModelArg::OpenAI => Model::OpenAI,
            ModelArg::DeepSeek => Model::DeepSeek,
            ModelArg::Ollama => Model::Ollama,
            ModelArg::Auto => Model::Auto,
        }
    }
}

pub async fn run() -> anyhow::Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Command::Analyze(args) => {
            let file_path = &args.file;
            if !file_path.exists() {
                anyhow::bail!("File not found: {}", file_path.display());
            }
            let model: Model = args.model.into();
            let deep = args.deep;

            eprintln!("🔍 Parsing {}...", file_path.display());
            let start = Instant::now();

            let format_override = args.format.to_format();
            let entries = parse_log_file(file_path, format_override)?;
            let min_level = args.min_level.to_level();
            let entries: Vec<_> = entries
                .into_iter()
                .filter(|e| {
                    let level = e.level.unwrap_or(crate::types::Level::Unknown);
                    level.severity() <= min_level.severity()
                })
                .collect();
            eprintln!("   Parsed {} log entries (after --min-level filter)", entries.len());

            let summary = aggregate(&entries);
            eprintln!(
                "   Found {} error groups, {} anomalies",
                summary.error_groups.len(),
                summary.anomalies.len()
            );

            let backend = create_backend(model, deep).await?;
            eprintln!(
                "🤖 Analyzing with {} ({})...",
                backend.model_name(),
                backend.actual_model(deep)
            );
            let response = crate::ai::with_retry(|| backend.analyze(&summary)).await?;

            let elapsed = start.elapsed().as_secs_f64();
            render_report(&summary, &response, elapsed, backend.model_name());
            Ok(())
        }
    }
}
