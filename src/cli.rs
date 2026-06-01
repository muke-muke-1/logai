use crate::aggregator::aggregate;
use crate::ai::create_backend;
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
    /// 分析日志文件
    Analyze(AnalyzeArgs),
    /// 实时监听日志文件，周期性 AI 分析
    Watch(WatchArgs),
    /// 交互式 TUI 日志浏览器
    Interactive(InteractiveArgs),
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

    /// 导出报告到 HTML 文件
    #[arg(long)]
    pub output: Option<PathBuf>,
}

#[derive(clap::Args)]
pub struct WatchArgs {
    /// 日志文件路径
    pub file: PathBuf,

    /// AI 模型后端（默认自动检测）
    #[arg(short, long, default_value = "auto")]
    pub model: ModelArg,

    /// 使用深度/更强模型进行分析
    #[arg(long, default_value_t = false)]
    pub deep: bool,

    /// 强制日志格式
    #[arg(short, long, default_value = "auto")]
    pub format: FormatArg,

    /// 最低日志级别
    #[arg(long, default_value = "info")]
    pub min_level: LevelArg,

    /// 时间窗口（秒），默认 30
    #[arg(long, default_value_t = 30)]
    pub window: u64,

    /// 启动时分析的最大行数（默认 10000）
    #[arg(long, default_value_t = 10000)]
    pub max_initial_lines: usize,
}

#[derive(clap::Args)]
pub struct InteractiveArgs {
    /// 日志文件路径
    pub file: PathBuf,

    /// 实时模式：监听文件变化，自动刷新 TUI
    #[arg(long, default_value_t = false)]
    pub live: bool,

    /// AI 模型后端（默认自动检测）
    #[arg(short, long, default_value = "auto")]
    pub model: ModelArg,

    /// 强制日志格式
    #[arg(short, long, default_value = "auto")]
    pub format: FormatArg,

    /// 最低日志级别
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
    pub fn to_format(&self) -> Option<crate::types::Format> {
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
    pub fn to_level(&self) -> crate::types::Level {
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
            eprintln!(
                "   Parsed {} log entries (after --min-level filter)",
                entries.len()
            );

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

            // HTML export
            if let Some(ref output_path) = args.output {
                let html = crate::renderer_html::render_report_html(
                    &summary,
                    &response,
                    elapsed,
                    backend.model_name(),
                );
                std::fs::write(output_path, &html)?;
                eprintln!("   HTML 报告已保存到 {}", output_path.display());
            }
            Ok(())
        }
        Command::Watch(args) => crate::watcher::watch_file(args).await,
        Command::Interactive(args) => {
            // Interactive is synchronous (TUI needs terminal control)
            crate::tui::run_interactive(args.file, args.live)
        }
    }
}
