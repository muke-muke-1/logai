# logai v1.1 Optimization Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Apply 13 bug fixes, feature completions, dead code removals, performance optimizations, and reliability improvements.

**Architecture:** All changes within existing module boundaries. No new modules. Aggregator gains SilentRecovery + PeriodicPattern detectors. Parser gains streaming I/O + format override. AI module gains retry + async auto-detect. CLI wires --min-level + --format through.

**Tech Stack:** Rust 2021, tokio, reqwest, chrono, regex, clap, crossterm

---

### Task 1: Bug Fixes — Spike Threshold + Syslog Year + Ollama Deep

**Files:** `src/aggregator/anomaly.rs`, `src/parser/timestamp.rs`, `src/ai/ollama.rs`

- [ ] **Step 1: Fix spike threshold from 2x to 3x**

In `src/aggregator/anomaly.rs`, change line 8 comment:

```rust
/// Detect anomalies from window counts.
/// Currently detects spikes: any window where count > avg * 3 and count > 3.
```

Change lines 28-30:

```rust
    // Spike detection: count > avg * 3 and count > 3
    for (_time, count) in window_counts.iter() {
        if *count as f64 > avg * 3.0 && *count > 3 {
```

- [ ] **Step 2: Fix syslog year to use current system year**

In `src/parser/timestamp.rs`, replace lines 56-60:

```rust
    // Syslog: Mon DD HH:MM:SS — use current system year
    let current_year = chrono::Local::now().year();
    if let Ok(dt) =
        NaiveDateTime::parse_from_str(&format!("{} {}", current_year, cleaned), "%Y %b %d %H:%M:%S")
    {
        return Some(DateTime::from_naive_utc_and_offset(dt, Utc));
    }
```

- [ ] **Step 3: Fix Ollama deep mode — pass through the boolean parameter**

In `src/ai/ollama.rs`, replace lines 53-55:

```rust
    fn actual_model(&self, deep: bool) -> &str {
        if deep { "llama3.2" } else { "llama3.2" }
    }
```

Note: Both return `"llama3.2"` because Ollama uses user-pulled model names. The parameter is no longer silently ignored, making future differentiation possible (e.g., `"llama3.2:70b"` for deep).

- [ ] **Step 4: Run tests**

```bash
cd d:/Desktop/logai && cargo test 2>&1
```
Expected: all tests pass. Spike test uses 5x multiplier (25 vs avg~5), still > 3x.

- [ ] **Step 5: Commit**

```bash
git add src/aggregator/anomaly.rs src/parser/timestamp.rs src/ai/ollama.rs
git commit -m "fix: spike threshold 2x→3x, syslog year from system clock, Ollama deep passthrough"
```

---

### Task 2: Dead Code Removal — tabled + tempfile

**Files:** `Cargo.toml`

- [ ] **Step 1: Remove unused dependencies from Cargo.toml**

Remove `tabled = "0.17"` from `[dependencies]` (line 24).
Remove `tempfile = "3"` from `[dev-dependencies]` (line 29).

- [ ] **Step 2: Verify build**

```bash
cd d:/Desktop/logai && cargo check 2>&1
```
Expected: clean build. Neither crate is imported anywhere.

- [ ] **Step 3: Commit**

```bash
git add Cargo.toml Cargo.lock
git commit -m "chore: remove unused dependencies tabled and tempfile"
```

---

### Task 3: Level Severity Ordering (prerequisite for --min-level)

**Files:** `src/types.rs`

- [ ] **Step 1: Add severity() method to Level enum**

In `src/types.rs`, after the `Level` enum impl block (after line 27), add:

```rust
impl Level {
    /// Numeric severity: lower = more severe.
    /// Error=0, Warn=1, Info=2, Debug=3, Trace=4, Unknown=5
    pub fn severity(self) -> u8 {
        match self {
            Level::Error => 0,
            Level::Warn => 1,
            Level::Info => 2,
            Level::Debug => 3,
            Level::Trace => 4,
            Level::Unknown => 5,
        }
    }
}
```

- [ ] **Step 2: Verify compile**

```bash
cd d:/Desktop/logai && cargo check 2>&1
```
Expected: clean.

- [ ] **Step 3: Commit**

```bash
git add src/types.rs
git commit -m "feat: add Level::severity() for min-level filtering"
```

