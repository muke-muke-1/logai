use crate::types::{
    AiResponse, AnalysisSummary, FixSuggestion, Level, MultiSourceSummary, RootCause, Severity,
};
use crossterm::style::{Color, Stylize};

/// Detect terminal width, returns (width, is_narrow: width < 80)
fn terminal_width() -> (u16, bool) {
    match crossterm::terminal::size() {
        Ok((w, _h)) if w > 0 => (w, w < 80),
        _ => (80, false), // fallback: assume normal width
    }
}

/// Render the full analysis report to terminal
pub fn render_report(
    summary: &AnalysisSummary,
    response: &AiResponse,
    elapsed_secs: f64,
    model_name: &str,
) {
    let (_width, narrow) = terminal_width();
    print_header(summary, elapsed_secs, narrow);
    print_overview(summary, narrow);
    print_root_causes(&response.root_causes, narrow);
    print_fix_suggestions(&response.fix_suggestions, narrow);
    print_footer(elapsed_secs, model_name);
}

fn print_header(summary: &AnalysisSummary, elapsed: f64, narrow: bool) {
    println!();
    if narrow {
        println!("{}", "── 📊 logai 分析报告 ──".with(Color::DarkBlue));
        let time_info = match (summary.time_start, summary.time_end) {
            (Some(s), Some(e)) => format!("{} → {}", s.format("%H:%M:%S"), e.format("%H:%M:%S")),
            _ => "N/A".to_string(),
        };
        println!(
            "{}",
            format!("{} 行 · {} · {:.1}s", summary.total_lines, time_info, elapsed)
                .with(Color::DarkBlue)
        );
    } else {
        println!(
            "{}",
            "╔══════════════════════════════════════════════════════╗".with(Color::DarkBlue)
        );
        println!(
            "{}",
            "║          📊 logai 分析报告                              ║".with(Color::DarkBlue)
        );
        let time_info = match (summary.time_start, summary.time_end) {
            (Some(s), Some(e)) => format!("{} → {}", s.format("%H:%M:%S"), e.format("%H:%M:%S")),
            _ => "N/A".to_string(),
        };
        println!(
            "{}",
            format!(
                "║  {} 行 · {} · {:.1}s                                          ║",
                summary.total_lines, time_info, elapsed
            )
            .with(Color::DarkBlue)
        );
        println!(
            "{}",
            "╚══════════════════════════════════════════════════════╝".with(Color::DarkBlue)
        );
    }
}

fn print_overview(summary: &AnalysisSummary, narrow: bool) {
    let total = summary.total_lines.max(1) as f64;
    let error_count = summary.level_distribution.get(&Level::Error).unwrap_or(&0);
    let warn_count = summary.level_distribution.get(&Level::Warn).unwrap_or(&0);
    let error_rate = *error_count as f64 / total * 100.0;
    let warn_rate = *warn_count as f64 / total * 100.0;

    let error_bar = if narrow { String::new() } else { "█".repeat((error_rate / 2.0) as usize) };
    let warn_bar = if narrow { String::new() } else { "█".repeat((warn_rate / 2.0) as usize) };

    println!();
    if narrow {
        println!(
            "{}",
            "┌─ 📋 概览 ──────────────────────────┐".with(Color::DarkGrey)
        );
        println!(
            "  错误: {:.1}% ({})  警告: {:.1}% ({})",
            error_rate, error_count, warn_rate, warn_count
        );
    } else {
        println!(
            "{}",
            "┌─ 📋 概览 ──────────────────────────────────────────────┐".with(Color::DarkGrey)
        );
        println!(
            "  错误率: {:.1}% ({})    警告率: {:.1}% ({})",
            error_rate, error_count, warn_rate, warn_count
        );
        if error_rate > 0.0 {
            println!(
                "  ERROR  {} {}%",
                error_bar.with(Color::Red),
                format!("{:.1}", error_rate).with(Color::Red)
            );
        }
        if warn_rate > 0.0 {
            println!(
                "  WARN   {} {}%",
                warn_bar.with(Color::Yellow),
                format!("{:.1}", warn_rate).with(Color::Yellow)
            );
        }
    }
    println!(
        "{}",
        if narrow {
            "└────────────────────────────────────┘".with(Color::DarkGrey)
        } else {
            "└────────────────────────────────────────────────────────┘".with(Color::DarkGrey)
        }
    );
}

