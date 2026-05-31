pub mod anomaly;
pub mod bucketer;
pub mod signature;
pub mod token_budget;

use crate::types::{AnalysisSummary, Anomaly, ErrorGroup, Level, LogEntry};
use std::collections::HashMap;

/// Full aggregation pipeline: LogEntry list → AnalysisSummary
pub fn aggregate(entries: &[LogEntry]) -> AnalysisSummary {
    let total_lines = entries.len();

    // Time range
    let timestamps: Vec<_> = entries.iter().filter_map(|e| e.timestamp).collect();
    let time_start = timestamps.iter().min().copied();
    let time_end = timestamps.iter().max().copied();

    // Level distribution
    let mut level_distribution: HashMap<Level, usize> = HashMap::new();
    for entry in entries {
        let level = entry.level.unwrap_or(Level::Unknown);
        *level_distribution.entry(level).or_insert(0) += 1;
    }

    // Group by error signature
    let groups = signature::group_by_signature(entries);

    let mut error_groups: Vec<ErrorGroup> = Vec::new();
    let mut all_anomalies: Vec<Anomaly> = Vec::new();

    for (group_idx, (sig, indices)) in groups.iter().enumerate() {
        let count = indices.len();

        let first_seen = indices
            .iter()
            .filter_map(|&i| entries[i].timestamp)
            .min();
        let last_seen = indices
            .iter()
            .filter_map(|&i| entries[i].timestamp)
            .max();

        // Up to 3 raw samples (clones are necessary — summary outlives entries)
        let samples: Vec<String> = indices
            .iter()
            .take(3)
            .map(|&i| entries[i].raw_line.clone())
            .collect();

        // Representative stack trace
        let stack_trace = indices
            .iter()
            .find_map(|&i| entries[i].stack_trace.clone());

        // Trend from time-based bucketing
        let timestamps_for_group: Vec<Option<chrono::DateTime<chrono::Utc>>> =
            indices.iter().map(|&i| entries[i].timestamp).collect();
        let window_counts =
            bucketer::bucket_by_time(&timestamps_for_group, bucketer::DEFAULT_WINDOW_SECS);
        let trend = bucketer::compute_trend(&window_counts);

        // Anomaly detection
        let window_data: Vec<(chrono::DateTime<chrono::Utc>, usize)> = indices
            .iter()
            .filter_map(|&i| entries[i].timestamp.map(|t| (t, 1usize)))
            .collect();
        let mut anomalies = anomaly::detect_anomalies(&window_data, group_idx);

        // New error check
        if anomaly::is_new_error(first_seen, time_start) {
            anomalies.push(Anomaly::NewError { group_index: group_idx });
        }

        all_anomalies.extend(anomalies);

        error_groups.push(ErrorGroup {
            signature: sig.clone(),
            count,
            first_seen,
            last_seen,
            samples,
            stack_trace,
            trend,
        });
    }

    token_budget::build_summary(
        error_groups,
        all_anomalies,
        level_distribution,
        total_lines,
        time_start,
        time_end,
    )
}
