use crate::aggregator::aggregate;
use crate::parser::parse_log_file;
use crate::types::{AnalysisSummary, Anomaly, ErrorGroup};
use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Style, Stylize},
    text::{Line, Span, Text},
    widgets::{Block, Borders, Clear, List, ListItem, Paragraph, Wrap},
    Frame, Terminal,
};
use std::io;
use std::path::PathBuf;

// ============================================================
// Theme system
// ============================================================

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Theme {
    Dark,
    Light,
}

impl Theme {
    pub fn toggle(&mut self) {
        *self = match self {
            Theme::Dark => Theme::Light,
            Theme::Light => Theme::Dark,
        };
    }

    /// (bg, fg, highlight, error, warn, info, selected, border)
    pub fn colors(&self) -> ThemeColors {
        match self {
            Theme::Dark => ThemeColors {
                bg: Color::Black,
                fg: Color::White,
                highlight: Color::Cyan,
                error: Color::Red,
                warn: Color::Yellow,
                info: Color::Green,
                selected: Color::DarkGray,
                border: Color::DarkGray,
            },
            Theme::Light => ThemeColors {
                bg: Color::White,
                fg: Color::Black,
                highlight: Color::Blue,
                error: Color::Red,
                warn: Color::DarkGray, // yellow invisible on white
                info: Color::Green,
                selected: Color::LightCyan,
                border: Color::Gray,
            },
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct ThemeColors {
    pub bg: Color,
    pub fg: Color,
    pub highlight: Color,
    pub error: Color,
    pub warn: Color,
    pub info: Color,
    pub selected: Color,
    pub border: Color,
}

// ============================================================
// Application state
// ============================================================

pub struct AppState {
    pub summary: AnalysisSummary,
    pub groups: Vec<ErrorGroup>,
    pub selected_index: usize,
    pub search_query: String,
    pub theme: Theme,
    pub show_help: bool,
    pub live_mode: bool,
    pub should_quit: bool,
}

impl AppState {
    pub fn new(summary: AnalysisSummary) -> Self {
        let groups = summary.error_groups.clone();
        AppState {
            summary,
            groups,
            selected_index: 0,
            search_query: String::new(),
            theme: Theme::Dark,
            show_help: false,
            live_mode: false,
            should_quit: false,
        }
    }

    /// Return indices of groups matching the current search query
    pub fn filtered_groups(&self) -> Vec<usize> {
        if self.search_query.is_empty() {
            (0..self.groups.len()).collect()
        } else {
            let q = self.search_query.to_lowercase();
            self.groups
                .iter()
                .enumerate()
                .filter(|(_, g)| g.signature.to_lowercase().contains(&q))
                .map(|(i, _)| i)
                .collect()
        }
    }

    /// Select previous group
    pub fn select_prev(&mut self) {
        let filtered = self.filtered_groups();
        if filtered.is_empty() {
            return;
        }
        if let Some(pos) = filtered.iter().position(|&i| i == self.selected_index) {
            let new_pos = if pos == 0 { filtered.len() - 1 } else { pos - 1 };
            self.selected_index = filtered[new_pos];
        } else {
            self.selected_index = filtered[0];
        }
    }

    /// Select next group
    pub fn select_next(&mut self) {
        let filtered = self.filtered_groups();
        if filtered.is_empty() {
            return;
        }
        if let Some(pos) = filtered.iter().position(|&i| i == self.selected_index) {
            let new_pos = if pos + 1 >= filtered.len() { 0 } else { pos + 1 };
            self.selected_index = filtered[new_pos];
        } else {
            self.selected_index = filtered[0];
        }
    }
}

// ============================================================
// Main event loop
// ============================================================

/// Start interactive TUI
pub fn run_interactive(file_path: PathBuf, live: bool) -> anyhow::Result<()> {
    // Parse log file
    let entries = parse_log_file(&file_path, None)?;
    if entries.is_empty() {
        println!("没有找到日志条目。");
        return Ok(());
    }

    let summary = aggregate(&entries);
    if summary.error_groups.is_empty() {
        println!("✅ 没有发现错误。日志看起来很干净！");
        return Ok(());
    }

    let mut app = AppState::new(summary);
    app.live_mode = live;

    // Setup terminal
    let mut stdout = io::stdout();
    crossterm::execute!(stdout, crossterm::terminal::EnterAlternateScreen)?;
    crossterm::terminal::enable_raw_mode()?;

    let mut terminal = Terminal::new(ratatui::backend::CrosstermBackend::new(stdout))?;

    // Main loop
    while !app.should_quit {
        terminal.draw(|f| render_ui(f, &app))?;

        if event::poll(std::time::Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                if key.kind == KeyEventKind::Press {
                    handle_key_event(key, &mut app);
                }
            }
        }
    }

    // Restore terminal
    crossterm::terminal::disable_raw_mode()?;
    crossterm::execute!(io::stdout(), crossterm::terminal::LeaveAlternateScreen)?;

    Ok(())
}

// ============================================================
// Keyboard event handling
// ============================================================

fn handle_key_event(key: event::KeyEvent, app: &mut AppState) {
    if app.show_help {
        // When help overlay is visible, any key dismisses it
        app.show_help = false;
        return;
    }

    match key.code {
        KeyCode::Char('q') => app.should_quit = true,
        KeyCode::Esc => app.should_quit = true,
        KeyCode::Up | KeyCode::Char('k') => app.select_prev(),
        KeyCode::Down | KeyCode::Char('j') => app.select_next(),
        KeyCode::Char('t') => app.theme.toggle(),
        KeyCode::Char('?') => app.show_help = true,
        KeyCode::Char('/') => {
            app.search_query.clear();
        }
        KeyCode::Backspace => {
            let _ = app.search_query.pop();
        }
        KeyCode::Char(c) => {
            app.search_query.push(c);
        }
        _ => {}
    }
}

// ============================================================
// UI rendering
// ============================================================

fn render_ui(f: &mut Frame, app: &AppState) {
    let colors = app.theme.colors();

    // Main layout: vertical split (body + status bar)
    let main_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(1), Constraint::Length(1)])
        .split(f.area());

