# logai watch v1.2 Design

**Date:** 2026-05-31
**Status:** draft
**Scope:** Real-time log monitoring subcommand — `logai watch <file>`

---

## Motivation

`logai analyze` is great for post-mortem debugging, but production issues demand real-time awareness. `logai watch` adds live tailing + periodic AI analysis so you can catch problems as they happen, not 30 minutes later when someone finally checks the logs.

---

## Architecture

```
logai watch app.log --window 30
        │
        ▼
┌──────────────────────────────────┐
│  Watcher (notify)                 │  ← IN_MODIFY events on the file
│  Detects new data written to file │
└──────────────┬───────────────────┘
               ▼
┌──────────────────────────────────┐
│  Incremental Reader               │  ← BufReader::seek to last position
│  Reads only new lines since last  │     On truncate: reset to 0
│  read; parses into Vec<LogEntry>  │     On delete: wait for reappearance
└──────────────┬───────────────────┘
               ▼
┌──────────────────────────────────┐
│  Accumulator                      │  ← Append new entries to accumulated Vec
│  Runs full aggregate() each tick  │     aggregate is O(n) — safe for typical
│  Detects spikes, new errors, etc. │     log volumes over a monitoring session
└──────────────┬───────────────────┘
               ▼
┌──────────────────────────────────┐
│  AI Analysis (per window tick)    │  ← Reuses create_backend + analyze
│  Only triggers every --window sec │     with_retry on failure
└──────────────┬───────────────────┘
               ▼
┌──────────────────────────────────┐
│  Scrolling Output                 │  ← println! with timestamp prefix
│  "[14:03:27] 🔴 Spike detected..." │     render_report for the summary
└──────────────────────────────────┘
```

**New file:** `src/watcher.rs` — notify event loop + incremental read + time-window trigger
**Modified:**
- `src/cli.rs` — add `Watch` subcommand and `WatchArgs`
- `src/parser/mod.rs` — expose `parse_lines(&[String])` helper for incremental parsing
- `src/main.rs` — no changes (subcommand dispatch already generic)

**Reused (no changes):**
- `src/aggregator/` — `aggregate()` called each window tick
- `src/ai/` — `create_backend()`, `analyze()`, `with_retry()`
- `src/renderer.rs` — `render_report()` per tick

---

## CLI Interface

```bash
# Minimal
logai watch app.log

# Full
logai watch app.log \
    --window 30 \           # Time window in seconds (default: 30)
    --model deepseek \      # AI backend
    --deep \                # Deep/stronger model
    --format json \         # Force log format
    --min-level warn \      # Minimum log level
    --max-initial-lines 10000  # Max lines to analyze on startup (default: 10000)
```

```rust
#[derive(clap::Args)]
pub struct WatchArgs {
    pub file: PathBuf,

    #[arg(long, default_value_t = 30)]
    pub window: u64,

    #[arg(short, long, default_value = "auto")]
    pub model: ModelArg,

    #[arg(long, default_value_t = false)]
    pub deep: bool,

    #[arg(short, long, default_value = "auto")]
    pub format: FormatArg,

    #[arg(long, default_value = "info")]
    pub min_level: LevelArg,

    #[arg(long, default_value_t = 10000)]
    pub max_initial_lines: usize,
}
```

---

## Behavior Specification

### Startup

1. Verify file exists — if not, error and exit immediately
2. Read the file content (last `--max-initial-lines` lines only for files exceeding that)
3. Parse with format detection (or `--format` override), apply `--min-level` filter
4. Run `aggregate()` + AI `analyze()` + `render_report()` — same as `analyze` subcommand
5. Print separator `--- Watching for new log entries (window: 30s) ---`
6. Record current file size as `last_position`

### Watch Loop

1. Wait for `notify` `IN_MODIFY` event on the file
2. On event: seek to `last_position`, read new bytes, parse new lines into `Vec<LogEntry>`
3. Apply `--min-level` filter to new entries
4. Append filtered entries to accumulated `Vec<LogEntry>`
5. Update `last_position`
6. Every `--window` seconds: if any new entries accumulated, run `aggregate()` + AI `analyze()` + print scrolling output
7. Between window ticks: batch new entries silently (no analysis)

### Output Format

```
🔍 Parsing app.log...
   Parsed 4230 log entries
   Found 5 error groups, 2 anomalies
🤖 Analyzing with DeepSeek (deepseek-chat)...

╔══════════════════════════════════════════════════════╗
║          📊 logai 分析报告 (initial)                   ║
... (full report) ...
╚══════════════════════════════════════════════════════╝

--- Watching for new log entries (window: 30s) ---

[14:03:27] 📊 Window #1 · +47 lines · 5.2s
... (analysis output) ...

[14:04:00] 📊 Window #2 · +12 lines · 3.1s
... (analysis output) ...
```

