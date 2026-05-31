# logai watch v1.2 设计

**日期:** 2026-06-01
**状态:** 草案
**范围:** 实时日志监控子命令 — `logai watch <文件>`

---

## 动机

`logai analyze` 擅长事后排查，但线上问题需要实时感知。`logai watch` 提供实时 tail + 周期性 AI 分析，让你在问题发生的当下就捕捉到，而不是 30 分钟后有人翻日志才发现。

---

## 架构

```
logai watch app.log --window 30
        │
        ▼
┌──────────────────────────────────┐
│  监听器 (notify)                   │  ← 监听文件的 IN_MODIFY 事件
│  检测文件是否有新数据写入            │
└──────────────┬───────────────────┘
               ▼
┌──────────────────────────────────┐
│  增量读取器                        │  ← BufReader seek 到上次读取位置
│  只读取上次读取之后的新行            │     截断时：重置到文件头
│  解析为 Vec<LogEntry>              │     删除时：等待文件重新出现
└──────────────┬───────────────────┘
               ▼
┌──────────────────────────────────┐
│  累积器                            │  ← 将新条目追加到累积的 Vec 中
│  每次窗口触发时全量重跑 aggregate()   │     aggregate 是 O(n)，典型监控
│  检测 spike、新错误等异常            │     场景的日志量完全够用
└──────────────┬───────────────────┘
               ▼
┌──────────────────────────────────┐
│  AI 分析（每时间窗口触发一次）       │  ← 复用 create_backend + analyze
│  只在 --window 秒间隔时触发         │     失败时 with_retry 重试一次
└──────────────┬───────────────────┘
               ▼
┌──────────────────────────────────┐
│  滚动输出                          │  ← 带时间戳的 println!
│  "[14:03:27] 🔴 检测到 spike..."    │     用 render_report 输出摘要
└──────────────────────────────────┘
```

**新增文件:** `src/watcher.rs` — notify 事件循环 + 增量读取 + 时间窗口触发
**修改文件:**
- `src/cli.rs` — 新增 `Watch` 子命令和 `WatchArgs`
- `src/parser/mod.rs` — 暴露 `parse_lines(&[String])` 辅助函数，供增量解析使用
- `src/main.rs` — 无需改动（子命令分发已是通用的）

**复用（不改动）:**
- `src/aggregator/` — 每个窗口触发时调用 `aggregate()`
- `src/ai/` — `create_backend()`、`analyze()`、`with_retry()`
- `src/renderer.rs` — 每个窗口触发时调用 `render_report()`

---

## CLI 接口

```bash
# 最简用法
logai watch app.log

# 完整参数
logai watch app.log \
    --window 30 \              # 时间窗口（秒），默认 30
    --model deepseek \         # AI 后端
    --deep \                   # 深度/更强模型
    --format json \            # 强制日志格式
    --min-level warn \         # 最低日志级别
    --max-initial-lines 10000  # 启动时分析的最大行数（默认 10000）
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

## 行为规范

### 启动阶段

1. 验证文件存在 — 不存在则报错并立即退出
2. 读取文件内容（如果超过 `--max-initial-lines` 行，只读最后那么多行）
3. 自动检测格式（或用 `--format` 强制指定），应用 `--min-level` 过滤
4. 运行 `aggregate()` + AI `analyze()` + `render_report()` — 与 `analyze` 子命令一致
5. 打印分隔线 `--- 正在监听新日志 (窗口: 30秒) ---`
6. 记录当前文件大小作为 `last_position`

### 监听循环

1. 等待 `notify` 的 `IN_MODIFY` 事件
2. 收到事件后：seek 到 `last_position`，读取新增字节，解析新行为 `Vec<LogEntry>`
3. 对新条目应用 `--min-level` 过滤
4. 将过滤后的条目追加到累积的 `Vec<LogEntry>` 中
5. 更新 `last_position`
6. 每 `--window` 秒：如果累积了新条目，运行 `aggregate()` + AI `analyze()` + 打印滚动输出
7. 窗口间隔期间：静默累积新条目（不触发分析）

### 输出格式

```
🔍 正在解析 app.log...
   已解析 4230 条日志
   发现 5 个错误分组, 2 个异常
🤖 正在用 DeepSeek (deepseek-chat) 分析...

╔══════════════════════════════════════════════════════╗
║          📊 logai 分析报告 (初始)                      ║
... (完整报告) ...
╚══════════════════════════════════════════════════════╝

--- 正在监听新日志 (窗口: 30秒) ---

[14:03:27] 📊 窗口 #1 · +47 行 · 耗时 5.2秒
... (分析输出) ...

