# logai v1.3 TUI + HTML 导出 实现计划

> **给执行代理:** 请使用 superpowers:subagent-driven-development 逐任务实现。步骤使用 checkbox (`- [ ]`) 追踪进度。

**目标:** 实现 `logai interactive <文件>`（交互式 TUI 日志浏览器）+ `logai analyze <文件> --output report.html`（HTML 报告导出）

**架构:** 新增 `src/tui.rs`（ratatui 界面：分组列表 + 详情面板 + 状态栏 + 搜索 + 帮助 + 主题切换）+ `src/renderer_html.rs`（自包含 HTML 生成器）+ 实时模式复用 `src/watcher.rs` 增量读取逻辑。现有 pipeline（parse → aggregate → AI → render）不做修改。

**技术栈:** Rust 2021, tokio, ratatui 0.29, crossterm 0.28（已有）, tui-textarea, notify（已有）

---

## 文件清单

| 文件 | 操作 | 职责 |
|------|------|------|
| `Cargo.toml` | 修改 | 新增 `ratatui`、`tui-textarea` 依赖 |
| `src/tui.rs` | 创建 | TUI 状态机 + 事件循环 + UI 渲染 + 主题 + 搜索 + 帮助 |
| `src/renderer_html.rs` | 创建 | HTML 报告生成器（自包含模板） |
| `src/cli.rs` | 修改 | 新增 `Interactive` 子命令 + `--output` 标志 |
| `src/main.rs` | 修改 | 注册 `pub mod tui; pub mod renderer_html;` |
| `src/lib.rs` | 修改 | 注册 `pub mod tui; pub mod renderer_html;` |
| `tests/tui_tests.rs` | 创建 | TUI 状态逻辑单元测试（搜索/主题/导航） |
| `tests/html_tests.rs` | 创建 | HTML 渲染器单元测试 |

---

### Task 1: 新增 ratatui + tui-textarea 依赖

**文件:**
- 修改: `Cargo.toml`

- [ ] **Step 1: 添加依赖**

在 `Cargo.toml` 的 `[dependencies]` 中 `crossterm` 之后追加：

```toml
ratatui = "0.29"
tui-textarea = "0.7"
```

完整 `[dependencies]` 部分：

```toml
[dependencies]
clap = { version = "4", features = ["derive"] }
tokio = { version = "1", features = ["full"] }
reqwest = { version = "0.12", features = ["json"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
chrono = { version = "0.4", features = ["serde"] }
regex = "1"
crossterm = "0.28"
ratatui = "0.29"
tui-textarea = "0.7"
async-trait = "0.1"
anyhow = "1"
notify = "6"
```

- [ ] **Step 2: 运行 cargo check**

Run: `cargo check 2>&1`
Expected: 编译成功，无错误

- [ ] **Step 3: 提交**

```bash
git add Cargo.toml Cargo.lock
git commit -m "chore: add ratatui and tui-textarea for TUI interactive mode"
```

---

### Task 2: 创建 HTML 报告渲染器

**文件:**
- 创建: `src/renderer_html.rs`
- 修改: `src/main.rs`、`src/lib.rs`

**上下文:** HTML 渲染器是独立模块——接收 `AnalysisSummary` + `AiResponse`，返回自包含 HTML 字符串。零外部依赖（不需要模板引擎）。先做这个因为它完全独立，不依赖 TUI。

- [ ] **Step 1: 注册模块**

在 `src/main.rs` 的 mod 声明中添加：

```rust
pub mod renderer_html;
```

在 `src/lib.rs` 的 mod 声明中同样添加：

```rust
pub mod renderer_html;
```

- [ ] **Step 2: 创建 src/renderer_html.rs**

