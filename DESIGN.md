# logai 设计系统

> 版本: 1.0 · 2026-06-02 · 覆盖 v0.3.x

## 架构概览

```
CLI (clap) ──→ parse ──→ aggregate ──→ AI backend ──→ render
                 │            │              │              │
                 │            │              │              ├── Terminal (renderer.rs)
                 │            │              │              ├── HTML (renderer_html.rs)
                 │            │              │              └── TUI (tui.rs)
                 │            │              │
                 ▼            ▼              ▼
            parser/     aggregator/      ai/
```

## 颜色令牌 (Color Tokens)

所有视觉输出（终端 / TUI / HTML）共享同一套语义颜色令牌。每个令牌在暗色和亮色主题下有不同色值。

| 令牌 | 语义 | TUI (暗色) | TUI (亮色) | HTML 暗色 | HTML 亮色 |
|------|------|-----------|-----------|----------|----------|
| `bg` | 页面/画布背景 | `Black` | `White` | `#1a1a2e` | `#fafafa` |
| `fg` | 正文文字 | `White` | `Black` | `#e0e0e0` | `#1a1a2e` |
| `highlight` | 交互高亮 / 链接 | `Cyan` | `Blue` | `#00bcd4` | `#1565c0` |
| `error` | 错误 / 根因强调 | `Red` | `Red` | `#e94560` | `#c62828` |
| `warn` | 警告 / 异常标记 | `Yellow` | `DarkGray` | `#f0a500` | `#e65100` |
| `info` | 成功 / 趋势 | `Green` | `Green` | `#16c79a` | `#2e7d32` |
| `selected` | 选中/卡片背景 | `DarkGray` | `LightCyan` | `#16213e` | `#e8eaf6` |
| `border` | 边框 / 分隔线 | `DarkGray` | `Gray` | `#0f3460` | `#c5cae9` |

### 使用约定

- **error** 仅用于根因描述、ERROR 级别标签，不可用于信息性文字
- **warn** 用于异常标记（⚠）和 WARN 级别，亮色主题下须确保对比度 ≥ 4.5:1
- **highlight** 用于按键提示、可交互元素
- **border** 仅用于边框和分隔线，不可用于正文

## 字体

| 用途 | 字体栈 | 回退 |
|------|--------|------|
| HTML 正文 | `Inter` → `Noto Sans SC` | `-apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif` |
| HTML 等宽/代码 | `JetBrains Mono` → `Cascadia Code` | `'Fira Code', 'Consolas', monospace` |
| TUI | 终端默认等宽字体 | — |

**加载策略**: Google Fonts CDN，`display=swap` 确保无闪烁。

## 间距

- TUI: 由终端字符单元格决定，通过 `Rect` 计算
- HTML: 
  - 页面 `max-width: 960px`, `padding: 24px`（移动端 12px）
  - 卡片间距 `gap: 24px`（移动端 12px）
  - 卡片内边距 `padding: 16px`
  - 圆角统一 `border-radius: 8px`

## 命名约定

### 代码

- **Rust 标识符**: `snake_case`，英文（API 契约）
- **用户可见字符串**: 简体中文（状态栏、帮助、错误提示）
- **CLI 标志**: 英文 kebab-case (`--max-initial-lines`)

### 文件

| 文件 | 职责 |
|------|------|
| `src/main.rs` | 入口 + 模块声明 |
| `src/cli.rs` | clap CLI 定义 + 命令分发 |
| `src/types.rs` | 公共数据类型 |
| `src/parser/mod.rs` | 解析入口 + 格式检测 |
| `src/aggregator/mod.rs` | 聚合管道 |
| `src/ai/mod.rs` | AI 后端 trait + 重试 |
| `src/renderer.rs` | 终端输出 |
| `src/renderer_html.rs` | HTML 报告生成 |
| `src/tui.rs` | 交互式 TUI |
| `src/watcher.rs` | `logai watch` 实时监听 |
| `DESIGN.md` | 本文件 |

## 主题切换

### TUI

- 按键 `t` 切换暗色 ↔ 亮色
- `Theme::Dark` / `Theme::Light` 枚举
- 通过 `ThemeColors` 结构体注入所有渲染函数

### HTML

- `data-theme="dark"` / `data-theme="light"` 属性挂载在 `<html>` 元素
- CSS 自定义属性响应 `[data-theme]` 选择器
- JS `toggleTheme()` 切换属性 + 刷新 Chart.js 图表颜色
- 按钮固定在右上角 `position: fixed`

## 兼容性

- TUI: Windows Terminal / iTerm2 / kitty / gnome-terminal（需要 truecolor 支持）
- HTML: Chrome 90+ / Firefox 90+ / Safari 15+ / Edge 90+
- 移动端: iOS Safari 15+ / Android Chrome 90+
