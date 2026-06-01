use logai::ai::parse_ai_response;

#[test]
fn test_parse_ai_response_valid_json() {
    let input = r#"{
        "root_causes": [
            {
                "description": "Connection pool exhausted",
                "evidence": ["5 timeouts in 30s"],
                "severity": "Critical"
            }
        ],
        "summary": "Database connection pool too small",
        "fix_suggestions": [
            {
                "action": "Increase pool size",
                "code_snippet": "pool.max_size = 50",
                "reference": "https://docs.example.com/pool"
            }
        ],
        "confidence": 0.92
    }"#;

    let result = parse_ai_response(input).unwrap();
    assert_eq!(result.root_causes.len(), 1);
    assert_eq!(
        result.root_causes[0].description,
        "Connection pool exhausted"
    );
    assert_eq!(result.summary, "Database connection pool too small");
    assert_eq!(result.fix_suggestions.len(), 1);
    assert_eq!(result.fix_suggestions[0].action, "Increase pool size");
    assert!((result.confidence - 0.92).abs() < 0.001);
}

#[test]
fn test_parse_ai_response_json_in_markdown() {
    let input = r#"Here is my analysis:

```json
{
    "root_causes": [],
    "summary": "No issues found",
    "fix_suggestions": [],
    "confidence": 1.0
}
```

Let me know if you need more detail."#;

    let result = parse_ai_response(input).unwrap();
    assert_eq!(result.summary, "No issues found");
    assert!(result.root_causes.is_empty());
    assert!((result.confidence - 1.0).abs() < 0.001);
}

#[test]
fn test_parse_ai_response_malformed_graceful_degradation() {
    let input = "Sorry, I couldn't analyze this log — the format is unrecognized. No JSON here.";

    let result = parse_ai_response(input).unwrap();
    // Should still return Ok (graceful degradation), not Err
    assert!(result.summary.contains("Unable to parse"));
    assert!(result.confidence < 0.1);
    // Root causes should contain the fallback with raw text
    assert_eq!(result.root_causes.len(), 1);
    assert!(result.root_causes[0]
        .description
        .contains("AI response parsing failed"));
}