fn print_root_causes(causes: &[RootCause], narrow: bool) {
    if causes.is_empty() {
        println!("\n✅ 未发现明显的根因问题");
        return;
    }
    println!();
    let box_w: usize = if narrow { 36 } else { 54 };
    for (i, cause) in causes.iter().enumerate() {
        let severity_icon = match cause.severity {
            Severity::Critical => "🔴",
            Severity::High => "🟠",
            Severity::Medium => "🟡",
            Severity::Low => "🟢",
        };
        println!(
            "{}",
            format!(
                "┌─ {} 根因 {}/{} {}┐",
                severity_icon,
                i + 1,
                causes.len(),
                "─".repeat(box_w.saturating_sub(20))
            )
            .with(Color::DarkGrey)
        );
        println!();
        let desc = if narrow && cause.description.len() > 60 {
            format!("{}...", &cause.description[..57])
        } else {
            cause.description.clone()
        };
        println!("  {}", desc.with(Color::Red).bold());
        println!();
        if !cause.evidence.is_empty() {
            println!("  {}", "证据:".with(Color::DarkGrey));
            for ev in &cause.evidence {
                let ev_display = if narrow && ev.len() > 60 {
                    format!("{}...", &ev[..57])
                } else {
                    ev.clone()
                };
                println!("  • {}", ev_display);
            }
            println!();
        }
        let severity_str = match cause.severity {
            Severity::Critical => "严重",
            Severity::High => "高",
            Severity::Medium => "中",
            Severity::Low => "低",
        };
        println!(
            "  {} {}",
            "严重程度:".with(Color::DarkGrey),
            format!("{} {}", severity_icon, severity_str).bold()
        );
        println!(
            "{}",
            format!("└{}┘", "─".repeat(box_w)).with(Color::DarkGrey)
        );
        println!();
    }
}

fn print_fix_suggestions(suggestions: &[FixSuggestion], narrow: bool) {
    if suggestions.is_empty() {
        return;
    }
    let box_w: usize = if narrow { 36 } else { 54 };
    println!(
        "{}",
        format!("┌─ 🛠️ 修复建议 {}┐", "─".repeat(box_w.saturating_sub(14)))
            .with(Color::DarkGrey)
    );
    println!();
    for (i, fix) in suggestions.iter().enumerate() {
        let action = if narrow && fix.action.len() > 60 {
            format!("{}...", &fix.action[..57])
        } else {
            fix.action.clone()
        };
        println!(
            "  {}. {}",
            (i + 1).to_string().with(Color::Green).bold(),
            action.with(Color::Green)
        );
        if let Some(ref code) = fix.code_snippet {
            let code_display = if narrow && code.len() > 60 {
                format!("{}...", &code[..57])
            } else {
                code.clone()
            };
            println!("     {}", code_display.with(Color::Cyan));
        }
        if let Some(ref ref_url) = fix.reference {
            println!("     参考: {}", ref_url.clone().with(Color::DarkGrey));
        }
        println!();
    }
    println!(
        "{}",
        format!("└{}┘", "─".repeat(box_w)).with(Color::DarkGrey)
    );
}

fn print_footer(elapsed: f64, model_name: &str) {
    println!(
        "{}",
        format!(
            "总耗时 {:.1}s | AI: {} | 日志数据未上传",
            elapsed, model_name
        )
        .with(Color::DarkGrey)
    );
    println!();
}

