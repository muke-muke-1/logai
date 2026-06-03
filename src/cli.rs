use crate::aggregator::{aggregate, detect_cross_correlations};
use crate::ai::create_backend;
use crate::parser::{detect_format, load_config_file, parse_log_file};
use crate::renderer::render_multi_source;
use crate::types::{Model, MultiSourceSummary, ParseConfig, SourceAnalysis};
use clap::{Parser, ValueEnum};
use std::path::PathBuf;
use std::time::Instant;

/// 内嵌示例日志（无 API key 也可试用的演示数据）
pub const SAMPLE_LOG: &str = r#"{"time":"2026-06-03T08:03:12Z","level":"error","message":"Connection pool exhausted: timeout after 30s","stack":"  at pool.acquire (db/pool.go:47)\n  at query.execute (db/query.go:128)"}
{"time":"2026-06-03T08:03:15Z","level":"error","message":"Connection pool exhausted: timeout after 30s","stack":"  at pool.acquire (db/pool.go:47)"}
{"time":"2026-06-03T08:03:18Z","level":"error","message":"Connection pool exhausted: timeout after 31s"}
{"time":"2026-06-03T08:04:01Z","level":"warn","message":"Slow query detected: 2.3s SELECT * FROM orders"}
{"time":"2026-06-03T08:04:45Z","level":"error","message":"SSL certificate verification failed: certificate expired"}
{"time":"2026-06-03T08:05:00Z","level":"error","message":"Connection pool exhausted: timeout after 30s"}
{"time":"2026-06-03T08:05:12Z","level":"info","message":"Health check passed"}
{"time":"2026-06-03T08:06:00Z","level":"error","message":"SSL certificate verification failed"}
{"time":"2026-06-03T08:06:30Z","level":"warn","message":"Memory usage at 85%"}
{"time":"2026-06-03T08:07:00Z","level":"error","message":"Connection pool exhausted: timeout after 32s","stack":"  at pool.acquire (db/pool.go:47)"}"#;

#[derive(Parser)]
#[command(
    name = "logai",
    about = "AI 驱动的日志分析 CLI",
    subcommand_required = false,
    args_conflicts_with_subcommands = true
)]
pub struct Cli {
    /// 日志文件路径（不指定子命令时默认打开交互式 TUI）
    #[arg(required = false)]
    pub file: Option<PathBuf>,

    #[command(subcommand)]
    pub command: Option<Command>,
}

#[derive(clap::Subcommand)]
pub enum Command {
    /// 分析日志文件（支持多文件关联分析）
    Analyze(AnalyzeArgs),
    /// 实时监听日志文件，周期性 AI 分析
    Watch(WatchArgs),
    /// 交互式 TUI 日志浏览器
    Interactive(InteractiveArgs),
    /// 生成 Shell 补全脚本
    Completions(CompletionsArgs),
    /// 在当前目录生成 logai.toml 配置模板
    Init(InitArgs),
}

#[derive(clap::Args)]
pub struct CompletionsArgs {
    /// Shell 类型
    #[arg(value_enum)]
    pub shell: clap_complete::Shell,
}

#[derive(clap::Args)]
pub struct InitArgs {
    /// 强制覆盖已存在的 logai.toml
    #[arg(long, default_value_t = false)]
    pub force: bool,
}

#[derive(clap::Args)]
pub struct AnalyzeArgs {
    /// 日志文件路径（可指定多个，自动关联分析）
    #[arg(required = true, num_args = 1..)]
    pub files: Vec<PathBuf>,

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

    /// 导出报告到 HTML 文件
    #[arg(long)]
    pub output: Option<PathBuf>,

    /// 分析后自动打开交互式 TUI 浏览器
    #[arg(long, default_value_t = false)]
    pub tui: bool,

    /// 仅解析聚合，跳过 AI 分析（无需 API key）
    #[arg(long, default_value_t = false)]
    pub dry_run: bool,

    /// 使用内嵌示例日志演示（无需文件或 API key）
    #[arg(long, default_value_t = false, conflicts_with = "files")]
    pub sample: bool,

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

    // Default mode: no subcommand with a file → TUI
    if cli.command.is_none() {
        if let Some(file) = cli.file {
            if !file.exists() {
                anyhow::bail!("文件不存在: {}", file.display());
            }
            return crate::tui::run_interactive(file, false, Model::Auto, false);
        }
        // No subcommand and no file → print help
        Cli::parse_from(["logai", "--help"]);
        return Ok(());
    }

