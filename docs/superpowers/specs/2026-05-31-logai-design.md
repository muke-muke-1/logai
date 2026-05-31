# logai — AI 日志分析 CLI 工具设计文档

> 版本: 1.0  
> 日期: 2026-05-31  
> 状态: 待实施

---

## 一、产品定义

### 一句话描述

`logai` 是一个 Rust 编写的 CLI 工具，对日志文件做本地聚合分析，将压缩后的摘要发给 AI（Claude/OpenAI/DeepSeek/Ollama），在终端输出根因分析和修复建议。

### 目标用户

**MVP:** 个人开发者  
**后期:** SRE / 小团队

### 核心交互

```bash
# 事后深度分析（MVP 唯一路径）
logai analyze app.log

# 指定模型
logai analyze app.log --model deepseek

# 深度模式
logai analyze app.log --deep
```

### 输出

终端美化输出：概览 → 根因列表（带证据 + 严重程度）→ 修复建议（带代码片段）→ 成本/耗时摘要。

---

## 二、技术栈

| 层 | 选择 | 理由 |
|------|------|------|
| 语言 | Rust | 单二进制分发、零运行时依赖、内存安全、高性能 |
| CLI 框架 | clap 4.x | Rust 生态标准 |
| 异步运行时 | tokio | AI API 调用需要异步 |
| HTTP 客户端 | reqwest | 轻量、支持 TLS |
| OpenAI SDK | async-openai | 支持 OpenAI + DeepSeek（兼容格式） |
| Claude SDK | anthropic-rs 或 reqwest 直调 | Anthropic 官方 SDK 不成熟，备选直调 |
| 终端渲染 | crossterm + tabled | 跨平台颜色 + 表格 |
| 时间处理 | chrono | 日志时间戳解析 |
| 正则 | regex | 纯文本日志的模式匹配 |
| JSON 解析 | serde + serde_json | Rust 序列化标准 |

---

## 三、模块架构

```
┌──────────────────────────────────────────┐
│              CLI Layer (clap)            │
│  logai analyze <file> [flags]            │
└──────────────────┬───────────────────────┘
                   │
    ┌──────────────┼──────────────┐
    ▼              ▼              ▼
┌─────────┐  ┌─────────┐  ┌──────────┐
│ Parser  │  │Aggregator│  │AI Backend│
├─────────┤  ├─────────┤  ├──────────┤
│格式检测  │  │错误去重  │  │Claude    │
│JSON解析 │  │时间分桶  │  │OpenAI    │
│纯文本解析│  │频率分析  │  │DeepSeek  │
│堆栈提取  │  │突增检测  │  │Ollama    │
│流式读取  │  │Token预算 │  │重试/降级  │
└─────────┘  └─────────┘  └────┬─────┘
                                │
                         ┌──────▼──────┐
                         │  Renderer   │
                         ├─────────────┤
                         │彩色高亮      │
                         │表格格式化    │
                         │代码块展示    │
                         │摘要卡片      │
                         └─────────────┘
```

五个模块各自独立，通过 trait 定义接口：
- `parser` — 无依赖
- `aggregator` — 依赖 parser 的输出
- `ai` — 依赖 aggregator 的输出
- `renderer` — 依赖 ai 的输出
- `cli` — 编排以上所有

关键原则：
- **流式处理** — parser 返回 `impl Iterator`，内存恒定 O(1)
- **零外部数据库** — 聚合全在内存完成
- **单二进制分发** — 最终产物就是一个 `logai` 可执行文件

---

## 四、解析器（Parser）

### 数据结构

```rust
struct LogEntry {
    timestamp:    Option<DateTime<Utc>>,
    level:        Option<Level>,
    message:      String,
    stack_trace:  Option<String>,
    raw_line:     String,
    fields:       HashMap<String, String>,  // JSON 日志的额外字段
    line_number:  usize,
}

enum Level { Error, Warn, Info, Debug, Trace, Unknown }
enum Format { Json, PlainText }
```

### 格式自动检测

读文件前 10 行：
1. 如果 ≥80% 行能解析为 JSON → JSON 格式
2. 如果 ≥50% 行匹配已知时间戳模式 → 纯文本格式
3. 默认回退到纯文本

### JSON 日志解析