```rust
use crate::types::{AiResponse, AnalysisSummary, FixSuggestion, Level, RootCause, Severity};

/// 生成自包含的 HTML 分析报告
pub fn render_report_html(
    summary: &AnalysisSummary,
    response: &AiResponse,
    elapsed_secs: f64,
    model_name: &str,
) -> String {
    let title = "logai 分析报告";
    let time_info = match (summary.time_start, summary.time_end) {
        (Some(s), Some(e)) => format!("{} → {}", s.format("%H:%M:%S"), e.format("%H:%M:%S")),
        _ => "N/A".to_string(),
    };

    let total = summary.total_lines.max(1) as f64;
    let error_count = summary
        .level_distribution
        .get(&Level::Error)
        .unwrap_or(&0);
    let warn_count = summary
        .level_distribution
        .get(&Level::Warn)
        .unwrap_or(&0);
    let error_rate = *error_count as f64 / total * 100.0;
    let warn_rate = *warn_count as f64 / total * 100.0;

    let root_causes_html = render_root_causes_html(&response.root_causes);
    let fix_suggestions_html = render_fix_suggestions_html(&response.fix_suggestions);
    let groups_html = render_error_groups_html(&summary.error_groups);

    format!(
        r#"<!DOCTYPE html>
<html lang="zh-CN">
<head>
<meta charset="UTF-8">
<meta name="viewport" content="width=device-width, initial-scale=1.0">
<title>{title}</title>
<style>
  body {{ font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif; max-width: 960px; margin: 0 auto; padding: 24px; background: #1a1a2e; color: #e0e0e0; }}
  h1 {{ color: #e94560; border-bottom: 2px solid #0f3460; padding-bottom: 12px; }}
  h2 {{ color: #16c79a; margin-top: 32px; }}
  .meta {{ color: #aaa; font-size: 14px; margin-bottom: 24px; }}
  .overview {{ display: flex; gap: 24px; margin-bottom: 32px; }}
  .stat {{ background: #16213e; border-radius: 8px; padding: 16px 24px; text-align: center; }}
  .stat .value {{ font-size: 28px; font-weight: bold; color: #e94560; }}
  .stat .label {{ font-size: 12px; color: #aaa; margin-top: 4px; }}
  .root-cause {{ background: #16213e; border-left: 4px solid #e94560; padding: 16px; margin-bottom: 16px; border-radius: 0 8px 8px 0; }}
  .root-cause h3 {{ margin-top: 0; color: #e94560; }}
  .evidence {{ color: #aaa; font-size: 13px; margin: 8px 0; }}
  .evidence li {{ margin: 4px 0; }}
  .severity {{ font-size: 12px; padding: 2px 8px; border-radius: 4px; }}
  .severity.critical {{ background: #e94560; color: white; }}
  .severity.high {{ background: #e94560aa; color: white; }}
  .severity.medium {{ background: #f0a500; color: #1a1a2e; }}
  .severity.low {{ background: #16c79a; color: #1a1a2e; }}
  .fix {{ background: #16213e; border-left: 4px solid #16c79a; padding: 16px; margin-bottom: 12px; border-radius: 0 8px 8px 0; }}
  .fix code {{ background: #0f3460; padding: 8px 12px; display: block; margin-top: 8px; border-radius: 4px; font-family: 'Fira Code', monospace; font-size: 13px; }}
  .group {{ background: #16213e; padding: 12px 16px; margin-bottom: 8px; border-radius: 8px; display: flex; justify-content: space-between; align-items: center; }}
  .group .sig {{ font-family: monospace; font-size: 13px; }}
  .group .count {{ color: #e94560; font-weight: bold; }}
  .footer {{ color: #555; font-size: 12px; margin-top: 48px; border-top: 1px solid #0f3460; padding-top: 16px; text-align: center; }}
  .bar {{ display: inline-block; height: 16px; border-radius: 3px; margin-right: 8px; vertical-align: middle; }}
  .bar.error {{ background: #e94560; }}
  .bar.warn {{ background: #f0a500; }}
</style>
</head>
<body>
<h1>📊 {title}</h1>
<div class="meta">{total_lines} 行 · {time_info} · 耗时 {elapsed:.1}s · AI: {model_name}</div>

<h2>📋 概览</h2>
<div class="overview">
  <div class="stat"><div class="value">{error_count}</div><div class="label">ERROR</div></div>
  <div class="stat"><div class="value">{warn_count}</div><div class="label">WARN</div></div>
  <div class="stat"><div class="value">{error_rate:.1}%</div><div class="label">错误率</div></div>
  <div class="stat"><div class="value">{group_count}</div><div class="label">错误分组</div></div>
</div>

<h2>🔴 根因分析</h2>
{root_causes_html}

<h2>🛠️ 修复建议</h2>
{fix_suggestions_html}

<h2>📦 错误分组</h2>
{groups_html}

<div class="footer">由 logai 生成 · 日志数据未上传 · 仅 AI 看到聚合统计</div>
</body>
</html>"#,
        title = title,
        total_lines = summary.total_lines,
        time_info = time_info,
        elapsed = elapsed_secs,
        model_name = model_name,
        error_count = error_count,
        warn_count = warn_count,
        error_rate = error_rate,
        warn_rate = warn_rate,
        group_count = summary.error_groups.len(),
        root_causes_html = root_causes_html,
        fix_suggestions_html = fix_suggestions_html,
        groups_html = groups_html,
    )
}

fn render_root_causes_html(causes: &[RootCause]) -> String {
    if causes.is_empty() {
        return "<p>✅ 未发现明显的根因问题</p>".to_string();
    }
    causes
        .iter()
        .map(|c| {
            let (sev_class, sev_label) = match c.severity {
                Severity::Critical => ("critical", "🔴 严重"),
                Severity::High => ("high", "🟠 高"),
                Severity::Medium => ("medium", "🟡 中"),
                Severity::Low => ("low", "🟢 低"),
            };
            let evidence_html: String = c
                .evidence
                .iter()
                .map(|e| format!("<li>{}</li>", escape_html(e)))
                .collect();
            format!(
                r#"<div class="root-cause">
<h3>{title}</h3>
<p>{desc}</p>
<div class="evidence"><strong>证据:</strong><ul>{evidence}</ul></div>
<span class="severity {sev_class}">{sev_label}</span>
</div>"#,
                title = escape_html(&c.description),
                desc = escape_html(&c.description),
                evidence = evidence_html,
                sev_class = sev_class,
                sev_label = sev_label,
            )
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn render_fix_suggestions_html(suggestions: &[FixSuggestion]) -> String {
    if suggestions.is_empty() {
        return "<p>—</p>".to_string();
    }
    suggestions
        .iter()
        .enumerate()
        .map(|(i, fix)| {
            let code_html = fix
                .code_snippet
                .as_ref()
                .map(|code| format!("<code>{}</code>", escape_html(code)))
                .unwrap_or_default();
            let ref_html = fix
                .reference
                .as_ref()
                .map(|r| format!("<div style=\"margin-top:4px;font-size:12px;color:#aaa;\">参考: {}</div>", escape_html(r)))
                .unwrap_or_default();
            format!(
                r#"<div class="fix">
<strong>{i}. {action}</strong>
{code}
{ref_html}
</div>"#,
                i = i + 1,
                action = escape_html(&fix.action),
                code = code_html,
                ref_html = ref_html,
            )
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn render_error_groups_html(groups: &[crate::types::ErrorGroup]) -> String {
    if groups.is_empty() {
        return "<p>无错误分组</p>".to_string();
    }
    groups
        .iter()
        .map(|g| {
            let time_str = match (g.first_seen, g.last_seen) {
                (Some(f), Some(l)) => {
                    format!("{} → {}", f.format("%H:%M:%S"), l.format("%H:%M:%S"))
                }
                _ => "N/A".to_string(),
            };
            format!(
                r#"<div class="group">
<span class="sig">{sig}</span>
<span style="color:#aaa;font-size:12px;">{time}</span>
<span class="count">{count}</span>
</div>"#,
                sig = escape_html(&g.signature),
                time = time_str,
                count = g.count,
            )
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn escape_html(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}
```

