pub mod json;
pub mod plain_text;
pub mod timestamp;

use crate::types::{Format, LogEntry};
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::Path;

/// Detect log format from the first N lines.
/// - >=80% valid JSON → Json
/// - >=50% timestamp matches → PlainText
/// - Otherwise → PlainText (fallback)
pub fn detect_format(first_lines: &[String]) -> Format {
    if first_lines.is_empty() {
        return Format::PlainText;
    }

    let json_count = first_lines
        .iter()
        .filter(|line| serde_json::from_str::<serde_json::Value>(line).is_ok())
        .count();
    let timestamp_count = first_lines
        .iter()
        .filter(|line| timestamp::detect_timestamp(line))
        .count();

    let total = first_lines.len();
    if json_count as f64 / total as f64 >= 0.8 {
        Format::Json
    } else if timestamp_count as f64 / total as f64 >= 0.5 {
        Format::PlainText
    } else {
        Format::PlainText
    }
}

/// Parse a log file. Streams lines, auto-detects format, returns all LogEntries.
pub fn parse_log_file(path: impl AsRef<Path>) -> anyhow::Result<Vec<LogEntry>> {
    let file = File::open(path)?;
    let reader = BufReader::new(file);
    let lines: Vec<String> = reader
        .lines()
        .filter_map(|l| l.ok())
        .collect();

    // Detect format from first 10 lines
    let sample: Vec<String> = lines.iter().take(10).cloned().collect();
    let format = detect_format(&sample);

    match format {
        Format::Json => {
            let entries: Vec<LogEntry> = lines
                .iter()
                .enumerate()
                .filter_map(|(i, line)| json::parse_json_line(line, i + 1))
                .collect();
            Ok(entries)
        }
        Format::PlainText => {
            Ok(plain_text::parse_plain_text(&lines))
        }
    }
}
