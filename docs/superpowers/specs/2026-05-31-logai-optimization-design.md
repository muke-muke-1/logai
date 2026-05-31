# logai v1.1 Optimization Design

**Date:** 2026-05-31
**Status:** draft
**Scope:** Bug fixes, dead code removal, performance improvements, and completion of unfinished features

---

## Motivation

The logai MVP is complete and functional, but exploration revealed 13 issues across three categories: bugs, dead/incomplete code, and performance. This design addresses all of them in a single optimization pass.

---

## Category 1: Bug Fixes

### 1.1 Spike Threshold Mismatch

**File:** `src/aggregator/anomaly.rs`
**Problem:** Spike detection uses `avg * 2.0`, but the design spec specifies `avg * 3.0`.
**Fix:** Change the multiplier from `2.0` to `3.0`.

### 1.2 Hardcoded Syslog Year

**File:** `src/parser/timestamp.rs`
**Problem:** Syslog timestamps lack a year component; the parser hardcodes `2026`.
**Fix:** Use `chrono::Local::now().year()` to infer the current system year at parse time. This is a best-effort heuristic — log files from previous years will still be slightly off, but far better than a hardcoded constant.

### 1.3 Ollama Deep Mode Ignored

**File:** `src/ai/ollama.rs`
**Problem:** `actual_model()` returns `"llama3.2"` regardless of the `_deep` parameter.
**Fix:** Return different model strings for default vs deep mode (e.g., `"llama3.2"` vs `"llama3.2:latest"`). While Ollama doesn't distinguish like cloud APIs, the parameter should at least flow through transparently rather than being silently discarded.

---

## Category 2: Complete Unfinished Features

### 2.1 `--min-level` Filtering

**Files:** `src/cli.rs`, `src/parser/mod.rs` (or new filter step)
**Problem:** The `--min-level` CLI flag is parsed but never applied. Users can't filter out low-severity log entries.
**Fix:**
- Add a filter step in `cli.rs` between parsing and aggregation
- Map `Level` to numeric severity: `Error(0) < Warn(1) < Info(2) < Debug(3) < Trace(4)`
- If `--min-level warn`, keep entries where `level <= Warn`
- If the flag is not provided, pass through all entries unchanged

### 2.2 `--format` Forced Format

**Files:** `src/cli.rs`, `src/parser/mod.rs`
**Problem:** The `--format` CLI flag is parsed but never used; auto-detection always runs.
**Fix:**
- Add an `Option<Format>` parameter to `parse_log_file()`
- When `Some(f)`, skip `detect_format()` and parse with the user-specified format
- When `None` (default), run auto-detection as before

### 2.3 SilentRecovery Detection

**Files:** `src/aggregator/anomaly.rs`, `src/types.rs` (already has variant), `src/ai/prompt.rs`
**Problem:** The `Anomaly::SilentRecovery` variant exists but is never constructed.
**Fix:**
- Logic: for each error group with ≥2 occurrences, check if it appeared in the first half of time windows but has zero count in the most recent 2 windows
- Report at most 3 SilentRecovery anomalies
- Add prompt text in `prompt.rs`: "Silent Recovery — 以下错误在前半段曾出现但最近已消失，请判断是真正消失还是暂时静默"

### 2.4 PeriodicPattern Detection

**Files:** `src/aggregator/anomaly.rs`, `src/types.rs` (already has variant), `src/ai/prompt.rs`
**Problem:** The `Anomaly::PeriodicPattern` variant exists but is never constructed.
**Fix:**
- Logic: for error groups appearing in ≥3 windows, compute the intervals between consecutive appearances
- If the standard deviation of intervals < 30% of the mean interval, mark as periodic
- Report at most 3 PeriodicPattern anomalies
- Add prompt text in `prompt.rs`: "Periodic Pattern — 以下错误以固定间隔重复出现，请分析可能的定时任务或周期性触发器"

---

## Category 3: Dead Code Removal

### 3.1 Remove `tabled` Dependency

**File:** `Cargo.toml`
**Problem:** The `tabled` crate is listed as a dependency but never imported or used. The renderer uses `crossterm` with manual formatting.
**Fix:** Remove `tabled` from `[dependencies]` in `Cargo.toml`.

### 3.2 Remove `tempfile` Dev-Dependency

**File:** `Cargo.toml`
**Problem:** `tempfile` is listed as a dev-dependency but no test file uses it.
**Fix:** Remove `tempfile` from `[dev-dependencies]` in `Cargo.toml`.