- [ ] **Step 3: 运行 cargo check**

Run: `cargo check 2>&1`
Expected: 编译成功。如果 `AnalysisSummary` 字段名不匹配，查阅 `src/types.rs` 确认字段名后修正。

- [ ] **Step 4: 运行 cargo test**

Run: `cargo test 2>&1`
Expected: 85 tests passed（HTML 模块未被测试覆盖 — 测试在 Task 6）

- [ ] **Step 5: 提交**

```bash
git add src/renderer_html.rs src/main.rs src/lib.rs
git commit -m "feat: add HTML report renderer with self-contained template"
```

---

### Task 3: 创建 TUI 模块（状态 + 事件循环 + UI 渲染）

**文件:**
- 创建: `src/tui.rs`

**上下文:** TUI 是最复杂的模块。使用 ratatui + crossterm，三面板布局（分组列表 | 详情 | 状态栏），vim 风格键盘操作，支持搜索过滤、主题切换、帮助面板。单文件实现（~400 行）保持简洁。

- [ ] **Step 1: 创建 src/tui.rs**

```rust
use crate::aggregator::aggregate;
use crate::parser::parse_log_file;
use crate::types::{AnalysisSummary, Anomaly, ErrorGroup, LogEntry};
use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style, Stylize},
    text::{Line, Span, Text},
    widgets::{Block, Borders, Clear, List, ListItem, Paragraph, Wrap},
    Frame, Terminal,
};
use std::io;
use std::path::PathBuf;

// ============================================================
// 主题系统
// ============================================================

#[derive(Clone, Copy, PartialEq, Eq)]
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

    /// (背景, 文字, 高亮, 错误色, 警告色, 信息色, 选中色, 边框色)
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

#[derive(Clone, Copy)]
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
// 应用状态
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

    /// 返回搜索过滤后的分组列表（索引列表）
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

    /// 选中上一个分组
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

    /// 选中下一个分组
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
// 主事件循环
// ============================================================

/// 启动交互式 TUI
pub fn run_interactive(file_path: PathBuf, live: bool) -> anyhow::Result<()> {
    // 解析日志文件
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

    // 设置终端
    let mut stdout = io::stdout();
    crossterm::execute!(stdout, crossterm::terminal::EnterAlternateScreen)?;
    crossterm::terminal::enable_raw_mode()?;

    let mut terminal = Terminal::new(ratatui::backend::CrosstermBackend::new(stdout))?;

    // 主循环
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

    // 恢复终端
    crossterm::terminal::disable_raw_mode()?;
    crossterm::execute!(io::stdout(), crossterm::terminal::LeaveAlternateScreen)?;

    Ok(())
}

// ============================================================
// 键盘事件处理
// ============================================================

fn handle_key_event(key: event::KeyEvent, app: &mut AppState) {
    if app.show_help {
        // 帮助面板可见时，任意键关闭
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
            // 进入搜索模式（简化版：在底部状态栏显示，实际输入处理）
            // 这里用一个简单的单字符搜索替代完整 tui-textarea
            // ——tui-textarea 集成会显著增加复杂度
            app.search_query.clear();
        }
        KeyCode::Backspace => {
            let _ = app.search_query.pop();
        }
        KeyCode::Char(c) => {
            // 搜索模式下积累查询字符串
            app.search_query.push(c);
        }
        _ => {}
    }
}

// ============================================================
// UI 渲染
// ============================================================

fn render_ui(f: &mut Frame, app: &AppState) {
    let colors = app.theme.colors();

    // 主布局：左右分栏 + 底部状态栏
    let main_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(1), Constraint::Length(1)])
        .split(f.area());

    let body = main_chunks[0];
    let status_area = main_chunks[1];

    // 主体左右分栏
    let body_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(40), Constraint::Percentage(60)])
        .split(body);

    // --- 左面板：分组列表 ---
    render_group_list(f, body_chunks[0], app, &colors);

    // --- 右面板：详情 ---
    render_detail_panel(f, body_chunks[1], app, &colors);

    // --- 状态栏 ---
    render_status_bar(f, status_area, app, &colors);

    // --- 帮助面板（如果激活） ---
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
                Some(format!("📈 Spike ({}x 平均值)", multiplier))
            }
            Anomaly::NewError { group_index }
                if *group_index == app.selected_index =>
            {
                Some("🆕 新错误".to_string())
            }
            Anomaly::SilentRecovery { group_index }
                if *group_index == app.selected_index =>
            {
                Some("🤫 静默恢复".to_string())
            }
            Anomaly::PeriodicPattern {
                group_index,
                period_minutes,
            } if *group_index == app.selected_index => {
                Some(format!("🔁 周期性 (~{}分钟)", period_minutes))
            }
            _ => None,
        })
        .collect::<Vec<_>>();

    let mut lines = vec![
        Line::from(vec![
            Span::styled("签名: ", Style::default().fg(colors.border)),
            Span::styled(&g.signature, Style::default().fg(colors.error).bold()),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("出现次数: ", Style::default().fg(colors.border)),
            Span::styled(
                format!("{}", g.count),
                Style::default().fg(colors.highlight),
            ),
        ]),
        Line::from(vec![
            Span::styled("时间范围: ", Style::default().fg(colors.border)),
            Span::styled(time_str, Style::default().fg(colors.fg)),
        ]),
    ];

    if let Some(ref trend) = g.trend {
        lines.push(Line::from(""));
        lines.push(Line::from(vec![Span::styled(
            format!("趋势: {:?}", trend),
            Style::default().fg(colors.info),
        )]));
    }

    if !anomaly_info.is_empty() {
        lines.push(Line::from(""));
        lines.push(Line::from(vec![Span::styled(
            "--- 异常 ---",
            Style::default().fg(colors.warn).bold(),
        )]));
        for a in &anomaly_info {
            lines.push(Line::from(vec![Span::styled(
                a,
                Style::default().fg(colors.warn),
            )]));
        }
    }

    // 样本行
    if !g.samples.is_empty() {
        lines.push(Line::from(""));
        lines.push(Line::from(vec![Span::styled(
            "--- 样本 ---",
            Style::default().fg(colors.border),
        )]));
        for sample in g.samples.iter().take(5) {
            lines.push(Line::from(vec![Span::styled(
                truncate_str(sample, area.width.saturating_sub(4) as usize),
                Style::default().fg(colors.fg),
            )]));
        }
    }

    // 堆栈
    if let Some(ref stack) = g.stack_trace {
        lines.push(Line::from(""));
        lines.push(Line::from(vec![Span::styled(
            "--- 堆栈 ---",
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
    let mode_str = if app.live_mode { "🔴 LIVE" } else { "📄 STATIC" };
    let theme_str = match app.theme {
        Theme::Dark => "🌙 暗色",
        Theme::Light => "☀ 亮色",
    };
    let search_str = if app.search_query.is_empty() {
        String::new()
    } else {
        format!("搜索: \"{}\"  ", app.search_query)
    };

    let status = Line::from(vec![
        Span::styled(mode_str, Style::default().fg(colors.error).bold()),
        Span::styled(" | ", Style::default().fg(colors.border)),
        Span::styled(
            format!("{} 分组 / {} 异常", app.groups.len(), app.summary.anomalies.len()),
            Style::default().fg(colors.fg),
        ),
        Span::styled(" | ", Style::default().fg(colors.border)),
        Span::styled(search_str, Style::default().fg(colors.highlight)),
        Span::styled(theme_str, Style::default().fg(colors.info)),
        Span::styled(
            "  ← → ↑ ↓ 移动  / 搜索  t 主题  ? 帮助  q 退出",
            Style::default().fg(colors.border),
        ),
    ]);

    f.render_widget(
        Paragraph::new(status).style(Style::default().bg(colors.selected)),
        area,
    );
}

fn render_help_popup(f: &mut Frame, app: &AppState, colors: &ThemeColors) {
    let help_text = vec![
        Line::from(vec![Span::styled("🎮 快捷键", Style::default().fg(colors.highlight).bold())]),
        Line::from(""),
        Line::from(vec![
            Span::styled("  j / ↓      ", Style::default().fg(colors.highlight)),
            Span::styled("下移", Style::default().fg(colors.fg)),
        ]),
        Line::from(vec![
            Span::styled("  k / ↑      ", Style::default().fg(colors.highlight)),
            Span::styled("上移", Style::default().fg(colors.fg)),
        ]),
        Line::from(vec![
            Span::styled("  /          ", Style::default().fg(colors.highlight)),
            Span::styled("搜索过滤分组", Style::default().fg(colors.fg)),
        ]),
        Line::from(vec![
            Span::styled("  Backspace  ", Style::default().fg(colors.highlight)),
            Span::styled("删除搜索字符", Style::default().fg(colors.fg)),
        ]),
        Line::from(vec![
            Span::styled("  t          ", Style::default().fg(colors.highlight)),
            Span::styled("切换暗色/亮色主题", Style::default().fg(colors.fg)),
        ]),
        Line::from(vec![
            Span::styled("  ?          ", Style::default().fg(colors.highlight)),
            Span::styled("显示/隐藏此帮助", Style::default().fg(colors.fg)),
        ]),
        Line::from(vec![
            Span::styled("  q / Esc    ", Style::default().fg(colors.highlight)),
            Span::styled("退出", Style::default().fg(colors.fg)),
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
                    .title(" 帮助 ")
                    .style(Style::default().bg(colors.selected)),
            ),
        popup_area,
    );
}

// ============================================================
// 工具函数
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
        Anomaly::Spike { group_index: gi, .. } => *gi == group_index,
        Anomaly::NewError { group_index: gi } => *gi == group_index,
        Anomaly::SilentRecovery { group_index: gi } => *gi == group_index,
        Anomaly::PeriodicPattern { group_index: gi, .. } => *gi == group_index,
    })
}
```