    let body = main_chunks[0];
    let status_area = main_chunks[1];

    // Body horizontal split
    let body_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(40), Constraint::Percentage(60)])
        .split(body);

    // --- Left panel: group list ---
    render_group_list(f, body_chunks[0], app, &colors);

    // --- Right panel: detail ---
    render_detail_panel(f, body_chunks[1], app, &colors);

    // --- Status bar ---
    render_status_bar(f, status_area, app, &colors);

    // --- Help popup (if active) ---
    if app.show_help {
        render_help_popup(f, app, &colors);
    }
}

fn render_group_list(f: &mut Frame, area: Rect, app: &AppState, colors: &ThemeColors) {
    let filtered = app.filtered_groups();
    let items: Vec<ListItem> = filtered
        .iter()
        .map(|&idx| {
            let g = &app.groups[idx];
            let marker = if idx == app.selected_index { "▶ " } else { "  " };
            let count_style = if g.count > 100 {
                Style::default().fg(colors.error).bold()
            } else {
                Style::default().fg(colors.warn)
            };
            let anomaly_icon = if has_anomaly(idx, &app.summary.anomalies) {
                "⚠ "
            } else {
                ""
            };

            let line = Line::from(vec![
                Span::styled(
                    format!("{}{}", marker, anomaly_icon),
                    Style::default().fg(colors.highlight),
                ),
                Span::styled(
                    truncate_sig(&g.signature, area.width.saturating_sub(20) as usize),
                    Style::default().fg(colors.fg),
                ),
                Span::styled(format!(" ({})", g.count), count_style),
            ]);

            if idx == app.selected_index {
                ListItem::new(line).style(Style::default().bg(colors.selected))
            } else {
                ListItem::new(line)
            }
        })
        .collect();

    let title = if app.search_query.is_empty() {
        format!(" 错误分组 ({}) ", filtered.len())
    } else {
        format!(" 错误分组 ({}/{} 匹配) ", filtered.len(), app.groups.len())
    };

    let list = List::new(items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(colors.border))
                .title(title),
        )
        .highlight_style(Style::default().bg(colors.selected));

    f.render_widget(list, area);
}

