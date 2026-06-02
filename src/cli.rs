use crate::aggregator::{aggregate, detect_cross_correlations};
use crate::ai::create_backend;
use crate::parser::{detect_format, load_config_file, parse_log_file};
use crate::renderer::render_multi_source;
use crate::types::{Model, MultiSourceSummary, ParseConfig, SourceAnalysis};
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
    /// 日志文件路径（可指定多个，自动关联分析）
    #[arg(required = true, num_args = 1..)]
    pub files: Vec<PathBuf>,

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

    /// 分析后自动打开交互式 TUI 浏览器
    #[arg(long, default_value_t = false)]
    pub tui: bool,

    // ── 自定义解析规则 ──
    /// 自定义时间戳格式 (strftime 风格)
    #[arg(long)]
    pub parse_timestamp_format: Option<String>,

    /// JSON 日志的级别字段名 (默认自动检测)
    #[arg(long)]
    pub parse_level_field: Option<String>,

    /// JSON 日志的消息字段名 (默认自动检测)
    #[arg(long)]
    pub parse_message_field: Option<String>,

    /// 纯文本日志级别提取的正则表达式
    #[arg(long)]
    pub parse_level_pattern: Option<String>,

    /// 堆栈跟踪标记行 (如 "  at "、"\t")
    #[arg(long)]
    pub parse_stack_marker: Option<String>,

    /// 自定义解析规则文件路径 (TOML 格式)
    #[arg(long)]
    pub rules_file: Option<PathBuf>,
}

impl AnalyzeArgs {
    /// Build a ParseConfig from CLI flags
    pub fn parse_config_cli(&self) -> Option<ParseConfig> {
        let cfg = ParseConfig {
            timestamp_format: self.parse_timestamp_format.clone(),
            level_field: self.parse_level_field.clone(),
            message_field: self.parse_message_field.clone(),
            level_pattern: self.parse_level_pattern.clone(),
            stack_trace_marker: self.parse_stack_marker.clone(),
        };
        if cfg.timestamp_format.is_none()
            && cfg.level_field.is_none()
            && cfg.message_field.is_none()
            && cfg.level_pattern.is_none()
            && cfg.stack_trace_marker.is_none()
        {
            None
        } else {
            Some(cfg)
        }
    }

    /// Build final ParseConfig: CLI > config file > auto-detect
    pub fn parse_config(&self) -> ParseConfig {
        let file_cfg = load_config_file(self.rules_file.as_deref());
        ParseConfig::merge(self.parse_config_cli(), file_cfg)
    }
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

    // ── 自定义解析规则 ──
    /// 自定义时间戳格式 (strftime 风格)
    #[arg(long)]
    pub parse_timestamp_format: Option<String>,

    /// JSON 日志的级别字段名 (默认自动检测)
    #[arg(long)]
    pub parse_level_field: Option<String>,

    /// JSON 日志的消息字段名 (默认自动检测)
    #[arg(long)]
    pub parse_message_field: Option<String>,

    /// 纯文本日志级别提取的正则表达式
    #[arg(long)]
    pub parse_level_pattern: Option<String>,

    /// 堆栈跟踪标记行 (如 "  at "、"\t")
    #[arg(long)]
    pub parse_stack_marker: Option<String>,

    /// 自定义解析规则文件路径 (TOML 格式)
    #[arg(long)]
    pub rules_file: Option<PathBuf>,
}

impl WatchArgs {
    /// Build a ParseConfig from CLI flags
    pub fn parse_config_cli(&self) -> Option<ParseConfig> {
        let cfg = ParseConfig {
            timestamp_format: self.parse_timestamp_format.clone(),
            level_field: self.parse_level_field.clone(),
            message_field: self.parse_message_field.clone(),
            level_pattern: self.parse_level_pattern.clone(),
            stack_trace_marker: self.parse_stack_marker.clone(),
        };
        if cfg.timestamp_format.is_none()
            && cfg.level_field.is_none()
            && cfg.message_field.is_none()
            && cfg.level_pattern.is_none()
            && cfg.stack_trace_marker.is_none()
        {
            None
        } else {
            Some(cfg)
        }
    }

    /// Build final ParseConfig: CLI > config file > auto-detect
    pub fn parse_config(&self) -> ParseConfig {
        let file_cfg = load_config_file(self.rules_file.as_deref());
        ParseConfig::merge(self.parse_config_cli(), file_cfg)
    }
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

    /// 使用深度/更强模型进行分析
    #[arg(long, default_value_t = false)]
    pub deep: bool,

    /// 强制日志格式
    #[arg(short, long, default_value = "auto")]
    pub format: FormatArg,

    /// 最低日志级别
    #[arg(long, default_value = "info")]
    pub min_level: LevelArg,

    // ── 自定义解析规则 ──
    /// 自定义时间戳格式 (strftime 风格)
    #[arg(long)]
    pub parse_timestamp_format: Option<String>,

    /// JSON 日志的级别字段名 (默认自动检测)
    #[arg(long)]
    pub parse_level_field: Option<String>,

    /// JSON 日志的消息字段名 (默认自动检测)
    #[arg(long)]
    pub parse_message_field: Option<String>,