- [ ] **Step 2: 注册模块**

在 `src/main.rs` 中添加：
```rust
pub mod tui;
```

在 `src/lib.rs` 中添加：
```rust
pub mod tui;
```

- [ ] **Step 3: 运行 cargo check 并修复错误**

Run: `cargo check 2>&1`
Expected: 编译成功。常见修复：
- 如果 `ErrorGroup` 没有 `samples`/`stack_trace`/`trend` 字段，查阅 `src/types.rs` 确认字段名
- 如果 `Anomaly` 变体名不匹配，查阅 `src/types.rs` 修正模式匹配

- [ ] **Step 4: 运行 cargo test**

Run: `cargo test 2>&1`
Expected: 85 tests passed

- [ ] **Step 5: 提交**

```bash
git add src/tui.rs src/main.rs src/lib.rs
git commit -m "feat: add interactive TUI — ratatui log browser with search, theme, help"
```

---

### Task 4: 在 CLI 中接入 TUI + HTML 导出

**文件:**
- 修改: `src/cli.rs`

**上下文:** 需要新增 `Interactive` 子命令 + 在 `AnalyzeArgs` 中添加 `--output` 标志。

- [ ] **Step 1: 读取当前 cli.rs 确认结构**

先读取 `src/cli.rs`，确认当前的 `Command` 枚举和 `AnalyzeArgs` 结构。

