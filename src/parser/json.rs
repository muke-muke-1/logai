use crate::parser::timestamp::parse_timestamp;
use crate::types::{Level, LogEntry};
use serde_json::Value;
use std::collections::HashMap;

/// Parse a single JSON log line into a LogEntry. Returns None if the line is not valid JSON.
pub fn parse_json_line(line: &str, line_number: usize) -> Option<LogEntry> {
    let value: Value = serde_json::from_str(line).ok()?;
    let obj = value.as_object()?;

    // Extract timestamp: time / timestamp / @timestamp
    let timestamp = obj
        .get("time")
        .or_else(|| obj.get("timestamp"))
        .or_else(|| obj.get("@timestamp"))
        .and_then(|v| v.as_str())
        .and_then(parse_timestamp);

    // Extract level: level / severity
    let level = obj
        .get("level")
        .or_else(|| obj.get("severity"))
        .and_then(|v| v.as_str())
        .map(parse_level)
        .unwrap_or(Level::Unknown);

    // Extract message: message / msg
    let message = obj
        .get("message")
        .or_else(|| obj.get("msg"))
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();

    // Extract stack trace: stack_trace / stack / backtrace / error.stack
    let stack_trace = obj
        .get("stack_trace")
        .or_else(|| obj.get("stack"))
        .or_else(|| obj.get("backtrace"))
        .or_else(|| obj.get("error").and_then(|e| e.get("stack")))
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    // Extra fields: exclude standard keys
    let standard_keys = [
        "time",
        "timestamp",
        "@timestamp",
        "level",
        "severity",
        "message",
        "msg",
        "stack_trace",
        "stack",
        "backtrace",
        "error",
    ];
    let mut fields = HashMap::new();
    for (k, v) in obj.iter() {
        if !standard_keys.contains(&k.as_str()) {
            if let Some(s) = v.as_str() {
                fields.insert(k.clone(), s.to_string());
            } else {
                fields.insert(k.clone(), v.to_string());
            }
        }
    }

    Some(LogEntry {
        timestamp,
        level: Some(level),
        message,
        stack_trace,
        raw_line: line.to_string(),
        fields,
        line_number,
    })
}

fn parse_level(s: &str) -> Level {
    match s.to_lowercase().as_str() {
        "error" | "err" | "fatal" | "panic" | "critical" => Level::Error,
        "warn" | "warning" => Level::Warn,
        "info" | "information" => Level::Info,
        "debug" => Level::Debug,
        "trace" => Level::Trace,
        _ => Level::Unknown,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_json_error_line() {
        let line = r#"{"time":"2026-05-31T08:03:12Z","level":"error","msg":"Connection timeout","stack":"at connect() line 42"}"#;
        let entry = parse_json_line(line, 1).unwrap();
        assert_eq!(entry.level, Some(Level::Error));
        assert_eq!(entry.message, "Connection timeout");
        assert!(entry.timestamp.is_some());
        assert!(entry.stack_trace.is_some());
    }

    #[test]
    fn test_parse_json_with_extra_fields() {
        let line = r#"{"timestamp":"2026-05-31T08:03:12Z","level":"info","message":"ok","user_id":"42","duration_ms":"150"}"#;
        let entry = parse_json_line(line, 1).unwrap();
        assert_eq!(entry.level, Some(Level::Info));
        assert_eq!(entry.fields.get("user_id").unwrap(), "42");
        assert_eq!(entry.fields.get("duration_ms").unwrap(), "150");
    }

    #[test]
    fn test_parse_json_not_json() {
        let line = "this is not json at all";
        assert!(parse_json_line(line, 1).is_none());
    }

    #[test]
    fn test_parse_json_no_level() {
        let line = r#"{"time":"2026-05-31T08:03:12Z","msg":"just a message"}"#;
        let entry = parse_json_line(line, 1).unwrap();
        assert_eq!(entry.level, Some(Level::Unknown));
    }
}
