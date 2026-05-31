use crate::parser::timestamp::{detect_timestamp, extract_timestamp_str, parse_timestamp};
use crate::types::{Level, LogEntry};
use regex::Regex;
use std::sync::LazyLock;

static LEVEL_PATTERN: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r"(?i)\b(error|err|fatal|panic|critical|warn|warning|info|information|debug|trace)\b",
    )
    .unwrap()
});

/// Parse plain text log lines from an iterator (streaming-friendly).
/// Same state machine as parse_plain_text but takes an iterator.
pub fn parse_plain_text_iter<I>(lines: I) -> Vec<LogEntry>
where
    I: Iterator<Item = String>,
{
    let mut entries: Vec<LogEntry> = Vec::new();
    let mut current_stack: Vec<String> = Vec::new();
    let mut line_number = 0usize;

    for line in lines {
        line_number += 1;
        let is_indented = line.starts_with(' ') || line.starts_with('\t');
        let is_stack_continuation = line.contains("Traceback")
            || line.contains("panic:")
            || line.contains("Exception in thread")
            || line.trim_start().starts_with("at ")
            || line.trim_start().starts_with("... ");

        if (is_indented || is_stack_continuation) && !entries.is_empty() {
            current_stack.push(line.clone());
            continue;
        }

        if detect_timestamp(&line) {
            if let Some(last) = entries.last_mut() {
                if !current_stack.is_empty() {
                    last.stack_trace = Some(current_stack.join("\n"));
                    current_stack.clear();
                }
            }
            let entry = parse_log_line(&line, line_number);
            entries.push(entry);
        } else if entries.is_empty() {
            let entry = LogEntry {
                timestamp: None,
                level: Some(Level::Unknown),
                message: line.clone(),
                stack_trace: None,
                raw_line: line.clone(),
                fields: std::collections::HashMap::new(),
                line_number,
            };
            entries.push(entry);
        } else if let Some(last) = entries.last_mut() {
            if last.message.is_empty() {
                last.message = line.clone();
            } else {
                last.message.push(' ');
                last.message.push_str(&line);
            }
            // Keep raw_line updated for the first entry in a group
            if last.raw_line.is_empty() {
                last.raw_line = line.clone();
            }
        }
    }

    // Flush remaining stack
    if let Some(last) = entries.last_mut() {
        if !current_stack.is_empty() {
            last.stack_trace = Some(current_stack.join("\n"));
        }
    }

    entries
}

/// Parse plain text log lines into a Vec of LogEntry.
/// Uses a state machine: timestamped lines start new entries,
/// indented/traceback lines are appended to the previous entry's stack trace.
pub fn parse_plain_text(lines: &[String]) -> Vec<LogEntry> {
    let mut entries: Vec<LogEntry> = Vec::new();
    let mut current_stack: Vec<String> = Vec::new();
    let mut line_number = 0usize;

    for line in lines {
        line_number += 1;
        let is_indented = line.starts_with(' ') || line.starts_with('\t');
        let is_stack_continuation = line.contains("Traceback")
            || line.contains("panic:")
            || line.contains("Exception in thread")
            || line.trim_start().starts_with("at ")
            || line.trim_start().starts_with("... ");

        // If this is a stack/continuation line, append to current stack buffer
        if (is_indented || is_stack_continuation) && !entries.is_empty() {
            current_stack.push(line.clone());
            continue;
        }

        // Found a new timestamped line → flush pending stack to previous entry
        if detect_timestamp(line) {
            if let Some(last) = entries.last_mut() {
                if !current_stack.is_empty() {
                    last.stack_trace = Some(current_stack.join("\n"));
                    current_stack.clear();
                }
            }
            let entry = parse_log_line(line, line_number);
            entries.push(entry);
        } else if entries.is_empty() {
            // Before first entry: treat as a standalone message
            let entry = LogEntry {
                timestamp: None,
                level: Some(Level::Unknown),
                message: line.clone(),
                stack_trace: None,
                raw_line: line.clone(),
                fields: std::collections::HashMap::new(),
                line_number,
            };
            entries.push(entry);
        } else {
            // Possibly a message continuation line (non-indented, non-timestamp)
            if let Some(last) = entries.last_mut() {
                if last.message.is_empty() {
                    last.message = line.clone();
                } else {
                    last.message.push(' ');
                    last.message.push_str(line);
                }
            }
        }
    }

    // Flush any remaining stack to the last entry
    if let Some(last) = entries.last_mut() {
        if !current_stack.is_empty() {
            last.stack_trace = Some(current_stack.join("\n"));
        }
    }

    entries
}