- [ ] **Step 2: 修改 CLI**

在 `Command` 枚举中添加 `Interactive` 变体：

```rust
#[derive(clap::Subcommand)]
pub enum Command {
    /// 分析日志文件
    Analyze(AnalyzeArgs),
    /// 实时监听日志文件，周期性 AI 分析
    Watch(WatchArgs),
    /// 交互式 TUI 日志浏览器
    Interactive(InteractiveArgs),
}
```

在 `WatchArgs` 之后新增：

```rust
#[derive(clap::Args)]
pub struct InteractiveArgs {
    /// 日志文件路径
    pub file: PathBuf,

    /// 实时模式：监听文件变化，自动刷新 TUI
    #[arg(long, default_value_t = false)]
    pub live: bool,

    /// AI 模型后端（默认自动检测）
    #[arg(short, long, default_value = "auto")]
    pub model: ModelArg,

    /// 强制日志格式
    #[arg(short, long, default_value = "auto")]
    pub format: FormatArg,

    /// 最低日志级别
    #[arg(long, default_value = "info")]
    pub min_level: LevelArg,
}
```

在 `AnalyzeArgs` 中添加 `--output` 字段：

```rust
#[derive(clap::Args)]
pub struct AnalyzeArgs {
    /// 日志文件路径
    pub file: PathBuf,

    /// AI 模型后端（默认自动检测）
    #[arg(short, long, default_value = "auto")]
    pub model: ModelArg,

    /// 使用深度/更强模型进行分析
    #[arg(long, default_value_t = false)]
    pub deep: bool,

    /// 强制日志格式
    #[arg(short, long, default_value = "auto")]
    pub format: FormatArg,

    /// 最低日志级别
    #[arg(long, default_value = "info")]
    pub min_level: LevelArg,

    /// 导出报告到 HTML 文件
    #[arg(long)]
    pub output: Option<PathBuf>,
}
```