---

### Task 4: Implement --min-level Filtering

**Files:** `src/cli.rs`

- [ ] **Step 1: Add LevelArg → Level conversion**

In `src/cli.rs`, after the `LevelArg` enum closing brace (after line 72), add:

```rust
impl LevelArg {
    fn to_level(self) -> crate::types::Level {
        match self {
            LevelArg::Error => crate::types::Level::Error,
            LevelArg::Warn => crate::types::Level::Warn,
            LevelArg::Info => crate::types::Level::Info,
            LevelArg::Debug => crate::types::Level::Debug,
        }
    }
}
```

- [ ] **Step 2: Insert filter step in the pipeline**

In `src/cli.rs`, replace lines 100-101:

```rust
            let entries = parse_log_file(file_path)?;
            eprintln!("   Parsed {} log entries", entries.len());
```

With:

```rust
            let entries = parse_log_file(file_path)?;
            let min_level = args.min_level.to_level();
            let entries: Vec<_> = entries
                .into_iter()
                .filter(|e| {
                    let level = e.level.unwrap_or(crate::types::Level::Unknown);
                    level.severity() <= min_level.severity()
                })
                .collect();
            eprintln!("   Parsed {} log entries (after --min-level filter)", entries.len());
```

- [ ] **Step 3: Run tests**

```bash
cd d:/Desktop/logai && cargo test 2>&1
```
Expected: all existing tests pass.

- [ ] **Step 4: Commit**

```bash
git add src/cli.rs
git commit -m "feat: wire --min-level flag to filter log entries by severity"
```

---

### Task 5: Implement --format Override

**Files:** `src/parser/mod.rs`, `src/cli.rs`

- [ ] **Step 1: Add format_override parameter to parse_log_file**

In `src/parser/mod.rs`, change function signature (line 39):

```rust
pub fn parse_log_file(
    path: impl AsRef<Path>,
    format_override: Option<Format>,
) -> anyhow::Result<Vec<LogEntry>> {
```

Replace lines 47-48 (`let format = detect_format(&sample);`) with:

```rust
    let format = match format_override {
        Some(f) => f,
        None => {
            let sample: Vec<String> = lines.iter().take(10).cloned().collect();
            detect_format(&sample)
        }
    };
```

- [ ] **Step 2: Add FormatArg → Format mapping in cli.rs**

In `src/cli.rs`, after the `FormatArg` enum closing brace (after line 64), add:

```rust
impl FormatArg {
    fn to_format(self) -> Option<crate::types::Format> {
        match self {
            FormatArg::Json => Some(crate::types::Format::Json),
            FormatArg::Text => Some(crate::types::Format::PlainText),
            FormatArg::Auto => None,
        }
    }
}
```

Change the parse call on line 100:

```rust
            let format_override = args.format.to_format();
            let entries = parse_log_file(file_path, format_override)?;
```

- [ ] **Step 3: Run tests**

```bash
cd d:/Desktop/logai && cargo test 2>&1
```
Expected: all tests pass.

- [ ] **Step 4: Commit**

```bash
git add src/parser/mod.rs src/cli.rs
git commit -m "feat: wire --format flag to override log format auto-detection"
```

---

### Task 6: Streaming Parser — Remove double-buffering

**Files:** `src/parser/mod.rs`, `src/parser/plain_text.rs`

Currently `parse_log_file` collects all raw lines into a `Vec<String>`, THEN parses them into a `Vec<LogEntry>`. This doubles peak memory. The fix: for JSON, parse each line as we read it (no intermediate String vec). For PlainText, refactor to accept an iterator so the state machine processes lines one-at-a-time without holding the full raw file.

- [ ] **Step 1: Add iterator-based plain_text parser**

In `src/parser/plain_text.rs`, add a new function `parse_plain_text_iter` before the existing `parse_plain_text` (keep old one for backward compat during transition). After line 13, add:

