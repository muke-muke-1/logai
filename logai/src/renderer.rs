use crate::types::{AiResponse, AnalysisSummary, FixSuggestion, Level, RootCause, Severity};
use crossterm::style::{Color, Stylize};

/// Render the full analysis report to terminal
pub fn render_report(
    summary: &AnalysisSummary,
    response: &AiResponse,
    elapsed_secs: f64,
    model_name: &str,
) {
    print_header(summary, elapsed_secs);
    print_overview(summary);
    print_root_causes(&response.root_causes);
    print_fix_suggestions(&response.fix_suggestions);
    print_footer(elapsed_secs, model_name);
}

fn print_header(summary: &AnalysisSummary, elapsed: f64) {
    println!();
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

fn print_overview(summary: &AnalysisSummary) {
    let total = summary.total_lines.max(1) as f64;
    let error_count = summary.level_distribution.get(&Level::Error).unwrap_or(&0);
    let warn_count = summary.level_distribution.get(&Level::Warn).unwrap_or(&0);
    let error_rate = *error_count as f64 / total * 100.0;
    let warn_rate = *warn_count as f64 / total * 100.0;

    let error_bar = "█".repeat((error_rate / 2.0) as usize);
    let warn_bar = "█".repeat((warn_rate / 2.0) as usize);

    println!();
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
    println!(
        "{}",
        "└────────────────────────────────────────────────────────┘".with(Color::DarkGrey)
    );
}

fn print_root_causes(causes: &[RootCause]) {
    if causes.is_empty() {
        println!("\n✅ 未发现明显的根因问题");
        return;
    }
    println!();
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
                "┌─ {} 根因 {}/{} ──────────────────────────────────────┐",
                severity_icon,
                i + 1,
                causes.len()
            )
            .with(Color::DarkGrey)
        );
        println!();
        println!("  {}", cause.description.clone().with(Color::Red).bold());
        println!();
        if !cause.evidence.is_empty() {
            println!("  {}", "证据:".with(Color::DarkGrey));
            for ev in &cause.evidence {
                println!("  • {}", ev);
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
            "└────────────────────────────────────────────────────────┘".with(Color::DarkGrey)
        );
        println!();
    }
}

fn print_fix_suggestions(suggestions: &[FixSuggestion]) {
    if suggestions.is_empty() {
        return;
    }
    println!(
        "{}",
        "┌─ 🛠️ 修复建议 ──────────────────────────────────────────┐".with(Color::DarkGrey)
    );
    println!();
    for (i, fix) in suggestions.iter().enumerate() {
        println!(
            "  {}. {}",
            (i + 1).to_string().with(Color::Green).bold(),
            fix.action.clone().with(Color::Green)
        );
        if let Some(ref code) = fix.code_snippet {
            println!("     {}", code.clone().with(Color::Cyan));
        }
        if let Some(ref ref_url) = fix.reference {
            println!("     参考: {}", ref_url.clone().with(Color::DarkGrey));
        }
        println!();
    }
    println!(
        "{}",
        "└────────────────────────────────────────────────────────┘".with(Color::DarkGrey)
    );
}

fn print_footer(elapsed: f64, model_name: &str) {
    println!(
        "{}",
        format!("总耗时 {:.1}s | AI: {} | 日志数据未上传", elapsed, model_name)
            .with(Color::DarkGrey)
    );
    println!();
}
