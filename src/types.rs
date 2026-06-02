use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

/// 日志级别
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Level {
    Error,
    Warn,
    Info,
    Debug,
    Trace,
    Unknown,
}

impl std::fmt::Display for Level {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Level::Error => write!(f, "ERROR"),
            Level::Warn => write!(f, "WARN"),
            Level::Info => write!(f, "INFO"),
            Level::Debug => write!(f, "DEBUG"),
            Level::Trace => write!(f, "TRACE"),
            Level::Unknown => write!(f, "UNKNOWN"),
        }
    }
}

impl Level {
    /// Numeric severity: lower = more severe.
    /// Error=0, Warn=1, Info=2, Debug=3, Trace=4, Unknown=5
    pub fn severity(self) -> u8 {
        match self {
            Level::Error => 0,
            Level::Warn => 1,
            Level::Info => 2,
            Level::Debug => 3,
            Level::Trace => 4,
            Level::Unknown => 5,
        }
    }
}

/// Filter log entries by minimum severity level.
/// Keeps entries whose level is at or above `min_level` severity.
pub fn filter_by_level(entries: Vec<LogEntry>, min_level: Level) -> Vec<LogEntry> {
    entries
        .into_iter()
        .filter(|e| {
            let level = e.level.unwrap_or(Level::Unknown);
            level.severity() <= min_level.severity()
        })
        .collect()
}

/// 日志格式
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Format {
    Json,
    PlainText,
}

/// 自定义解析配置
/// Priority: CLI flags > config file > auto-detect
#[derive(Debug, Clone, Default)]
pub struct ParseConfig {
    /// Custom timestamp format string (e.g. "%Y-%m-%d %H:%M:%S")
    pub timestamp_format: Option<String>,
    /// JSON field name for log level (e.g. "severity", "level")
    pub level_field: Option<String>,
    /// JSON field name for the message body (e.g. "message", "msg")
    pub message_field: Option<String>,
    /// Plain-text regex pattern for log level extraction
    pub level_pattern: Option<String>,
    /// Marker string that signals a stack trace continuation line
    pub stack_trace_marker: Option<String>,
}

impl ParseConfig {
    /// Create ParseConfig by merging CLI flags over file config
    pub fn merge(cli: Option<ParseConfig>, file: Option<ParseConfig>) -> Self {
        let file = file.unwrap_or_default();
        let cli = cli.unwrap_or_default();
        ParseConfig {
            timestamp_format: cli.timestamp_format.or(file.timestamp_format),
            level_field: cli.level_field.or(file.level_field),
            message_field: cli.message_field.or(file.message_field),
            level_pattern: cli.level_pattern.or(file.level_pattern),
            stack_trace_marker: cli.stack_trace_marker.or(file.stack_trace_marker),
        }
    }
}

/// 一条解析后的日志
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogEntry {
    pub timestamp: Option<DateTime<Utc>>,
    pub level: Option<Level>,
    pub message: String,
    pub stack_trace: Option<String>,
    pub raw_line: String,
    pub fields: HashMap<String, String>,
    pub line_number: usize,
}

/// 错误组趋势
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Trend {
    Rising,
    Falling,
    Stable,
}

/// 一组相同签名的错误
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorGroup {
    pub signature: String,
    pub count: usize,
    pub first_seen: Option<DateTime<Utc>>,
    pub last_seen: Option<DateTime<Utc>>,
    pub samples: Vec<String>,
    pub stack_trace: Option<String>,
    pub trend: Trend,
}

/// 异常类型
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Anomaly {
    Spike {
        group_index: usize,
        multiplier: f64,
    },
    NewError {
        group_index: usize,
    },
    SilentRecovery {
        group_index: usize,
    },
    PeriodicPattern {
        group_index: usize,
        period_minutes: u32,
    },
}

/// 聚合后的分析摘要
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalysisSummary {
    pub total_lines: usize,
    pub time_start: Option<DateTime<Utc>>,
    pub time_end: Option<DateTime<Utc>>,
    pub error_groups: Vec<ErrorGroup>,
    pub anomalies: Vec<Anomaly>,
    pub level_distribution: HashMap<Level, usize>,
}

/// 严重程度
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Severity {
    Critical,
    High,
    Medium,
    Low,
}

/// AI 返回的根因
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RootCause {
    pub description: String,
    pub evidence: Vec<String>,
    pub severity: Severity,
}

/// AI 返回的修复建议
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FixSuggestion {
    pub action: String,
    pub code_snippet: Option<String>,
    pub reference: Option<String>,
}

/// AI 响应
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiResponse {
    pub root_causes: Vec<RootCause>,
    pub summary: String,
    pub fix_suggestions: Vec<FixSuggestion>,
    pub confidence: f32,
}

/// AI 后端选择
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Model {
    Claude,
    OpenAI,
    DeepSeek,
    Ollama,
    Auto,
}

/// 单个日志源的分析结果
#[derive(Debug, Clone)]
pub struct SourceAnalysis {
    /// 源名称（文件名）
    pub name: String,
    /// 文件路径
    pub path: PathBuf,
    /// 解析出的日志条目
    pub entries: Vec<LogEntry>,
    /// 聚合分析摘要
    pub summary: AnalysisSummary,
    /// 检测到的格式
    pub format: Format,
}

/// 跨源关联
#[derive(Debug, Clone)]
pub struct Correlation {
    /// 源 A 的名称
    pub source_a: String,
    /// 源 B 的名称
    pub source_b: String,
    /// 关联强度 0.0–1.0
    pub score: f32,
    /// 关联原因描述
    pub description: String,
}

/// 多源分析结果
#[derive(Debug, Clone)]
pub struct MultiSourceSummary {
    /// 各源分析
    pub sources: Vec<SourceAnalysis>,
    /// 跨源关联
    pub correlations: Vec<Correlation>,
}

impl MultiSourceSummary {
    pub fn total_errors(&self) -> usize {
        self.sources
            .iter()
            .map(|s| {
                s.summary
                    .level_distribution
                    .get(&Level::Error)
                    .unwrap_or(&0)
            })
            .sum()
    }

    pub fn total_lines(&self) -> usize {
        self.sources.iter().map(|s| s.summary.total_lines).sum()
    }
}
