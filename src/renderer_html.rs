use crate::types::{
    AiResponse, AnalysisSummary, ErrorGroup, FixSuggestion, Level, RootCause, Severity,
};
use std::collections::BTreeMap;

/// Generate a self-contained HTML analysis report with Chart.js interactive charts
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
    let error_count = summary.level_distribution.get(&Level::Error).unwrap_or(&0);
    let warn_count = summary.level_distribution.get(&Level::Warn).unwrap_or(&0);
    let error_rate = *error_count as f64 / total * 100.0;

    let root_causes_html = render_root_causes_html(&response.root_causes);
    let fix_suggestions_html = render_fix_suggestions_html(&response.fix_suggestions);
    let groups_html = render_error_groups_html(&summary.error_groups);

    // Build Chart.js data
    let chart_data = build_chart_data(summary);

    format!(
        r#"<!DOCTYPE html>
<html lang="zh-CN">
<head>
<meta charset="UTF-8">
<meta name="viewport" content="width=device-width, initial-scale=1.0">
<title>{title}</title>
<link rel="preconnect" href="https://fonts.googleapis.com">
<link rel="preconnect" href="https://fonts.gstatic.com" crossorigin>
<link href="https://fonts.googleapis.com/css2?family=Inter:opsz,wght@14..32,400..700&family=Noto+Sans+SC:wght@400;700&family=JetBrains+Mono:wght@400;700&display=swap" rel="stylesheet">
<script src="https://cdn.jsdelivr.net/npm/chart.js@4"></script>
<style>
  /* ======== Color tokens (aligned with TUI ThemeColors) ======== */
  :root {{
    --color-bg: #1a1a2e;
    --color-fg: #e0e0e0;
    --color-highlight: #00bcd4;
    --color-error: #e94560;
    --color-warn: #f0a500;
    --color-info: #16c79a;
    --color-selected: #16213e;
    --color-border: #0f3460;
    --color-code-bg: #0d1b3e;
    --color-muted: #888;
    --shadow: 0 2px 8px rgba(0,0,0,0.3);
  }}
  [data-theme="light"] {{
    --color-bg: #fafafa;
    --color-fg: #1a1a2e;
    --color-highlight: #1565c0;
    --color-error: #c62828;
    --color-warn: #e65100;
    --color-info: #2e7d32;
    --color-selected: #e8eaf6;
    --color-border: #c5cae9;
    --color-code-bg: #eceff1;
    --color-muted: #666;
    --shadow: 0 2px 8px rgba(0,0,0,0.08);
  }}

  /* ======== Base ======== */
  *, *::before, *::after {{ box-sizing: border-box; }}
  body {{
    font-family: 'Inter', 'Noto Sans SC', -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif;
    max-width: 960px; margin: 0 auto; padding: 24px;
    background: var(--color-bg); color: var(--color-fg);
    transition: background 0.3s, color 0.3s;
  }}
  h1 {{ color: var(--color-error); border-bottom: 2px solid var(--color-border); padding-bottom: 12px; }}
  h2 {{ color: var(--color-info); margin-top: 32px; }}
  .meta {{ color: var(--color-muted); font-size: 14px; margin-bottom: 24px; }}

  /* ======== Theme toggle ======== */
  .theme-toggle {{
    position: fixed; top: 16px; right: 24px; z-index: 100;
    background: var(--color-selected); color: var(--color-fg);
    border: 1px solid var(--color-border); border-radius: 20px;
    padding: 6px 14px; cursor: pointer; font-size: 13px;
    font-family: inherit; transition: all 0.2s;
    box-shadow: var(--shadow);
  }}
  .theme-toggle:hover {{ filter: brightness(1.1); }}

  /* ======== Overview stats ======== */
  .overview {{ display: flex; gap: 24px; margin-bottom: 32px; flex-wrap: wrap; }}
  .stat {{
    background: var(--color-selected); border-radius: 8px;
    padding: 16px 24px; text-align: center; box-shadow: var(--shadow);
  }}
  .stat .value {{ font-size: 28px; font-weight: bold; color: var(--color-error); }}
  .stat .label {{ font-size: 12px; color: var(--color-muted); margin-top: 4px; }}

  /* ======== Charts ======== */
  .charts {{ display: grid; grid-template-columns: 1fr 1fr; gap: 24px; margin-bottom: 32px; }}
  .charts .wide {{ grid-column: 1 / -1; }}
  .chart-box {{
    background: var(--color-selected); border-radius: 8px;
    padding: 16px; box-shadow: var(--shadow);
  }}
  .chart-box h3 {{ color: var(--color-muted); font-size: 13px; margin: 0 0 12px 0; }}
  .chart-box canvas {{ max-height: 280px; }}

  /* ======== Root cause cards ======== */
  .root-cause {{
    background: var(--color-selected); border-left: 4px solid var(--color-error);
    padding: 16px; margin-bottom: 16px; border-radius: 0 8px 8px 0;
    box-shadow: var(--shadow);
  }}
  .root-cause h3 {{ margin-top: 0; color: var(--color-error); }}
  .evidence {{ color: var(--color-muted); font-size: 13px; margin: 8px 0; }}
  .evidence li {{ margin: 4px 0; }}
  .severity {{ font-size: 12px; padding: 2px 8px; border-radius: 4px; }}
  .severity.critical {{ background: var(--color-error); color: white; }}
  .severity.high {{ background: color-mix(in srgb, var(--color-error) 70%, transparent); color: white; }}
  .severity.medium {{ background: var(--color-warn); color: #1a1a2e; }}
  .severity.low {{ background: var(--color-info); color: #1a1a2e; }}

  /* ======== Fix suggestions ======== */
  .fix {{
    background: var(--color-selected); border-left: 4px solid var(--color-info);
    padding: 16px; margin-bottom: 12px; border-radius: 0 8px 8px 0;
    box-shadow: var(--shadow);
  }}
  .fix code {{
    background: var(--color-code-bg); padding: 8px 12px; display: block;
    margin-top: 8px; border-radius: 4px;
    font-family: 'JetBrains Mono', 'Cascadia Code', 'Fira Code', 'Consolas', monospace;
    font-size: 13px;
  }}

  /* ======== Error groups ======== */
  .group {{
    background: var(--color-selected); padding: 12px 16px;
    margin-bottom: 8px; border-radius: 8px;
    display: flex; justify-content: space-between; align-items: center;
    flex-wrap: wrap; gap: 8px; box-shadow: var(--shadow);
  }}
  .group .sig {{ font-family: 'JetBrains Mono', 'Consolas', monospace; font-size: 13px; }}
  .group .count {{ color: var(--color-error); font-weight: bold; }}

  /* ======== Footer ======== */
  .footer {{
    color: var(--color-muted); font-size: 12px; margin-top: 48px;
    border-top: 1px solid var(--color-border); padding-top: 16px; text-align: center;
  }}

  /* ======== Responsive ======== */
  @media (max-width: 900px) {{
    .charts {{ grid-template-columns: 1fr; }}
    .overview {{ gap: 16px; }}
  }}
  @media (max-width: 700px) {{
    body {{ padding: 12px; }}
    .overview {{ gap: 12px; }}
    .stat {{ padding: 12px 16px; flex: 1 1 40%; }}
    .stat .value {{ font-size: 22px; }}
    .charts canvas {{ max-height: 220px; }}
    .group {{ flex-direction: column; align-items: flex-start; }}
    .theme-toggle {{ top: 8px; right: 12px; padding: 4px 10px; font-size: 12px; }}
  }}
</style>
</head>
<body data-theme="dark">
<button class="theme-toggle" onclick="toggleTheme()" title="切换亮色/暗色主题">🌓 主题</button>

<h1>📊 {title}</h1>
<div class="meta">{total_lines} 行 · {time_info} · 耗时 {elapsed:.1}s · AI: {model_name}</div>

<h2>📋 概览</h2>
<div class="overview">
  <div class="stat"><div class="value">{error_count}</div><div class="label">ERROR</div></div>
  <div class="stat"><div class="value">{warn_count}</div><div class="label">WARN</div></div>
  <div class="stat"><div class="value">{error_rate:.1}%</div><div class="label">错误率</div></div>
  <div class="stat"><div class="value">{group_count}</div><div class="label">错误分组</div></div>
</div>

<h2>📈 交互图表</h2>
<div class="charts">
  <div class="chart-box wide">
    <h3>错误趋势（时间线）</h3>
    <canvas id="timelineChart"></canvas>
  </div>
  <div class="chart-box">
    <h3>日志级别分布</h3>
    <canvas id="levelPieChart"></canvas>
  </div>
  <div class="chart-box">
    <h3>TOP 错误分组</h3>
    <canvas id="groupsBarChart"></canvas>
  </div>
</div>

<h2>🔴 根因分析</h2>
{root_causes_html}

<h2>🛠️ 修复建议</h2>
{fix_suggestions_html}

<h2>📦 错误分组</h2>
{groups_html}

<div class="footer">由 logai 生成 · 日志数据未上传 · 仅 AI 看到聚合统计</div>

<script>
// Theme toggle
function toggleTheme() {{
  const el = document.documentElement;
  const current = el.getAttribute('data-theme');
  const next = current === 'light' ? 'dark' : 'light';
  el.setAttribute('data-theme', next);
  // Re-render charts with new theme colors
  updateChartColors(next);
}}

function getChartColors(theme) {{
  const style = getComputedStyle(document.documentElement);
  return {{
    grid: theme === 'light' ? '#ddd' : '#2a2a4e',
    text: style.getPropertyValue('--color-muted').trim() || '#888',
    error: style.getPropertyValue('--color-error').trim() || '#e94560',
    info: style.getPropertyValue('--color-info').trim() || '#16c79a',
    warn: style.getPropertyValue('--color-warn').trim() || '#f0a500',
  }};
}}

let charts = {{}};
function updateChartColors(theme) {{
  const c = getChartColors(theme);
  Object.values(charts).forEach(ch => {{
    if (ch.options && ch.options.scales) {{
      if (ch.options.scales.x) {{ ch.options.scales.x.grid = {{ color: c.grid }}; ch.options.scales.x.ticks = {{ color: c.text }}; }}
      if (ch.options.scales.y) {{ ch.options.scales.y.grid = {{ color: c.grid }}; ch.options.scales.y.ticks = {{ color: c.text }}; }}
    }}
    if (ch.options && ch.options.plugins && ch.options.plugins.legend) {{
      ch.options.plugins.legend.labels = {{ color: c.text }};
    }}
    ch.update();
  }});
}}

{chart_js}
</script>
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
        group_count = summary.error_groups.len(),
        root_causes_html = root_causes_html,
        fix_suggestions_html = fix_suggestions_html,
        groups_html = groups_html,
        chart_js = chart_data,
    )
}

/// Build Chart.js JavaScript for interactive charts
fn build_chart_data(summary: &AnalysisSummary) -> String {
    // --- Level distribution data ---
    let level_order = [
        Level::Error,
        Level::Warn,
        Level::Info,
        Level::Debug,
        Level::Trace,
    ];
    let level_colors = ["#e94560", "#f0a500", "#16c79a", "#4a9eff", "#666"];
    let level_labels: Vec<&str> = level_order
        .iter()
        .filter(|l| summary.level_distribution.get(l).unwrap_or(&0) > &0)
        .map(|l| match l {
            Level::Error => "ERROR",
            Level::Warn => "WARN",
            Level::Info => "INFO",
            Level::Debug => "DEBUG",
            Level::Trace => "TRACE",
            Level::Unknown => "UNKNOWN",
        })
        .collect();
    let level_counts: Vec<usize> = level_order
        .iter()
        .filter(|l| summary.level_distribution.get(l).unwrap_or(&0) > &0)
        .map(|l| *summary.level_distribution.get(l).unwrap_or(&0))
        .collect();

    // --- Top error groups bar data ---
    let top_n = 10usize.min(summary.error_groups.len());
    let bar_labels: Vec<String> = summary.error_groups[..top_n]
        .iter()
        .map(|g| {
            let s = if g.signature.len() > 40 {
                format!("{}...", &g.signature[..37])
            } else {
                g.signature.clone()
            };
            escape_js_str(&s)
        })
        .collect();
    let bar_counts: Vec<usize> = summary.error_groups[..top_n]
        .iter()
        .map(|g| g.count)
        .collect();

    // --- Timeline: bucket error group first_seen times into windows ---
    let (timeline_labels, timeline_counts) = build_timeline_data(summary);

    format!(
        r#"// Level distribution pie chart
const levelCtx = document.getElementById('levelPieChart').getContext('2d');
charts.level = new Chart(levelCtx, {{
  type: 'doughnut',
  data: {{
    labels: [{level_labels}],
    datasets: [{{
      data: [{level_counts}],
      backgroundColor: [{level_bg}],
      borderColor: '#1a1a2e',
      borderWidth: 2
    }}]
  }},
  options: {{
    responsive: true,
    maintainAspectRatio: true,
    plugins: {{ legend: {{ labels: {{ color: '#aaa' }} }} }}
  }}
}});

// Top error groups bar chart
const barCtx = document.getElementById('groupsBarChart').getContext('2d');
charts.bar = new Chart(barCtx, {{
  type: 'bar',
  data: {{
    labels: [{bar_labels}],
    datasets: [{{
      label: '出现次数',
      data: [{bar_counts}],
      backgroundColor: '#e94560',
      borderRadius: 4
    }}]
  }},
  options: {{
    indexAxis: 'y',
    responsive: true,
    maintainAspectRatio: true,
    plugins: {{ legend: {{ display: false }} }},
    scales: {{
      x: {{ ticks: {{ color: '#aaa' }}, grid: {{ color: '#2a2a4e' }} }},
      y: {{ ticks: {{ color: '#aaa', font: {{ size: 10 }} }}, grid: {{ display: false }} }}
    }}
  }}
}});

// Timeline chart
const timeCtx = document.getElementById('timelineChart').getContext('2d');
charts.timeline = new Chart(timeCtx, {{
  type: 'line',
  data: {{
    labels: [{timeline_labels}],
    datasets: [{{
      label: '错误数 / 窗口',
      data: [{timeline_counts}],
      borderColor: '#e94560',
      backgroundColor: 'rgba(233, 69, 96, 0.15)',
      fill: true,
      tension: 0.3,
      pointRadius: 3,
      pointBackgroundColor: '#e94560'
    }}]
  }},
  options: {{
    responsive: true,
    maintainAspectRatio: true,
    plugins: {{ legend: {{ labels: {{ color: '#aaa' }} }} }},
    scales: {{
      x: {{ ticks: {{ color: '#aaa', maxTicksLimit: 12 }}, grid: {{ color: '#2a2a4e' }} }},
      y: {{ ticks: {{ color: '#aaa' }}, grid: {{ color: '#2a2a4e' }}, beginAtZero: true }}
    }}
  }}
}});"#,
        level_labels = level_labels
            .iter()
            .map(|l| format!("'{}'", l))
            .collect::<Vec<_>>()
            .join(","),
        level_counts = level_counts
            .iter()
            .map(|c| c.to_string())
            .collect::<Vec<_>>()
            .join(","),
        level_bg = level_colors[..level_labels.len()]
            .iter()
            .map(|c| format!("'{}'", c))
            .collect::<Vec<_>>()
            .join(","),
        bar_labels = bar_labels
            .iter()
            .map(|l| format!("'{}'", l))
            .collect::<Vec<_>>()
            .join(","),
        bar_counts = bar_counts
            .iter()
            .map(|c| c.to_string())
            .collect::<Vec<_>>()
            .join(","),
        timeline_labels = timeline_labels
            .iter()
            .map(|l| format!("'{}'", l))
            .collect::<Vec<_>>()
            .join(","),
        timeline_counts = timeline_counts
            .iter()
            .map(|c| c.to_string())
            .collect::<Vec<_>>()
            .join(","),
    )
}

/// Build timeline data by bucketing error group first_seen timestamps
fn build_timeline_data(summary: &AnalysisSummary) -> (Vec<String>, Vec<usize>) {
    let mut bucket_map: BTreeMap<i64, usize> = BTreeMap::new();
    let window_secs: i64 = 300; // 5-minute windows

    for group in &summary.error_groups {
        if let Some(ts) = group.first_seen {
            let bucket = ts.timestamp() / window_secs;
            *bucket_map.entry(bucket).or_insert(0) += group.count;
        }
    }

    // If no timestamps, return empty
    if bucket_map.is_empty() {
        return (vec![], vec![]);
    }

    let labels: Vec<String> = bucket_map
        .keys()
        .map(|&bucket| {
            let ts = bucket * window_secs;
            let dt = chrono::DateTime::<chrono::Utc>::from_timestamp(ts, 0)
                .unwrap_or(chrono::DateTime::UNIX_EPOCH);
            dt.format("%H:%M").to_string()
        })
        .collect();
    let counts: Vec<usize> = bucket_map.values().copied().collect();

    (labels, counts)
}

/// Escape a string for safe inclusion in JavaScript single-quoted string
fn escape_js_str(s: &str) -> String {
    s.replace('\\', "\\\\")
        .replace('\'', "\\'")
        .replace('\n', " ")
        .replace('\r', "")
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
                .map(|r| {
                    format!(
                        "<div style=\"margin-top:4px;font-size:12px;color:#aaa;\">参考: {}</div>",
                        escape_html(r)
                    )
                })
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

fn render_error_groups_html(groups: &[ErrorGroup]) -> String {
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

/// Generate a multi-source HTML analysis report with per-source sections and correlation panel
pub fn render_report_html_multi(
    multi: &crate::types::MultiSourceSummary,
    response: Option<&AiResponse>,
    elapsed_secs: f64,
    model_name: &str,
) -> String {
    let title = "logai 多源分析报告";
    let total_lines = multi.total_lines();
    let total_errors = multi.total_errors();

    let sources_html: String = multi
        .sources
        .iter()
        .map(|source| {
            let error_count = source
                .summary
                .level_distribution
                .get(&Level::Error)
                .unwrap_or(&0);
            let warn_count = source
                .summary
                .level_distribution
                .get(&Level::Warn)
                .unwrap_or(&0);
            let groups_html = render_error_groups_html(&source.summary.error_groups);
            format!(
                r#"<h2>📁 {name}</h2>
<div class="meta">{lines} 行 · ERROR: {errors} · WARN: {warns} · {groups} 分组 · {anomalies} 异常</div>
<div class="groups-container">{groups_html}</div>"#,
                name = escape_html(&source.name),
                lines = source.summary.total_lines,
                errors = error_count,
                warns = warn_count,
                groups = source.summary.error_groups.len(),
                anomalies = source.summary.anomalies.len(),
                groups_html = groups_html,
            )
        })
        .collect::<Vec<_>>()
        .join("\n");

    // Correlation panel
    let correlations_html = if multi.correlations.is_empty() {
        "<p>未检测到显著的跨源关联</p>".to_string()
    } else {
        multi
            .correlations
            .iter()
            .map(|c| {
                let score_pct = (c.score * 100.0) as u32;
                let score_color = if c.score > 0.7 {
                    "#e94560"
                } else if c.score > 0.4 {
                    "#f0a500"
                } else {
                    "#16c79a"
                };
                format!(
                    r#"<div class="correlation">
<span style="color:{score_color};font-weight:bold;">{score}%</span>
<span>{a} ↔ {b}</span>
<p style="color:var(--color-muted);font-size:13px;">{desc}</p>
</div>"#,
                    score = score_pct,
                    score_color = score_color,
                    a = escape_html(&c.source_a),
                    b = escape_html(&c.source_b),
                    desc = escape_html(&c.description),
                )
            })
            .collect::<Vec<_>>()
            .join("\n")
    };

    let root_causes_html = if let Some(resp) = response {
        render_root_causes_html(&resp.root_causes)
    } else {
        "<p>—</p>".to_string()
    };
    let fix_suggestions_html = if let Some(resp) = response {
        render_fix_suggestions_html(&resp.fix_suggestions)
    } else {
        "<p>—</p>".to_string()
    };

    // Build Chart.js data from first source only
    let chart_data = if let Some(first) = multi.sources.first() {
        build_chart_data(&first.summary)
    } else {
        String::new()
    };

    format!(
        r#"<!DOCTYPE html>
<html lang="zh-CN">
<head>
<meta charset="UTF-8">
<meta name="viewport" content="width=device-width, initial-scale=1.0">
<title>{title}</title>
<link rel="preconnect" href="https://fonts.googleapis.com">
<link rel="preconnect" href="https://fonts.gstatic.com" crossorigin>
<link href="https://fonts.googleapis.com/css2?family=Inter:opsz,wght@14..32,400..700&family=Noto+Sans+SC:wght@400;700&family=JetBrains+Mono:wght@400;700&display=swap" rel="stylesheet">
<script src="https://cdn.jsdelivr.net/npm/chart.js@4"></script>
<style>
  :root {{
    --color-bg: #1a1a2e; --color-fg: #e0e0e0; --color-highlight: #00bcd4;
    --color-error: #e94560; --color-warn: #f0a500; --color-info: #16c79a;
    --color-selected: #16213e; --color-border: #0f3460; --color-code-bg: #0d1b3e;
    --color-muted: #888; --shadow: 0 2px 8px rgba(0,0,0,0.3);
  }}
  [data-theme="light"] {{
    --color-bg: #fafafa; --color-fg: #1a1a2e; --color-highlight: #1565c0;
    --color-error: #c62828; --color-warn: #e65100; --color-info: #2e7d32;
    --color-selected: #e8eaf6; --color-border: #c5cae9; --color-code-bg: #eceff1;
    --color-muted: #666; --shadow: 0 2px 8px rgba(0,0,0,0.08);
  }}
  *, *::before, *::after {{ box-sizing: border-box; }}
  body {{
    font-family: 'Inter', 'Noto Sans SC', sans-serif; max-width: 960px;
    margin: 0 auto; padding: 24px; background: var(--color-bg); color: var(--color-fg);
    transition: background 0.3s, color 0.3s;
  }}
  h1 {{ color: var(--color-error); border-bottom: 2px solid var(--color-border); padding-bottom: 12px; }}
  h2 {{ color: var(--color-info); margin-top: 32px; }}
  .meta {{ color: var(--color-muted); font-size: 14px; margin-bottom: 24px; }}
  .theme-toggle {{
    position: fixed; top: 16px; right: 24px; z-index: 100;
    background: var(--color-selected); color: var(--color-fg);
    border: 1px solid var(--color-border); border-radius: 20px;
    padding: 6px 14px; cursor: pointer; font-size: 13px; font-family: inherit;
  }}
  .theme-toggle:hover {{ filter: brightness(1.1); }}
  .overview {{ display: flex; gap: 24px; margin-bottom: 32px; flex-wrap: wrap; }}
  .stat {{ background: var(--color-selected); border-radius: 8px; padding: 16px 24px; text-align: center; }}
  .stat .value {{ font-size: 28px; font-weight: bold; color: var(--color-error); }}
  .stat .label {{ font-size: 12px; color: var(--color-muted); margin-top: 4px; }}
  .charts {{ display: grid; grid-template-columns: 1fr 1fr; gap: 24px; margin-bottom: 32px; }}
  .charts .wide {{ grid-column: 1 / -1; }}
  .chart-box {{ background: var(--color-selected); border-radius: 8px; padding: 16px; }}
  .chart-box h3 {{ color: var(--color-muted); font-size: 13px; margin: 0 0 12px 0; }}
  .chart-box canvas {{ max-height: 280px; }}
  .root-cause {{ background: var(--color-selected); border-left: 4px solid var(--color-error); padding: 16px; margin-bottom: 16px; border-radius: 0 8px 8px 0; }}
  .fix {{ background: var(--color-selected); border-left: 4px solid var(--color-info); padding: 16px; margin-bottom: 12px; border-radius: 0 8px 8px 0; }}
  .fix code {{ background: var(--color-code-bg); padding: 8px 12px; display: block; margin-top: 8px; border-radius: 4px; font-family: 'JetBrains Mono', monospace; font-size: 13px; }}
  .group {{ background: var(--color-selected); padding: 12px 16px; margin-bottom: 8px; border-radius: 8px; display: flex; justify-content: space-between; align-items: center; flex-wrap: wrap; gap: 8px; }}
  .group .sig {{ font-family: 'JetBrains Mono', monospace; font-size: 13px; }}
  .group .count {{ color: var(--color-error); font-weight: bold; }}
  .correlation {{ background: var(--color-selected); padding: 12px 16px; margin-bottom: 8px; border-radius: 8px; border-left: 4px solid var(--color-warn); }}
  .footer {{ color: var(--color-muted); font-size: 12px; margin-top: 48px; border-top: 1px solid var(--color-border); padding-top: 16px; text-align: center; }}
  .severity {{ font-size: 12px; padding: 2px 8px; border-radius: 4px; }}
  .severity.critical {{ background: var(--color-error); color: white; }}
  .severity.high {{ background: color-mix(in srgb, var(--color-error) 70%, transparent); color: white; }}
  .severity.medium {{ background: var(--color-warn); color: #1a1a2e; }}
  .severity.low {{ background: var(--color-info); color: #1a1a2e; }}
  @media (max-width: 900px) {{ .charts {{ grid-template-columns: 1fr; }} }}
  @media (max-width: 700px) {{ body {{ padding: 12px; }} .overview {{ gap: 12px; }} .stat {{ padding: 12px 16px; }} }}
</style>
</head>
<body data-theme="dark">
<button class="theme-toggle" onclick="toggleTheme()">🌓 主题</button>

<h1>📊 {title}</h1>
<div class="meta">{total_lines} 行 · {total_errors} 错误 · {sources} 个文件 · 耗时 {elapsed:.1}s · AI: {model}</div>

<h2>📋 概览</h2>
<div class="overview">
  <div class="stat"><div class="value">{sources_count}</div><div class="label">日志文件</div></div>
  <div class="stat"><div class="value">{total_errors}</div><div class="label">错误总数</div></div>
  <div class="stat"><div class="value">{total_lines}</div><div class="label">日志行数</div></div>
  <div class="stat"><div class="value">{elapsed:.1}s</div><div class="label">耗时</div></div>
</div>

<h2>📈 交互图表（首个文件）</h2>
<div class="charts">
  <div class="chart-box wide"><h3>错误趋势（时间线）</h3><canvas id="timelineChart"></canvas></div>
  <div class="chart-box"><h3>日志级别分布</h3><canvas id="levelPieChart"></canvas></div>
  <div class="chart-box"><h3>TOP 错误分组</h3><canvas id="groupsBarChart"></canvas></div>
</div>

<h2>🔗 跨源关联</h2>
{correlations_html}

{sources_html}

<h2>🔴 根因分析</h2>
{root_causes_html}

<h2>🛠️ 修复建议</h2>
{fix_suggestions_html}

<div class="footer">由 logai 生成 · 日志数据未上传 · 仅 AI 看到聚合统计</div>

<script>
function toggleTheme() {{
  const el = document.documentElement;
  const current = el.getAttribute('data-theme');
  const next = current === 'light' ? 'dark' : 'light';
  el.setAttribute('data-theme', next);
  updateChartColors(next);
}}
function getChartColors(theme) {{
  const style = getComputedStyle(document.documentElement);
  return {{ grid: theme === 'light' ? '#ddd' : '#2a2a4e', text: style.getPropertyValue('--color-muted').trim() || '#888' }};
}}
let charts = {{}};
function updateChartColors(theme) {{
  const c = getChartColors(theme);
  Object.values(charts).forEach(ch => {{
    if (ch.options && ch.options.scales) {{
      if (ch.options.scales.x) {{ ch.options.scales.x.grid = {{ color: c.grid }}; ch.options.scales.x.ticks = {{ color: c.text }}; }}
      if (ch.options.scales.y) {{ ch.options.scales.y.grid = {{ color: c.grid }}; ch.options.scales.y.ticks = {{ color: c.text }}; }}
    }}
    if (ch.options && ch.options.plugins && ch.options.plugins.legend) {{
      ch.options.plugins.legend.labels = {{ color: c.text }};
    }}
    ch.update();
  }});
}}
{chart_data}
</script>
</body>
</html>"#,
        title = title,
        total_lines = total_lines,
        total_errors = total_errors,
        sources = multi.sources.len(),
        sources_count = multi.sources.len(),
        elapsed = elapsed_secs,
        model = model_name,
        correlations_html = correlations_html,
        sources_html = sources_html,
        root_causes_html = root_causes_html,
        fix_suggestions_html = fix_suggestions_html,
        chart_data = chart_data,
    )
}

fn escape_html(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}