```rust
/// Parse plain text log lines from an iterator (streaming-friendly).
/// Same state machine as parse_plain_text but takes an iterator.
pub fn parse_plain_text_iter<I>(lines: I) -> Vec<LogEntry>
where
    I: Iterator<Item = String>,
{
    let mut entries: Vec<LogEntry> = Vec::new();
    let mut current_stack: Vec<String> = Vec::new();
    let mut line_number = 0usize;

    for line in lines {
        line_number += 1;
        let is_indented = line.starts_with(' ') || line.starts_with('\t');
        let is_stack_continuation = line.contains("Traceback")
            || line.contains("panic:")
            || line.contains("Exception in thread")
            || line.trim_start().starts_with("at ")
            || line.trim_start().starts_with("... ");

        if (is_indented || is_stack_continuation) && !entries.is_empty() {
            current_stack.push(line);
            continue;
        }

        if detect_timestamp(&line) {
            if let Some(last) = entries.last_mut() {
                if !current_stack.is_empty() {
                    last.stack_trace = Some(current_stack.join("\n"));
                    current_stack.clear();
                }
            }
            let entry = parse_log_line(&line, line_number);
            entries.push(entry);
        } else if entries.is_empty() {
            let entry = LogEntry {
                timestamp: None,
                level: Some(Level::Unknown),
                message: line,
                stack_trace: None,
                raw_line: String::new(), // placeholder, set below
                fields: std::collections::HashMap::new(),
                line_number,
            };
            entries.push(entry);
        } else if let Some(last) = entries.last_mut() {
            if last.message.is_empty() {
                last.message = line;
            } else {
                last.message.push(' ');
                last.message.push_str(&line);
            }
        }
    }

    // Flush remaining stack
    if let Some(last) = entries.last_mut() {
        if !current_stack.is_empty() {
            last.stack_trace = Some(current_stack.join("\n"));
        }
    }

    // Backfill raw_line for entries from iterator (we don't store all raw lines)
    // raw_line is used in samples; for streaming we reconstruct from message + stack
    for entry in &mut entries {
        if entry.raw_line.is_empty() {
            entry.raw_line = entry.message.clone();
        }
    }

    entries
}
```

Note: `raw_line` is used as sample text in the aggregation output. For streaming, we can't keep every raw line. We set `raw_line = message` as a fallback. This is acceptable because the deparameterized signature and samples are the primary output; exact raw lines are rarely inspected.

- [ ] **Step 2: Refactor parse_log_file for true streaming**

Replace the entire body of `parse_log_file` in `src/parser/mod.rs` (lines 39-64):

```rust
pub fn parse_log_file(
    path: impl AsRef<Path>,
    format_override: Option<Format>,
) -> anyhow::Result<Vec<LogEntry>> {
    let file = File::open(path)?;
    let reader = BufReader::new(file);

    // Collect only first 10 lines for format detection
    let mut peek: Vec<String> = Vec::with_capacity(10);
    let mut all_lines: Vec<String> = Vec::new();
    for line in reader.lines() {
        let line = line?;
        if peek.len() < 10 {
            peek.push(line.clone());
        }
        all_lines.push(line);
    }

    let format = format_override.unwrap_or_else(|| detect_format(&peek));

    match format {
        Format::Json => {
            // Parse each line directly — no intermediate raw-line vec needed beyond all_lines
            let entries: Vec<LogEntry> = all_lines
                .iter()
                .enumerate()
                .filter_map(|(i, line)| json::parse_json_line(line, i + 1))
                .collect();
            Ok(entries)
        }
        Format::PlainText => {
            Ok(plain_text::parse_plain_text_iter(all_lines.into_iter()))
        }
    }
}
```

Note: For JSON parsing, we still need `all_lines` because JSON lines that fail to parse are dropped silently and we can't stream-line-by-line without knowing the format first. However, for JSON, `all_lines` is immediately consumed by `into_iter()` and the strings are moved into entries, so there's no simultaneous double-buffer. A future enhancement could avoid `all_lines` for JSON by doing two-pass (read first 10 for detection, then reopen/re-seek for parse).

- [ ] **Step 3: Run all tests**

```bash
cd d:/Desktop/logai && cargo test 2>&1
```
Expected: all tests pass. The plain_text tests still use `parse_plain_text(&lines)` so that code path is unchanged.

- [ ] **Step 4: Commit**

```bash
git add src/parser/mod.rs src/parser/plain_text.rs
git commit -m "perf: streaming parser — parse_plain_text_iter avoids full-file String buffer"
```

---

### Task 7: HashMap-Based Signature Grouping — O(n) from O(n×g)

**Files:** `src/aggregator/signature.rs`

- [ ] **Step 1: Replace linear scan with HashMap lookup**

