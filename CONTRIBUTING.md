# 贡献指南 / Contributing

logai 欢迎贡献！无论是修复 bug、改进文档、添加功能、还是报告问题。

## 开发环境

```bash
# 前提条件: Rust 1.80+
rustup default stable
rustup update

# 克隆仓库
git clone https://github.com/muke-muke-1/logai.git
cd logai

# 构建
cargo build

# 运行测试
cargo test

# 代码检查
cargo fmt -- --check
cargo clippy -- -D warnings
```

## 项目结构

```
src/
├── main.rs              # 入口 + 模块声明
├── cli.rs               # clap CLI 定义 + 命令分发
├── types.rs             # 公共数据类型
├── errors.rs            # 结构化错误类型 (E001-E006)
├── parser/
│   ├── mod.rs           # 解析入口 + 格式检测 + 配置加载
│   ├── json.rs          # JSON 日志解析
│   ├── plain_text.rs    # 纯文本日志解析
│   └── timestamp.rs     # 时间戳检测与解析
├── aggregator/
│   ├── mod.rs           # 聚合管道 + 跨源关联
│   ├── signature.rs     # 错误签名去重
│   ├── bucketer.rs      # 时间窗口分桶
│   ├── anomaly.rs       # 异常检测
│   └── token_budget.rs  # AI token 预算控制
├── ai/
│   ├── mod.rs           # AI 后端 trait + 重试逻辑
│   ├── claude.rs        # Claude 后端
│   ├── openai.rs        # OpenAI 后端
│   ├── deepseek.rs      # DeepSeek 后端
│   ├── ollama.rs        # Ollama 后端
│   └── prompt.rs        # AI prompt 构建
├── renderer.rs          # 终端输出
├── renderer_html.rs     # HTML 报告生成
├── tui.rs               # 交互式 TUI
└── watcher.rs           # 实时文件监听
```

## 提交流程

1. Fork 仓库，创建 feature 分支: `git checkout -b feature/my-feature`
2. 编写代码，确保:
   - `cargo test` 全部通过
   - `cargo fmt` 格式化通过
   - `cargo clippy -- -D warnings` 无警告
3. 提交: `git commit -m "feat: description"`
4. 推送并创建 PR

## Commit 约定

- `feat:` 新功能
- `fix:` 修复
- `docs:` 文档
- `chore:` 构建/配置
- `style:` 格式化
- `refactor:` 重构

## 测试

```bash
# 全部测试
cargo test

# 特定模块
cargo test --test integration_tests
cargo test --test html_tests

# 带测试日志
cargo test -- --nocapture
```

## 问题反馈

- 使用 GitHub Issues 报告 bug 或提议功能
- 附带最小复现步骤和日志样例
- 安全漏洞请私下报告，勿公开 issue
