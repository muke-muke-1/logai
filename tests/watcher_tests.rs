use std::fs;
use std::io::Write;
use tempfile::NamedTempFile;

fn write_temp_log(content: &str) -> std::path::PathBuf {
    let mut tmp = NamedTempFile::new().unwrap();
    tmp.write_all(content.as_bytes()).unwrap();
    let path = tmp.path().to_path_buf();
    std::mem::forget(tmp);
    path
}

#[test]
fn test_parse_empty_file() {
    let path = write_temp_log("");
    let entries = logai::parser::parse_log_file(&path, None).unwrap();
    assert!(entries.is_empty());
    fs::remove_file(&path).ok();
}

#[test]
fn test_parse_single_json_line() {
    let path = write_temp_log(
        r#"{"timestamp":"2026-06-01T08:00:00Z","level":"ERROR","message":"something failed"}"#,
    );
    let entries = logai::parser::parse_log_file(&path, None).unwrap();
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].level, Some(logai::types::Level::Error));
    fs::remove_file(&path).ok();
}

#[test]
fn test_parse_multiple_json_lines() {
    let mut content = String::new();
    for i in 0..5 {
        content.push_str(&format!(
            r#"{{"timestamp":"2026-06-01T08:00:{:02}Z","level":"INFO","message":"msg{}"}}"#,
            i, i
        ));
        content.push('\n');
    }
    let path = write_temp_log(&content);
    let entries = logai::parser::parse_log_file(&path, None).unwrap();
    assert_eq!(entries.len(), 5);
    fs::remove_file(&path).ok();
}

#[test]
fn test_parse_with_format_override() {
    let path =
        write_temp_log(r#"{"timestamp":"2026-06-01T08:00:00Z","level":"ERROR","message":"fail"}"#);
    let entries = logai::parser::parse_log_file(&path, Some(logai::types::Format::Json)).unwrap();
    assert_eq!(entries.len(), 1);
    fs::remove_file(&path).ok();
}

#[test]
fn test_parse_file_not_found() {
    let result = logai::parser::parse_log_file("nonexistent_xyz_123.log", None);
    assert!(result.is_err());
}

#[test]
fn test_many_json_lines() {
    let mut content = String::new();
    for i in 0..25 {
        content.push_str(&format!(
            r#"{{"timestamp":"2026-06-01T08:00:{:02}Z","level":"INFO","message":"line{}"}}"#,
            i, i
        ));
        content.push('\n');
    }
    let path = write_temp_log(&content);
    let entries = logai::parser::parse_log_file(&path, None).unwrap();
    assert_eq!(entries.len(), 25);
    fs::remove_file(&path).ok();
}

#[test]
fn test_plain_text_parsing() {
    let content = "\
2026-06-01 08:00:00 ERROR something broke
2026-06-01 08:00:01 WARN disk almost full
";
    let path = write_temp_log(content);
    let entries = logai::parser::parse_log_file(&path, None).unwrap();
    assert!(!entries.is_empty());
    fs::remove_file(&path).ok();
}