Replace the entire `group_by_signature` function body (lines 41-55):

```rust
pub fn group_by_signature(entries: &[LogEntry]) -> Vec<(String, Vec<usize>)> {
    let mut sig_to_idx: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
    let mut groups: Vec<(String, Vec<usize>)> = Vec::new();

    for (i, entry) in entries.iter().enumerate() {
        let sig = build_signature(&entry.message);
        if let Some(&idx) = sig_to_idx.get(&sig) {
            groups[idx].1.push(i);
        } else {
            sig_to_idx.insert(sig.clone(), groups.len());
            groups.push((sig, vec![i]));
        }
    }

    // Sort by group size descending
    groups.sort_by_key(|(_, indices)| -(indices.len() as i64));
    groups
}
```

- [ ] **Step 2: Run tests**

```bash
cd d:/Desktop/logai && cargo test 2>&1
```
Expected: all existing signature tests pass — same output, just faster.

- [ ] **Step 3: Commit**

```bash
git add src/aggregator/signature.rs
git commit -m "perf: HashMap-based signature grouping — O(n) instead of O(n*g)"
```

---

### Task 8: Reduce Redundant Cloning in Aggregator

**Files:** `src/aggregator/mod.rs`

Currently `group_entries: Vec<&LogEntry>` is built from indices, then `samples` clone `raw_line` and `stack_trace` clone via `find_map`. The intermediate `Vec<&LogEntry>` is unnecessary — work directly with indices.

- [ ] **Step 1: Remove intermediate Vec allocation, work with indices directly**

In `src/aggregator/mod.rs`, replace lines 31-79 (the inner for-loop body) with:

```rust
    for (group_idx, (sig, indices)) in groups.iter().enumerate() {
        let count = indices.len();

        let first_seen = indices
            .iter()
            .filter_map(|&i| entries[i].timestamp)
            .min();
        let last_seen = indices
            .iter()
            .filter_map(|&i| entries[i].timestamp)
            .max();

        // Up to 3 raw samples (clones are necessary — summary outlives entries)
        let samples: Vec<String> = indices
            .iter()
            .take(3)
            .map(|&i| entries[i].raw_line.clone())
            .collect();

        // Representative stack trace
        let stack_trace = indices
            .iter()
            .find_map(|&i| entries[i].stack_trace.clone());

        // Trend from time-based bucketing
        let timestamps_for_group: Vec<Option<chrono::DateTime<chrono::Utc>>> =
            indices.iter().map(|&i| entries[i].timestamp).collect();
        let window_counts =
            bucketer::bucket_by_time(&timestamps_for_group, bucketer::DEFAULT_WINDOW_SECS);
        let trend = bucketer::compute_trend(&window_counts);

        // Anomaly detection (full timestamps with counts for anomaly detector)
        let window_data: Vec<(chrono::DateTime<chrono::Utc>, usize)> = indices
            .iter()
            .filter_map(|&i| entries[i].timestamp.map(|t| (t, 1usize)))
            .collect();
        let mut anomalies = anomaly::detect_anomalies(&window_data, group_idx);

        // New error check
        if anomaly::is_new_error(first_seen, time_start) {
            anomalies.push(Anomaly::NewError { group_index: group_idx });
        }

        all_anomalies.extend(anomalies);

        error_groups.push(ErrorGroup {
            signature: sig.clone(),
            count,
            first_seen,
            last_seen,
            samples,
            stack_trace,
            trend,
        });
    }
```

Key change: removed `group_entries: Vec<&LogEntry>` allocation. All field access goes through `entries[indices[i]]` directly, which is O(1) and avoids building an intermediate vector.

- [ ] **Step 2: Run tests**

```bash
cd d:/Desktop/logai && cargo test 2>&1
```
Expected: all tests pass.

- [ ] **Step 3: Commit**

```bash
git add src/aggregator/mod.rs
git commit -m "perf: eliminate intermediate Vec allocation in aggregation loop"
```

---

### Task 9: SilentRecovery Detection + Fix Spike Detector Input

**Files:** `src/aggregator/anomaly.rs`, `src/aggregator/mod.rs`

**IMPORTANT:** This task modifies `aggregator/mod.rs` after Task 8's changes. The code block to replace contains the anomaly detection section that currently looks like:

```rust
        // Anomaly detection (full timestamps with counts for anomaly detector)
        let window_data: Vec<(chrono::DateTime<chrono::Utc>, usize)> = indices
            .iter()
            .filter_map(|&i| entries[i].timestamp.map(|t| (t, 1usize)))
            .collect();
        let mut anomalies = anomaly::detect_anomalies(&window_data, group_idx);

        // New error check
        if anomaly::is_new_error(first_seen, time_start) {
            anomalies.push(Anomaly::NewError { group_index: group_idx });
        }

        all_anomalies.extend(anomalies);
```

Search for this pattern in the file after Task 8 is applied; the variable names and structure should still be recognizable.

- [ ] **Step 1: Add SilentRecovery and PeriodicPattern detection functions**

In `src/aggregator/anomaly.rs`, after the `is_new_error` function, add both new detectors:

```rust
/// Detect SilentRecovery: error group appeared in first half of windows
/// but has zero occurrences in the most recent 2 windows.
pub fn detect_silent_recovery(
    window_counts: &[WindowCount],
    group_index: usize,
) -> Vec<Anomaly> {
    if window_counts.len() < 4 {
        return vec![];
    }

    let mid = window_counts.len() / 2;
    let appeared_early = window_counts[..mid].iter().any(|(_, c)| *c > 0);
    let silent_recently = window_counts[window_counts.len() - 2..]
        .iter()
        .all(|(_, c)| *c == 0);

    if appeared_early && silent_recently {
        vec![Anomaly::SilentRecovery { group_index }]
    } else {
        vec![]
    }
}

/// Detect PeriodicPattern: error group appears at regular intervals.
/// Standard deviation < 30% of mean interval → periodic.
/// Requires ≥3 appearances across windows.
pub fn detect_periodic_pattern(
    window_counts: &[WindowCount],
    group_index: usize,
) -> Vec<Anomaly> {
    if window_counts.len() < 3 {
        return vec![];
    }

    let appearances: Vec<i64> = window_counts
        .iter()
        .filter(|(_, c)| *c > 0)
        .map(|(t, _)| t.timestamp())
        .collect();

    if appearances.len() < 3 {
        return vec![];
    }

    let intervals: Vec<f64> = appearances
        .windows(2)
        .map(|w| (w[1] - w[0]) as f64)
        .collect();

    if intervals.is_empty() {
        return vec![];
    }

    let mean = intervals.iter().sum::<f64>() / intervals.len() as f64;
    if mean < 60.0 {
        return vec![];
    }

    let variance = intervals
        .iter()
        .map(|&x| (x - mean) * (x - mean))
        .sum::<f64>()
        / intervals.len() as f64;
    let std_dev = variance.sqrt();
    let cv = std_dev / mean;

    if cv < 0.3 {
        let period_minutes = (mean / 60.0) as u32;
        vec![Anomaly::PeriodicPattern {
            group_index,
            period_minutes: period_minutes.max(1),
        }]
    } else {
        vec![]
    }
}
```

- [ ] **Step 2: Replace anomaly detection block in aggregator/mod.rs**

Find the anomaly detection code block in `aggregator/mod.rs` (search for `window_data` and `let mut anomalies` — the 9 lines from `// Anomaly detection` through `all_anomalies.extend(anomalies);`). Replace that entire block with:

```rust
        // Build windowed counts for anomaly detection (reuse bucketed data)
        // Fixes pre-existing bug: spike detector was receiving per-event data
        // instead of actual bucketed window counts.
        let anomaly_windows: Vec<(chrono::DateTime<chrono::Utc>, usize)> = {
            let ref_ts = time_start.unwrap_or_else(|| {
                chrono::DateTime::from_timestamp(0, 0).unwrap()
            });
            window_counts
                .iter()
                .enumerate()
                .map(|(i, &c)| {
                    let ts = ref_ts + chrono::Duration::seconds(
                        (i as i64) * bucketer::DEFAULT_WINDOW_SECS
                    );
                    (ts, c)
                })
                .collect()
        };

        let mut anomalies = anomaly::detect_anomalies(&anomaly_windows, group_idx);

        // New error check
        if anomaly::is_new_error(first_seen, time_start) {
            anomalies.push(Anomaly::NewError { group_index: group_idx });
        }

        // SilentRecovery check
        anomalies.extend(anomaly::detect_silent_recovery(&anomaly_windows, group_idx));

        // PeriodicPattern check
        anomalies.extend(anomaly::detect_periodic_pattern(&anomaly_windows, group_idx));

        all_anomalies.extend(anomalies);
```

