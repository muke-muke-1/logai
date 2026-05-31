use crate::types::{AnalysisSummary, Anomaly, Level};

pub fn build_analysis_prompt(summary: &AnalysisSummary) -> String {
    let mut prompt = String::new();

    prompt.push_str("你是一个专业的日志分析工程师。请分析以下日志摘要，找出根因并提供修复建议。\n\n");

    // Overview
    prompt.push_str("## 概览\n");
    prompt.push_str(&format!("- 总行数: {}\n", summary.total_lines));
    if let (Some(start), Some(end)) = (summary.time_start, summary.time_end) {
        let duration = end - start;
        prompt.push_str(&format!(
            "- 时间范围: {} ~ {} (跨度 {} 分钟)\n",
            start.format("%Y-%m-%d %H:%M:%S"),
            end.format("%Y-%m-%d %H:%M:%S"),
            duration.num_minutes()
        ));
    }
    let total = summary.total_lines.max(1) as f64;
    let error_count = summary.level_distribution.get(&Level::Error).unwrap_or(&0);
    let _warn_count = summary.level_distribution.get(&Level::Warn).unwrap_or(&0);
    prompt.push_str(&format!("- 错误率: {:.1}% ({} 条)\n", *error_count as f64 / total * 100.0, error_count));
    prompt.push_str(&format!(
        "- 日志级别分布: ERROR {}, WARN {}, INFO {}, DEBUG {}, TRACE {}, UNKNOWN {}\n",
        summary.level_distribution.get(&Level::Error).unwrap_or(&0),
        summary.level_distribution.get(&Level::Warn).unwrap_or(&0),
        summary.level_distribution.get(&Level::Info).unwrap_or(&0),
        summary.level_distribution.get(&Level::Debug).unwrap_or(&0),
        summary.level_distribution.get(&Level::Trace).unwrap_or(&0),
        summary.level_distribution.get(&Level::Unknown).unwrap_or(&0),
    ));

    // Top errors
    if !summary.error_groups.is_empty() {
        prompt.push_str("\n## TOP 错误\n");
        for (i, group) in summary.error_groups.iter().enumerate() {
            prompt.push_str(&format!("\n### 错误 {} | 出现 {} 次", i + 1, group.count));
            if let (Some(first), Some(last)) = (group.first_seen, group.last_seen) {
                prompt.push_str(&format!(
                    " | 首次 {} | 最后 {} | 趋势: {:?}",
                    first.format("%H:%M:%S"),
                    last.format("%H:%M:%S"),
                    group.trend
                ));
            }
            prompt.push_str(&format!("\n签名: {}\n", group.signature));
            if !group.samples.is_empty() {
                prompt.push_str("样本:\n");
                for sample in &group.samples {
                    prompt.push_str(&format!("  {}\n", sample));
                }
            }
            if let Some(ref stack) = group.stack_trace {
                prompt.push_str(&format!("堆栈:\n```\n{}\n```\n", stack));
            }
        }
    }

    // Anomalies
    if !summary.anomalies.is_empty() {
        prompt.push_str("\n## 检测到的异常\n");
        for anomaly in &summary.anomalies {
            match anomaly {
                Anomaly::Spike { group_index, multiplier } => {
                    prompt.push_str(&format!("⚠️ 错误组 {}: 频次突增 {:.1} 倍\n", group_index + 1, multiplier));
                }
                Anomaly::NewError { group_index } => {
                    prompt.push_str(&format!("🆕 错误组 {}: 新出现的错误\n", group_index + 1));
                }
                Anomaly::SilentRecovery { group_index } => {
                    prompt.push_str(&format!("✅ 错误组 {}: 已静默恢复\n", group_index + 1));
                }
                Anomaly::PeriodicPattern { group_index, period_minutes } => {
                    prompt.push_str(&format!("🔁 错误组 {}: 周期性出现，约每 {} 分钟一次\n", group_index + 1, period_minutes));
                }
            }
        }
    }

    prompt.push_str("\n\n请以 JSON 格式回复，不要带 markdown 标记，只返回纯 JSON。格式如下：\n");
    prompt.push_str(r#"{
  "root_causes": [
    {
      "description": "根因描述",
      "evidence": ["证据1", "证据2"],
      "severity": "Critical"
    }
  ],
  "summary": "一句话总结",
  "fix_suggestions": [
    {
      "action": "修复操作",
      "code_snippet": "修复代码（可选）",
      "reference": "参考文档链接（可选）"
    }
  ],
  "confidence": 0.85
}"#);

    prompt
}