    match cli.command.unwrap() {
        Command::Completions(args) => {
            let mut cmd = <Cli as clap::CommandFactory>::command();
            let name = cmd.get_name().to_string();
            clap_complete::generate(args.shell, &mut cmd, name, &mut std::io::stdout());
            Ok(())
        }
        Command::Init(args) => {
            let path = std::env::current_dir()?.join("logai.toml");
            if path.exists() && !args.force {
                anyhow::bail!(
                    "logai.toml 已存在。使用 --force 覆盖，或手动编辑: {}",
                    path.display()
                );
            }
            let template = r#"# logai 配置文件
# 优先级: CLI 标志 > 本文件 > 自动检测

[parse]
# 自定义时间戳格式 (strftime 风格)
# timestamp_format = "%Y-%m-%d %H:%M:%S"

# JSON 日志的级别字段名
# level_field = "severity"

# JSON 日志的消息字段名
# message_field = "msg"

# 纯文本日志级别提取的正则表达式
# level_pattern = "(?i)\\b(error|warn|info|debug|trace)\\b"

# 堆栈跟踪标记行
# stack_trace_marker = "  at "
"#;
            std::fs::write(&path, template)?;
            eprintln!("✅ 已创建配置文件: {}", path.display());
            Ok(())
        }
        Command::Analyze(args) => {
            let deep = args.deep;
            let min_level = args.min_level.to_level();
            let format_override = args.format.to_format();
            let model: Model = args.model.clone().into();
            let dry_run = args.dry_run;
            let use_sample = args.sample;

            // Load parse config (CLI flags > config file > auto-detect)
            let parse_config = args.parse_config();
            let has_custom_rules = parse_config.timestamp_format.is_some()
                || parse_config.level_field.is_some()
                || parse_config.message_field.is_some()
                || parse_config.level_pattern.is_some()
                || parse_config.stack_trace_marker.is_some();

            if use_sample {
                eprintln!("📋 使用内嵌示例日志（无需 API key）");
            }

            // Determine file list: sample or real files
            let file_paths: Vec<PathBuf> = if use_sample {
                // Write sample log to a temp file
                let tmp_dir = std::env::temp_dir().join("logai");
                std::fs::create_dir_all(&tmp_dir)?;
                let sample_path = tmp_dir.join("sample.log");
                std::fs::write(&sample_path, SAMPLE_LOG)?;
                vec![sample_path]
            } else {
                // Validate all files exist
                for f in &args.files {
                    if !f.exists() {
                        anyhow::bail!("文件不存在: {}", f.display());
                    }
                }
                args.files.clone()
            };

            let multi_file = file_paths.len() > 1;
            let start = Instant::now();

            // ── Parse each file into a SourceAnalysis ──
            let mut sources: Vec<SourceAnalysis> = Vec::new();
            for file_path in &file_paths {
                eprintln!("🔍 解析 {}...", file_path.display());
                let entries = parse_log_file(file_path, format_override)?;
                let entries = crate::types::filter_by_level(entries, min_level);
                let summary = aggregate(&entries);

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

            // ── Dry run: render without AI ──
            if dry_run {
                let elapsed = start.elapsed().as_secs_f64();
                eprintln!("📊 仅解析聚合模式（--dry-run），跳过 AI 分析");
                render_multi_source(&multi_summary, None, elapsed, "无 (dry-run)");

                // HTML export still works in dry-run mode
                if let Some(ref output_path) = args.output {
                    if let Some(first) = multi_summary.sources.first() {
                        let empty_response = crate::types::AiResponse {
                            root_causes: vec![],
                            summary: "(dry-run — 未执行 AI 分析)".into(),
                            fix_suggestions: vec![],
                            confidence: 0.0,
                        };
                        let html = crate::renderer_html::render_report_html(
                            &first.summary,
                            &empty_response,
                            elapsed,
                            "无 (dry-run)",
                        );
                        std::fs::write(output_path, &html)?;
                        eprintln!("   HTML 报告已保存到 {}", output_path.display());
                    }
                }
                return Ok(());
            }

            // Use the first source's summary for AI analysis
            if let Some(first) = multi_summary.sources.first() {
                let backend = create_backend(model, deep).await?;
                eprintln!(
                    "🤖 正在用 {} ({}) 分析...",
                    backend.model_name(),
                    backend.actual_model(deep)
                );
                let response = crate::ai::with_retry(
                    || backend.analyze(&first.summary),
                    |n, e| {
                        eprintln!("   ⚠️  第 {n} 次尝试失败: {e}，重试中...");
                    },
                )
                .await?;

                let elapsed = start.elapsed().as_secs_f64();
                let model_name = backend.model_name().to_string();

                render_multi_source(&multi_summary, Some(&response), elapsed, &model_name);

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

                if args.tui {
                    crate::tui::run_interactive_multi(multi_summary, model, deep)?;
                }
            } else {
                eprintln!("⚠️ 未解析到任何日志条目");
            }

            // Clean up sample temp file
            if use_sample {
                if let Some(p) = file_paths.first() {
                    let _ = std::fs::remove_file(p);
                }
            }

            Ok(())
        }
        Command::Watch(args) => crate::watcher::watch_file(args).await,
        Command::Interactive(args) => {
            let model: Model = args.model.into();
            crate::tui::run_interactive(args.file, args.live, model, args.deep)
        }
    }
}