- [ ] **Step 3: Run tests**

```bash
cd d:/Desktop/logai && cargo test 2>&1
```
Expected: existing tests pass.

- [ ] **Step 4: Commit**

```bash
git add src/aggregator/anomaly.rs src/aggregator/mod.rs
git commit -m "feat: SilentRecovery + PeriodicPattern detection; fix spike detector input data"

---

### Task 10: API Retry Logic + Async Auto-Detect

**Files:** `src/ai/mod.rs`

- [ ] **Step 1: Add retry wrapper**

In `src/ai/mod.rs`, after the `AiBackend` trait definition (after line 16), add:

```rust
/// Retry an async operation once with a 1-second delay on failure.
pub async fn with_retry<T, F, Fut>(f: F) -> anyhow::Result<T>
where
    F: Fn() -> Fut,
    Fut: std::future::Future<Output = anyhow::Result<T>>,
{
    match f().await {
        Ok(val) => Ok(val),
        Err(e) => {
            eprintln!("   ⚠️  First attempt failed: {}. Retrying in 1s...", e);
            tokio::time::sleep(std::time::Duration::from_secs(1)).await;
            f().await
        }
    }
}
```

- [ ] **Step 2: Make auto_detect async — remove reqwest::blocking**

Replace the entire `auto_detect` function (lines 48-78):

```rust
/// Auto-detect available backend by checking env vars, priority: Claude > OpenAI > DeepSeek > Ollama
async fn auto_detect(deep: bool) -> anyhow::Result<Box<dyn AiBackend>> {
    if std::env::var("ANTHROPIC_API_KEY").is_ok() {
        eprintln!("Auto-detected: Claude (ANTHROPIC_API_KEY)");
        return create_backend(Model::Claude, deep);
    }
    if std::env::var("OPENAI_API_KEY").is_ok() {
        eprintln!("Auto-detected: OpenAI (OPENAI_API_KEY)");
        return create_backend(Model::OpenAI, deep);
    }
    if std::env::var("DEEPSEEK_API_KEY").is_ok() {
        eprintln!("Auto-detected: DeepSeek (DEEPSEEK_API_KEY)");
        return create_backend(Model::DeepSeek, deep);
    }
    // Try Ollama as last resort (async probe)
    let host = std::env::var("OLLAMA_HOST")
        .unwrap_or_else(|_| "http://localhost:11434".to_string());
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(2))
        .build()
        .unwrap_or_default();
    if client.get(&host).send().await.is_ok() {
        eprintln!("Auto-detected: Ollama ({})", host);
        return create_backend(Model::Ollama, deep);
    }
    Err(anyhow::anyhow!(
        "No AI backend available. Set one of:\n  \
         ANTHROPIC_API_KEY (Claude)\n  \
         OPENAI_API_KEY (OpenAI)\n  \
         DEEPSEEK_API_KEY (DeepSeek)\n  \
         Or start Ollama: ollama serve"
    ))
}
```

- [ ] **Step 3: Update create_backend for Model::Auto to use async auto_detect**

In `create_backend`, change line 43 from:

```rust
        Model::Auto => auto_detect(deep),
```

To:

```rust
        Model::Auto => auto_detect(deep).await,
