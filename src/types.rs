use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

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

/// 日志格式
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Format {
    Json,
    PlainText,
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
