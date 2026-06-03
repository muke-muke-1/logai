# 升级指南 / Upgrade Guide

## v0.3.0 → v0.4.0

### 破坏性变更

无。v0.4.0 向后兼容 v0.3.0。

### 新增功能

- `logai <file>` — 不指定子命令时默认打开 TUI（原来需要 `logai interactive <file>`）
- `logai analyze --dry-run` — 仅解析聚合，不调用 AI（无需 API key）
- `logai analyze --sample` — 使用内嵌示例日志演示（无需文件或 API key）
- `logai init` — 在当前目录生成 `logai.toml` 配置模板
- `logai completion <shell>` — 生成 shell 补全脚本
- 结构化错误码 (E001-E006) — 每个错误附带修复提示

### 行为变更

- `logai`（无参数）打印帮助，不再报错
- API key 缺失错误现在检测已设置的后端并建议缺失的
- `--help` 文本统一为中文

### 迁移步骤

1. 升级: `cargo install logai --force`
2. （可选）生成配置模板: `logai init`
3. （可选）安装 shell 补全:
   ```bash
   # bash
   source <(logai completion bash)
   # zsh
   source <(logai completion zsh)
   # fish
   logai completion fish | source
   ```

## v0.2.1 → v0.3.0

### 破坏性变更

- `logai analyze` 的文件参数现在支持多个文件: `logai analyze app.log db.log`
  - 单文件用法不变，完全兼容
- help 文本统一为中文

### 新增功能

- 多源关联分析: 同时分析多个日志文件，检测跨源因果链
- `analyze --tui`: 分析后自动打开交互式浏览器
- HTML 亮/暗主题切换，Google Fonts
- 自定义解析规则: `--parse-*` CLI 标志 + `logai.toml` 配置文件
- 终端宽度自适应，窄屏简化输出

### 迁移步骤

1. 升级: `cargo install logai --force`
2. 检查自动化脚本中 `logai analyze` 的硬编码文件名
3. （可选）创建 `logai.toml` 自定义解析规则
