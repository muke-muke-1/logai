pub mod json;
pub mod plain_text;
pub mod timestamp;

use crate::types::{Format, LogEntry};
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

/// Parse a log file. Streams lines, auto-detects format, returns all LogEntries.
pub fn parse_log_file(
    path: impl AsRef<Path>,
    format_override: Option<Format>,
) -> anyhow::Result<Vec<LogEntry>> {
    let file = File::open(path)?;
    let reader = BufReader::new(file);
    let lines: Vec<String> = reader.lines().map_while(Result::ok).collect();

    // Detect format from first 10 lines (or use override)
    let format = match format_override {
        Some(f) => f,
        None => {
            let sample: Vec<String> = lines.iter().take(10).cloned().collect();
            detect_format(&sample)
        }
    };

    match format {
        Format::Json => {
            let entries: Vec<LogEntry> = lines
                .iter()
                .enumerate()
                .filter_map(|(i, line)| json::parse_json_line(line, i + 1))
                .collect();
            Ok(entries)
        }
        Format::PlainText => Ok(plain_text::parse_plain_text_iter(lines.into_iter())),
    }
}