/// Parse a single line known to contain a timestamp into a LogEntry
fn parse_log_line(line: &str, line_number: usize) -> LogEntry {
    let timestamp = extract_timestamp_str(line).and_then(|s| parse_timestamp(&s));
    let level = extract_level(line);
    let message = extract_message(line);

    LogEntry {
        timestamp,
        level: Some(level),
        message,
        stack_trace: None,
        raw_line: line.to_string(),
        fields: std::collections::HashMap::new(),
        line_number,
    }
}

/// Extract log level from a line using keyword matching
fn extract_level(line: &str) -> Level {
    LEVEL_PATTERN
        .captures(line)
        .map(|caps| caps.get(1).unwrap().as_str())
        .map(|s| match s.to_lowercase().as_str() {
            "error" | "err" | "fatal" | "panic" | "critical" => Level::Error,
            "warn" | "warning" => Level::Warn,
            "info" | "information" => Level::Info,
            "debug" => Level::Debug,
            "trace" => Level::Trace,
            _ => Level::Unknown,
        })
        .unwrap_or(Level::Unknown)
}

/// Extract the message body from a log line: remove timestamp and level keyword
fn extract_message(line: &str) -> String {
    let mut msg = line.to_string();
    if let Some(ts) = extract_timestamp_str(line) {
        msg = msg.replacen(&ts, "", 1);
    }
    msg = LEVEL_PATTERN.replace(&msg, "").to_string();
    msg = msg
        .trim_start_matches(&[' ', '-', ':', '[', ']', ','] as &[_])
        .to_string();
    if msg.is_empty() {
        msg = line.to_string();
    }
    msg
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_level_error() {
        assert_eq!(extract_level("[2026-05-31] ERROR something"), Level::Error);
    }

    #[test]
    fn test_extract_level_warn() {
        assert_eq!(extract_level("WARNING: disk almost full"), Level::Warn);
    }

    #[test]
    fn test_extract_level_none() {
        assert_eq!(extract_level("just a plain message"), Level::Unknown);
    }

    #[test]
    fn test_parse_python_log() {
        let lines = vec![
            "[2026-05-31 08:03:12] ERROR - db/connection.py:234".to_string(),
            "Connection timeout after 30 seconds".to_string(),
            "Traceback (most recent call last):".to_string(),
            "  File \"db/connection.py\", line 234, in acquire".to_string(),
            "    raise TimeoutError(\"Connection pool exhausted\")".to_string(),
        ];
        let entries = parse_plain_text(&lines);
        assert_eq!(entries.len(), 1);
        let entry = &entries[0];
        assert_eq!(entry.level, Some(Level::Error));
        assert!(entry.message.to_lowercase().contains("timeout"));
        assert!(entry.stack_trace.is_some());
        assert!(entry.timestamp.is_some());
    }

    #[test]
    fn test_parse_multiple_entries() {
        let lines = vec![
            "[2026-05-31 08:03:12] INFO Request processed".to_string(),
            "[2026-05-31 08:03:13] ERROR Connection failed".to_string(),
        ];
        let entries = parse_plain_text(&lines);
        assert_eq!(entries.len(), 2);
    }

    #[test]
    fn test_parse_line_without_timestamp() {
        let lines = vec![
            "just some plain text".to_string(),
            "  indented continuation".to_string(),
        ];
        let entries = parse_plain_text(&lines);
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].level, Some(Level::Unknown));
    }

    #[test]
    fn test_stack_trace_concatenation() {
        let lines = vec![
            "ERROR: something broke".to_string(),
            "  at function_a() line 10".to_string(),
            "  at function_b() line 20".to_string(),
            "[2026-05-31 08:04:00] INFO next thing".to_string(),
        ];
        let entries = parse_plain_text(&lines);
        assert_eq!(entries.len(), 2);
        assert!(entries[0].stack_trace.is_some());
        let stack = entries[0].stack_trace.as_ref().unwrap();
        assert!(stack.contains("function_a"));
        assert!(stack.contains("function_b"));
    }
}
