# Changelog

## [0.2.0] - 2026-06-01

### Added
- TUI live mode: `logai interactive app.log --live` now polls the file every second and auto-refreshes the error group display
- TUI AI 对话面板: press `a` on any error group to ask AI for root cause analysis, with scrollable response popup
- HTML report now includes Chart.js interactive charts: timeline (error trends), doughnut (level distribution), horizontal bar (top error groups)
- InteractiveArgs now supports `--deep` flag and passes model selection to TUI
- `chat()` method added to `AiBackend` trait, implemented for all 4 backends (Claude, OpenAI, DeepSeek, Ollama)
- watcher now responds to Ctrl+C with graceful shutdown and cleanup
- `filter_by_level()` helper extracted to `types.rs` — reusable across all subcommands
- 6 new tests: 3 for `parse_ai_response` (valid JSON, markdown-wrapped, graceful degradation) + 3 for anomaly capping and token budget trimming
- crates.io metadata: `readme`, `documentation` fields, proper `repository` URL
- Homebrew formula at `pkg/homebrew/logai.rb`

### Changed
- `build_signature()` uses `Cow<str>` to avoid allocation when no regex pattern matches
- `parse_log_file()` consumes line strings via `into_iter()` for the PlainText path
- Test count: 95 → 101 (all passing)

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