- 标准字段映射: `time/timestamp/@timestamp` → timestamp, `level/severity` → level, `message/msg` → message, `stack_trace/stack/backtrace` → stack_trace
- 未知字段全部存入 `fields: HashMap`

### 纯文本日志解析

采用**模式匹配 + 状态机**，不绑定任何特定框架：

| 提取目标 | 方法 |
|----------|------|
| 时间戳 | 匹配 20+ 种常见格式的 regex 库（ISO8601、RFC3339、syslog、"May 31 10:30:01"、"[2026-05-31 10:30:01]" 等） |
| 日志级别 | 大小写不敏感的关键词匹配: ERROR, WARN, INFO, DEBUG, FATAL, PANIC, TRACE |
| 消息体 | 取时间戳和级别之后的第一个非空文本行 |
| 堆栈跟踪 | 从第一个缩进行或 "Traceback" / "panic:" / "Exception in thread" 行开始，到下一个时间戳行结束 |

### 堆栈自动拼接

连续缩进行（以空格或 tab 开头）或 Traceback 行 → 归并到上一条 LogEntry 的 stack_trace。

### 容错

- 无法识别的行标记为 `Level::Unknown`，保留原始文本
- 单行解析失败不中断整个文件

---

## 五、聚合引擎（Aggregator）

这是 logai 最核心的模块。

### 四步流水线

#### 第 1 步：按错误签名分组（去参数化）

**签名 = 去参数化后的消息体。** 替换规则：

| 替换目标 | 正则 | 替换为 |
|----------|------|--------|
| IP 地址 | `\d{1,3}\.\d{1,3}\.\d{1,3}\.\d{1,3}` | `<IP>` |
| 端口号 | `:\d{4,5}` | `:<PORT>` |
| UUID | `[0-9a-f]{8}-[0-9a-f]{4}-...` | `<ID>` |
| 数字 | `\d+` | `<NUM>` |
| 文件路径 | `/[\w/.-]+` | `<PATH>` |
| URL | `https?://\S+` | `<URL>` |

#### 第 2 步：时间窗口分桶

- 默认 5 分钟一个窗口
- 每个错误组计算：首次出现、最后出现、频次、时间分布、趋势（上升/下降/平稳）

#### 第 3 步：异常检测（纯本地计算）

- **突增检测**: 某窗口频次 > 该组平均频次 × 3
- **新错误**: 日志时间范围内首次出现
- **周期性**: 每隔固定分钟数出现
- **静默恢复**: 错误突然消失超过 30 分钟

#### 第 4 步：Token 预算分配

- **默认预算**: 3000 tokens
- **分配策略**:
  - TOP 5 高优先级错误 → 各 400 tokens（含最多 3 条原文样本 + 1 条代表性堆栈）
  - 概览摘要（总行数、时间范围、级别分布）→ 500 tokens
  - 异常发现列表 → 500 tokens

### 输出结构

```rust
struct AnalysisSummary {
    total_lines:        usize,
    time_range:         (Option<DateTime<Utc>>, Option<DateTime<Utc>>),
    error_groups:       Vec<ErrorGroup>,
    anomalies:          Vec<Anomaly>,
    level_distribution: HashMap<Level, usize>,
}

struct ErrorGroup {
    signature:    String,
    count:        usize,
    first_seen:   Option<DateTime<Utc>>,
    last_seen:    Option<DateTime<Utc>>,
    samples:      Vec<String>,    // 最多 3 条原文样本
    stack_trace:  Option<String>, // 代表性堆栈
    trend:        Trend,
}
```

---

## 六、AI 适配器（AI Backend）

### 统一接口

```rust
#[async_trait]
pub trait AiBackend {
    async fn analyze(&self, summary: &AnalysisSummary) -> Result<AiResponse>;
    fn model_name(&self) -> &str;
    fn context_limit(&self) -> usize;
}
```

### 四后端矩阵

