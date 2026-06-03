# Changelog

## [0.4.0] - 2026-06-03

### Added
- `--dry-run` mode: parse + aggregate without AI (no API key needed)
- `--sample` flag: analyze built-in demo log for instant first experience
- `logai init`: generate `logai.toml` config template in current directory
- `logai completion <shell>`: shell completion scripts (bash, zsh, fish)
- Structured error codes (E001-E006) with remediation hints
- Auto-detect available API keys in error messages
- `logai <file>` (no subcommand) defaults to interactive TUI
- Multi-source HTML reports with per-source sections + correlation panel
- CONTRIBUTING.md, UPGRADE.md, GitHub issue templates

### Changed
- All CLI help text unified to Chinese
- Missing API key errors now list all available backend options
- New dependency: `clap_complete` for shell completions

## [0.3.0] - 2026-06-02

### Added
- `analyze --tui`: open interactive TUI browser after analysis, reusing pre-parsed data
- HTML theme toggle: light/dark switch with Google Fonts (Inter, Noto Sans SC, JetBrains Mono)
- DESIGN.md: unified color tokens mapping between TUI `ThemeColors` and HTML CSS variables
- Terminal width detection: `< 80` column mode simplifies output, TUI layout adapts responsively
- Custom parse config: `--parse-*` CLI flags, `logai.toml` auto-loading, `--rules-file` support
- Multi-source correlation: `logai analyze app.log db.log` — per-source analysis + cross-source correlation
- Multi-source TUI: Tab key cycles between sources, correlation panel shows cross-source links
- Parse config priority: CLI flags > config file > auto-detect

### Changed
- `logai analyze` now accepts multiple file arguments (backward compatible — single file still works)
- TUI status bar and layout adapt to narrow terminals (< 80 columns)
- HTML report uses CSS custom properties for theming, responsive breakpoints at 900px/700px
- New dependency: `toml` for config file parsing

## [0.2.1] - 2026-06-01

### Changed
- TUI detail panel reordered: error signature most prominent, metadata compact, stack trace collapsible via Enter
- AI retry with 3x exponential backoff (1s/2s/4s) and callback reporter for CLI/TUI compatibility
- All user-facing strings unified to Chinese: status bar, help popup, anomaly labels

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
