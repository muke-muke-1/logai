# logai 🪵🔍

> AI reads your logs so you don't have to. It's like hiring a senior engineer who works for $0.003 per session and never complains about meetings.
>
> AI 帮你读日志。就像一个每次只收 2 分钱、还从不抱怨开会的高级工程师。

<p align="center">
  <img src="https://img.shields.io/badge/language-Rust-orange?style=flat-square" alt="Rust">
  <img src="https://img.shields.io/badge/price-~$0.003%2Fanalysis-green?style=flat-square" alt="cheap">
  <img src="https://img.shields.io/badge/models-Claude%20%7C%20OpenAI%20%7C%20DeepSeek%20%7C%20Ollama-blue?style=flat-square" alt="multi-model">
  <img src="https://img.shields.io/badge/config-zero-brightgreen?style=flat-square" alt="zero config">
  <img src="https://img.shields.io/badge/privacy-your%20logs%20stay%20home-red?style=flat-square" alt="privacy first">
</p>

---

## 🤔 Why? / 为什么？

You've been there. It's 2 AM. Production is down. You're staring at 500MB of logs, `grep`-ing like a caveman, copying chunks into ChatGPT, praying it doesn't hallucinate a fix that deletes the database.

**Stop doing that.**

你经历过。凌晨两点，线上挂了。你盯着 500MB 的日志，像原始人一样 `grep`，复制粘贴到 ChatGPT，祈祷它不会幻觉出一个删库跑路的方案。

**别再这样了。**

---

## ✨ What logai Does / 做什么

1. You point it at a log file
2. It parses, deduplicates, and compresses the chaos into a compact summary
3. AI analyzes the summary (your raw logs **never** leave your machine)
4. You get root cause analysis + fix suggestions with actual code snippets — in your terminal

---

1. 指向一个日志文件
2. 自动解析、去重、压缩成紧凑摘要
3. AI 只分析摘要（原始日志**绝不**离开你的机器）
4. 终端里直接输出根因分析 + 带代码片段的修复建议

```
╔══════════════════════════════════════════════════════╗
║          📊 logai 分析报告                            ║
║  10 行 · 08:03:12 → 18:29:46 · 4.3s                   ║
╚══════════════════════════════════════════════════════╝
┌─ 📋 概览 ────────────────────────────────────────┐
  错误率: 80.0%    警告率: 10.0%
  ERROR  ████████████████████████████████████ 80.0%
└──────────────────────────────────────────────────┘

┌─ 🔴 根因 1/2 ──────────────────────────────────┐
  数据库连接池耗尽
  证据: • 5 次连接超时 • 持续 10 小时 • 堆栈指向 ConnectionPool
  严重程度: 🔴 严重
└──────────────────────────────────────────────────┘

┌─ 🛠️ 修复建议 ──────────────────────────────────┐
  1. 增大数据库连接池
     pool_max_size: 10 → 50
  2. 更新 SSL 证书
     sudo certbot renew
└──────────────────────────────────────────────────┘
总耗时 4.3s | AI: DeepSeek | 日志数据未上传
```

---

## 🚀 Quick Start / 快速开始

### Install / 安装

```bash
cargo install logai
```

### Use / 使用

```bash
# Set one API key — pick your fighter / 选一个
export DEEPSEEK_API_KEY="sk-..."   # 💰 cheapest / 最便宜
export ANTHROPIC_API_KEY="sk-ant-..."  # 🧠 smartest / 最聪明
export OPENAI_API_KEY="sk-..."     # 🏢 enterprise vibes / 企业范

# Analyze / 分析
logai analyze app.log

# Multi-source / 多源关联 (cross-file correlation)
logai analyze app.log db.log nginx.log

# Choose model / 指定模型
logai analyze app.log --model deepseek

# Go deep / 深度模式 (uses stronger model / 用更强的模型)
logai analyze app.log --deep

# Analyze then browse in TUI / 分析后进入交互式浏览器
logai analyze app.log --tui

# TUI with multi-source / 多源 TUI (Tab 键切换来源)
logai analyze app.log db.log --tui

# Watch mode / 实时监控 (periodic AI analysis as logs grow)
logai watch app.log --window 30

# Interactive TUI / 交互式终端 (browse errors, search, ask AI per-error)
logai interactive app.log --live

# Export HTML report / 导出 HTML 报告（含 Chart.js 交互图表 + 亮/暗主题）
logai analyze app.log --output report.html

# Custom parse rules / 自定义解析规则
logai analyze app.log --parse-timestamp-format "%Y-%m-%d %H:%M:%S" --parse-level-field severity
# Or via config file / 或者用配置文件
logai analyze app.log --rules-file logai.toml
```