    /// 纯文本日志级别提取的正则表达式
    #[arg(long)]
    pub parse_level_pattern: Option<String>,

    /// 堆栈跟踪标记行 (如 "  at "、"\t")
    #[arg(long)]
    pub parse_stack_marker: Option<String>,

    /// 自定义解析规则文件路径 (TOML 格式)
    #[arg(long)]
    pub rules_file: Option<PathBuf>,
}

impl InteractiveArgs {
    /// Build a ParseConfig from CLI flags
    pub fn parse_config_cli(&self) -> Option<ParseConfig> {
        let cfg = ParseConfig {
            timestamp_format: self.parse_timestamp_format.clone(),
            level_field: self.parse_level_field.clone(),
            message_field: self.parse_message_field.clone(),
            level_pattern: self.parse_level_pattern.clone(),
            stack_trace_marker: self.parse_stack_marker.clone(),
        };
        if cfg.timestamp_format.is_none()
            && cfg.level_field.is_none()
            && cfg.message_field.is_none()
            && cfg.level_pattern.is_none()
            && cfg.stack_trace_marker.is_none()
        {
            None
        } else {
            Some(cfg)
        }
    }

    /// Build final ParseConfig: CLI > config file > auto-detect
    pub fn parse_config(&self) -> ParseConfig {
        let file_cfg = load_config_file(self.rules_file.as_deref());
        ParseConfig::merge(self.parse_config_cli(), file_cfg)
    }
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
            let deep = args.deep;
            let min_level = args.min_level.to_level();
            let format_override = args.format.to_format();
            let model: Model = args.model.clone().into();

            // Load parse config (CLI flags > config file > auto-detect)
            let parse_config = args.parse_config();
            let has_custom_rules = parse_config.timestamp_format.is_some()
                || parse_config.level_field.is_some()
                || parse_config.message_field.is_some()
                || parse_config.level_pattern.is_some()
                || parse_config.stack_trace_marker.is_some();

            // Validate all files exist
            for f in &args.files {
                if !f.exists() {
                    anyhow::bail!("文件不存在: {}", f.display());
                }
            }

            let multi_file = args.files.len() > 1;
            let start = Instant::now();

            // ── Parse each file into a SourceAnalysis ──
            let mut sources: Vec<SourceAnalysis> = Vec::new();
            for file_path in &args.files {
                eprintln!("🔍 解析 {}...", file_path.display());
                let entries = parse_log_file(file_path, format_override)?;
                let entries = crate::types::filter_by_level(entries, min_level);
                let summary = aggregate(&entries);

                // Detect actual format
                let format = {
                    let sample: Vec<String> = entries
                        .iter()
                        .take(10)
                        .map(|e| e.raw_line.clone())
                        .collect();
                    detect_format(&sample)
                };

                let name = file_path
                    .file_name()
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or_else(|| file_path.display().to_string());

                eprintln!(
                    "   已解析 {} 条日志，{} 个错误分组 ({} 异常)",
                    entries.len(),
                    summary.error_groups.len(),
                    summary.anomalies.len()
                );

                sources.push(SourceAnalysis {
                    name,
                    path: file_path.clone(),
                    entries,
                    summary,
                    format,
                });
            }

            if has_custom_rules {
                eprintln!("   📋 已加载自定义解析规则");
            }

            let correlations = if multi_file {
                eprintln!("🔗 检测跨源关联...");
                detect_cross_correlations(&sources)
            } else {
                Vec::new()
            };

            let multi_summary = MultiSourceSummary {
                sources,
                correlations,
            };

            // Use the first source's summary for AI analysis
            if let Some(first) = multi_summary.sources.first() {
                let backend = create_backend(model, deep).await?;
                eprintln!(
                    "🤖 正在用 {} ({}) 分析...",
                    backend.model_name(),
                    backend.actual_model(deep)
                );
                let response = crate::ai::with_retry(|| backend.analyze(&first.summary), |n, e| {
                    eprintln!("   ⚠️  第 {n} 次尝试失败: {e}，重试中...");
                })
                .await?;

                let elapsed = start.elapsed().as_secs_f64();
                let model_name = backend.model_name().to_string();

                // Render multi-source output
                render_multi_source(&multi_summary, Some(&response), elapsed, &model_name);

                // HTML export
                if let Some(ref output_path) = args.output {
                    let html = crate::renderer_html::render_report_html(
                        &first.summary,
                        &response,
                        elapsed,
                        &model_name,
                    );
                    std::fs::write(output_path, &html)?;
                    eprintln!("   HTML 报告已保存到 {}", output_path.display());
                }

                // TUI mode
                if args.tui {
                    crate::tui::run_interactive_multi(multi_summary, model, deep)?;
                }
            } else {
                eprintln!("⚠️ 未解析到任何日志条目");
            }

            Ok(())
        }
        Command::Watch(args) => crate::watcher::watch_file(args).await,
        Command::Interactive(args) => {
            // Interactive is synchronous (TUI needs terminal control)
            let model: Model = args.model.into();
            crate::tui::run_interactive(args.file, args.live, model, args.deep)
        }
    }
}
