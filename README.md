<div align="center">

# 🧹 TrueClean

**跨平台磁盘清理 + AI Agent 桌面应用 · Cross-platform disk cleaner with a built-in AI agent**

类 CleanMyMac：扫描磁盘占用 → 可视化分类占比 → 识别系统/缓存数据 → 让 AI 助手分析并（在你确认后）安全清理。

![status](https://img.shields.io/badge/status-开发中%20WIP-orange) ![platform](https://img.shields.io/badge/platform-macOS%20%7C%20Windows%20%7C%20Linux-blue) ![stack](https://img.shields.io/badge/stack-Tauri%202%20%2B%20React%2018-informational) ![license](https://img.shields.io/badge/license-MIT-green)

</div>

---

> ## 🚧 项目状态：开发中，尚未完成（Work In Progress）
>
> **这是一个正在积极开发的早期项目，功能尚未全部完成，不建议在生产环境/重要数据上使用。**
>
> 当前为**首个可编译运行的版本**：前后端均能编译、核心扫描内核带单元测试。但很多模块仍是「可用基线」，跨平台（尤其 Windows / Linux）的清理路径、应用卸载、启动项管理仍需打磨；UI 交互、真机点测、错误兜底也在持续完善中。
>
> ⚠️ 涉及删除文件的操作请谨慎：默认走回收站（可恢复），但请务必先理解再清理。欢迎试用、提 Issue、贡献代码。
>
> _This is an early, actively-developed project. It compiles and runs, but it is **not finished** — many modules are baseline implementations and cross-platform coverage (especially Windows/Linux) still needs work. Don't trust it with important data yet._

---

## 📖 这是什么

TrueClean 想解决一个常见痛点：**硬盘满了，但你不知道空间被什么吃掉了，也不敢乱删。**

它做三件事：

1. **看清楚** —— 扫描整块硬盘或某个目录，把占用按类别（系统 / 应用 / 开发文件 / 媒体 / 缓存 / 日志 / 文档 / 下载 / 压缩包等）拆开，用矩形树图和旭日图直观展示，还能逐层下钻到具体文件夹。
2. **清得安全** —— 自动识别真正的「垃圾」（各类缓存、日志、临时文件、浏览器缓存、开发缓存、回收站），区分「绝对安全可删」和「需你确认」，删除默认进回收站。
3. **让 AI 帮你判断** —— 内置一个 AI 助手面板。它能**真正调用上面的扫描/分析能力**（不是空谈），帮你看「哪些能安全清理、哪些是缓存、哪些大文件可以归档」，给出预计释放空间和风险等级，并在你确认后执行清理。

---

## ✨ 功能详解

### 🗂️ 概览 Overview
- 显示每个磁盘卷的**使用率环形图**、总容量 / 已用 / 可用空间。
- 一键开始扫描的醒目入口，以及各功能的引导卡片。

### 🔍 磁盘扫描与可视化 Scan & Visualize
- 选择整块磁盘或任意目录，进行**并行递归扫描**（Rust 内核，处理大量文件依然快）。
- **分类占比**：把所有内容按 11 个类别（系统 / 应用程序 / 开发文件 / 文档 / 媒体 / 缓存 / 日志 / 废纸篓 / 下载 / 压缩包 / 其他）统计占用与文件数。
- **两种可视化**：矩形树图（Treemap）和旭日图（Sunburst），按类别配色，可悬停看详情、点击下钻。
- **文件树下钻**：从根目录一层层进入最大的子目录，带面包屑返回。
- 扫描过程有**实时进度**（已扫描文件数 / 字节数 / 当前路径），可随时取消。

### 🧽 系统垃圾清理 System Junk
分组扫描并清理：
- 用户缓存 / 系统缓存 / 应用缓存
- 日志文件、临时文件
- 浏览器缓存（Chrome / Safari / Firefox 等）
- 开发缓存（Xcode DerivedData 等）、语言包缓存（npm / cargo / pip / gradle 等）
- 回收站

每组显示体积、是否「推荐清理」，可组级 / 项级勾选，底部汇总将释放的空间。

### 📦 大文件查找 Large & Old Files
- 按「最小大小」+「至少多少天未修改」筛出体积大且陈旧的文件，按大小排序，可勾选删除。

### 👯 重复文件 Duplicate Finder
- 基于 **blake3 内容哈希**精确去重（先按大小分桶再哈希，速度快且不误判），按组展示，每组保留 1 个、其余可删，显示**可回收空间**。

### 🗑️ 应用卸载 Uninstaller
- 列出已安装应用（名称 / 版本 / 体积 / 最近使用时间）。
- 卸载时连带清理**残留文件**（缓存、偏好设置、Application Support、日志等），而不是只删主程序。

### ⚡ 启动项管理 Startup Items
- 列出开机自启动项（登录项 / 启动代理 / 服务等）及启用状态，可一键启停，加快开机。

### 🤖 AI 助手 AI Agent（核心亮点）
- 右侧抽屉式对话面板，内置**强力中文系统提示词**。
- **真正会用工具**：Agent 可调用 `列出磁盘 / 扫描目录 / 扫系统垃圾 / 找大文件 / 找重复 / 列应用 / 列启动项 / 清理路径 / 清空回收站` 等能力，基于**真实扫描数据**给建议，绝不编造路径和体积。
- 输出结构化建议：按「**可立即清理 / 建议复核 / 不要动**」分组，给出每项的预计释放空间与风险等级。
- **多模型 Provider 可选**：Claude（默认）/ OpenAI / 本地 Ollama，在设置里切换。
- 支持**流式输出**与工具调用过程可视化（能看到 Agent 调了哪个工具、参数和结果）。

---

## 🔒 安全设计

- **删除默认走回收站**（可恢复）；永久删除需显式选择。
- 所有破坏性操作（清理 / 卸载 / 清空回收站）在 UI 上都需**二次确认**。
- AI 提示词设有**安全红线**：绝不建议删除系统关键路径（`/System`、`/usr`、Windows、Program Files 系统部分等）。
- 明确区分「**绝对安全可删**（缓存 / 日志 / 临时 / 回收站）」与「**需用户确认**（文档 / 媒体 / 大文件 / 应用）」。
- **API Key 仅保存在你本地的配置文件中**，应用本身不内置、不上传任何密钥。

---

## 🛠️ 技术栈

| 层 | 选型 |
|---|---|
| 桌面框架 | [Tauri 2](https://tauri.app)（体积小 ~10MB、原生性能、安全） |
| 后端 | Rust（并行扫描内核：rayon / blake3 / sysinfo / trash / walkdir） |
| 前端 | React 18 + TypeScript + Vite 6 |
| 状态管理 | zustand |
| 可视化 | d3-hierarchy（Treemap / Sunburst）+ d3-shape |
| AI | 多 Provider 适配（Claude / OpenAI / Ollama）+ 工具调用 + 流式 |
| 平台 | macOS / Windows / Linux |

---

## 🚀 开发与运行

前置：Rust（stable）、Node ≥ 18、[pnpm](https://pnpm.io)。Linux 还需 Tauri 的[系统依赖](https://tauri.app/start/prerequisites/)（webkit2gtk 等）。

```bash
pnpm install            # 安装前端依赖
pnpm tauri dev          # 开发模式（启动 Vite + 弹出 Tauri 窗口，首次编译约 1-2 分钟）
```

构建发布安装包：

```bash
pnpm tauri build        # 产出对应平台的安装包
```

仅做验证（不弹窗）：

```bash
pnpm build                         # 前端类型检查 + 打包
cd src-tauri && cargo check        # 后端编译检查
cd src-tauri && cargo test --lib   # 后端单元测试
```

### 配置 AI

应用内打开「设置」：
- **Provider**：`claude`（默认）/ `openai` / `ollama`
- **Model**：如 `claude-sonnet-4-6`、`gpt-4o`、`llama3.1` 等
- 填入对应 **API Key**（Claude / OpenAI）或 **Ollama 地址**（默认 `http://localhost:11434`）

---

## 🏗️ 项目结构

```
src/                       前端（React + TS）
├── lib/        types.ts(数据真源) · ipc.ts(命令封装) · format.ts
├── store/      scanStore · agentStore · settingsStore (zustand)
├── components/ layout · scan(可视化) · cleanup · agent · ui · settings
└── styles/     tokens.css(设计令牌, 深浅双主题) · global.css

src-tauri/src/             后端（Rust + Tauri）
├── model.rs    全部 IPC 数据结构（与 types.ts 一一对应）
├── scanner/    walker · tree · categories · engine（并行扫描内核）
├── cleaning/   paths(平台路径表) · junk · large_old · trash · duplicates · uninstaller · startup
├── agent/      prompt · tools(工具调用) · runner(对话循环) · providers/(claude/openai/ollama)
├── commands/   scan · cleanup · system · agent · settings（Tauri 命令）
└── state.rs    全局状态（设置 / 取消标志 / 上次扫描缓存）
```

数据契约见 [`docs/CONTRACT.md`](docs/CONTRACT.md)：Rust `model.rs` 与 TS `types.ts` 为单一真源，改动需同步两侧。

---

## 🗺️ Roadmap / 完成度

**已完成（首版）**
- [x] 跨平台项目骨架（Tauri 2 + React + TS），前后端均可编译
- [x] 并行磁盘扫描内核 + 分类 + 占比统计（含单元测试）
- [x] Treemap / Sunburst / 分类条 / 文件树可视化
- [x] 系统垃圾、大文件、重复文件、应用卸载、启动项的后端实现与面板
- [x] AI Agent：多 Provider + 工具调用 + 流式 + 强力提示词
- [x] 设置（Provider / Model / Key / 回收站默认）

**进行中 / 待完善**
- [ ] Windows / Linux 的清理路径表、卸载残留、启动项管理打磨
- [ ] 真机端到端点测与 UI 细节、空态 / 错误态打磨
- [ ] 扫描结果缓存与增量扫描、超大目录性能优化
- [ ] 清理前的安全快照 / 撤销
- [ ] 国际化（中 / 英）完善
- [ ] 应用图标与品牌视觉、打包签名

---

## 🤝 贡献

项目处于早期，欢迎 Issue 和 PR。请勿提交任何密钥；删除相关逻辑改动请格外谨慎并补充测试。

## 📄 License

MIT