That's it. No config file. No YAML. No "please install these 47 dependencies first."

就这些。没有配置文件。没有 YAML。没有"请先安装这 47 个依赖"。

---

## 🎯 Features / 功能

| Feature | Why You Care / 为什么重要 |
|---------|--------------------------|
| 🔍 **Auto-detect format** / 自动检测格式 | JSON? Plain text? Python traceback? Apache? Don't care. It just works. |
| 🧹 **Smart dedup** / 智能去重 | 10,000 identical errors → 1 group. AI won't get bored reading repeats. |
| 🔒 **Privacy-first** / 隐私优先 | Your raw logs stay on your machine. AI only sees aggregated stats. No PII leakage. |
| 🤖 **4 AI backends** / 四个 AI 后端 | Claude, OpenAI, DeepSeek, Ollama (free!). Mix and match. |
| 📊 **Anomaly detection** / 异常检测 | Spikes, new errors, periodic patterns — caught before AI even looks. |
| 🔗 **Multi-source correlation** / 多源关联 | Analyze multiple log files together. Cross-source causal chains. Tab between sources in TUI. |
| 🎨 **Pretty terminal output** / 终端美化 | Color-coded. Tables. Code snippets. Auto-adapts to narrow terminals (< 80 cols). |
| 🖥️ **Interactive TUI** / 交互式终端 | Vim keys, live refresh, search/filter, dark/light theme. Press `a` to ask AI about any error. |
| 📈 **HTML reports** / HTML 报告 | Self-contained HTML with Chart.js + light/dark theme toggle. Google Fonts. Mobile responsive. |
| 👁️ **Watch mode** / 实时监听 | Point at a log file, get periodic AI analysis as new lines arrive. Ctrl+C to stop. |
| 🛠️ **Custom parse rules** / 自定义解析 | Override timestamp format, level field, message field via CLI flags or `logai.toml`. |
| 🔄 **AI retry** / AI 重试 | 3x exponential backoff (1s→2s→4s). No more one-shot API failures. |
| ⚡ **One binary** / 单二进制 | No runtime. No Docker. No Python venv hell. Just one file. |
| 🆓 **Zero config** / 零配置 | If you can type `logai analyze`, you're already using it. |

---

## 📊 vs The Competition / 跟竞品比

| | logai | `grep \| sort \| uniq -c` | Pasting into ChatGPT | Your colleague at 3 AM |
|---|:---:|:---:|:---:|:---:|
| Automatic analysis / 自动分析 | ✅ | ❌ | 🤷 | 😴 |
| Privacy / 隐私 | ✅ | ✅ | ❌ | ❌ |
| Multi-model / 多模型 | ✅ | — | ❌ | "I only know Python" |
| Zero config / 零配置 | ✅ | ✅ | ❌ | ❌ |
| Won't judge your code / 不吐槽你的代码 | ✅ | ✅ | ✅ | ❌ |
| Cost per analysis / 单次成本 | ~$0.003 | Free / 免费 | $20/mo | Pizza + beer / 披萨+啤酒 |
| Works at 2 AM / 凌晨两点能用 | ✅ | ✅ | ✅ | 📵 "I'm sleeping" |

---

## 🤖 Supported AI Backends / 支持的 AI 后端

| Backend | `--model` | Default Model | Cost / 1M tokens | Best For |
|---------|-----------|---------------|:---:|----------|
| **DeepSeek** | `deepseek` | deepseek-chat | $0.27 | 💰 Best value / 性价比之王 |
| **Claude** | `claude` | claude-haiku-4-5 | $1.00 | 🧠 Best analysis quality |
| **OpenAI** | `openai` | gpt-4o-mini | $0.15 | 🏢 Enterprise default |
| **Ollama** | `ollama` | llama3.2 | FREE! 🤯 | 🔒 100% local / 完全本地 |

Auto-detection priority / 自动检测优先级: Claude > OpenAI > DeepSeek > Ollama

> **Pro tip / 小贴士:** DeepSeek gives you ~80% of Claude's quality at ~25% of the price. For most bugs, it's all you need.
>
> DeepSeek 用 Claude 四分之一的价格，提供八成功力。大多数 bug 用它就够了。

---

## 📖 How It Works / 原理

