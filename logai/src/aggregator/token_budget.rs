use crate::types::{AnalysisSummary, Anomaly, ErrorGroup, Level};
use std::collections::HashMap;

const TOP_N_ERRORS: usize = 5;

/// Build the final AnalysisSummary from error groups, anomalies, and stats.
/// Trims groups to TOP_N and limits samples to 3 per group.
pub fn build_summary(
    groups: Vec<ErrorGroup>,
    anomalies: Vec<Anomaly>,
    level_dist: HashMap<Level, usize>,
    total_lines: usize,
    time_start: Option<chrono::DateTime<chrono::Utc>>,
    time_end: Option<chrono::DateTime<chrono::Utc>>,
) -> AnalysisSummary {
    let mut sorted_groups = groups;
    sorted_groups.sort_by_key(|g| -(g.count as i64));

    let trimmed_groups: Vec<ErrorGroup> = sorted_groups
        .into_iter()
        .take(TOP_N_ERRORS)
        .map(|g| {
            let samples = g.samples.into_iter().take(3).collect();
            ErrorGroup { samples, ..g }
        })
        .collect();

    AnalysisSummary {
        total_lines,
        time_start,
        time_end,
        error_groups: trimmed_groups,
        anomalies,
        level_distribution: level_dist,
    }
}

/// Rough token estimate: 4 characters ≈ 1 token
pub fn estimate_tokens(text: &str) -> usize {
    text.len() / 4
}

/// Default token budget for the AI prompt
pub fn token_budget() -> usize {
    3000
}
