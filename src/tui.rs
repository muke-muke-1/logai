use crate::aggregator::aggregate;
use crate::ai::create_backend;
use crate::parser::{detect_format, parse_lines, parse_log_file};
use crate::types::{AnalysisSummary, Anomaly, ErrorGroup, Model};
use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Style, Stylize},
    text::{Line, Span, Text},
    widgets::{Block, Borders, Clear, List, ListItem, Paragraph, Wrap},
    Frame, Terminal,
};
use std::fs::File;
use std::io::{self, Read, Seek, SeekFrom};
use std::path::PathBuf;
use std::time::Instant;

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
// AI panel state
// ============================================================

#[derive(Clone, PartialEq, Eq)]
pub enum AiPanelMode {
    Hidden,
    Asking,
    Waiting,
    ShowingResponse,
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
    // AI panel fields
    pub ai_panel: AiPanelMode,
    pub ai_question: String,
    pub ai_response: String,
    pub ai_scroll: u16,
    pub model: Model,
    pub deep: bool,
    pub stack_expanded: bool,
}

impl AppState {
    pub fn new(summary: AnalysisSummary, model: Model, deep: bool) -> Self {
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
            ai_panel: AiPanelMode::Hidden,
            ai_question: String::new(),
            ai_response: String::new(),
            ai_scroll: 0,
            model,
            deep,
            stack_expanded: false,
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
            let new_pos = if pos == 0 {
                filtered.len() - 1
            } else {
                pos - 1
            };
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
            let new_pos = if pos + 1 >= filtered.len() {
                0
            } else {
                pos + 1
            };
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
pub fn run_interactive(
    file_path: PathBuf,
    live: bool,
    model: Model,
    deep: bool,
) -> anyhow::Result<()> {
    // Parse log file
    let entries = parse_log_file(&file_path, None)?;
    if entries.is_empty() {
        println!("没有找到日志条目。");
        return Ok(());
    }

    // Detect format once (for live-mode incremental parsing)
    let format = {
        let sample: Vec<String> = entries
            .iter()
            .take(10)
            .map(|e| e.raw_line.clone())
            .collect();
        detect_format(&sample)
    };

    let summary = aggregate(&entries);
    if summary.error_groups.is_empty() {
        println!("✅ 没有发现错误。日志看起来很干净！");
        return Ok(());
    }

    let mut app = AppState::new(summary, model, deep);
    app.live_mode = live;

    // Live-mode state: track file position for incremental reads
    let mut last_position = std::fs::metadata(&file_path)?.len();
    let mut all_entries = entries;
    let mut last_check = Instant::now();
    let poll_interval = std::time::Duration::from_secs(1);

    // Setup terminal
    let mut stdout = io::stdout();
    crossterm::execute!(stdout, crossterm::terminal::EnterAlternateScreen)?;
    crossterm::terminal::enable_raw_mode()?;

    let mut terminal = Terminal::new(ratatui::backend::CrosstermBackend::new(stdout))?;

    // Main loop
    while !app.should_quit {
        terminal.draw(|f| render_ui(f, &app))?;

        // Live mode: poll file for changes
        if app.live_mode && last_check.elapsed() >= poll_interval {
            last_check = Instant::now();
            if let Ok(metadata) = std::fs::metadata(&file_path) {
                let current_size = metadata.len();
                if current_size > last_position {
                    // Read new content
                    if let Ok(mut file) = File::open(&file_path) {
                        if file.seek(SeekFrom::Start(last_position)).is_ok() {
                            let mut buf = String::new();
                            if file.read_to_string(&mut buf).is_ok() {
                                let new_lines: Vec<String> =
                                    buf.lines().map(String::from).collect();
                                if !new_lines.is_empty() {
                                    let parsed = parse_lines(&new_lines, format);
                                    all_entries.extend(parsed);
                                    let new_summary = aggregate(&all_entries);
                                    if !new_summary.error_groups.is_empty() {
                                        app.summary = new_summary.clone();
                                        app.groups = new_summary.error_groups;
                                        if app.selected_index >= app.groups.len() {
                                            app.selected_index = app.groups.len().saturating_sub(1);
                                        }
                                    }
                                }
                            }
                        }
                    }
                    last_position = current_size;
                } else if current_size < last_position {
                    // File truncated — reset
                    last_position = 0;
                    all_entries.clear();
                }
            }
        }

        // AI panel: fire async call when user submits question
        if app.ai_panel == AiPanelMode::Waiting {
            let group = if app.selected_index < app.groups.len() {
                app.groups[app.selected_index].clone()
            } else {
                app.ai_response = "没有选中的错误分组".to_string();
                app.ai_panel = AiPanelMode::ShowingResponse;
                continue;
            };
            let question = app.ai_question.clone();

            // Build a focused prompt about this specific error group
            let mut prompt = String::new();
            prompt.push_str("你是一个专业的日志分析工程师。用户正在查看以下错误：\n\n");
            prompt.push_str(&format!("错误签名: {}\n", group.signature));
            prompt.push_str(&format!("出现次数: {}\n", group.count));
            if let (Some(fs), Some(ls)) = (group.first_seen, group.last_seen) {
                prompt.push_str(&format!(
                    "时间范围: {} → {}\n",
                    fs.format("%Y-%m-%d %H:%M:%S"),
                    ls.format("%Y-%m-%d %H:%M:%S")
                ));
            }
            prompt.push_str(&format!("趋势: {:?}\n", group.trend));
            if !group.samples.is_empty() {
                prompt.push_str("\n样本日志:\n");
                for s in &group.samples {
                    prompt.push_str(&format!("  {}\n", s));
                }
            }
            if let Some(ref stack) = group.stack_trace {
                prompt.push_str(&format!("\n堆栈跟踪:\n{}\n", stack));
            }
            prompt.push_str(&format!(
                "\n用户的追问：\n{}\n\n请分析这个错误的可能原因，并提供具体的修复建议。用中文回答，直接给出分析结论，不要JSON格式。",
                question
            ));

            // Call AI via block_in_place (TUI is sync, AI is async)
            let result = tokio::task::block_in_place(|| {
                tokio::runtime::Handle::current().block_on(async {
                    let backend = create_backend(app.model, app.deep).await?;
                    backend.chat(&prompt).await
                })
            });

            match result {
                Ok(response) => {
                    app.ai_response = response;
                }
                Err(e) => {
                    app.ai_response = format!(
                        "❌ AI 调用失败: {}\n\n请确认已设置对应的 API Key 环境变量。",
                        e
                    );
                }
            }
            app.ai_panel = AiPanelMode::ShowingResponse;
            app.ai_scroll = 0;
        }

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

    // AI panel key handling takes priority
    match app.ai_panel {
        AiPanelMode::Asking => {
            match key.code {
                KeyCode::Esc => {
                    app.ai_panel = AiPanelMode::Hidden;
                    app.ai_question.clear();
                }
                KeyCode::Enter => {
                    if !app.ai_question.trim().is_empty() {
                        app.ai_panel = AiPanelMode::Waiting;
                    }
                }
                KeyCode::Backspace => {
                    let _ = app.ai_question.pop();
                }
                KeyCode::Char(c) => {
                    app.ai_question.push(c);
                }
                _ => {}
            }
            return;
        }
        AiPanelMode::Waiting => {
            // Block all input while waiting
            return;
        }
        AiPanelMode::ShowingResponse => {
            match key.code {
                KeyCode::Esc | KeyCode::Char('q') => {
                    app.ai_panel = AiPanelMode::Hidden;
                    app.ai_response.clear();
                    app.ai_scroll = 0;
                }
                KeyCode::Down | KeyCode::Char('j') => {
                    app.ai_scroll = app.ai_scroll.saturating_add(1);
                }
                KeyCode::Up | KeyCode::Char('k') => {
                    app.ai_scroll = app.ai_scroll.saturating_sub(1);
                }
                _ => {}
            }
            return;
        }
        AiPanelMode::Hidden => { /* fall through to normal keys */ }
    }

    match key.code {
        KeyCode::Char('q') => app.should_quit = true,
        KeyCode::Esc => app.should_quit = true,
        KeyCode::Up | KeyCode::Char('k') => app.select_prev(),
        KeyCode::Down | KeyCode::Char('j') => app.select_next(),
        KeyCode::Char('t') => app.theme.toggle(),
        KeyCode::Char('?') => app.show_help = true,
        KeyCode::Char('a') => {
            app.ai_panel = AiPanelMode::Asking;
            app.ai_question.clear();
            app.ai_response.clear();
            app.ai_scroll = 0;
        }
        KeyCode::Char('/') => {
            app.search_query.clear();
        }
        KeyCode::Backspace => {
            let _ = app.search_query.pop();
        }
        KeyCode::Enter => {
            app.stack_expanded = !app.stack_expanded;
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

    // --- AI panel popup (if active) ---
    if app.ai_panel != AiPanelMode::Hidden {
        render_ai_panel(f, app, &colors);
    }
}

fn render_group_list(f: &mut Frame, area: Rect, app: &AppState, colors: &ThemeColors) {
    let filtered = app.filtered_groups();
    let items: Vec<ListItem> = filtered
        .iter()
        .map(|&idx| {
            let g = &app.groups[idx];
            let marker = if idx == app.selected_index {
                "▶ "
            } else {
                "  "
            };
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
                    truncate_str(&g.signature, area.width.saturating_sub(20) as usize),
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
    let empty_text =
        Text::from("选择一个错误分组查看详情\n\nj/k 移动  / 搜索  t 主题  ? 帮助  q 退出");

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
                Some(format!("突增 ({}x 平均值)", multiplier))
            }
            Anomaly::NewError { group_index } if *group_index == app.selected_index => {
                Some("新错误".to_string())
            }
            Anomaly::SilentRecovery { group_index } if *group_index == app.selected_index => {
                Some("静默恢复".to_string())
            }
            Anomaly::PeriodicPattern {
                group_index,
                period_minutes,
            } if *group_index == app.selected_index => {
                Some(format!("周期性 (~{}分钟)", period_minutes))
            }
            _ => None,
        })
        .collect::<Vec<_>>();

    // === NEW ORDER: 错误签名 → 元数据 → 异常 → 样本 → 堆栈(可折叠) ===

    let mut lines = vec![];

    // 1. Error signature — most important, prominent
    lines.push(Line::from(vec![
        Span::styled(
            &g.signature,
            Style::default().fg(colors.error).bold(),
        ),
    ]));
    lines.push(Line::from(""));

    // 2. Compact metadata: count + time + trend in one visual block
    lines.push(Line::from(vec![
        Span::styled(
            format!("出现 {} 次", g.count),
            Style::default().fg(colors.highlight),
        ),
        Span::styled("  |  ", Style::default().fg(colors.border)),
        Span::styled(time_str, Style::default().fg(colors.fg)),
    ]));
    lines.push(Line::from(vec![Span::styled(
        format!("趋势: {:?}", g.trend),
        Style::default().fg(colors.info),
    )]));

    // 3. Anomalies (if any)
    if !anomaly_info.is_empty() {
        lines.push(Line::from(""));
        for a in &anomaly_info {
            lines.push(Line::from(vec![Span::styled(
                format!("⚠ {}", a),
                Style::default().fg(colors.warn),
            )]));
        }
    }

    // 4. Samples — evidence
    if !g.samples.is_empty() {
        lines.push(Line::from(""));
        lines.push(Line::from(vec![Span::styled(
            "── 原始日志 ──",
            Style::default().fg(colors.border),
        )]));
        for sample in g.samples.iter().take(5) {
            lines.push(Line::from(vec![Span::styled(
                truncate_str(sample, area.width.saturating_sub(4) as usize),
                Style::default().fg(colors.fg),
            )]));
        }
    }

    // 5. Stack trace — collapsible with Enter
    if let Some(ref stack) = g.stack_trace {
        lines.push(Line::from(""));
        let collapse_hint = if app.stack_expanded {
            "── 堆栈跟踪 (Enter 折叠) ──"
        } else {
            "── 堆栈跟踪 (Enter 展开) ──"
        };
        lines.push(Line::from(vec![Span::styled(
            collapse_hint,
            Style::default().fg(colors.border),
        )]));

        let stack_lines: Vec<&str> = stack.lines().collect();
        let show_count = if app.stack_expanded {
            stack_lines.len()
        } else {
            stack_lines.len().min(3)
        };
        for stack_line in stack_lines.iter().take(show_count) {
            lines.push(Line::from(vec![Span::styled(
                truncate_str(stack_line, area.width.saturating_sub(4) as usize),
                Style::default().fg(Color::DarkGray),
            )]));
        }
        if !app.stack_expanded && stack_lines.len() > 3 {
            lines.push(Line::from(vec![Span::styled(
                format!("... 还有 {} 行，按 Enter 展开", stack_lines.len() - 3),
                Style::default().fg(colors.border),
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
            format!(
                "{} groups / {} anomalies",
                app.groups.len(),
                app.summary.anomalies.len()
            ),
            Style::default().fg(colors.fg),
        ),
        Span::styled(" | ", Style::default().fg(colors.border)),
        Span::styled(search_str, Style::default().fg(colors.highlight)),
        Span::styled(theme_str, Style::default().fg(colors.info)),
        Span::styled(
            "  j/k down/up  / search  t theme  a ask-ai  ? help  q quit",
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
            Span::styled("  a          ", Style::default().fg(colors.highlight)),
            Span::styled(
                "Ask AI about selected error",
                Style::default().fg(colors.fg),
            ),
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
        Paragraph::new(Text::from(help_text)).block(
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
// AI panel rendering
// ============================================================

fn render_ai_panel(f: &mut Frame, app: &AppState, colors: &ThemeColors) {
    let popup_area = centered_rect(70, 70, f.area());
    f.render_widget(Clear, popup_area);

    match app.ai_panel {
        AiPanelMode::Asking => {
            let cursor = if app.ai_question.is_empty() {
                "█".to_string()
            } else {
                "".to_string()
            };
            let text = vec![
                Line::from(vec![Span::styled(
                    "🤖 向 AI 追问根因",
                    Style::default().fg(colors.highlight).bold(),
                )]),
                Line::from(""),
                Line::from(vec![Span::styled(
                    "输入你的问题，按 Enter 提交，Esc 取消",
                    Style::default().fg(colors.border),
                )]),
                Line::from(""),
                Line::from(vec![Span::styled(
                    format!("> {}{}", app.ai_question, cursor),
                    Style::default().fg(colors.fg),
                )]),
            ];

            f.render_widget(
                Paragraph::new(Text::from(text))
                    .block(
                        Block::default()
                            .borders(Borders::ALL)
                            .border_style(Style::default().fg(colors.highlight))
                            .title(" AI 追问 ")
                            .style(Style::default().bg(colors.selected)),
                    )
                    .wrap(Wrap { trim: true }),
                popup_area,
            );
        }
        AiPanelMode::Waiting => {
            let text = vec![
                Line::from(vec![Span::styled(
                    "🤖 正在分析...",
                    Style::default().fg(colors.highlight).bold(),
                )]),
                Line::from(""),
                Line::from(vec![Span::styled(
                    "请稍候，AI 正在分析你选中的错误...",
                    Style::default().fg(colors.fg),
                )]),
            ];

            f.render_widget(
                Paragraph::new(Text::from(text))
                    .block(
                        Block::default()
                            .borders(Borders::ALL)
                            .border_style(Style::default().fg(colors.warn))
                            .title(" AI 分析中 ")
                            .style(Style::default().bg(colors.selected)),
                    )
                    .wrap(Wrap { trim: true }),
                popup_area,
            );
        }
        AiPanelMode::ShowingResponse => {
            let mut lines: Vec<Line> = vec![
                Line::from(vec![Span::styled(
                    "🤖 AI 分析结果",
                    Style::default().fg(colors.highlight).bold(),
                )]),
                Line::from(""),
            ];

            // Add response lines, scrolled
            let display_lines: Vec<&str> = app.ai_response.lines().collect();
            let max_scroll = display_lines
                .len()
                .saturating_sub(popup_area.height.saturating_sub(6) as usize);
            let scroll = (app.ai_scroll as usize).min(max_scroll);
            let visible = &display_lines[scroll..];

            for line in visible
                .iter()
                .take(popup_area.height.saturating_sub(6) as usize)
            {
                lines.push(Line::from(vec![Span::styled(
                    *line,
                    Style::default().fg(colors.fg),
                )]));
            }

            // Scroll indicator
            if max_scroll > 0 {
                lines.push(Line::from(""));
                lines.push(Line::from(vec![Span::styled(
                    format!(
                        "--- {}/{} (j/k 滚动, q/Esc 关闭) ---",
                        scroll + 1,
                        max_scroll + 1
                    ),
                    Style::default().fg(colors.border),
                )]));
            } else {
                lines.push(Line::from(""));
                lines.push(Line::from(vec![Span::styled(
                    "按 q/Esc 关闭",
                    Style::default().fg(colors.border),
                )]));
            }

            f.render_widget(
                Paragraph::new(Text::from(lines))
                    .block(
                        Block::default()
                            .borders(Borders::ALL)
                            .border_style(Style::default().fg(colors.highlight))
                            .title(" AI 回答 ")
                            .style(Style::default().bg(colors.selected)),
                    )
                    .wrap(Wrap { trim: true }),
                popup_area,
            );
        }
        AiPanelMode::Hidden => { /* unreachable */ }
    }
}

// ============================================================
// Utility functions
// ============================================================

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