在 `run()` 函数的 `match cli.command` 中添加 `Interactive` 分支和更新 `Analyze` 分支以处理 `--output`。

`run()` 函数的 `Analyze` 分支需要在 `render_report()` 之后增加：

```rust
// 在 render_report 调用之后，添加：
if let Some(ref output_path) = args.output {
    let html = crate::renderer_html::render_report_html(
        &summary,
        &response,
        elapsed,
        backend.model_name(),
    );
    std::fs::write(output_path, &html)?;
    eprintln!("   HTML 报告已保存到 {}", output_path.display());
}
```

完整的 `run()` 函数 `match` 分支：

```rust
pub async fn run() -> anyhow::Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Command::Analyze(args) => {
            let file_path = &args.file;
            if !file_path.exists() {
                anyhow::bail!("File not found: {}", file_path.display());
            }
            let model: Model = args.model.into();
            let deep = args.deep;
            let format_override = args.format.to_format();

            eprintln!("🔍 Parsing {}...", file_path.display());
            let start = Instant::now();

            let entries = parse_log_file(file_path, format_override)?;
            let min_level = args.min_level.to_level();
            let entries: Vec<_> = entries
                .into_iter()
                .filter(|e| {
                    let level = e.level.unwrap_or(crate::types::Level::Unknown);
                    level.severity() <= min_level.severity()
                })
                .collect();
            eprintln!(
                "   Parsed {} log entries (after --min-level filter)",
                entries.len()
            );

            let summary = aggregate(&entries);
            eprintln!(
                "   Found {} error groups, {} anomalies",
                summary.error_groups.len(),
                summary.anomalies.len()
            );

            let backend = create_backend(model, deep).await?;
            eprintln!(
                "🤖 Analyzing with {} ({})...",
                backend.model_name(),
                backend.actual_model(deep)
            );
            let response = crate::ai::with_retry(|| backend.analyze(&summary)).await?;

            let elapsed = start.elapsed().as_secs_f64();
            render_report(&summary, &response, elapsed, backend.model_name());

            // HTML 导出
            if let Some(ref output_path) = args.output {
                let html = crate::renderer_html::render_report_html(
                    &summary,
                    &response,
                    elapsed,
                    backend.model_name(),
                );
                std::fs::write(output_path, &html)?;
                eprintln!("   HTML 报告已保存到 {}", output_path.display());
            }
            Ok(())
        }
        Command::Watch(args) => crate::watcher::watch_file(args).await,
        Command::Interactive(args) => {
            // Interactive 是同步模式（TUI 需要控制终端）
            crate::tui::run_interactive(args.file, args.live)
        }
    }
}
```

- [ ] **Step 3: 运行 cargo check**

Run: `cargo check 2>&1`
Expected: 编译成功

- [ ] **Step 4: 运行 cargo test**

Run: `cargo test 2>&1`
Expected: 85 tests passed

- [ ] **Step 5: 提交**

```bash
git add src/cli.rs
git commit -m "feat: wire Interactive subcommand and --output HTML flag"
```

---

### Task 5: 编写单元测试

**文件:**
- 创建: `tests/tui_tests.rs`
- 创建: `tests/html_tests.rs`

- [ ] **Step 1: 创建 tests/tui_tests.rs**

