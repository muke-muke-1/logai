//! 结构化错误类型，每个错误有唯一错误码和修复提示。
//!
//! 用法:
//! ```ignore
//! use logai::errors::{LogaiError, ErrorCode};
//! return Err(LogaiError::file_not_found("app.log").into());
//! ```

use std::path::Path;

/// 错误码枚举
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorCode {
    /// E001: 文件不存在
    FileNotFound,
    /// E002: 日志解析错误（含行号）
    ParseError,
    /// E003: 缺少 API key
    MissingApiKey,
    /// E004: AI 调用失败（网络/限流/模型）
    AiCallFailed,
    /// E005: 配置错误
    ConfigError,
    /// E006: IO 错误
    IoError,
}

impl ErrorCode {
    pub fn code(&self) -> &str {
        match self {
            ErrorCode::FileNotFound => "E001",
            ErrorCode::ParseError => "E002",
            ErrorCode::MissingApiKey => "E003",
            ErrorCode::AiCallFailed => "E004",
            ErrorCode::ConfigError => "E005",
            ErrorCode::IoError => "E006",
        }
    }

    pub fn description(&self) -> &str {
        match self {
            ErrorCode::FileNotFound => "文件不存在",
            ErrorCode::ParseError => "日志解析错误",
            ErrorCode::MissingApiKey => "缺少 API key",
            ErrorCode::AiCallFailed => "AI 调用失败",
            ErrorCode::ConfigError => "配置错误",
            ErrorCode::IoError => "IO 错误",
        }
    }
}

impl std::fmt::Display for ErrorCode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "[{}] {}", self.code(), self.description())
    }
}

/// 结构化日志分析错误
#[derive(Debug)]
pub struct LogaiError {
    pub code: ErrorCode,
    pub message: String,
    pub hint: Option<String>,
    pub line_number: Option<usize>,
}

impl LogaiError {
    /// E001: 文件不存在
    pub fn file_not_found(path: impl AsRef<Path>) -> Self {
        let path_str = path.as_ref().display().to_string();
        LogaiError {
            code: ErrorCode::FileNotFound,
            message: format!("文件不存在: {}", path_str),
            hint: Some(format!(
                "请确认文件路径正确。当前目录: {}",
                std::env::current_dir()
                    .map(|p| p.display().to_string())
                    .unwrap_or_else(|_| "?".into())
            )),
            line_number: None,
        }
    }

    /// E002: 解析错误
    pub fn parse_error(line: usize, detail: &str) -> Self {
        LogaiError {
            code: ErrorCode::ParseError,
            message: format!("第 {} 行解析失败: {}", line, detail),
            hint: Some("尝试用 --format json 或 --format text 强制指定格式。也可用 --parse-timestamp-format 自定义时间戳格式。".into()),
            line_number: Some(line),
        }
    }

    /// E003: 缺少 API key
    pub fn missing_api_key(provider: &str, env_var: &str) -> Self {
        let available = Self::detect_available_keys();
        let hint = if available.is_empty() {
            "未检测到任何 AI 后端的 API key。请设置以下环境变量之一:\n  \
                 export DEEPSEEK_API_KEY=\"sk-...\"   # 推荐，最便宜\n  \
                 export ANTHROPIC_API_KEY=\"sk-ant-...\"  # 最强分析质量\n  \
                 export OPENAI_API_KEY=\"sk-...\"     # 企业默认\n  \
                 \n注册链接: https://platform.deepseek.com/"
                .to_string()
        } else {
            format!(
                "{} 需要设置环境变量 {}。\n已检测到的可用后端: {}\n设置: export {}=\"你的key\"",
                provider,
                env_var,
                available.join(", "),
                env_var
            )
        };
        LogaiError {
            code: ErrorCode::MissingApiKey,
            message: format!("缺少 {} API key: 未设置环境变量 {}", provider, env_var),
            hint: Some(hint),
            line_number: None,
        }
    }

    /// E004: AI 调用失败
    pub fn ai_call_failed(detail: &str, attempts: u32) -> Self {
        LogaiError {
            code: ErrorCode::AiCallFailed,
            message: format!("AI 调用失败（已重试 {} 次）: {}", attempts, detail),
            hint: Some(
                "请检查:\n  \
                 1. 网络连接是否正常\n  \
                 2. API key 是否有效且未过期\n  \
                 3. 是否触发 API 限流（等待几分钟后重试）\n  \
                 4. 模型是否可用（尝试 --model auto 自动切换）"
                    .into(),
            ),
            line_number: None,
        }
    }

    /// E005: 配置错误
    pub fn config_error(detail: &str) -> Self {
        LogaiError {
            code: ErrorCode::ConfigError,
            message: format!("配置错误: {}", detail),
            hint: Some("检查 logai.toml 格式是否正确。用 `logai init` 生成模板。".into()),
            line_number: None,
        }
    }

    /// E006: IO 错误
    pub fn io_error(detail: &str) -> Self {
        LogaiError {
            code: ErrorCode::IoError,
            message: format!("IO 错误: {}", detail),
            hint: Some("请检查磁盘空间和文件权限。".into()),
            line_number: None,
        }
    }

    /// 检测当前环境中已设置的 API key
    fn detect_available_keys() -> Vec<String> {
        let mut keys = Vec::new();
        if std::env::var("ANTHROPIC_API_KEY").is_ok() {
            keys.push("Claude (ANTHROPIC_API_KEY)".into());
        }
        if std::env::var("OPENAI_API_KEY").is_ok() {
            keys.push("OpenAI (OPENAI_API_KEY)".into());
        }
        if std::env::var("DEEPSEEK_API_KEY").is_ok() {
            keys.push("DeepSeek (DEEPSEEK_API_KEY)".into());
        }
        if std::env::var("OLLAMA_HOST").is_ok() {
            keys.push("Ollama (OLLAMA_HOST)".into());
        }
        keys
    }
}

impl std::fmt::Display for LogaiError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} {}", self.code, self.message)?;
        if let Some(ref hint) = self.hint {
            write!(f, "\n   💡 {}", hint)?;
        }
        if let Some(line) = self.line_number {
            write!(f, "\n   📍 第 {} 行", line)?;
        }
        Ok(())
    }
}

impl std::error::Error for LogaiError {}

/// 用结构化错误包装 anyhow 结果
pub trait LogaiResultExt<T> {
    fn file_not_found(self, path: impl AsRef<Path>) -> anyhow::Result<T>;
    fn parse_error(self, line: usize, detail: &str) -> anyhow::Result<T>;
    fn missing_api_key(self, provider: &str, env_var: &str) -> anyhow::Result<T>;
    fn ai_call_failed(self, detail: &str, attempts: u32) -> anyhow::Result<T>;
}

impl<T, E: std::fmt::Display> LogaiResultExt<T> for Result<T, E> {
    fn file_not_found(self, path: impl AsRef<Path>) -> anyhow::Result<T> {
        self.map_err(|_| anyhow::Error::from(LogaiError::file_not_found(path)))
    }

    fn parse_error(self, line: usize, detail: &str) -> anyhow::Result<T> {
        self.map_err(|_| anyhow::Error::from(LogaiError::parse_error(line, detail)))
    }

    fn missing_api_key(self, provider: &str, env_var: &str) -> anyhow::Result<T> {
        self.map_err(|_| anyhow::Error::from(LogaiError::missing_api_key(provider, env_var)))
    }

    fn ai_call_failed(self, detail: &str, attempts: u32) -> anyhow::Result<T> {
        self.map_err(|_| anyhow::Error::from(LogaiError::ai_call_failed(detail, attempts)))
    }
}