[14:04:00] 📊 窗口 #2 · +12 行 · 耗时 3.1秒
... (分析输出) ...
```

### 错误处理

| 场景 | 行为 |
|------|------|
| **启动时文件不存在** | 报错并立即退出 |
| **文件被截断**（文件大小 < last_position） | 打印 `⚠️ 检测到文件截断，正在重置...`，将 `last_position` 重置为 0，重新读取最多 `--max-initial-lines` 行，重置累积条目 |
| **文件被删除**（如 logrotate mv） | 打印 `⚠️ 文件消失，等待重新出现...`，每 1 秒轮询 `Path::exists()`，文件重新出现后重新打开，重置状态，打印 `✅ 文件已恢复，继续监听...` |
| **文件原地轮转**（mv 旧文件 + touch 新文件） | notify 通过 `IN_CREATE` 或 `IN_MOVED_FROM` + `IN_MOVED_TO` 检测到新 inode，重新打开文件，重置状态 |
| **AI API 调用失败** | 打印 `⚠️ AI 分析失败: <错误信息> — 跳过本次窗口`，保留累积条目供下个窗口使用 |
| **Ctrl+C 退出** | 打印摘要：`⏹️ 已监听 12分30秒 · 4 次分析 · 2 次告警 · 共 523 行`，正常退出 |
| **监听期间文件超过 max_initial_lines** | 无影响 — `max_initial_lines` 仅对启动阶段生效，监听期间会累积所有新行 |

---

## 实现要点

### 增量解析器复用

现有 `parse_log_file()` 一次性读取整个文件。需要新增一个更低层的入口：

```rust
// 新增于 src/parser/mod.rs
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

监听器在启动时调用一次 `detect_format()`，后续每批新行都用已确定的 format 调用 `parse_lines()`。

### notify 事件循环

使用 `notify` crate 的 `Watcher` + `mpsc` 通道：

```rust
let (tx, rx) = std::sync::mpsc::channel();
let mut watcher = notify::recommended_watcher(move |res| {
    if let Ok(event) = res { tx.send(event).ok(); }
})?;
watcher.watch(file_path, RecursiveMode::NonRecursive)?;
```

主循环：`tokio::select!` 在 `rx.recv()`（文件事件）和 `tokio::time::sleep(window)`（分析触发）之间切换。

### Tokio 兼容性

`notify::recommended_watcher` 运行在自己的线程上。通过 `std::sync::mpsc` 通道桥接到异步运行时。主监听循环是 async 函数，用 `select!` 在通道和定时器之间切换。

---

## 测试计划

| 测试 | 类型 | 验证内容 |
|------|------|----------|
| `test_watch_initial_analysis` | 集成测试 | 启动时分析已有文件内容 |
| `test_watch_detects_new_lines` | 集成测试 | 追加文件内容 → 窗口触发 → 触发分析 |
| `test_watch_no_analysis_without_new_lines` | 单元测试 | 窗口内无新数据 → 不调用 AI |
| `test_watch_truncate_reset` | 单元测试 | 文件截断 → 状态重置 |
| `test_watch_file_not_found` | 集成测试 | 文件不存在 → 报错退出 |
| `test_watch_ctrl_c_summary` | 单元测试 | 摘要格式化包含正确的计数 |
| `test_incremental_read_resumes_from_position` | 单元测试 | seek + 读取只返回新行 |
| `test_max_initial_lines` | 单元测试 | 大文件 → 启动时只分析最后 N 行 |
| `test_parse_lines_json` | 单元测试 | `parse_lines()` 对 JSON 格式正确工作 |
| `test_parse_lines_plain_text` | 单元测试 | `parse_lines()` 对纯文本格式正确工作 |

---

## 不做

- 多文件监听（`logai watch *.log`）— 独立功能，另行设计
- 桌面通知（系统托盘、推送）— 独立功能，另行设计
- 跨重启持久化状态 — v1.2 不需要
- 仅异常时触发 AI — 当前版本只做时间窗口触发
- 固定仪表盘 TUI — 当前版本只做滚动输出

---

## 依赖

| Crate | 用途 | 是否新增？ |
|-------|------|-----------|
| `notify` 6.x | 文件系统事件监听 | **新增** |
| 其余全部 | 复用现有 Cargo.toml | 已有 |

---

## 风险评估

- **性能:** 每个窗口全量重跑 `aggregate()` 是 O(n)。30 分钟监听、每分钟 500 条错误，n ≈ 15000，aggregate 耗时 <10ms。不是瓶颈。
- **内存:** 无上限累积所有条目可能持续增长。v1.3 再考虑加可配置的环形缓冲区。目前典型监控场景（数小时、中等日志量）不会超过几百 MB。
- **notify 在 Windows 上:** `notify` crate 通过 `ReadDirectoryChangesW` 支持 Windows。CI 已配置 Windows 环境下测试验证。
- **AI 成本:** 30 秒窗口，最坏情况每小时约 120 次 API 调用。DeepSeek 价格（约 ¥0.002/1M tokens），每次调用消耗约 3K tokens → 约 ¥0.006/次 → 约 ¥0.7/小时。完全可接受。
