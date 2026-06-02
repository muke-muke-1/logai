pub mod json;
pub mod plain_text;
pub mod timestamp;

use crate::types::{Format, LogEntry, ParseConfig};
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::Path;

/// Detect log format from the first N lines.
/// - >=80% valid JSON → Json
/// - Otherwise → PlainText (fallback)
pub fn detect_format(first_lines: &[String]) -> Format {
    if first_lines.is_empty() {
        return Format::PlainText;
    }

    let json_count = first_lines
        .iter()
        .filter(|line| serde_json::from_str::<serde_json::Value>(line).is_ok())
        .count();

    let total = first_lines.len();
    if json_count as f64 / total as f64 >= 0.8 {
        Format::Json
    } else {
        Format::PlainText
    }
}

/// Load ParseConfig from a TOML config file.
/// Supports `logai.toml` (auto-detected in CWD) or a custom path via `--rules-file`.
pub fn load_config_file(path: Option<&std::path::Path>) -> Option<ParseConfig> {
    let file_path: Option<std::path::PathBuf> = if let Some(p) = path {
        if p.exists() {
            Some(p.to_path_buf())
        } else {
            eprintln!("   ⚠️  指定规则文件不存在: {}，忽略", p.display());
            None
        }
    } else {
        let default_path = std::env::current_dir()
            .unwrap_or_default()
            .join("logai.toml");
        if default_path.exists() {
            Some(default_path)
        } else {
            None
        }
    };

    let file_path = file_path?;
    eprintln!("   📋 加载解析配置: {}", file_path.display());

    match std::fs::read_to_string(&file_path) {
        Ok(content) => match parse_config_toml(&content) {
            Ok(cfg) => Some(cfg),
            Err(e) => {
                eprintln!("   ⚠️  解析配置文件失败: {}，忽略", e);
                None
            }
        },
        Err(e) => {
            eprintln!("   ⚠️  读取配置文件失败: {}，忽略", e);
            None
        }
    }
}

/// Parse a TOML config string into ParseConfig.
/// Expected format:
/// ```toml
/// [parse]
/// timestamp_format = "%Y-%m-%d %H:%M:%S"
/// level_field = "level"
/// message_field = "message"
/// level_pattern = "(?i)\b(error|warn|info|debug|trace)\b"
/// stack_trace_marker = "  at "
/// ```
fn parse_config_toml(content: &str) -> anyhow::Result<ParseConfig> {
    let val: toml::Value = toml::from_str(content)?;
    let parse = val
        .get("parse")
        .ok_or_else(|| anyhow::anyhow!("缺少 [parse] 节"))?;

    Ok(ParseConfig {
        timestamp_format: parse
            .get("timestamp_format")
            .and_then(|v| v.as_str())
            .map(String::from),
        level_field: parse
            .get("level_field")
            .and_then(|v| v.as_str())
            .map(String::from),
        message_field: parse
            .get("message_field")
            .and_then(|v| v.as_str())
            .map(String::from),
        level_pattern: parse
            .get("level_pattern")
            .and_then(|v| v.as_str())
            .map(String::from),
        stack_trace_marker: parse
            .get("stack_trace_marker")
            .and_then(|v| v.as_str())
            .map(String::from),
    })
}

/// 按指定格式解析一批日志行（供 watch 模式增量解析使用）
pub fn parse_lines(lines: &[String], format: Format) -> Vec<LogEntry> {
    match format {
        Format::Json => lines
            .iter()
            .enumerate()
            .filter_map(|(i, line)| json::parse_json_line(line, i + 1))
            .collect(),
        Format::PlainText => plain_text::parse_plain_text_iter(lines.iter().cloned()),
    }
}

/// Parse a log file. Streams lines, auto-detects format from first 10 lines, returns all LogEntries.
pub fn parse_log_file(
    path: impl AsRef<Path>,
    format_override: Option<Format>,
) -> anyhow::Result<Vec<LogEntry>> {
    let file = File::open(path)?;
    let reader = BufReader::new(file);

    // Collect all lines — stream-like read with single allocation
    let lines: Vec<String> = reader.lines().map_while(Result::ok).collect();

    // Detect format from first 10 lines (or use override)
    let format = match format_override {
        Some(f) => f,
        None => {
            let sample: Vec<String> = lines.iter().take(10).cloned().collect();
            detect_format(&sample)
        }
    };

    // Parse in-place — reuse line strings where possible
    let entries = match format {
        Format::Json => lines
            .iter()
            .enumerate()
            .filter_map(|(i, line)| json::parse_json_line(line, i + 1))
            .collect(),
        Format::PlainText => {
            // Consume lines to avoid double allocation
            plain_text::parse_plain_text_iter(lines.into_iter())
        }
    };

    Ok(entries)
}
