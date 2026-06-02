pub mod anomaly;
pub mod bucketer;
pub mod signature;
pub mod token_budget;

use crate::types::{
    AnalysisSummary, Anomaly, Correlation, ErrorGroup, Level, LogEntry, SourceAnalysis,
};
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

        let first_seen = indices.iter().filter_map(|&i| entries[i].timestamp).min();
        let last_seen = indices.iter().filter_map(|&i| entries[i].timestamp).max();

        // Up to 3 raw samples (clones are necessary — summary outlives entries)
        let samples: Vec<String> = indices
            .iter()
            .take(3)
            .map(|&i| entries[i].raw_line.clone())
            .collect();

        // Representative stack trace
        let stack_trace = indices.iter().find_map(|&i| entries[i].stack_trace.clone());

        // Trend from time-based bucketing
        let timestamps_for_group: Vec<Option<chrono::DateTime<chrono::Utc>>> =
            indices.iter().map(|&i| entries[i].timestamp).collect();
        let window_counts =
            bucketer::bucket_by_time(&timestamps_for_group, bucketer::DEFAULT_WINDOW_SECS);
        let trend = bucketer::compute_trend(&window_counts);

        // Build windowed counts for anomaly detection (reuse bucketed data)
        // FIX: previously passed per-event (timestamp,1) pairs to spike detector,
        // which meant spike detection never actually worked in production.
        // Now uses proper windowed counts from bucketer.
        let anomaly_windows: Vec<(chrono::DateTime<chrono::Utc>, usize)> = {
            let ref_ts = first_seen.unwrap_or(chrono::DateTime::UNIX_EPOCH);
            window_counts
                .iter()
                .enumerate()
                .map(|(i, &c)| {
                    let ts = ref_ts
                        + chrono::Duration::seconds((i as i64) * bucketer::DEFAULT_WINDOW_SECS);
                    (ts, c)
                })
                .collect()
        };

        let mut anomalies = anomaly::detect_anomalies(&anomaly_windows, group_idx);

        // New error check
        if anomaly::is_new_error(first_seen, time_start) {
            anomalies.push(Anomaly::NewError {
                group_index: group_idx,
            });
        }

        // SilentRecovery check
        anomalies.extend(anomaly::detect_silent_recovery(&anomaly_windows, group_idx));

        // PeriodicPattern check
        anomalies.extend(anomaly::detect_periodic_pattern(
            &anomaly_windows,
            group_idx,
        ));

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

    // Cap anomaly types to at most 3 each per spec
    let mut spike_count = 0;
    let mut new_error_count = 0;
    let mut recovery_count = 0;
    let mut periodic_count = 0;
    all_anomalies.retain(|a| match a {
        Anomaly::Spike { .. } => {
            spike_count += 1;
            spike_count <= 3
        }
        Anomaly::NewError { .. } => {
            new_error_count += 1;
            new_error_count <= 3
        }
        Anomaly::SilentRecovery { .. } => {
            recovery_count += 1;
            recovery_count <= 3
        }
        Anomaly::PeriodicPattern { .. } => {
            periodic_count += 1;
            periodic_count <= 3
        }
    });

    token_budget::build_summary(
        error_groups,
        all_anomalies,
        level_distribution,
        total_lines,
        time_start,
        time_end,
    )
}

/// Detect cross-source correlations based on time overlap and error signature similarity.
pub fn detect_cross_correlations(sources: &[SourceAnalysis]) -> Vec<Correlation> {
    let mut correlations = Vec::new();

    for i in 0..sources.len() {
        for j in (i + 1)..sources.len() {
            let a = &sources[i];
            let b = &sources[j];

            // Correlation 1: Error burst overlap in time windows
            if let (Some(a_start), Some(a_end), Some(b_start), Some(b_end)) = (
                a.summary.time_start,
                a.summary.time_end,
                b.summary.time_start,
                b.summary.time_end,
            ) {
                let overlap_start = if a_start > b_start { a_start } else { b_start };
                let overlap_end = if a_end < b_end { a_end } else { b_end };
                let overlap_dur = (overlap_end - overlap_start).num_seconds();
                let total_dur = (if a_end > b_end { a_end } else { b_end }
                    - if a_start < b_start { a_start } else { b_start })
                .num_seconds();

                if total_dur > 0 {
                    let time_overlap = overlap_dur.max(0) as f64 / total_dur as f64;
                    if time_overlap > 0.5 {
                        correlations.push(Correlation {
                            source_a: a.name.clone(),
                            source_b: b.name.clone(),
                            score: time_overlap as f32,
                            description: format!(
                                "时间窗口高度重叠 ({:.0}%)，{} 与 {} 的错误可能由同一事件触发",
                                time_overlap * 100.0,
                                a.name,
                                b.name
                            ),
                        });
                    }
                }
            }

            // Correlation 2: Similar error rate patterns
            let a_errors = *a
                .summary
                .level_distribution
                .get(&Level::Error)
                .unwrap_or(&0) as f64;
            let b_errors = *b
                .summary
                .level_distribution
                .get(&Level::Error)
                .unwrap_or(&0) as f64;
            let a_total = a.summary.total_lines.max(1) as f64;
            let b_total = b.summary.total_lines.max(1) as f64;

            let a_rate = a_errors / a_total;
            let b_rate = b_errors / b_total;

            let rate_similarity = if a_rate.max(b_rate) > 0.0 {
                1.0 - (a_rate - b_rate).abs() / a_rate.max(b_rate)
            } else {
                0.0
            };

            if rate_similarity > 0.5 && a_errors > 0.0 && b_errors > 0.0 {
                correlations.push(Correlation {
                    source_a: a.name.clone(),
                    source_b: b.name.clone(),
                    score: rate_similarity as f32,
                    description: format!(
                        "错误率相似 ({:.0}% vs {:.0}%)，{} 与 {} 可能受相同根因影响",
                        a_rate * 100.0,
                        b_rate * 100.0,
                        a.name,
                        b.name
                    ),
                });
            }
        }
    }

    // Sort by score descending, keep top 5
    correlations.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap());
    correlations.truncate(5);
    correlations
}