---

## Category 4: Performance Optimizations

### 4.1 Streaming Log Parser

**File:** `src/parser/mod.rs`
**Problem:** `parse_log_file()` calls `lines.collect()`, loading the entire file into memory. A 500MB log file consumes 500MB+ RAM.
**Fix:**
- Remove the `collect()` call; process lines as a streaming iterator
- For JSON: each line is independently parseable — `BufRead::lines()` maps to `parse_json_line()` directly
- For PlainText: the state machine already processes line-by-line with a stack buffer; hold at most one stack trace in memory at a time
- Memory drops from O(file_size) to O(max_stack_trace_size)

### 4.2 HashMap-Based Signature Grouping

**File:** `src/aggregator/signature.rs`
**Problem:** `group_by_signature()` uses `.find()` on a `Vec<ErrorGroup>` for each entry, giving O(n × g) complexity where g can be large for noisy logs.
**Fix:**
- Maintain a `HashMap<String, usize>` mapping signature → index into the groups vector
- Lookup is O(1) instead of O(g), total complexity drops to O(n)
- The same `group_by_signature` public API is preserved; only the internal implementation changes

### 4.3 Reduce Redundant Cloning

**Files:** `src/aggregator/mod.rs`, `src/aggregator/signature.rs`
**Problem:** Sample lines and stack traces are cloned from entries unnecessarily in the aggregation pipeline.
**Fix:**
- Since entries live for the duration of `aggregate()`, use borrowed `&str` references in the intermediate structures where possible
- Only clone when constructing the final `AnalysisSummary` that is returned to the caller (since entries are dropped after `aggregate()` returns)
- If necessary, introduce a temporary lifetime parameter on intermediate types

---

## Category 5: Reliability

### 5.1 API Retry Logic

**File:** `src/ai/mod.rs`
**Problem:** The design spec mentions "retry once" on API failure, but no retry is implemented.
**Fix:**
- Add a wrapper in the `AiBackend` trait default implementation or in `cli.rs` around the `analyze()` call
- On first failure, wait 1 second, then retry once
- If the retry also fails, propagate the error

### 5.2 Async Auto-Detect

**File:** `src/ai/mod.rs`
**Problem:** `auto_detect()` creates a `reqwest::blocking::Client` inside an async context to probe Ollama. This blocks the tokio runtime thread.
**Fix:**
- Use `reqwest::Client` (async) with a short timeout for the Ollama liveness probe
- Make `auto_detect()` truly async throughout

---

## Files Changed

| File | Nature of change |
|------|------------------|
| `src/aggregator/anomaly.rs` | ~50 lines added (SilentRecovery + PeriodicPattern detection + spike threshold fix) |
| `src/aggregator/signature.rs` | ~15 lines refactored (HashMap grouping) |
| `src/aggregator/mod.rs` | ~10 lines adjusted (reduce cloning, pass refs) |
| `src/parser/mod.rs` | ~20 lines adjusted (streaming parse, format override) |
| `src/parser/timestamp.rs` | ~3 lines fixed (syslog year) |
| `src/cli.rs` | ~20 lines added (min-level filter, format passthrough) |
| `src/types.rs` | ~5 lines (add Level ordering impl if needed) |
| `src/ai/mod.rs` | ~20 lines adjusted (retry wrapper, async auto_detect) |
| `src/ai/ollama.rs` | ~5 lines fixed (deep model name) |
| `src/ai/prompt.rs` | ~20 lines adjusted (SilentRecovery/PeriodicPattern prompt blocks) |
| `Cargo.toml` | ~4 lines removed (tabled, tempfile) |
| `tests/` | ~30 lines new tests added |

**Estimated net delta:** ~150 lines added (after accounting for removals).

---

## Non-Goals

- `logai watch` (real-time monitoring) — separate feature, separate spec
- HTML report export — separate feature
- Multi-source correlation — separate feature
- Interactive TUI — separate feature

---

## Risk Assessment

- **Backward compatibility:** All changes are internal; CLI interface remains the same. `--min-level` and `--format` become functional rather than silently ignored — this is strictly additive. No breaking changes.
- **Performance:** Streaming parser and HashMap grouping are strict improvements. No regression risk.
- **Anomaly detection changes:** The spike threshold change (2x→3x) will report fewer spikes, which is the intended (spec-compliant) behavior. SilentRecovery and PeriodicPattern are additive — they create new anomaly reports that were previously absent.
