use chrono::{DateTime, Datelike, NaiveDateTime, Utc};
use regex::Regex;
use std::sync::LazyLock;

static TIMESTAMP_PATTERNS: LazyLock<Vec<Regex>> = LazyLock::new(|| {
    vec![
        // ISO8601 / RFC3339: 2026-05-31T08:03:12Z, 2026-05-31T08:03:12+00:00
        Regex::new(r"\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}(?:\.\d+)?(?:Z|[+-]\d{2}:\d{2})")
            .unwrap(),
        // Bracket: [2026-05-31 08:03:12], [2026-05-31 08:03:12.123]
        Regex::new(r"\[\d{4}-\d{2}-\d{2}\s+\d{2}:\d{2}:\d{2}(?:\.\d+)?\]").unwrap(),
        // Space-separated: 2026-05-31 08:03:12, 2026/05/31 08:03:12
        Regex::new(r"\d{4}[-/]\d{2}[-/]\d{2}\s+\d{2}:\d{2}:\d{2}(?:\.\d+)?").unwrap(),
        // Syslog: May 31 08:03:12
        Regex::new(
            r"(?i)(?:Jan|Feb|Mar|Apr|May|Jun|Jul|Aug|Sep|Oct|Nov|Dec)\s+\d{1,2}\s+\d{2}:\d{2}:\d{2}",
        )
        .unwrap(),
        // Python logging default: 2026-05-31 08:03:12,123
        Regex::new(r"\d{4}-\d{2}-\d{2}\s+\d{2}:\d{2}:\d{2},\d{3}").unwrap(),
        // Unix timestamp (seconds): 1717142592 (10 digits, 1.5e9 to 1.7e9 range)
        Regex::new(r"\b1[5-7]\d{8}\b").unwrap(),
        // Go log format: 2026/05/31 08:03:12
        Regex::new(r"\d{4}/\d{2}/\d{2}\s+\d{2}:\d{2}:\d{2}").unwrap(),
    ]
});

/// Parse a timestamp string, supporting 7+ common formats
pub fn parse_timestamp(s: &str) -> Option<DateTime<Utc>> {
    // ISO8601 / RFC3339
    if let Ok(dt) = DateTime::parse_from_rfc3339(s) {
        return Some(dt.with_timezone(&Utc));
    }
    // RFC2822
    if let Ok(dt) = DateTime::parse_from_rfc2822(s) {
        return Some(dt.with_timezone(&Utc));
    }
    // Strip brackets
    let cleaned = s.trim_start_matches('[').trim_end_matches(']');
    // YYYY-MM-DD HH:MM:SS
    if let Ok(dt) = NaiveDateTime::parse_from_str(cleaned, "%Y-%m-%d %H:%M:%S") {
        return Some(DateTime::from_naive_utc_and_offset(dt, Utc));
    }
    // YYYY-MM-DD HH:MM:SS.mmm
    if let Ok(dt) = NaiveDateTime::parse_from_str(cleaned, "%Y-%m-%d %H:%M:%S%.3f") {
        return Some(DateTime::from_naive_utc_and_offset(dt, Utc));
    }
    // YYYY/MM/DD HH:MM:SS
    if let Ok(dt) = NaiveDateTime::parse_from_str(cleaned, "%Y/%m/%d %H:%M:%S") {
        return Some(DateTime::from_naive_utc_and_offset(dt, Utc));
    }
    // Python logging: YYYY-MM-DD HH:MM:SS,mmm
    if let Ok(dt) = NaiveDateTime::parse_from_str(cleaned, "%Y-%m-%d %H:%M:%S,%3f") {
        return Some(DateTime::from_naive_utc_and_offset(dt, Utc));
    }
    // Syslog: Mon DD HH:MM:SS — use current system year
    let current_year = chrono::Local::now().year();
    if let Ok(dt) =
        NaiveDateTime::parse_from_str(&format!("{} {}", current_year, cleaned), "%Y %b %d %H:%M:%S")
    {
        return Some(DateTime::from_naive_utc_and_offset(dt, Utc));
    }
    // Unix timestamp (10 digits)
    if let Some(caps) = Regex::new(r"\b(1[5-7]\d{8})\b").unwrap().captures(s) {
        if let Ok(ts) = caps.get(1).unwrap().as_str().parse::<i64>() {
            return DateTime::from_timestamp(ts, 0);
        }
    }
    None
}

/// Detect if a line contains any known timestamp pattern
pub fn detect_timestamp(line: &str) -> bool {
    TIMESTAMP_PATTERNS.iter().any(|re| re.is_match(line))
}

/// Extract the timestamp string from a line
pub fn extract_timestamp_str(line: &str) -> Option<String> {
    TIMESTAMP_PATTERNS
        .iter()
        .find_map(|re| re.find(line))
        .map(|m| m.as_str().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Timelike;

    #[test]
    fn test_iso8601() {
        let dt = parse_timestamp("2026-05-31T08:03:12Z").unwrap();
        assert_eq!(dt.hour(), 8);
    }

    #[test]
    fn test_bracket_format() {
        let dt = parse_timestamp("[2026-05-31 08:03:12]").unwrap();
        assert_eq!(dt.hour(), 8);
    }

    #[test]
    fn test_space_separated() {
        assert!(parse_timestamp("2026-05-31 08:03:12").is_some());
    }

    #[test]
    fn test_syslog_format() {
        assert!(parse_timestamp("May 31 08:03:12").is_some());
    }

    #[test]
    fn test_python_logging() {
        assert!(parse_timestamp("2026-05-31 08:03:12,123").is_some());
    }

    #[test]
    fn test_go_format() {
        assert!(parse_timestamp("2026/05/31 08:03:12").is_some());
    }

    #[test]
    fn test_no_timestamp() {
        assert!(parse_timestamp("just a plain message").is_none());
    }

    #[test]
    fn test_detect_timestamp() {
        assert!(detect_timestamp("[2026-05-31 08:03:12] ERROR something"));
    }

    #[test]
    fn test_detect_timestamp_not_found() {
        assert!(!detect_timestamp("just a plain message"));
    }
}
