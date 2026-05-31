use crate::types::Anomaly;
use chrono::{DateTime, Utc};

/// Each window: (window start time, occurrence count for this error group)
pub type WindowCount = (DateTime<Utc>, usize);

/// Detect anomalies from window counts.
/// Currently detects spikes: any window where count > avg * 3 and count > 3.
pub fn detect_anomalies(window_counts: &[WindowCount], group_index: usize) -> Vec<Anomaly> {
    let mut anomalies = Vec::new();

    if window_counts.len() < 3 {
        return anomalies;
    }

    let non_zero: Vec<usize> = window_counts
        .iter()
        .map(|(_, c)| *c)
        .filter(|&c| c > 0)
        .collect();

    if non_zero.is_empty() {
        return anomalies;
    }

    let avg: f64 = non_zero.iter().sum::<usize>() as f64 / non_zero.len() as f64;

    // Spike detection: count > avg * 2 and count > 3
    for (_time, count) in window_counts.iter() {
        if *count as f64 > avg * 2.0 && *count > 3 {
            anomalies.push(Anomaly::Spike {
                group_index,
                multiplier: *count as f64 / avg.max(1.0),
            });
            break; // one spike report per group
        }
    }

    anomalies
}

/// Check if an error group is "new" — its first occurrence is >60 seconds after log start
pub fn is_new_error(
    group_first_seen: Option<DateTime<Utc>>,
    log_start: Option<DateTime<Utc>>,
) -> bool {
    match (group_first_seen, log_start) {
        (Some(first), Some(start)) => (first - start).num_seconds() > 60,
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    fn dt(ts: i64) -> DateTime<Utc> {
        Utc.timestamp_opt(ts, 0).unwrap()
    }

    #[test]
    fn test_detect_spike() {
        let windows = vec![
            (dt(0), 5),
            (dt(300), 5),
            (dt(600), 25), // spike!
            (dt(900), 5),
        ];
        let anomalies = detect_anomalies(&windows, 0);
        assert_eq!(anomalies.len(), 1);
    }

    #[test]
    fn test_no_anomaly() {
        let windows = vec![(dt(0), 10), (dt(300), 12)];
        let anomalies = detect_anomalies(&windows, 0);
        assert!(anomalies.is_empty());
    }
}