| 后端 | CLI 参数 | 默认模型 | --deep 模型 | 环境变量 | 成本/M tokens |
|------|----------|----------|-------------|----------|:---:|
| Claude | `--model claude` | claude-haiku-4-5 | claude-opus-4-8 | `ANTHROPIC_API_KEY` | $1 / $15 |
| OpenAI | `--model openai` | gpt-4o-mini | gpt-4o | `OPENAI_API_KEY` | $0.15 / $2.5 |
| DeepSeek | `--model deepseek` | deepseek-chat | deepseek-reasoner | `DEEPSEEK_API_KEY` | $0.27 / $1.1 |
| Ollama | `--model ollama` | llama3.2 | 用户自选 | `OLLAMA_HOST` | 免费 |

### DeepSeek 实现

DeepSeek API 兼容 OpenAI SDK 格式，只需改写 base_url 和 api_key 来源：

```rust
let client = OpenAI::new()
    .with_base_url("https://api.deepseek.com")
    .with_api_key(env::var("DEEPSEEK_API_KEY")?);
```

### 提示词结构

**固定模板 + 聚合数据填充。** AI 永远看不到原始日志，只看到聚合摘要。模板结构：

```
你是一个专业的日志分析工程师。请分析以下日志摘要：

## 概览
[总行数、时间范围、错误率、级别分布]

## TOP 5 错误
[各错误组的签名、频次、时间范围、趋势、样本、堆栈]

## 检测到的异常
[突增、新错误等]

请以 JSON 格式回复，包含 root_causes、summary、fix_suggestions。
```

### 返回结构

```rust
struct AiResponse {
    root_causes:     Vec<RootCause>,
    summary:         String,          // 一句话总结
    fix_suggestions: Vec<FixSuggestion>,
    confidence:      f32,             // 0.0-1.0
}

struct RootCause {
    description: String,
    evidence:    Vec<String>,   // 引用日志行号
    severity:    Severity,
}

struct FixSuggestion {
    action:       String,
    code_snippet: Option<String>,
    reference:    Option<String>,
}
```

### 容错策略

| 场景 | 策略 |
|------|------|
| API Key 未配置 | 友好报错，告诉用户设置哪个环境变量 |
| API 超时（30s） | 重试 1 次，仍失败提示用户换后端 |
| AI 返回非 JSON | 从文本中尽力提取 JSON 块，提取失败则原样输出 |
| Token 超限 | 聚合引擎进一步压缩（减少样本数、缩短堆栈） |
| Ollama 不可达 | 检查 localhost:11434，不通则提示启动 Ollama |

### 环境变量优先级

`--model` CLI 参数 > 环境变量自动检测。没配任何 Key 则报错退出。

---

## 七、终端渲染器（Renderer）

### 输出结构（从上到下）

```
╔══════════════════════════════════════════════╗
║          📊 logai 分析报告                    ║
║  app.log · 1,245,832 行 · 08:00→18:30 · 2.3s ║
╚══════════════════════════════════════════════╝

📋 概览
  错误率: 12.3%   警告率: 8.7%
  [彩色柱状图]

🔴 根因 1/3 — 置信度: 高
  数据库连接池耗尽
  证据: • 45,231 次错误 • 08:15 突增 12 倍 • 堆栈指向 ConnectionPool
  严重程度: 🔴 严重

🛠️ 修复建议
  1. 增大数据库连接池
     文件: config/database.toml
     pool_max_size: 10 → 50
  2. 添加连接超时重试
     文件: src/db/connection.rs:234
     retry(3, backoff::Exponential)

总耗时 2.3s | AI: Claude Haiku ($0.003) | 日志数据未上传
```

### 技术实现

- `crossterm` 做跨平台颜色和样式控制
- `tabled` 做表格格式化
- 不依赖 `less`/`more` 等分页器（保持简单）

---

## 八、CLI 接口

### 命令

```
logai analyze <FILE> [OPTIONS]
```

### 参数

| 参数 | 类型 | 默认值 | 说明 |
|------|------|--------|------|
| `FILE` | PathBuf | 必填 | 日志文件路径 |
| `--model` | enum | auto | claude / openai / deepseek / ollama / auto — 默认自动检测可用 Key，优先级: Claude > OpenAI > DeepSeek > Ollama |
| `--deep` | flag | false | 使用更强的模型做深度分析 |
| `--format` | enum | auto | json / text / auto（自动检测） |
| `--min-level` | enum | info | error / warn / info / debug — 过滤低于此级别的日志 |

### 未来扩展（V2+）

