use assert_cmd::Command;
use predicates::prelude::*;

#[test]
fn test_cli_help() {
    let mut cmd = Command::cargo_bin("logai").unwrap();
    cmd.arg("--help");
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("AI-powered log analysis"));
}

#[test]
fn test_analyze_help() {
    let mut cmd = Command::cargo_bin("logai").unwrap();
    cmd.arg("analyze").arg("--help");
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("Log file path"));
}

#[test]
fn test_analyze_file_not_found() {
    let mut cmd = Command::cargo_bin("logai").unwrap();
    cmd.arg("analyze").arg("nonexistent.log");
    cmd.assert()
        .failure()
        .stderr(predicate::str::contains("File not found"));
}

#[test]
fn test_analyze_parses_json_log_successfully() {
    // This test requires an API key — skip if none configured
    if std::env::var("DEEPSEEK_API_KEY").is_err() && std::env::var("OPENAI_API_KEY").is_err() && std::env::var("ANTHROPIC_API_KEY").is_err() {
        eprintln!("Skipping: no AI API key configured");
        return;
    }
    let model = if std::env::var("DEEPSEEK_API_KEY").is_ok() { "deepseek" }
        else if std::env::var("OPENAI_API_KEY").is_ok() { "openai" }
        else { "claude" };

    let mut cmd = Command::cargo_bin("logai").unwrap();
    cmd.arg("analyze")
        .arg("tests/fixtures/json_error.log")
        .arg("--model")
        .arg(model);
    cmd.assert().success();
}
