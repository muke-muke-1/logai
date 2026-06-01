use crate::types::{AiResponse, AnalysisSummary, ErrorGroup, FixSuggestion, Level, RootCause, Severity};

/// Generate a self-contained HTML analysis report
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

fn escape_html(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}