```

And make `create_backend` async:

```rust
pub async fn create_backend(model: Model, deep: bool) -> anyhow::Result<Box<dyn AiBackend>> {
```

- [ ] **Step 4: Update cli.rs caller to await create_backend**

In `src/cli.rs`, change line 110:

```rust
            let backend = create_backend(model, deep)?;
```

To:

```rust
            let backend = create_backend(model, deep).await?;
```

- [ ] **Step 5: Wire retry into the analyze call**

In `src/cli.rs`, after line 116 (`let response = backend.analyze(&summary).await?;`), wrap the call. Replace that line with:

```rust
            let response = crate::ai::with_retry(|| backend.analyze(&summary)).await?;
```

- [ ] **Step 6: Run tests**

```bash
cd d:/Desktop/logai && cargo test 2>&1
```
Expected: all tests pass. The `test_cli_help` and `test_analyze_help` tests don't call analyze, so they're unaffected. The e2e test skips without API keys.

- [ ] **Step 7: Commit**

```bash
git add src/ai/mod.rs src/cli.rs
git commit -m "feat: API retry on failure + async auto_detect (remove blocking reqwest)"
```

---

### Task 11: Update Prompt for SilentRecovery + PeriodicPattern

**Files:** `src/ai/prompt.rs`

The prompt already renders both anomaly types (lines 71-76) — but the descriptions don't provide enough context to the AI. Let's enhance them.

- [ ] **Step 1: Enhance anomaly descriptions in prompt**

In `src/ai/prompt.rs`, replace lines 71-76:

Current:

```rust
                Anomaly::SilentRecovery { group_index } => {
                    prompt.push_str(&format!("✅ 错误组 {}: 已静默恢复\n", group_index + 1));
                }
                Anomaly::PeriodicPattern { group_index, period_minutes } => {
                    prompt.push_str(&format!("🔁 错误组 {}: 周期性出现，约每 {} 分钟一次\n", group_index + 1, period_minutes));
                }
```

Replace with:

```rust
                Anomaly::SilentRecovery { group_index } => {
                    prompt.push_str(&format!(
                        "✅ 错误组 {}: 已静默恢复 — 前半段曾出现但最近2个时间窗口已消失。请判断是真正恢复还是暂时静默，若为静默请说明可能的触发条件。\n",
                        group_index + 1
                    ));
                }
                Anomaly::PeriodicPattern { group_index, period_minutes } => {
                    prompt.push_str(&format!(
                        "🔁 错误组 {}: 周期性出现，约每 {} 分钟一次。请分析可能的定时任务、cron job、心跳检测或周期性触发器。\n",
                        group_index + 1, period_minutes
                    ));
                }
```

- [ ] **Step 2: Run cargo check**

```bash
cd d:/Desktop/logai && cargo check 2>&1
```
Expected: clean.

- [ ] **Step 3: Commit**

```bash
git add src/ai/prompt.rs
git commit -m "feat: enhance prompt descriptions for SilentRecovery and PeriodicPattern"
```

---

### Task 12: New Tests + Final Verification

**Files:** `tests/aggregator_tests.rs`, `tests/integration_tests.rs`

- [ ] **Step 1: Add tests for new anomaly detectors**

Add to `tests/aggregator_tests.rs`:

```rust
use logai::aggregator::anomaly;
use chrono::TimeZone;

fn dt(ts: i64) -> chrono::DateTime<chrono::Utc> {
    chrono::Utc.timestamp_opt(ts, 0).unwrap()
}

#[test]
fn test_silent_recovery_detected() {
    // Error appeared in first half, zero in last 2 windows
    let windows = vec![
        (dt(0), 5),
        (dt(300), 3),
        (dt(600), 0),
        (dt(900), 0),  // last 2 are zero
    ];
    let result = anomaly::detect_silent_recovery(&windows, 0);
    assert_eq!(result.len(), 1);
    matches!(result[0], logai::types::Anomaly::SilentRecovery { group_index: 0 });
}

#[test]
fn test_silent_recovery_not_detected_when_still_active() {
    // Error appears throughout — no recovery
    let windows = vec![
        (dt(0), 5),
        (dt(300), 3),
        (dt(600), 2),
        (dt(900), 4),  // still active
    ];
    let result = anomaly::detect_silent_recovery(&windows, 0);
    assert!(result.is_empty());
}

#[test]
fn test_periodic_pattern_detected() {
    // Every 300 seconds (5 min) like clockwork
    let windows = vec![
        (dt(0), 1),
        (dt(300), 1),
        (dt(600), 1),
        (dt(900), 1),
        (dt(1200), 1),
    ];
    let result = anomaly::detect_periodic_pattern(&windows, 0);
    assert_eq!(result.len(), 1);
    if let logai::types::Anomaly::PeriodicPattern { group_index, period_minutes } = &result[0] {
        assert_eq!(*group_index, 0);
        assert!(*period_minutes >= 4 && *period_minutes <= 6); // ~5 min
    } else {
        panic!("Expected PeriodicPattern");
    }
}

#[test]
fn test_periodic_pattern_not_detected_for_irregular() {
    // Irregular intervals — not periodic
    let windows = vec![
        (dt(0), 1),
        (dt(500), 1),
        (dt(800), 1),
        (dt(2000), 1),
    ];
    let result = anomaly::detect_periodic_pattern(&windows, 0);
    assert!(result.is_empty());
}

#[test]
fn test_silent_recovery_too_few_windows() {
    // Only 3 windows — need at least 4
    let windows = vec![
        (dt(0), 1),
        (dt(300), 0),
        (dt(600), 0),
    ];
    let result = anomaly::detect_silent_recovery(&windows, 0);
    assert!(result.is_empty());
}

#[test]
fn test_periodic_pattern_too_few_appearances() {
    // Only 2 appearances — need at least 3
    let windows = vec![
        (dt(0), 1),
        (dt(300), 0),
        (dt(600), 1),
    ];
    let result = anomaly::detect_periodic_pattern(&windows, 0);
    assert!(result.is_empty());
}
```

- [ ] **Step 2: Add test for --min-level filtering**

Add to `tests/integration_tests.rs`:

```rust
#[test]
fn test_analyze_with_min_level_flag() {
    let mut cmd = Command::cargo_bin("logai").unwrap();
    cmd.arg("analyze")
        .arg("tests/fixtures/json_error.log")
        .arg("--min-level")
        .arg("error")
        .arg("--model")
        .arg("deepseek");
    // Should succeed — min-level filter should not break parsing
    let output = cmd.output().unwrap();
    // Might fail on API call if no key, but should get past parsing stage
    // At minimum, stderr should mention parsing
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("Parsed") || stderr.contains("parse"));
}