```
your-huge-log-file.log (500 MB of suffering)
        │
        ▼
┌─────────────────────────┐
│  Parser / 解析器         │  ← Rust, streaming, auto-detects format
│  "Is this JSON? Python?  │
│   Apache? Don't care."   │
└───────────┬─────────────┘
            ▼
┌─────────────────────────┐
│  Aggregator / 聚合引擎   │  ← The secret sauce / 核心秘方
│  • 10K errors → 5 groups│
│  • IPs → <IP>           │
│  • UUIDs → <ID>         │
│  • Spikes detected       │
│  • Token budget: 3000    │
└───────────┬─────────────┘
            ▼
┌─────────────────────────┐
│  AI Analysis / AI 分析   │  ← AI only sees summary, NOT raw logs
│  "Here's a 2KB summary.  │     AI 只看摘要，看不到原始日志
│   What's wrong?"         │
└───────────┬─────────────┘
            ▼
┌─────────────────────────┐
│  Terminal Output / 终端  │  ← Pretty colors. Code snippets.
│  "Fix this: ..."         │     Actually useful at 2 AM.
└─────────────────────────┘
```

The magic / 魔法: Your logs **never leave your machine**. AI only receives aggregated statistics — error counts, patterns, stack traces. No IPs. No user data. No secrets.

---

## 🧪 Supported Log Formats / 支持的日志格式

| Format | Examples |
|--------|----------|
| **JSON** / 结构化 | `{"time":"...","level":"error","msg":"..."}` |
| **Python** | `[2026-05-31 08:03:12] ERROR - db/conn.py:234\nTraceback...` |
| **Go** | `2026/05/31 08:03:12 ERROR: connection refused` |
| **Apache/Nginx** | `192.168.1.1 - - [31/May/2026:08:03:12 +0000] "GET / HTTP/1.1" 500` |
| **Syslog** | `May 31 08:03:12 server kernel: ERROR: out of memory` |
| **ISO8601** / RFC3339 | `2026-05-31T08:03:12Z ERROR something broke` |
| **Anything with timestamps** | If it has a date and the word "error", logai probably handles it. |

> Basically: if a human can tell it's a log, logai can too.
>
> 简单说：只要人类看得出这是日志，logai 就能解析。

---

## 🔧 Installation / 安装

### Option 1: Cargo (recommended / 推荐)

```bash
cargo install logai
```

### Option 2: Homebrew

```bash
brew tap muke-muke-1/logai
brew install logai
```

### Option 3: Pre-built binaries / 预编译

Download from [GitHub Releases](https://github.com/muke-muke-1/logai/releases).  
One file. Drop it in your `$PATH`. Done.

### Option 4: Build from source / 源码编译

```bash
git clone https://github.com/muke-muke-1/logai.git
cd logai
cargo build --release
# Binary at ./target/release/logai
```

---

## 🏗️ Development / 开发

```bash
# Run tests / 跑测试
cargo test

# Run with test fixtures / 用测试数据跑
export DEEPSEEK_API_KEY="sk-..."
cargo run -- analyze tests/fixtures/json_error.log --model deepseek
```

101 tests. 0 failures. Zero is a nice number.

---

## 🗺️ Roadmap / 路线图

- [x] Parse JSON + plain text logs / 解析 JSON + 纯文本
- [x] Smart aggregation + dedup / 智能聚合 + 去重
- [x] 4 AI backends / 四个 AI 后端
- [x] Pretty terminal output / 终端美化输出
- [x] `logai watch` — real-time monitoring / 实时监控
- [x] HTML report export / HTML 报告导出（含 Chart.js 交互图表）
- [x] Interactive TUI / 交互式终端界面（含 AI 对话面板）
- [x] Multi-source correlation / 多日志源关联分析
- [x] Custom parsing rules / 自定义解析规则
- [x] HTML dark/light theme + Google Fonts / HTML 亮暗主题
- [x] Terminal width responsive / 终端宽度自适应
- [x] AI retry with exponential backoff / AI 指数退避重试
- [x] crates.io 发布 + Homebrew formula
- [ ] CI/CD integration (`logai ci`) / CI/CD 集成
- [ ] Diff analysis (`logai diff`) / 变更差异分析
- [ ] JSON output (`--json`) / JSON 格式输出

---

## 🤝 Contributing / 贡献

Found a bug? Have an idea? PRs welcome.  

Just don't submit a PR that makes the README boring. The README has feelings too.

发现 bug？有想法？欢迎 PR。  
别把 README 改无聊了。README 也是有感情的。

---

## 📜 License / 许可证

MIT — Do whatever you want. Just don't blame us when the AI tells you to `rm -rf /`.

MIT — 随便用。AI 叫你 `rm -rf /` 的时候别怪我们就行。

---

<p align="center">
  <b>⭐ If logai saved your 2 AM debugging session, give it a star!</b>
  <br>
  <b>如果 logai 救了你凌晨两点的命，给它一颗星 ⭐</b>
</p>
