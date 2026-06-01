use logai::renderer_html::render_report_html;
use logai::types::{AiResponse, AnalysisSummary, FixSuggestion, Level, RootCause, Severity};
use std::collections::HashMap;

fn make_summary() -> AnalysisSummary {
    let mut level_distribution = HashMap::new();
    level_distribution.insert(Level::Error, 10);
    level_distribution.insert(Level::Warn, 5);

    AnalysisSummary {
        total_lines: 100,
        time_start: None,
        time_end: None,
        level_distribution,
        error_groups: vec![],
        anomalies: vec![],
    }
}

fn make_response() -> AiResponse {
    AiResponse {
        summary: "Test summary".into(),
        root_causes: vec![RootCause {
            description: "Connection pool exhausted".into(),
            severity: Severity::Critical,
            evidence: vec!["5 connection timeouts".into()],
        }],
        fix_suggestions: vec![FixSuggestion {
            action: "Increase pool size".into(),
            code_snippet: Some("pool_max_size: 10 -> 50".into()),
            reference: Some("https://example.com/docs".into()),
        }],
        confidence: 0.9,
    }
}

#[test]
fn test_html_contains_key_elements() {
    let summary = make_summary();
    let response = make_response();
    let html = render_report_html(&summary, &response, 4.2, "DeepSeek");

    assert!(html.contains("<!DOCTYPE html>"));
    assert!(html.contains("logai 分析报告"));
    assert!(html.contains("100 行"));
    assert!(html.contains("DeepSeek"));
    assert!(html.contains("Connection pool exhausted"));
    assert!(html.contains("Increase pool size"));
}

#[test]
fn test_html_escapes_user_content() {
    let summary = make_summary();
    let mut response = make_response();
    response.root_causes[0].description = "<script>alert('xss')</script>".into();

    let html = render_report_html(&summary, &response, 1.0, "test");
    // Should not contain raw script tag
    assert!(!html.contains("<script>alert"));
    assert!(html.contains("&lt;script&gt;"));
}

#[test]
fn test_html_empty_groups_shows_message() {
    let summary = make_summary();
    let response = make_response();
    let html = render_report_html(&summary, &response, 1.0, "test");
    assert!(html.contains("无错误分组"));
}
