# logai v0.3 待办

> 来源: /plan-design-review 全维度审查 (2026-06-01)
> 设计评分: 5/10 → 8/10

## P1 — v0.3 必须交付

- [x] **T1: TUI 详情面板信息优先级重排** (人工 ~20min / CC ~5min)
  - 渲染顺序: 根因摘要 → 修复建议(前2条) → 元数据(次数+时间) → 堆栈(Enter 折叠)
  - 文件: `src/tui.rs`
  - 验证: `cargo test tui_tests`

- [x] **T2: AI 调用自动重试 + 友好错误信息** (人工 ~1h / CC ~10min)
  - CLI: 3 次指数退避重试，失败后显示检查建议 (API key/网络)
  - TUI: 错误显示在对话框中，支持按键重试
  - 文件: `src/ai/mod.rs`, `src/tui.rs`
  - 验证: `cargo test ai_tests`

- [x] **T3: 统一中文化所有用户可见字符串** (人工 ~30min / CC ~10min)
  - TUI 状态栏、帮助面板、空状态提示全部改为中文
  - 代码标识符保持英文
  - 文件: `src/tui.rs`, `src/renderer.rs`, `src/renderer_html.rs`
  - 验证: `cargo test`, 手动检查各子命令输出

## P2 — v0.3 子版本

- [x] **T4: analyze --tui 分析后自动打开 TUI** (人工 ~30min / CC ~10min)
  - 一次解析共享数据，分析完直接进入 TUI 交互浏览
  - 文件: `src/cli.rs`, `src/tui.rs`
  - 验证: `cargo test integration_tests`

- [x] **T5: HTML 指定字体 + 亮/暗主题切换** (人工 ~1h / CC ~10min)
  - 正文: Inter / Noto Sans SC, 代码: JetBrains Mono / Consolas
  - JS 主题切换按钮，与 TUI 主题视觉语言一致
  - 文件: `src/renderer_html.rs`
  - 验证: `cargo test html_tests`

- [x] **T6: 创建 DESIGN.md 初稿 + 统一颜色令牌** (人工 ~30min / CC ~10min)
  - 颜色令牌: TUI ThemeColors ↔ HTML CSS 变量对齐
  - 记录字体选择、间距、命名约定
  - 文件: `DESIGN.md` (新建), `src/tui.rs`, `src/renderer_html.rs`

- [x] **T7: 终端宽度检测 + HTML 响应式** (人工 ~45min / CC ~10min)
  - crossterm::terminal::size() 动态宽度，< 80 列时简化输出
  - HTML: canvas 容器 100% 宽度，移动端单列
  - 文件: `src/tui.rs`, `src/renderer.rs`, `src/renderer_html.rs`

## P2 — v0.3 新功能

- [x] **T8: 自定义解析规则 CLI 标志 + 配置文件** (人工 ~2h / CC ~20min)
  - `--parse-*` 标志覆盖快速场景, `logai.toml` 或 `--rules-file` 覆盖复用场景
  - 优先级: CLI 标志 > 配置文件 > 自动检测
  - 文件: `src/cli.rs`, `src/parser.rs`, `src/types.rs`

- [x] **T9: 多源关联分析 分源视图 + 关联面板** (人工 ~3h / CC ~30min)
  - TUI: Tab 键切换来源 (app.log/db.log/nginx.log)，详情面板显示跨源因果链
  - CLI: 每个源独立 section + 底部关联分析 section
  - 文件: `src/tui.rs`, `src/cli.rs`, `src/renderer.rs`

## v0.4 — DX POLISH (来源: /plan-devex-review 2026-06-03)

> DX 评分: 5.3/10 → 目标 7.5/10 · TTHW: ~3min → 目标 < 1min

### P1 — v0.4 必须交付

- [x] **T10: --dry-run + --sample + logai init** (人工 ~2h / CC ~15min)
  - `--dry-run`: 解析 + 聚合 + 打印摘要，跳过 AI 调用
  - `--sample`: 内置示例日志，无需 API key 即可演示
  - `logai init`: 在当前目录生成 `logai.toml` 模板
  - 文件: `src/cli.rs`
  - 验证: `cargo test`, `logai analyze --dry-run tests/fixtures/json_error.log`

- [x] **T11: Shell 补全 + 统一中文 help + 默认 TUI** (人工 ~1.5h / CC ~15min)
  - `logai completion bash|zsh|fish` 子命令 (clap_complete)
  - 所有 help 文本统一为中文
  - `logai app.log`（无子命令）默认打开 TUI
  - 文件: `src/cli.rs`, `src/main.rs`, `Cargo.toml`
  - 验证: `cargo test`, `logai completion bash`

- [x] **T12: 结构化错误类型 + 错误码 + 修复提示** (人工 ~2h / CC ~15min)
  - ErrorCode 枚举: E001 文件不存在, E002 解析错误+行号, E003 缺少 API key+列出提供商, E004 AI调用失败+检查网络/限流
  - API key 错误自动检测已设置的 key 并建议缺失的
  - 文件: `src/errors.rs` (新建), `src/cli.rs`, `src/ai/mod.rs`, `src/parser/mod.rs`
  - 验证: `cargo test`, 手动触发每种错误

### P2 — v0.4 子版本

- [x] **T13: CONTRIBUTING.md + CI badge + issue 模板 + UPGRADE.md** (人工 ~1.5h / CC ~15min)
  - CONTRIBUTING.md: 构建、测试、PR 流程
  - CI badge 加入 README
  - `.github/ISSUE_TEMPLATE/` 的 bug_report 和 feature_request
  - UPGRADE.md: v0.3→v0.4 迁移指南
  - 文件: `CONTRIBUTING.md`, `UPGRADE.md`, `.github/ISSUE_TEMPLATE/*.md`, `README.md`

- [x] **T14: 多源 HTML 报告** (人工 ~1h / CC ~10min)
  - 扩展 `render_report_html` 支持 `MultiSourceSummary`
  - HTML 中分源 section + 关联面板
  - 文件: `src/renderer_html.rs`, `src/cli.rs`
  - 验证: `cargo test html_tests`

## NOT in scope (留给 v0.5+)

- AI 缓存降级 (需要设计缓存失效策略)
- TUI AI 调用异步化 (需要重构事件循环)
- 完整的跨平台 CI/CD (GitHub Actions release + Homebrew 自动更新)
- `logai ci` CI/CD 集成 (v0.5: GitHub Actions 模板)
- `logai diff` 差异分析 (v0.5: 基线 vs 当前对比)
- Linux 包 (.deb/.rpm) 和 Windows 安装器 (.msi)
- DX 遥测/分析 (需要隐私优先设计)
