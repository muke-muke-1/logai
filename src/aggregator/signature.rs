use crate::types::LogEntry;
use regex::Regex;
use std::sync::LazyLock;

static DEPARAM_RULES: LazyLock<Vec<(Regex, &str)>> = LazyLock::new(|| {
    vec![
        // URL (must come before IP and path)
        (Regex::new(r"https?://\S+").unwrap(), "<URL>"),
        // UUID
        (
            Regex::new(
                r"[0-9a-fA-F]{8}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{12}",
            )
            .unwrap(),
            "<ID>",
        ),
        // IPv4 address
        (
            Regex::new(r"\d{1,3}\.\d{1,3}\.\d{1,3}\.\d{1,3}").unwrap(),
            "<IP>",
        ),
        // Port number (must be after IP)
        (Regex::new(r":\d{4,5}").unwrap(), ":<PORT>"),
        // Hex strings (memory addresses, SHA hashes)
        (Regex::new(r"0x[0-9a-fA-F]{6,16}").unwrap(), "<HEX>"),
        // File path (Unix absolute)
        (Regex::new(r"/[\w/.-]+").unwrap(), "<PATH>"),
        // File path (Windows)
        (Regex::new(r"[A-Z]:\\[\w\\.-]+").unwrap(), "<PATH>"),
        // Numbers (do this last to not conflict with IP/port/hex)
        (Regex::new(r"\b\d+\b").unwrap(), "<NUM>"),
    ]
});

/// Build a deparameterized error signature from a log message.
/// Replaces IPs, UUIDs, numbers, URLs, paths with placeholders.
/// Uses Cow<str> to avoid allocation when no regex matches.
pub fn build_signature(message: &str) -> String {
    use std::borrow::Cow;
    let mut sig: Cow<str> = Cow::Borrowed(message);
    for (re, replacement) in DEPARAM_RULES.iter() {
        sig = Cow::Owned(re.replace_all(&sig, *replacement).into_owned());
    }
    // Collapse whitespace
    let mut result: String = sig.into_owned();
    result = result.split_whitespace().collect::<Vec<_>>().join(" ");
    result
}

/// Group LogEntries by their deparameterized error signature.
/// Returns Vec of (signature, Vec of entry indices), sorted by group size descending.
pub fn group_by_signature(entries: &[LogEntry]) -> Vec<(String, Vec<usize>)> {
    let mut sig_to_idx: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
    let mut groups: Vec<(String, Vec<usize>)> = Vec::new();

    for (i, entry) in entries.iter().enumerate() {
        let sig = build_signature(&entry.message);
        if let Some(&idx) = sig_to_idx.get(&sig) {
            groups[idx].1.push(i);
        } else {
            sig_to_idx.insert(sig.clone(), groups.len());
            groups.push((sig, vec![i]));
        }
    }

    // Sort by group size descending
    groups.sort_by_key(|(_, indices)| -(indices.len() as i64));
    groups
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_deparameterize_ip() {
        let sig = build_signature("Connection to 192.168.1.100:8080 failed");
        assert!(sig.contains("<IP>"));
        assert!(sig.contains("<PORT>"));
    }

    #[test]
    fn test_deparameterize_uuid() {
        let sig = build_signature("User 550e8400-e29b-41d4-a716-446655440000 not found");
        assert!(sig.contains("<ID>"));
        assert!(!sig.contains("550e8400"));
    }

    #[test]
    fn test_deparameterize_number() {
        let sig = build_signature("Retry attempt 42 of 100");
        assert!(!sig.contains("42"));
        assert!(!sig.contains("100"));
    }

    #[test]
    fn test_deparameterize_url() {
        let sig = build_signature("GET https://api.example.com/v1/users/42 timeout");
        assert!(sig.contains("<URL>"));
        assert!(!sig.contains("api.example.com"));
    }

    #[test]
    fn test_identical_messages_have_same_signature() {
        let s1 = build_signature("Error on port 5432 for user 42");
        let s2 = build_signature("Error on port 9999 for user 100");
        assert_eq!(s1, s2);
    }
}
