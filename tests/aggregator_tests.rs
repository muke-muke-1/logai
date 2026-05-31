use chrono::TimeZone;
use logai::aggregator::anomaly;
use logai::aggregator::signature::build_signature;

fn dt(ts: i64) -> chrono::DateTime<chrono::Utc> {
    chrono::Utc.timestamp_opt(ts, 0).unwrap()
}

#[test]
fn test_deparameterize_ip() {
    let msg = "Connection to 192.168.1.100:8080 failed";
    let sig = build_signature(msg);
    assert!(sig.contains("<IP>"));
    assert!(!sig.contains("192.168.1.100"));
}

#[test]
fn test_deparameterize_uuid() {
    let msg = "User 550e8400-e29b-41d4-a716-446655440000 not found";
    let sig = build_signature(msg);
    assert!(sig.contains("<ID>"));
    assert!(!sig.contains("550e8400"));
}

#[test]
fn test_same_signature_for_similar_errors() {
    let sig1 = build_signature("Connection to 192.168.1.1:5432 failed");
    let sig2 = build_signature("Connection to 10.0.0.5:5432 failed");
    assert_eq!(sig1, sig2);
}

#[test]
fn test_silent_recovery_detected() {
    let windows = vec![(dt(0), 5), (dt(300), 3), (dt(600), 0), (dt(900), 0)];
    let result = anomaly::detect_silent_recovery(&windows, 0);
    assert_eq!(result.len(), 1);
    matches!(
        result[0],
        logai::types::Anomaly::SilentRecovery { group_index: 0 }
    );
}

#[test]
fn test_silent_recovery_not_detected_when_still_active() {
    let windows = vec![(dt(0), 5), (dt(300), 3), (dt(600), 2), (dt(900), 4)];
    let result = anomaly::detect_silent_recovery(&windows, 0);
    assert!(result.is_empty());
}

#[test]
fn test_periodic_pattern_detected() {
    let windows = vec![
        (dt(0), 1),
        (dt(300), 1),
        (dt(600), 1),
        (dt(900), 1),
        (dt(1200), 1),
    ];
    let result = anomaly::detect_periodic_pattern(&windows, 0);
    assert_eq!(result.len(), 1);
    if let logai::types::Anomaly::PeriodicPattern {
        group_index,
        period_minutes,
    } = &result[0]
    {
        assert_eq!(*group_index, 0);
        assert!(*period_minutes >= 4 && *period_minutes <= 6);
    } else {
        panic!("Expected PeriodicPattern");
    }
}

#[test]
fn test_periodic_pattern_not_detected_for_irregular() {
    let windows = vec![(dt(0), 1), (dt(500), 1), (dt(800), 1), (dt(2000), 1)];
    let result = anomaly::detect_periodic_pattern(&windows, 0);
    assert!(result.is_empty());
}

#[test]
fn test_silent_recovery_too_few_windows() {
    let windows = vec![(dt(0), 1), (dt(300), 0), (dt(600), 0)];
    let result = anomaly::detect_silent_recovery(&windows, 0);
    assert!(result.is_empty());
}

#[test]
fn test_periodic_pattern_too_few_appearances() {
    let windows = vec![(dt(0), 1), (dt(300), 0), (dt(600), 1)];
    let result = anomaly::detect_periodic_pattern(&windows, 0);
    assert!(result.is_empty());
}