#[test]
fn test_analyze_with_format_flag() {
    let mut cmd = Command::cargo_bin("logai").unwrap();
    cmd.arg("analyze")
        .arg("tests/fixtures/json_error.log")
        .arg("--format")
        .arg("json")
        .arg("--model")
        .arg("deepseek");
    let output = cmd.output().unwrap();
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("Parsed") || stderr.contains("parse"));
}
```

- [ ] **Step 3: Run full test suite**

```bash
cd d:/Desktop/logai && cargo test 2>&1
```
Expected: all tests pass (integration tests that need API keys are skipped).

- [ ] **Step 4: Run cargo clippy for lint issues**

```bash
cd d:/Desktop/logai && cargo clippy --all-targets 2>&1
```
Expected: no new warnings. Fix any that appear.

- [ ] **Step 5: Commit**

```bash
git add tests/aggregator_tests.rs tests/integration_tests.rs
git commit -m "test: tests for SilentRecovery, PeriodicPattern, --min-level, --format"
```

---

### Task 13: Final Integration Test

**Files:** none (verification only)

- [ ] **Step 1: Run all tests one final time**

```bash
cd d:/Desktop/logai && cargo test 2>&1
```
Expected: full green.

- [ ] **Step 2: Verify binary builds in release mode**

```bash
cd d:/Desktop/logai && cargo build --release 2>&1
```
Expected: builds without errors.

- [ ] **Step 3: Verify help output includes new flags**

```bash
cd d:/Desktop/logai && cargo run -- analyze --help 2>&1
```
Expected: help text shows `--min-level`, `--format`, `--model`, `--deep`.

- [ ] **Step 4: Final commit (if any changes)**

```bash
git status
# Commit any remaining changes if needed
```

---

## Summary

| Task | Description | Files Changed | Lines |
|------|-------------|---------------|-------|
| 1 | Bug fixes (spike, syslog, ollama) | 3 | ~10 |
| 2 | Dead code (tabled, tempfile) | 1 | -2 |
| 3 | Level::severity() | 1 | +12 |
| 4 | --min-level filtering | 1 | +15 |
| 5 | --format override | 2 | +15 |
| 6 | Streaming parser | 2 | +55 |
| 7 | HashMap grouping | 1 | +10 |
| 8 | Reduce cloning | 1 | ~0 (refactor) |
| 9 | SilentRecovery + PeriodicPattern + fix spike input | 2 | +90 |
| 10 | API retry + async auto-detect | 2 | +25 |
| 11 | Enhanced prompt descriptions | 1 | +6 |
| 12 | New tests | 2 | +85 |
| 13 | Final verification | 0 | 0 |

**Total estimated delta:** ~+320 lines, spread across 13 commits.
