# TrueClean — 产品需求文档（PRD）

> 版本：0.1.0 · 状态：可立项基线 · 最后更新：2026-06-18
> 配套文档：[ARCHITECTURE.md](ARCHITECTURE.md) · [SECURITY.md](SECURITY.md) · [ROADMAP.md](ROADMAP.md) · [USER_GUIDE.md](USER_GUIDE.md)

---

## 1. 问题陈述（Problem）

硬盘空间永远不够用，而"满了之后怎么办"这件事对绝大多数用户来说是个黑盒：

1. **不知道空间被谁吃掉** —— 系统自带的存储管理工具粒度粗、可视化弱，用户看到"系统 80GB"却不知道里面是什么。
2. **不敢删** —— 缓存、日志、开发产物、重复文件散落各处，普通用户分不清"安全可删"和"删了会出事"。删错系统文件可能导致无法开机。
3. **清理工具要么贵要么蠢** —— 商业产品（CleanMyMac）年订阅 ¥300+ 且闭源；免费产品（BleachBit）界面陈旧、无 AI 辅助、跨平台体验割裂；可视化工具（DaisyDisk）只看不能清。
4. **AI 助手"空谈"** —— 市面上的 AI 桌面助手多数只能聊天，不能真正读取你的磁盘数据、不能基于真实扫描结果给建议，更不敢让它执行清理。

**一句话**：用户需要一款「看得清 + 删得安全 + AI 真帮忙」的跨平台磁盘清理工具。

---

## 2. 目标用户（Target Users）

| 用户画像 | 痛点 | 期望 |
|---|---|---|
| **开发者** | Xcode DerivedData、node_modules、cargo/pip/gradle 缓存动辄几十 GB；多项目重复文件 | 精准识别开发缓存，一键回收，不误删源码 |
| **内容创作者** | 视频/图片素材、设计源文件占用巨大，重复备份堆积 | 大文件可视化定位 + 重复内容去重 |
| **普通 Mac/PC 用户** | 不懂技术，怕删错，订阅清理软件太贵 | 免费/开源、安全默认、AI 帮忙判断 |
| **多平台用户** | 同时用 macOS + Windows + Linux，不想装三套工具 | 一套工具跨三平台，体验一致 |

**核心用户**：技术开发者与重度内容创作者（磁盘压力大、对工具品质有要求、愿意为效率付费或贡献代码）。

---

## 3. 核心场景（Core Scenarios）

### 场景 A：开发者释放磁盘空间
> 小张的 MacBook 512GB 又满了。打开 TrueClean → 概览页看到磁盘使用率 92% → 点"扫描" → 旭日图显示"开发文件"占 180GB → 下钻发现 DerivedData + node_modules 缓存占 60GB → 打开"系统垃圾"面板，开发缓存组标记为"推荐清理" → 勾选 → 确认清理（进回收站）→ 释放 58GB。

### 场景 B：AI 助手辅助决策
> 小李不确定哪些能删。打开 AI 面板问"帮我看看能安全清理多少空间" → Agent 自动调用 `scan_junk` + `list_volumes` → 基于真实数据回复：「可立即清理 12GB（缓存/日志）、建议复核 8GB（大文件）、不要动 80GB（系统）」→ 小李确认后 Agent 执行清理。

### 场景 C：重复文件去重
> 摄影师老王有多个备份目录。打开"重复文件"面板 → 选目录扫描 → blake3 内容哈希找出 200 组重复 → 每组保留 1 个 → 预计回收 45GB → 确认删除（进回收站）。

### 场景 D：卸载应用并清残留
> 小赵想卸载几个不用的 App。打开"应用卸载"面板 → 列出所有应用及体积 → 选一个卸载 → 连带清理 `~/Library/Application Support/xxx`、缓存、偏好文件 → 释放 3.2GB。

---

## 4. 功能清单（Feature Inventory · 对照已实现）

### 4.1 已实现（首版基线）

| 模块 | 功能 | 实现位置 | 状态 |
|---|---|---|---|
| **磁盘扫描** | 并行递归扫描（jwalk + rayon），取消、进度节流、跳过无权限项 | `scanner/{walker,engine,tree}` | 生产级 |
| **分类统计** | 11 类（系统/应用/开发/文档/媒体/缓存/日志/废纸篓/下载/压缩包/其他） | `scanner/categories.rs` | 完成 |
| **可视化** | Treemap + Sunburst + CategoryBar + FileTree 下钻 | `components/scan/*` | 完成 |
| **系统垃圾** | 9 组（用户/系统/应用缓存、日志、临时、浏览器缓存、开发缓存、语言缓存、回收站） | `cleaning/{junk,paths}.rs` | 完成 |
| **大文件查找** | 按最小大小 + 未修改天数筛选 | `cleaning/large_old.rs` | 完成 |
| **重复文件** | blake3 内容哈希，先按大小分桶再哈希 | `cleaning/duplicates.rs` | 完成 |
| **应用卸载** | 列出应用 + 连带清理残留 | `cleaning/uninstaller.rs` | 基线 |
| **启动项管理** | 列出/启停登录项、LaunchAgent | `cleaning/startup.rs` | 基线 |
| **安全删除** | 默认进回收站、保护路径过滤（`is_protected`） | `cleaning/{safety,trash}.rs` | 生产级 |
| **撤销/快照** | CleanManifest 记录 + `restore_last` 还原 | `cleaning/trash.rs` | 完成 |
| **AI Agent** | 多 Provider（Claude/OpenAI/Ollama）+ 9 工具 + 流式 + 工具循环 | `agent/*` | 基线 |
| **设置** | Provider/Model/Key/语言/回收站默认 | `commands/settings.rs` | 完成 |

### 4.2 待完善（路线图中）