fn render_detail_panel(f: &mut Frame, area: Rect, app: &AppState, colors: &ThemeColors) {
    let empty_text = Text::from("选择一个错误分组查看详情\n\n← → ↑ ↓ 移动  / 搜索  t 主题  ? 帮助  q 退出");

    if app.groups.is_empty() || app.selected_index >= app.groups.len() {
        let p = Paragraph::new(empty_text)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(colors.border))
                    .title(" 详情 "),
            )
            .wrap(Wrap { trim: true });
        f.render_widget(p, area);
        return;
    }

    let g = &app.groups[app.selected_index];
    let time_str = match (g.first_seen, g.last_seen) {
        (Some(fs), Some(ls)) => format!("{} → {}", fs.format("%H:%M:%S"), ls.format("%H:%M:%S")),
        _ => "N/A".to_string(),
    };

    let anomaly_info = app
        .summary
        .anomalies
        .iter()
        .filter_map(|a| match a {
            Anomaly::Spike {
                group_index,
                multiplier,
            } if *group_index == app.selected_index => {
                Some(format!("Spike ({}x average)", multiplier))
            }
            Anomaly::NewError { group_index }
                if *group_index == app.selected_index =>
            {
                Some("New error".to_string())
            }
            Anomaly::SilentRecovery { group_index }
                if *group_index == app.selected_index =>
            {
                Some("Silent recovery".to_string())
            }
            Anomaly::PeriodicPattern {
                group_index,
                period_minutes,
            } if *group_index == app.selected_index => {
                Some(format!("Periodic (~{}min)", period_minutes))
            }
            _ => None,
        })
        .collect::<Vec<_>>();

    let mut lines = vec![
        Line::from(vec![
            Span::styled("Signature: ", Style::default().fg(colors.border)),
            Span::styled(&g.signature, Style::default().fg(colors.error).bold()),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("Count: ", Style::default().fg(colors.border)),
            Span::styled(
                format!("{}", g.count),
                Style::default().fg(colors.highlight),
            ),
        ]),
        Line::from(vec![
            Span::styled("Time range: ", Style::default().fg(colors.border)),
            Span::styled(time_str, Style::default().fg(colors.fg)),
        ]),
    ];

    // Trend (always present, not optional)
    lines.push(Line::from(""));
    lines.push(Line::from(vec![Span::styled(
        format!("Trend: {:?}", g.trend),
        Style::default().fg(colors.info),
    )]));

    if !anomaly_info.is_empty() {
        lines.push(Line::from(""));
        lines.push(Line::from(vec![Span::styled(
            "--- Anomalies ---",
            Style::default().fg(colors.warn).bold(),
        )]));
        for a in &anomaly_info {
            lines.push(Line::from(vec![Span::styled(
                a,
                Style::default().fg(colors.warn),
            )]));
        }
    }

    // Sample lines
    if !g.samples.is_empty() {
        lines.push(Line::from(""));
        lines.push(Line::from(vec![Span::styled(
            "--- Samples ---",
            Style::default().fg(colors.border),
        )]));
        for sample in g.samples.iter().take(5) {
            lines.push(Line::from(vec![Span::styled(
                truncate_str(sample, area.width.saturating_sub(4) as usize),
                Style::default().fg(colors.fg),
            )]));
        }
    }

    // Stack trace
    if let Some(ref stack) = g.stack_trace {
        lines.push(Line::from(""));
        lines.push(Line::from(vec![Span::styled(
            "--- Stack Trace ---",
            Style::default().fg(colors.border),
        )]));
        for stack_line in stack.lines().take(10) {
            lines.push(Line::from(vec![Span::styled(
                truncate_str(stack_line, area.width.saturating_sub(4) as usize),
                Style::default().fg(Color::DarkGray),
            )]));
        }
    }

    let p = Paragraph::new(Text::from(lines))
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(colors.border))
                .title(" 详情 "),
        )
        .wrap(Wrap { trim: true });

    f.render_widget(p, area);
}

