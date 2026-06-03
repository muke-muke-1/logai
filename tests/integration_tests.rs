use assert_cmd::Command;
use predicates::prelude::*;

#[test]
fn test_cli_help() {
    let mut cmd = Command::cargo_bin("logai").unwrap();
    cmd.arg("--help");
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("AI 驱动的日志分析 CLI"));
}

#[test]
fn test_analyze_help() {
    let mut cmd = Command::cargo_bin("logai").unwrap();
    cmd.arg("analyze").arg("--help");
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("日志文件路径"));
}

#[test]
fn test_analyze_file_not_found() {
    let mut cmd = Command::cargo_bin("logai").unwrap();
    cmd.arg("analyze").arg("nonexistent.log");
    cmd.assert()
        .failure()
        .stderr(predicate::str::contains("文件不存在"));
}

#[test]
fn test_analyze_parses_json_log_successfully() {
    // This test requires an API key — skip if none configured
    if std::env::var("DEEPSEEK_API_KEY").is_err()
        && std::env::var("OPENAI_API_KEY").is_err()
        && std::env::var("ANTHROPIC_API_KEY").is_err()
    {
        eprintln!("Skipping: no AI API key configured");
        return;
    }
    let model = if std::env::var("DEEPSEEK_API_KEY").is_ok() {
        "deepseek"
    } else if std::env::var("OPENAI_API_KEY").is_ok() {
        "openai"
    } else {
        "claude"
    };

    let mut cmd = Command::cargo_bin("logai").unwrap();
    cmd.arg("analyze")
        .arg("tests/fixtures/json_error.log")
        .arg("--model")
        .arg(model);
    cmd.assert().success();
}

#[test]
fn test_analyze_with_min_level_flag() {
    let mut cmd = Command::cargo_bin("logai").unwrap();
    cmd.arg("analyze")
        .arg("tests/fixtures/json_error.log")
        .arg("--min-level")
        .arg("error")
        .arg("--model")
        .arg("deepseek");
    let output = cmd.output().unwrap();
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("已解析") || stderr.contains("解析") || stderr.contains("parse"));
}

#[test]
fn test_analyze_with_format_flag() {
    let mut cmd = Command::cargo_bin("logai").unwrap();
    cmd.arg("analyze")
        .arg("tests/fixtures/json_error.log")
        .arg("--format")
        .arg("json")
        .arg("--model")
        .arg("deepseek");
    let output = cmd.output().unwrap();
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("已解析") || stderr.contains("解析") || stderr.contains("parse"));
}

#[test]
fn test_watch_help() {
    let mut cmd = Command::cargo_bin("logai").unwrap();
    cmd.arg("watch").arg("--help");
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("实时监听日志文件"));
}

#[test]
fn test_watch_file_not_found() {
    let mut cmd = Command::cargo_bin("logai").unwrap();
    cmd.arg("watch").arg("nonexistent_watch.log");
    cmd.assert()
        .failure()
        .stderr(predicate::str::contains("文件不存在"));
}

#[test]
fn test_watch_shows_flags_in_help() {
    let mut cmd = Command::cargo_bin("logai").unwrap();
    cmd.arg("watch").arg("--help");
    let output = cmd.output().unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("--window"));
    assert!(stdout.contains("--max-initial-lines"));
    assert!(stdout.contains("--model"));
    assert!(stdout.contains("--min-level"));
    assert!(stdout.contains("--format"));
}

#[test]
fn test_interactive_help() {
    let mut cmd = Command::cargo_bin("logai").unwrap();
    cmd.arg("interactive").arg("--help");
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("交互式 TUI 日志浏览器"));
}

#[test]
fn test_interactive_live_flag_in_help() {
    let mut cmd = Command::cargo_bin("logai").unwrap();
    cmd.arg("interactive").arg("--help");
    let output = cmd.output().unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("--live"));
}

#[test]
fn test_analyze_output_flag_in_help() {
    let mut cmd = Command::cargo_bin("logai").unwrap();
    cmd.arg("analyze").arg("--help");
    let output = cmd.output().unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("--output"));
}
