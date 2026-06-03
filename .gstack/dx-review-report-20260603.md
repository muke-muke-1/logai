# logai DX Plan Review — Report

> 2026-06-03 · DX POLISH mode · CLI Tool · On-call backend engineer persona

## Developer Persona Card

```
TARGET DEVELOPER PERSONA
========================
Who:       On-call backend engineer debugging production at 2 AM
Context:   Found an error in Datadog, ssh'd into the server, staring at 500MB of logs
Tolerance: < 2 minutes for first useful output — if it takes longer, they fall back to grep
Expects:   Zero config, works in terminal, privacy (logs stay on machine), AI quality ≥ manual ChatGPT
```

## Empathy Narrative

"I'm on call. It's 2:14 AM. PagerDuty woke me up — payment processing error rate spiked to 15%.
I ssh into the prod box. There's a 200MB `app.log` from the last 3 hours. I remember someone mentioned
logai on HN last week. `cargo install logai` — okay, it's Rust, that's fast. `logai analyze app.log`.
'ANTHROPIC_API_KEY not set.' What? I don't have a Claude key. I use OpenAI. What key do I need?
Where do I get one? Let me read the error... it only mentions Anthropic. Let me try `--model openai`.
'OPENAI_API_KEY not set.' I have one somewhere... `export OPENAI_API_KEY=$(grep OPENAI ~/.zshrc |
head -1 | cut -d= -f2)`. Re-run. It works. 3 minutes later I have root cause analysis. The AI
correctly identified a connection pool exhaustion issue. That was useful — but those first 3 minutes
of API key confusion almost made me give up and just paste logs into ChatGPT."

## DX Scorecard

```
+====================================================================+
|              DX PLAN REVIEW — SCORECARD                             |
+====================================================================+
| Dimension            | Score  | Prior  | Trend  |
|----------------------|--------|--------|--------|
| Getting Started      |  4/10  |   —    |   —    |
| API/CLI/SDK          |  6/10  |   —    |   —    |
| Error Messages       |  5/10  |   —    |   —    |
| Documentation        |  7/10  |   —    |   —    |
| Upgrade Path         |  5/10  |   —    |   —    |
| Dev Environment      |  7/10  |   —    |   —    |
| Community            |  6/10  |   —    |   —    |
| DX Measurement       |  2/10  |   —    |   —    |
+--------------------------------------------------------------------+
| TTHW                 | ~3 min (blocked by API key setup)            |
| Competitive Rank     | Needs Work → Competitive (after fixes)       |
| Magical Moment       | Missing — first AI analysis should feel magic|
| Product Type         | CLI Tool                                    |
| Mode                 | DX POLISH                                    |
| Overall DX           |  5.3/10 → 7.5/10 (after fixes)              |
+====================================================================+
| DX PRINCIPLE COVERAGE                                               |
| Zero Friction      | GAP — API key blocks first run                 |
| Learn by Doing     | GAP — no --sample, no --dry-run, no playground |
| Fight Uncertainty  | GAP — errors lack codes and remediation hints  |
| Opinionated + Escape Hatches | COVERED — auto-detect + --model flag |
| Code in Context    | COVERED — real code snippets in fix suggestions |
| Magical Moments    | GAP — no instant "wow" on first run            |
+====================================================================+
```

## Implementation Tasks

- [ ] **T1 (P1, human: ~2h / CC: ~15min)** — CLI — Add `--dry-run`, `--sample`, and `logai init`
  - Surfaced by: Pass 1 Getting Started — API key requirement blocks first-run experience
  - `--dry-run`: parse + aggregate + print summary, skip AI call
  - `--sample`: bundled demo log that works without API key
  - `logai init`: bootstrap `logai.toml` in current directory
  - Files: `src/cli.rs`, embed sample log as `const`
  - Verify: `cargo test`, `logai analyze --dry-run tests/fixtures/json_error.log`

- [ ] **T2 (P1, human: ~1.5h / CC: ~15min)** — CLI — Shell completions + unified help language + default TUI
  - Surfaced by: Pass 2 CLI Design — no completions, mixed language, dual TUI paths
  - Add `clap_complete` dependency, `logai completion bash|zsh|fish` subcommand
  - Unify all help text to Chinese
  - `logai app.log` (no subcommand) defaults to TUI
  - Files: `src/cli.rs`, `src/main.rs`, `Cargo.toml`
  - Verify: `cargo test`, manual `logai completion bash | source /dev/stdin`

- [ ] **T3 (P1, human: ~2h / CC: ~15min)** — Errors — Structured error types with codes and remediation
  - Surfaced by: Pass 3 Error Messages — raw anyhow strings, no line numbers, no remediation
  - Error enum with codes: E001 (file not found), E002 (parse error + line), E003 (missing API key + list providers), E004 (AI call failed + check network/rate-limit)
  - API key error auto-detects which keys ARE set and suggests the missing one
  - Files: `src/errors.rs` (new), `src/cli.rs`, `src/ai/mod.rs`, `src/parser/mod.rs`
  - Verify: `cargo test`, trigger each error type manually

- [ ] **T4 (P2, human: ~1.5h / CC: ~15min)** — Docs — CONTRIBUTING.md + CI badge + issue templates + upgrade guide
  - Surfaced by: Passes 4-8 — missing community docs, no upgrade guidance
  - CONTRIBUTING.md: how to build, test, submit PRs
  - CI badge in README (if GitHub Actions exists)
  - `.github/ISSUE_TEMPLATE/bug_report.md` and `feature_request.md`
  - UPGRADE.md stub for v0.3→v0.4 migration
  - Files: `CONTRIBUTING.md`, `UPGRADE.md`, `.github/ISSUE_TEMPLATE/*.md`, `README.md`
  - Verify: Review rendered markdown

- [ ] **T5 (P2, human: ~1h / CC: ~10min)** — HTML — Multi-source HTML report
  - Surfaced by: Design doc gap — multi-source works in terminal/TUI but HTML only renders first source
  - Extend `render_report_html` to accept `MultiSourceSummary`
  - Per-source sections + correlation panel in HTML
  - Files: `src/renderer_html.rs`, `src/cli.rs`
  - Verify: `cargo test html_tests`

## NOT in scope (deferred to v0.5+)

- CI/CD integration (`logai ci`) — requires GitHub Actions template design
- Linux packages (.deb/.rpm) — requires CI pipeline
- Windows installer (.msi) — requires WiX toolchain
- DX telemetry/analytics — requires privacy-first design
- `logai diff` subcommand — requires two-file comparison architecture

## What already exists
- DESIGN.md: color tokens, fonts, spacing
- 101 tests: parse, aggregate, anomaly, AI, HTML, TUI, watcher, CLI integration
- crates.io + Homebrew distribution
- CHANGELOG.md (maintained)
- README.md (bilingual, strong)