### Error Handling

| Scenario | Behavior |
|----------|----------|
| **File not found at startup** | Error and exit immediately |
| **File truncated** (size < last_position) | Print `⚠️ File truncated, resetting...`, reset `last_position = 0`, re-read up to `--max-initial-lines` lines, reset accumulated entries |
| **File deleted** (logrotate mv) | Print `⚠️ File gone, waiting for reappearance...`, poll `Path::exists()` every 1s, on reappearance re-open, reset state, print `✅ File reappeared, resuming...` |
| **File rotated in-place** (mv old + touch new) | notify detects new inode via `IN_CREATE` or `IN_MOVED_FROM` + `IN_MOVED_TO`, re-open file, reset |
| **AI API call fails** | Print `⚠️ AI analysis failed: <error> — skipping this window`, keep accumulated entries for next window |
| **Ctrl+C** | Print summary: `⏹️ Watched 12m 30s · 4 analyses · 2 alerts · 523 lines total`, exit 0 |
| **File grows beyond max_initial_lines during watch** | No impact — `max_initial_lines` only applies at startup. Watch accumulates all new lines. |

---

## Implementation Notes

### Incremental Parser Reuse

The existing `parse_log_file()` reads a whole file. We need a lower-level entry point:

```rust
// NEW in src/parser/mod.rs
pub fn parse_lines(lines: &[String], format: Format) -> Vec<LogEntry> {
    match format {
        Format::Json => lines.iter().enumerate()
            .filter_map(|(i, l)| json::parse_json_line(l, i + 1))
            .collect(),
        Format::PlainText => plain_text::parse_plain_text_iter(
            lines.to_vec().into_iter()
        ),
    }
}
```

The watcher calls `detect_format()` once at startup, then calls `parse_lines()` with the resolved format for each batch of new lines.

### notify Event Loop

Use `notify` crate with `Watcher` + `mpsc` channel:

```rust
let (tx, rx) = std::sync::mpsc::channel();
let mut watcher = notify::recommended_watcher(move |res| {
    if let Ok(event) = res { tx.send(event).ok(); }
})?;
watcher.watch(file_path, RecursiveMode::NonRecursive)?;
```

Main loop: `tokio::select!` between `rx.recv()` (file events) and `tokio::time::sleep(window)` (analysis tick).

### Tokio Compatibility

`notify::recommended_watcher` runs on its own thread. Use `tokio::task::spawn_blocking` or a `std::sync::mpsc` channel to bridge into the async runtime. The main watch loop is an async function that `select!`s between the channel and a timer.

---

## Test Plan

| Test | Type | What it verifies |
|------|------|-----------------|
| `test_watch_initial_analysis` | Integration | Startup analyses existing file content |
| `test_watch_detects_new_lines` | Integration | Append to file → window tick → analysis triggered |
| `test_watch_no_analysis_without_new_lines` | Unit | Empty window → no AI call |
| `test_watch_truncate_reset` | Unit | File truncated → state reset |
| `test_watch_file_not_found` | Integration | Missing file → error exit |
| `test_watch_ctrl_c_summary` | Unit | Summary formatting includes correct counts |
| `test_incremental_read_resumes_from_position` | Unit | seek + read returns only new lines |
| `test_max_initial_lines` | Unit | Large file → only last N lines analyzed at startup |
| `test_parse_lines_json` | Unit | `parse_lines()` works for JSON |
| `test_parse_lines_plain_text` | Unit | `parse_lines()` works for plain text |

---

## Non-Goals

- Multi-file watching (`logai watch *.log`) — separate feature
- Desktop notifications (system tray, push) — separate feature
- Persistent state across restarts — not needed for v1.2
- Anomaly-only AI trigger — time-window-only for this version
- Fixed dashboard TUI — scrolling output only for this version

---

## Dependencies

| Crate | Purpose | New? |
|-------|---------|------|
| `notify` 6.x | File system events | **New** |
| All others | Reused from existing Cargo.toml | Existing |

---

## Risk Assessment

- **Performance:** Full `aggregate()` each window tick is O(n). For a 30-minute watch session with 500 errors/minute, n ≈ 15000 — aggregate runs in <10ms. Not a bottleneck.
- **Memory:** Accumulating all entries without bound could grow large. Address this in v1.3 if needed (e.g., configurable ring buffer). For now, typical monitoring sessions (hours at moderate volume) won't exceed a few hundred MB.
- **notify on Windows:** `notify` crate supports Windows via `ReadDirectoryChangesW`. Test on Windows CI.
- **AI cost:** With 30s window, worst case ~120 API calls/hour. At DeepSeek prices (~$0.27/1M tokens), each call consumes ~3K tokens → ~$0.0008/call → ~$0.10/hour. Acceptable.
