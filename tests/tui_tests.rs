use logai::tui::{AppState, Theme};
use logai::types::{AnalysisSummary, ErrorGroup, Model, Trend};
use std::collections::HashMap;

fn make_empty_summary() -> AnalysisSummary {
    AnalysisSummary {
        total_lines: 0,
        time_start: None,
        time_end: None,
        level_distribution: HashMap::new(),
        error_groups: vec![],
        anomalies: vec![],
    }
}

#[test]
fn test_theme_toggle() {
    let mut theme = Theme::Dark;
    assert_eq!(theme, Theme::Dark);
    theme.toggle();
    assert_eq!(theme, Theme::Light);
    theme.toggle();
    assert_eq!(theme, Theme::Dark);
}

#[test]
fn test_app_new_with_empty_summary() {
    let summary = make_empty_summary();
    let app = AppState::new(summary, Model::Auto, false);
    assert_eq!(app.selected_index, 0);
    assert_eq!(app.search_query, "");
    assert!(!app.show_help);
    assert!(app.filtered_groups().is_empty());
}

#[test]
fn test_app_search_filters_groups() {
    let mut summary = make_empty_summary();
    summary.error_groups = vec![
        ErrorGroup {
            signature: "ConnectionPool exhausted".into(),
            count: 5,
            first_seen: None,
            last_seen: None,
            samples: vec![],
            stack_trace: None,
            trend: Trend::Stable,
        },
        ErrorGroup {
            signature: "timeout reading from socket".into(),
            count: 2,
            first_seen: None,
            last_seen: None,
            samples: vec![],
            stack_trace: None,
            trend: Trend::Stable,
        },
        ErrorGroup {
            signature: "SSL certificate expired".into(),
            count: 1,
            first_seen: None,
            last_seen: None,
            samples: vec![],
            stack_trace: None,
            trend: Trend::Stable,
        },
    ];

    let mut app = AppState::new(summary, Model::Auto, false);
    assert_eq!(app.filtered_groups().len(), 3);

    // Search "ssl"
    app.search_query = "ssl".into();
    let filtered = app.filtered_groups();
    assert_eq!(filtered.len(), 1);
    assert_eq!(
        &app.groups[filtered[0]].signature.to_lowercase(),
        "ssl certificate expired"
    );

    // Search with no match
    app.search_query = "xyznotfound".into();
    assert!(app.filtered_groups().is_empty());

    // Clear search
    app.search_query.clear();
    assert_eq!(app.filtered_groups().len(), 3);
}

#[test]
fn test_app_select_next_prev() {
    let mut summary = make_empty_summary();
    for i in 0..5 {
        summary.error_groups.push(ErrorGroup {
            signature: format!("error {}", i),
            count: 1,
            first_seen: None,
            last_seen: None,
            samples: vec![],
            stack_trace: None,
            trend: Trend::Stable,
        });
    }

    let mut app = AppState::new(summary, Model::Auto, false);
    assert_eq!(app.selected_index, 0);

    app.select_next();
    assert_eq!(app.selected_index, 1);

    app.select_prev();
    assert_eq!(app.selected_index, 0);

    // Boundary: from 0 up wraps to last
    app.select_prev();
    assert_eq!(app.selected_index, 4);

    // Boundary: from last down wraps to 0
    app.select_next();
    assert_eq!(app.selected_index, 0);
}