/// Render multi-source analysis report
pub fn render_multi_source(
    multi: &MultiSourceSummary,
    response: Option<&AiResponse>,
    elapsed_secs: f64,
    model_name: &str,
) {
    let (width, narrow) = terminal_width();

    // ── Header ──
    println!();
    let header = format!("📊 logai 多源分析 ({} 个文件)", multi.sources.len());
    if narrow {
        println!("{}", header.with(Color::DarkBlue));
    } else {
        println!(
            "{}",
            format!(
                "╔══ {} ══╗",
                "═".repeat(width.saturating_sub(20) as usize)
            )
            .with(Color::DarkBlue)
        );
        println!("{}", format!("║  {}  ║", header).with(Color::DarkBlue));
        println!(
            "{}",
            format!(
                "╚══{}══╝",
                "═".repeat(width.saturating_sub(20) as usize)
            )
            .with(Color::DarkBlue)
        );
    }

    println!(
        "{}",
        format!(
            "{} 行 · {} 错误 · {:.1}s · {}",
            multi.total_lines(),
            multi.total_errors(),
            elapsed_secs,
            model_name
        )
        .with(Color::DarkGrey)
    );

    // ── Per-source sections ──
    for source in &multi.sources {
        println!();
        let title = format!("📁 {}", source.name);
        if narrow {
            println!("{}", format!("── {} ──", title).with(Color::Cyan).bold());
        } else {
            println!(
                "{}",
                format!("┌─ {} {}┐", title, "─".repeat(40)).with(Color::Cyan)
            );
        }

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

        println!(
            "  {} 行 · ERROR: {} · WARN: {} · {} 分组 · {} 异常",
            source.summary.total_lines,
            error_count,
            warn_count,
            source.summary.error_groups.len(),
            source.summary.anomalies.len()
        );

        // Top 3 error groups
        let top_n = 3usize.min(source.summary.error_groups.len());
        if top_n > 0 {
            println!();
            for (i, group) in source.summary.error_groups.iter().take(top_n).enumerate() {
                let sig = if narrow && group.signature.len() > 60 {
                    format!("{}...", &group.signature[..57])
                } else {
                    group.signature.clone()
                };
                println!(
                    "    {}. {} ({})",
                    (i + 1).to_string().with(Color::Red).bold(),
                    sig,
                    format!("{}", group.count).with(Color::Yellow)
                );
            }
            if source.summary.error_groups.len() > top_n {
                println!(
                    "    {}",
                    format!(
                        "    ... 还有 {} 个分组",
                        source.summary.error_groups.len() - top_n
                    )
                    .with(Color::DarkGrey)
                );
            }
        }

        if !narrow {
            println!("{}", format!("└{}┘", "─".repeat(44)).with(Color::Cyan));
        }
    }

    // ── Cross-source correlation ──
    if !multi.correlations.is_empty() {
        println!();
        println!(
            "{}",
            "┌─ 🔗 跨源关联分析 ──────────────────────────────┐".with(Color::Yellow)
        );
        for corr in &multi.correlations {
            let score_icon = if corr.score > 0.7 {
                "🔴"
            } else if corr.score > 0.4 {
                "🟡"
            } else {
                "🟢"
            };
            println!(
                "  {} {} ↔ {} ({:.0}%)",
                score_icon,
                corr.source_a.clone().with(Color::Cyan),
                corr.source_b.clone().with(Color::Cyan),
                corr.score * 100.0
            );
            println!("    {}", corr.description.clone().with(Color::DarkGrey));
            println!();
        }
        println!(
            "{}",
            "└────────────────────────────────────────────────────┘".with(Color::Yellow)
        );
    }

    // ── AI response (if available) ──
    if let Some(resp) = response {
        print_root_causes(&resp.root_causes, narrow);
        print_fix_suggestions(&resp.fix_suggestions, narrow);
    }

    print_footer(elapsed_secs, model_name);
}