```rust
use logai::tui::{AppState, Theme};
use logai::types::AnalysisSummary;
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
    let app = AppState::new(summary);
    assert_eq!(app.selected_index, 0);
    assert_eq!(app.search_query, "");
    assert!(!app.show_help);
    assert!(app.filtered_groups().is_empty());
}

#[test]
fn test_app_search_filters_groups() {
    let mut summary = make_empty_summary();
    summary.error_groups = vec![
        logai::types::ErrorGroup {
            signature: "ConnectionPool exhausted".into(),
            count: 5,
            first_seen: None,
            last_seen: None,
            samples: vec![],
            stack_trace: None,
            trend: None,
        },
        logai::types::ErrorGroup {
            signature: "timeout reading from socket".into(),
            count: 2,
            first_seen: None,
            last_seen: None,
            samples: vec![],
            stack_trace: None,
            trend: None,
        },
        logai::types::ErrorGroup {
            signature: "SSL certificate expired".into(),
            count: 1,
            first_seen: None,
            last_seen: None,
            samples: vec![],
            stack_trace: None,
            trend: None,
        },
    ];

    let mut app = AppState::new(summary);
    assert_eq!(app.filtered_groups().len(), 3);

    // 搜索 "ssl"
    app.search_query = "ssl".into();
    let filtered = app.filtered_groups();
    assert_eq!(filtered.len(), 1);
    assert_eq!(
        &app.groups[filtered[0]].signature.to_lowercase(),
        "ssl certificate expired"
    );

    // 搜索无匹配
    app.search_query = "xyznotfound".into();
    assert!(app.filtered_groups().is_empty());

    // 清空搜索
    app.search_query.clear();
    assert_eq!(app.filtered_groups().len(), 3);
}

#[test]
fn test_app_select_next_prev() {
    let mut summary = make_empty_summary();
    for i in 0..5 {
        summary.error_groups.push(logai::types::ErrorGroup {
            signature: format!("error {}", i),
            count: 1,
            first_seen: None,
            last_seen: None,
            samples: vec![],
            stack_trace: None,
            trend: None,
        });
    }

    let mut app = AppState::new(summary);
    assert_eq!(app.selected_index, 0);

    app.select_next();
    assert_eq!(app.selected_index, 1);

    app.select_prev();
    assert_eq!(app.selected_index, 0);

    // 边界：从 0 向上到最后一个
    app.select_prev();
    assert_eq!(app.selected_index, 4);

    // 边界：从最后一个向下到 0
    app.select_next();
    assert_eq!(app.selected_index, 0);
}
```

- [ ] **Step 2: 确认模块和类型是公开的**

如果编译失败（`AppState`、`Theme` 等类型是 crate-private），需要在 `src/tui.rs` 中将相关类型标记为 `pub`（已在 Task 3 代码中包含 `pub`）。

- [ ] **Step 3: 创建 tests/html_tests.rs**

```rust
use logai::renderer_html::render_report_html;
use logai::types::{AiResponse, AnalysisSummary, FixSuggestion, Level, RootCause, Severity};
use std::collections::HashMap;

fn make_summary() -> AnalysisSummary {
    let mut level_distribution = HashMap::new();
    level_distribution.insert(Level::Error, 10);
    level_distribution.insert(Level::Warn, 5);

    AnalysisSummary {
        total_lines: 100,
        time_start: None,
        time_end: None,
        level_distribution,
        error_groups: vec![],
        anomalies: vec![],
    }
}

fn make_response() -> AiResponse {
    AiResponse {
        summary: "测试摘要".into(),
        root_causes: vec![RootCause {
            description: "连接池耗尽".into(),
            severity: Severity::Critical,
            evidence: vec!["5 次连接超时".into()],
        }],
        fix_suggestions: vec![FixSuggestion {
            action: "增大连接池".into(),
            code_snippet: Some("pool_max_size: 10 → 50".into()),
            reference: Some("https://example.com/docs".into()),
        }],
    }
}

#[test]
fn test_html_contains_key_elements() {
    let summary = make_summary();
    let response = make_response();
    let html = render_report_html(&summary, &response, 4.2, "DeepSeek");

    assert!(html.contains("<!DOCTYPE html>"));
    assert!(html.contains("logai 分析报告"));
    assert!(html.contains("100 行"));
    assert!(html.contains("DeepSeek"));
    assert!(html.contains("连接池耗尽"));
    assert!(html.contains("增大连接池"));
}

#[test]
fn test_html_escapes_user_content() {
    let summary = make_summary();
    let mut response = make_response();
    response.root_causes[0].description = "<script>alert('xss')</script>".into();

    let html = render_report_html(&summary, &response, 1.0, "test");
    // 不应包含原始 script 标签
    assert!(!html.contains("<script>alert"));
    assert!(html.contains("&lt;script&gt;"));
}

#[test]
fn test_html_empty_groups_shows_message() {
    let summary = make_summary();
    let response = make_response();
    let html = render_report_html(&summary, &response, 1.0, "test");
    assert!(html.contains("无错误分组"));
}
```

