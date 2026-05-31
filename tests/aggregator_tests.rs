use logai::aggregator::signature::build_signature;

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
