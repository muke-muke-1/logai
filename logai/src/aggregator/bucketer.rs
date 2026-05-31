use crate::types::Trend;

/// Default time window size in seconds (5 minutes)
pub const DEFAULT_WINDOW_SECS: i64 = 300;

/// Compute trend from window counts: compare first half vs second half average
pub fn compute_trend(window_counts: &[usize]) -> Trend {
    if window_counts.len() < 2 {
        return Trend::Stable;
    }

    let mid = window_counts.len() / 2;
    let first_half_avg: f64 = window_counts[..mid].iter().sum::<usize>() as f64 / mid as f64;
    let second_half_avg: f64 = window_counts[mid..].iter().sum::<usize>() as f64
        / (window_counts.len() - mid) as f64;

    let threshold = 0.2; // 20% change is significant
    if second_half_avg > first_half_avg * (1.0 + threshold) {
        Trend::Rising
    } else if second_half_avg < first_half_avg * (1.0 - threshold) {
        Trend::Falling
    } else {
        Trend::Stable
    }
}

/// Bucket timestamps into fixed-size windows, return counts per window
pub fn bucket_by_time(
    timestamps: &[Option<chrono::DateTime<chrono::Utc>>],
    window_secs: i64,
) -> Vec<usize> {
    let valid: Vec<i64> = timestamps
        .iter()
        .filter_map(|t| t.map(|dt| dt.timestamp()))
        .collect();

    if valid.is_empty() {
        return vec![timestamps.len()];
    }

    let min_ts = valid.iter().min().unwrap();
    let max_ts = valid.iter().max().unwrap();
    let total_span = max_ts - min_ts;

    if total_span <= 0 {
        return vec![timestamps.len()];
    }

    let num_windows = ((total_span as f64) / (window_secs as f64)).ceil() as usize;
    let num_windows = num_windows.max(1).min(100);

    let mut buckets = vec![0usize; num_windows];
    for ts in &valid {
        let idx = ((ts - min_ts) as f64 / window_secs as f64).floor() as usize;
        if idx < num_windows {
            buckets[idx] += 1;
        }
    }

    buckets
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_trend_rising() {
        assert_eq!(compute_trend(&[10, 20, 50]), Trend::Rising);
    }

    #[test]
    fn test_trend_falling() {
        assert_eq!(compute_trend(&[50, 20, 10]), Trend::Falling);
    }

    #[test]
    fn test_trend_stable() {
        assert_eq!(compute_trend(&[10, 12, 9, 11]), Trend::Stable);
    }
}