- Windows/Linux 清理路径表与卸载残留打磨
- 真机端到端测试与 UI 五态（空/载/错/结果/进行）打磨
- 扫描结果缓存与增量扫描
- 国际化（中/英）完善
- 应用图标与品牌视觉、打包签名/自动更新
- CI/CD 与 E2E 测试

详见 [ROADMAP.md](ROADMAP.md)。

---

## 5. 非目标（Non-Goals）

明确**不做**的事，避免范围蔓延：

1. **不做云端同步** —— TrueClean 是纯本地桌面应用，用户数据不上传任何服务器（隐私声明见 [SECURITY.md](SECURITY.md)）。
2. **不做注册表/系统深度清理** —— 不碰 Windows 注册表、不删系统内核扩展，避免变"系统优化大师"式的高风险操作。
3. **不做杀毒/恶意软件扫描** —— 专注磁盘空间管理，安全检测交给专业产品。
4. **不做移动端** —— 焦点在桌面（macOS/Windows/Linux），不扩展到 iOS/Android。
5. **不做付费订阅** —— 开源 MIT 协议，AI 能力由用户自带 API Key（BYOK），TrueClean 不代售、不抽成。
6. **不内置任何 API Key** —— 用户自行配置，密钥仅存本地。

---

## 6. 成功指标（Success Metrics）

### 北极星指标（North Star）

> **月活用户平均单次释放磁盘空间 ≥ 5GB**

这个指标同时反映"产品被使用"和"产品有用"——用户愿意打开它，且每次都能腾出有意义的空间。

### 关键指标（Key Metrics）

| 维度 | 指标 | 1.0 目标 |
|---|---|---|
| **采用** | GitHub Star | ≥ 1,000 |
| **采用** | 月活用户（MAU） | ≥ 5,000 |
| **效果** | 平均单次清理释放空间 | ≥ 5GB |
| **效果** | AI 助手使用率（打开过 AI 面板的用户占比） | ≥ 40% |
| **安全** | 误删系统文件导致故障的报告率 | = 0（零容忍） |
| **安全** | 撤销成功率（to_trash 清理的可恢复比例） | ≥ 99% |
| **质量** | 扫描 10 万文件耗时 | < 15 秒 |
| **质量** | 崩溃率（session crash / total session） | < 0.1% |
| **社区** | 贡献者数量 | ≥ 10 |

---

## 7. 竞品分析（Competitive Analysis）

### 竞品矩阵

| 维度 | **TrueClean** | CleanMyMac | DaisyDisk | BleachBit |
|---|---|---|---|---|
| **定位** | 开源跨平台 AI 磁盘清理 | macOS 商业清理套件 | macOS 磁盘可视化 | 跨平台开源清理 |
| **平台** | macOS / Windows / Linux | 仅 macOS | 仅 macOS | macOS / Windows / Linux |
| **价格** | 免费（开源 MIT） | ¥300+/年订阅 | ¥68 买断 | 免费（GPL） |
| **AI 助手** | 多 Provider + 真实工具调用 | 无 | 无 | 无 |
| **可视化** | Treemap + Sunburst + 下钻 | 进度条为主 | 旭日图（强） | 无 |
| **安全撤销** | CleanManifest + restore | 有 | 无（只看） | 无 |
| **保护路径** | is_protected 硬编码红线 | 有（闭源不可审计） | N/A | 部分 |
| **开发缓存** | npm/cargo/pip/gradle/Xcode | 部分 | 无 | 部分 |
| **重复文件** | blake3 哈希 | 有 | 无 | 无 |
| **应用卸载残留** | 有 | 有 | 无 | 无 |
| **隐私** | 纯本地，不上传 | 部分遥测 | 纯本地 | 纯本地 |
| **可审计性** | 开源，安全逻辑可审 | 闭源 | 闭源 | 开源 |

### 差异化（Differentiation）

TrueClean 的三个核心壁垒：

1. **AI Agent 真正干活，不是空谈**
   - 竞品的"AI"多为静态规则或聊天机器人。TrueClean 的 Agent 可调用 9 个真实工具（扫描目录、找垃圾、找重复、列应用、清理路径…），基于**用户磁盘的真实数据**给建议，并在用户确认后执行。这是"AI + 工具调用"范式在磁盘清理领域的落地。

2. **安全可审计、可撤销**
   - 删除逻辑全开源，`is_protected` 保护路径表硬编码且可审计；默认进回收站 + CleanManifest 快照 + 一键 `restore_last` 撤销。竞品要么闭源不可审计（CleanMyMac），要么没有撤销（BleachBit）。

3. **跨平台 + 开发者友好**
   - 一套代码三平台；专门识别开发缓存（Xcode DerivedData、node_modules、cargo/pip/gradle 缓存）；BYOK 模式让用户自带 AI Key，无订阅成本。

---

## 8. 开放问题（Open Questions）

1. **AI Provider 成本**：BYOK 模式下，用户对 API 成本的接受度？是否需要内置 Ollama 本地模型引导（零成本）？
2. **企业版**：是否提供企业部署版（集中配置、审计日志）？当前非目标，但路线图 2.0 可考虑。
3. **插件化**：是否开放清理规则插件系统让社区贡献？1.0 不做，避免安全面扩大。

---

## 附录：术语表

| 术语 | 含义 |
|---|---|
| **BYOK** | Bring Your Own Key，用户自带 API Key |
| **CleanManifest** | 清理快照，记录被移入回收站的原始路径/体积/时间，用于撤销 |
| **is_protected** | 保护路径检查函数，命中系统关键路径则拒绝删除 |
| **Provider** | AI 模型提供方（Claude / OpenAI / Ollama） |
| **Treemap / Sunburst** | 矩形树图 / 旭日图，两种磁盘占用可视化方式 |