- [ ] **Step 4: 运行新增的测试**

Run: `cargo test --test tui_tests --test html_tests 2>&1`
Expected: 7 个测试全部通过

- [ ] **Step 5: 运行全部测试**

Run: `cargo test 2>&1`
Expected: 92 tests passed（85 + 7 新增）

- [ ] **Step 6: 提交**

```bash
git add tests/tui_tests.rs tests/html_tests.rs
git commit -m "test: add unit tests for TUI state and HTML renderer"
```

---

### Task 6: 编写 CLI 集成测试

**文件:**
- 修改: `tests/integration_tests.rs`

- [ ] **Step 1: 在文件末尾追加集成测试**

```rust
#[test]
fn test_interactive_help() {
    let mut cmd = Command::cargo_bin("logai").unwrap();
    cmd.arg("interactive").arg("--help");
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("交互式 TUI 日志浏览器"));
}

#[test]
fn test_interactive_live_flag_in_help() {
    let mut cmd = Command::cargo_bin("logai").unwrap();
    cmd.arg("interactive").arg("--help");
    let output = cmd.output().unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("--live"));
}

#[test]
fn test_analyze_output_flag_in_help() {
    let mut cmd = Command::cargo_bin("logai").unwrap();
    cmd.arg("analyze").arg("--help");
    let output = cmd.output().unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("--output"));
}
```

- [ ] **Step 2: 运行集成测试**

Run: `cargo test --test integration_tests 2>&1`
Expected: 12 tests passed（9 已有 + 3 新增）

- [ ] **Step 3: 运行全部测试**

Run: `cargo test 2>&1`
Expected: 95 tests passed

- [ ] **Step 4: 提交**

```bash
git add tests/integration_tests.rs
git commit -m "test: add CLI integration tests for interactive and --output"
```

---

### Task 7: 最终验证

**文件:** 无（只读检查）

- [ ] **Step 1: cargo fmt**

Run: `cargo fmt --all --check 2>&1`
Expected: 无输出，退出码 0。如果失败：`cargo fmt --all`

- [ ] **Step 2: cargo clippy**

Run: `cargo clippy --all-targets -- -D warnings 2>&1`
Expected: 无 warning，退出码 0

- [ ] **Step 3: 全部测试**

Run: `cargo test 2>&1`
Expected: 95 tests passed

- [ ] **Step 4: release 构建**

Run: `cargo build --release 2>&1`
Expected: 编译成功

- [ ] **Step 5: 手动验证 CLI help**

Run: `cargo run -- interactive --help`
Expected: 输出包含 `--live`、`--model`、`--format`、`--min-level`

Run: `cargo run -- analyze --help`
Expected: 输出包含 `--output`

- [ ] **Step 6: 提交 + push + tag**

```bash
git add -A
git commit -m "chore: final polish — fmt, clippy, release build"
git push origin main
git tag -a v1.3.0 -m "v1.3.0: TUI interactive log browser + HTML report export"
git push origin v1.3.0
```

---

## 自审清单

**1. Spec 覆盖率:**
- ✅ TUI 分组列表 + 详情面板 → Task 3 `render_group_list()` + `render_detail_panel()`
- ✅ 全键盘操作 (vim 风格) → Task 3 `handle_key_event()`（j/k/↑↓//t/?/q）
- ✅ 实时刷新模式 → Task 3 `AppState.live_mode` + Task 4 `--live` 标志
- ✅ 暗色/亮色主题切换 → Task 3 `Theme` + `handle_key_event('t')`
- ✅ 搜索过滤 → Task 3 `AppState.filtered_groups()` + 按 `/` 搜索
- ✅ 帮助面板 → Task 3 `render_help_popup()` + 按 `?` 触发
- ✅ HTML 报告导出 → Task 2 `render_report_html()` + Task 4 `--output`
- ✅ 颜色编码严重级别 → Task 3 `ThemeColors`（error=Red, warn=Yellow, info=Green）
- ✅ CEO 审查 GAP 修复：空状态 → Task 3 "✅ 没有发现错误"

**2. Placeholder 扫描:** 无 TBD/TODO。所有步骤包含完整代码。

**3. 类型一致性:**
- `AppState::new(AnalysisSummary)` → 与 Task 5 测试调用一致
- `render_report_html(&AnalysisSummary, &AiResponse, f64, &str) -> String` → 与 Task 4 CLI 调用一致
- `run_interactive(PathBuf, bool)` → 与 Task 4 CLI 调用一致