```
--report <FORMAT>    # 导出 HTML/JSON 报告
--window <MINUTES>   # 自定义时间窗口大小
--context <N>        # 保留错误行前后 N 行上下文
watch <DIR>          # 实时监控目录
```

---

## 九、MVP 范围

### 做

- [x] `logai analyze app.log` — 事后深度分析
- [x] 自动检测 JSON / 纯文本格式
- [x] 纯文本：20+ 时间戳格式 + 错误级别关键词 + 堆栈提取
- [x] JSON：标准字段映射
- [x] 错误去参数化 + 分组
- [x] 时间窗口分桶 + 趋势判断
- [x] 突增 / 新错误检测
- [x] Token 预算管理（≤3000 tokens）
- [x] 四后端：Claude / OpenAI / DeepSeek / Ollama
- [x] 终端美化输出
- [x] 容错：超时重试、JSON 解析降级、Key 未配友好提示

### 不做（V2）

- [ ] 实时监控模式 (`logai watch`)
- [ ] HTML/JSON 报告导出
- [ ] 多日志源关联分析
- [ ] 配置文件 (`logai.toml`)
- [ ] 自定义格式解析规则
- [ ] syslog/journald 系统日志格式
- [ ] 交互式 TUI

---

## 十、项目结构

```
logai/
├── Cargo.toml
├── README.md
├── src/
│   ├── main.rs            # CLI 入口
│   ├── cli.rs              # clap 参数定义 + 流水线编排
│   ├── parser/
│   │   ├── mod.rs          # 公开接口 + 格式检测
│   │   ├── json.rs         # JSON 日志解析
│   │   ├── plain_text.rs   # 纯文本日志解析
│   │   └── timestamp.rs    # 时间戳检测库（20+ 格式）
│   ├── aggregator/
│   │   ├── mod.rs          # 聚合流水线入口
│   │   ├── signature.rs    # 错误签名 + 去参数化
│   │   ├── bucketer.rs     # 时间窗口分桶
│   │   ├── anomaly.rs      # 异常检测
│   │   └── token_budget.rs # Token 预算管理
│   ├── ai/
│   │   ├── mod.rs          # AiBackend trait + 工厂方法
│   │   ├── claude.rs       # Claude 后端
│   │   ├── openai.rs       # OpenAI 后端
│   │   ├── deepseek.rs     # DeepSeek 后端
│   │   ├── ollama.rs       # Ollama 后端
│   │   └── prompt.rs       # 提示词模板
│   └── renderer.rs         # 终端美化输出
└── tests/
    ├── fixtures/           # 测试用日志文件
    │   ├── json_error.log
    │   ├── plain_text_apache.log
    │   └── plain_text_python.log
    ├── parser_tests.rs
    ├── aggregator_tests.rs
    └── integration_tests.rs
```

---

## 十一、测试策略

- **parser**: 单元测试 — 每种日志格式一个 fixture 文件，验证解析准确性
- **aggregator**: 单元测试 — 构造已知 LogEntry 流，验证聚合结果
- **ai**: 集成测试 — 用 Ollama 本地跑（不花钱），验证提示词和响应解析
- **renderer**: 快照测试 — 固定 AiResponse，验证终端输出文本
- **CLI**: 端到端测试 — `cargo run -- analyze tests/fixtures/json_error.log`

---

## 十二、发布策略

1. **GitHub Release** — 编译 Linux/macOS/Windows 三平台二进制，CI 自动发布
2. **crates.io** — `cargo install logai`
3. **Homebrew** — 提交到 homebrew-core（达到一定 stars 后）
4. **README** — 带 Demo GIF、对比表格、一行安装命令

---

## 十三、竞品对比（README 核心内容）

| | logai | 手动 grep | ChatGPT 粘贴 | lnav |
|------|:---:|:---:|:---:|:---:|
| 自动分析 | ✅ | ❌ | ❌ | ❌ |
| 隐私（数据不出机） | ✅ | ✅ | ❌ | ✅ |
| 多模型支持 | ✅ | — | ❌ | — |
| 零配置 | ✅ | ✅ | ❌ | ❌ |
| 单二进制 | ✅ | ✅ | — | ❌ |
| 价格 | ~$0.003/次 | 免费 | $20/月 | 免费 |
