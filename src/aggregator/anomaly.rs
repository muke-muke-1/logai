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

    // Spike detection: count > avg * 3 and count > 3
    for (_time, count) in window_counts.iter() {
        if *count as f64 > avg * 3.0 && *count > 3 {
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

/// Detect SilentRecovery: error group appeared in first half of windows
/// but has zero occurrences in the most recent 2 windows.
pub fn detect_silent_recovery(
    window_counts: &[WindowCount],
    group_index: usize,
) -> Vec<Anomaly> {
    if window_counts.len() < 4 {
        return vec![];
    }

    let mid = window_counts.len() / 2;
    let appeared_early = window_counts[..mid].iter().any(|(_, c)| *c > 0);
    let silent_recently = window_counts[window_counts.len() - 2..]
        .iter()
        .all(|(_, c)| *c == 0);

    if appeared_early && silent_recently {
        vec![Anomaly::SilentRecovery { group_index }]
    } else {
        vec![]
    }
}

/// Detect PeriodicPattern: error group appears at regular intervals.
/// Standard deviation < 30% of mean interval → periodic.
/// Requires ≥3 appearances across windows.
pub fn detect_periodic_pattern(
    window_counts: &[WindowCount],
    group_index: usize,
) -> Vec<Anomaly> {
    if window_counts.len() < 3 {
        return vec![];
    }

    let appearances: Vec<i64> = window_counts
        .iter()
        .filter(|(_, c)| *c > 0)
        .map(|(t, _)| t.timestamp())
        .collect();

    if appearances.len() < 3 {
        return vec![];
    }

    let intervals: Vec<f64> = appearances
        .windows(2)
        .map(|w| (w[1] - w[0]) as f64)
        .collect();

    if intervals.is_empty() {
        return vec![];
    }

    let mean = intervals.iter().sum::<f64>() / intervals.len() as f64;
    if mean < 60.0 {
        return vec![];
    }

    let variance = intervals
        .iter()
        .map(|&x| (x - mean) * (x - mean))
        .sum::<f64>()
        / intervals.len() as f64;
    let std_dev = variance.sqrt();
    let cv = std_dev / mean;

    if cv < 0.3 {
        let period_minutes = (mean / 60.0) as u32;
        vec![Anomaly::PeriodicPattern {
            group_index,
            period_minutes: period_minutes.max(1),
        }]
    } else {
        vec![]
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
            (dt(600), 50), // spike! (50 > avg 16.25 * 3 = 48.75)
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