fn render_status_bar(f: &mut Frame, area: Rect, app: &AppState, colors: &ThemeColors) {
    let mode_str = if app.live_mode { "LIVE" } else { "STATIC" };
    let theme_str = match app.theme {
        Theme::Dark => " Dark",
        Theme::Light => " Light",
    };
    let search_str = if app.search_query.is_empty() {
        String::new()
    } else {
        format!("Search: \"{}\"  ", app.search_query)
    };

    let status = Line::from(vec![
        Span::styled(mode_str, Style::default().fg(colors.error).bold()),
        Span::styled(" | ", Style::default().fg(colors.border)),
        Span::styled(
            format!("{} groups / {} anomalies", app.groups.len(), app.summary.anomalies.len()),
            Style::default().fg(colors.fg),
        ),
        Span::styled(" | ", Style::default().fg(colors.border)),
        Span::styled(search_str, Style::default().fg(colors.highlight)),
        Span::styled(theme_str, Style::default().fg(colors.info)),
        Span::styled(
            "  j/k down/up  / search  t theme  ? help  q quit",
            Style::default().fg(colors.border),
        ),
    ]);

    f.render_widget(
        Paragraph::new(status).style(Style::default().bg(colors.selected)),
        area,
    );
}

fn render_help_popup(f: &mut Frame, _app: &AppState, colors: &ThemeColors) {
    let help_text = vec![
        Line::from(vec![Span::styled(
            "Keyboard Shortcuts",
            Style::default().fg(colors.highlight).bold(),
        )]),
        Line::from(""),
        Line::from(vec![
            Span::styled("  j / Down   ", Style::default().fg(colors.highlight)),
            Span::styled("Move down", Style::default().fg(colors.fg)),
        ]),
        Line::from(vec![
            Span::styled("  k / Up     ", Style::default().fg(colors.highlight)),
            Span::styled("Move up", Style::default().fg(colors.fg)),
        ]),
        Line::from(vec![
            Span::styled("  /          ", Style::default().fg(colors.highlight)),
            Span::styled("Search filter groups", Style::default().fg(colors.fg)),
        ]),
        Line::from(vec![
            Span::styled("  Backspace  ", Style::default().fg(colors.highlight)),
            Span::styled("Delete last search char", Style::default().fg(colors.fg)),
        ]),
        Line::from(vec![
            Span::styled("  t          ", Style::default().fg(colors.highlight)),
            Span::styled("Toggle dark/light theme", Style::default().fg(colors.fg)),
        ]),
        Line::from(vec![
            Span::styled("  ?          ", Style::default().fg(colors.highlight)),
            Span::styled("Show/hide this help", Style::default().fg(colors.fg)),
        ]),
        Line::from(vec![
            Span::styled("  q / Esc    ", Style::default().fg(colors.highlight)),
            Span::styled("Quit", Style::default().fg(colors.fg)),
        ]),
    ];

    let popup_area = centered_rect(50, 60, f.area());
    f.render_widget(Clear, popup_area);
    f.render_widget(
        Paragraph::new(Text::from(help_text))
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(colors.highlight))
                    .title(" Help ")
                    .style(Style::default().bg(colors.selected)),
            ),
        popup_area,
    );
}

// ============================================================
// Utility functions
// ============================================================

fn truncate_sig(s: &str, max_len: usize) -> String {
    truncate_str(s, max_len)
}

fn truncate_str(s: &str, max_len: usize) -> String {
    if max_len == 0 {
        return String::new();
    }
    if s.len() <= max_len {
        s.to_string()
    } else if max_len <= 3 {
        s[..max_len].to_string()
    } else {
        format!("{}...", &s[..max_len - 3])
    }
}

fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}

fn has_anomaly(group_index: usize, anomalies: &[Anomaly]) -> bool {
    anomalies.iter().any(|a| match a {
        Anomaly::Spike {
            group_index: gi, ..
        } => *gi == group_index,
        Anomaly::NewError { group_index: gi } => *gi == group_index,
        Anomaly::SilentRecovery { group_index: gi } => *gi == group_index,
        Anomaly::PeriodicPattern {
            group_index: gi, ..
        } => *gi == group_index,
    })
}
