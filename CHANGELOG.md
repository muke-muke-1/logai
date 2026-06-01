# Changelog

## [0.2.0] - 2026-06-01

### Added
- TUI live mode: `logai interactive app.log --live` now polls the file every second and auto-refreshes the error group display
- watcher now responds to Ctrl+C with graceful shutdown and cleanup
- `filter_by_level()` helper extracted to `types.rs` — reusable across all subcommands
- 6 new tests: 3 for `parse_ai_response` (valid JSON, markdown-wrapped, graceful degradation) + 3 for anomaly capping and token budget trimming

### Changed
- `build_signature()` uses `Cow<str>` to avoid allocation when no regex pattern matches
- `parse_log_file()` consumes line strings via `into_iter()` for the PlainText path

### Removed
- Dead code in watcher: `_alert_count`, `_total_lines`, `_start_time` (incremented but never read)
- Dead wrapper: `truncate_sig()` in tui.rs (identical to `truncate_str()`)

## [0.1.0] - 2026-05-31

### Added
- Initial release: `analyze`, `watch`, and `interactive` subcommands
- 5-stage pipeline: parse → aggregate → AI backend → render
- TUI interactive log browser with ratatui (vim keys, search, theme toggle, help)
- HTML report export with self-contained dark-theme CSS
- AI backends: Claude, OpenAI, DeepSeek, Ollama (with auto-detect)
- 95 tests across parse, aggregate, anomaly detection, watcher, TUI, HTML, and CLI integration
