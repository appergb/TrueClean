# TrueClean — 多 Agent 完善任务书（可立项基线）

> 本文件是把 **可编译基线（`635cb9f`）** 推进到 **可立项、可发布、可信赖** 状态的施工蓝图。
> 每个 Agent 一段「可直接粘贴」的提示词，已按**文件归属**严格分区以避免并行冲突。
> 数据契约见 [`docs/CONTRACT.md`](CONTRACT.md)，是所有 Agent 的单一真源。

- **代码仓库（基线）**：`https://github.com/appergb/TrueClean.git` · 分支 `master` · commit `635cb9f`
- **技术栈**：Tauri 2 + Rust（后端）/ React 18 + TS + Vite 6（前端）/ zustand / d3-hierarchy
- **基线已验证**：`cargo check` ✅ · `pnpm build`（tsc + vite）✅ · scanner 带单元测试
- **当前缺口（本任务书要补齐的）**：安全撤销/快照、跨平台清理覆盖、测试覆盖率、Agent 健壮性、错误/空态 UI、品牌图标、CI/CD、打包签名/自动更新、i18n、立项文档

---

## 0. 使用方式

1. 让每个 Agent 先 `git clone` 基线（或在已 clone 的仓库 `git checkout master && git pull`）。
2. **每个 Agent 从 `master` 切自己的分支**：`git checkout -b agent/<AGENT-ID>`（如 `agent/A1-scanner`）。
3. 给该 Agent 粘贴：**§1 共享前导（Shared Preamble）** + **它自己的任务块**。
4. Agent 完成后跑「验收门禁」，绿了再发 PR 回 `master`。
5. 合并顺序见 §2 依赖波次；同波次内可完全并行（文件零重叠）。

---

## 1. 共享前导（粘给每个 Agent 的开头）

```
你是 TrueClean 项目的一名子系统工程师。TrueClean 是一个跨平台桌面磁盘清理 + AI Agent 应用
（类 CleanMyMac）：Tauri 2 + Rust 后端，React 18 + TypeScript + Vite 6 前端，zustand 状态，
d3-hierarchy 可视化。仓库：https://github.com/appergb/TrueClean.git，基线分支 master。

【铁律，违反即作废】
1. 数据契约冻结：src-tauri/src/model.rs 与 src/lib/types.ts 字段一一对应、camelCase，禁止改字段名。
   Tauri 命令名、事件名（scan://progress、agent://event/{sessionId}）必须逐字一致。
   如确需新增字段/命令，必须同时改 model.rs + types.ts + docs/CONTRACT.md，并在 PR 里显式说明。
2. 只动属于你的文件（见任务块「文件归属」）。绝不改别的 Agent 的文件，绝不改下列冻结文件：
   model.rs、error.rs、state.rs、lib.rs、main.rs、所有 mod.rs、commands/settings.rs、
   src/lib/{types.ts,ipc.ts,format.ts}、src/main.tsx、src/styles/{tokens.css,global.css}。
   如这些文件确实挡路，停下来在 PR 描述里提出，不要擅自改。
3. 安全第一：任何删除/卸载/清空操作默认走回收站（trash crate），破坏性操作必须二次确认；
   绝不硬编码或上传任何 API Key；绝不建议/执行删除系统关键路径（/System、/usr、Windows、
   Program Files 系统部分等）。
4. 风格对齐：Rust 跑 cargo fmt + clippy（-D warnings）、不 unwrap 生产代码、错误用 AppError/Result；
   TS 不用 any、公共 API 显式类型、不留 console.log、组件 props 命名 interface。
   文件保持小而内聚（<400 行，硬上限 800），新代码注释密度对齐现有代码。
5. 改动要外科手术式：每一行改动都能追溯到任务目标，不顺手重构无关代码。

【验收门禁（必须全绿才算完成，PR 里贴输出）】
- 后端：cd src-tauri && cargo fmt --all -- --check && cargo clippy --all-targets -- -D warnings && cargo test
- 前端：pnpm install && pnpm build      # = tsc --noEmit && vite build
- 你新增的功能必须有对应测试，并说明如何手动验证。

先读 docs/CONTRACT.md 和 docs/AGENT_TASKS.md（本任务书）里你那一节，再动手。下面是你的任务：
```

---

## 2. 依赖波次与并行（同波次文件零重叠，可并发）

| 波次 | Agents（可并行） | 说明 |
|---|---|---|
| **A 后端硬化** | A1 A2 A3 A4 | 各占独立后端模块。A3 只读 A2 的 `paths.rs`（签名冻结）。先合并这波，前端依赖稳定的命令行为 |
| **B 前端打磨** | B1 B2 B3 B4 | 各占独立前端模块。**B1 先建 i18n 框架**，B2/B3/B4 各自维护自己命名空间的语言文件 |
| **C 可发布** | C1 → C2，C3 并行 | C1 出图标 → C2 接图标进 tauri.conf。C3 文档独立可并行 |
| **D 集成验收** | D1 | 最后一个。E2E + `tauri build` 全平台 + 跨 Agent 集成修复 + 对照「可立项清单」终验 |

### 文件归属矩阵（防冲突）

| Agent | 拥有（可改/可建） |
|---|---|
| A1 | `src-tauri/src/scanner/*`（walker/tree/categories/engine，不含 mod.rs 导出签名） |
| A2 | `src-tauri/src/cleaning/{paths,junk,large_old,trash}.rs` + 新建 `cleaning/safety.rs` |
| A3 | `src-tauri/src/cleaning/{duplicates,uninstaller,startup}.rs` |
| A4 | `src-tauri/src/agent/*`（prompt/tools/runner/providers/*） |
| B1 | `src/App.tsx`、`src/components/layout/*`、`src/components/ui/*`、新建 `src/i18n/*`、`src/hooks/useTheme.ts`、新建 ui/locale store |
| B2 | `src/components/scan/*`、`src/hooks/useScan.ts`、`src/store/scanStore.ts`、`src/i18n/locales/*/scan.ts` |
| B3 | `src/components/cleanup/*`、`src/components/settings/*`、`src/store/settingsStore.ts`、`src/i18n/locales/*/cleanup.ts` |
| B4 | `src/components/agent/*`、`src/hooks/useAgent.ts`、`src/store/agentStore.ts`、`src/i18n/locales/*/agent.ts` |
| C1 | `src-tauri/icons/*`、`assets/branding/*`（新建）；产出图标清单交给 C2 |
| C2 | `.github/workflows/*`、`.eslintrc.*`/`.prettierrc`/`rustfmt.toml`/`clippy.toml`、`src-tauri/tauri.conf.json`、`src-tauri/Cargo.toml`（仅加 updater 插件）、`package.json`（仅加 lint 脚本/依赖） |
| C3 | `docs/*`（PRD/架构/威胁模型/路线图/用户手册）、`README.md`、`CONTRIBUTING.md` |
| D1 | `tests/e2e/*`（新建）、`playwright.config.ts`；集成期可跨文件修 bug，但须在 PR 注明并尽量小改 |

> 唯一需排序的两处：**C1 在 C2 之前**（图标→配置）；i18n 框架由 **B1 先落地**，其余 B 再填词。

---

## 3. Agent 任务块（逐段粘贴）

### A1 — 扫描内核：性能 / 取消 / 缓存 / 增量 / 测试

```
任务：把磁盘扫描内核打磨到生产级。文件归属：src-tauri/src/scanner/*（walker/tree/categories/
engine.rs；不要改 mod.rs 的对外签名，scan_tree 与 classify 的签名见 CONTRACT §3）。

目标：
1. 取消正确性：扫描中途 cancel 必须快速、干净地停止（检查 AtomicBool 的粒度足够细），
   返回时不 panic、不留半成品；为「扫到一半取消」写测试。
2. 进度节流：scan://progress 事件不要每个文件都发（会刷爆前端）；按时间或文件数节流
   （如每 200ms 或每 N 个文件聚合一次），保证 UI 流畅。验证：扫大目录时事件频率合理。
3. 性能：用 rayon/jwalk 并行；避免对每个文件重复 stat；大目录（>50万文件）不爆内存。
   加一个 benches/ 或测试基准说明，记录扫描 X 文件耗时量级。
4. 健壮性：无权限目录、符号链接环、超长路径、特殊字符要跳过而非崩溃，并计数「跳过/报错」项。
5. 缓存：在 scanner 内提供「按根路径缓存上次 ScanResult + mtime 指纹」的纯函数能力
   （供 state.last_scan 复用；不要改 state.rs，只暴露可被 commands 调用的函数；如需新函数，
   在 mod.rs 仅追加导出且不改既有签名——若必须改导出，PR 里提出）。增量扫描可作为后续，
   但至少给出「目录指纹是否变化」的判断函数 + 测试。
6. 分类：categories::classify 覆盖更多真实后缀/路径模式（开发文件、媒体、压缩包、缓存、日志），
   补全 11 个类别的判定并加表驱动测试（给定路径 → 期望 Category）。

验收：cargo fmt/clippy/test 全绿；新增取消、分类、指纹、跳过计数的单元测试；
PR 描述写明：扫 ~/（或某大目录）的耗时、事件数量级、被跳过项统计。
```

### A2 — 清理核心 + 安全快照/撤销（信任的基石）

```
任务：清理核心健壮化，并新增「清理前快照 + 一键撤销」的安全网。文件归属：
src-tauri/src/cleaning/{paths,junk,large_old,trash}.rs + 新建 cleaning/safety.rs。
（duplicates/uninstaller/startup 归 A3，勿动；paths.rs 你拥有，但 application_dirs 等函数
被 A3 只读复用——可以增强实现，但不要改它们的签名/语义。）

目标：
1. 安全红线（最重要）：新建 cleaning/safety.rs，提供 is_protected(path) -> bool，硬编码
   绝不可删的系统关键路径表（macOS /System /usr /bin /Library/Apple，Windows C:\Windows、
   Program Files 系统组件，Linux /usr /bin /etc /boot 等）。clean_paths 与 empty_trash
   在执行前必须用它过滤/拒绝，命中即报 AppError 并跳过，绝不删。写覆盖测试。
2. 撤销/快照：clean_paths(to_trash=true) 时记录一份「清理清单」（被移动到回收站的原始路径 +
   时间 + 体积），提供 restore_last(manifest) 能力把回收站项还原回原位（trash crate 支持的范围内；
   不支持的平台要明确返回「不支持」而非假装成功）。清单存到 app 配置目录的 json。
   写测试：建临时文件 → clean(to_trash) → restore → 断言文件回来了。
3. clean_paths：永久删除（to_trash=false）必须先过 is_protected；统计每条成功/失败/释放字节，
   汇总进 CleanReport（不改 CleanReport 字段，按现有结构填充）；单条失败不中断整体。
4. junk.rs：分组（用户缓存/系统缓存/应用缓存/日志/临时/浏览器缓存/开发缓存/语言缓存/回收站）
   的「推荐清理」标记要保守正确；体积统计用并行且对超大目录设上限/超时，避免卡死。补测试。
5. paths.rs：补全/校正各平台候选路径（参考现有 cfg(target_os) 写法），所有返回路径都 exists()。
   Windows/Linux 的浏览器缓存、语言缓存尽量补全（Chrome/Edge/Brave/Firefox、npm/pip/cargo/gradle）。

验收：cargo fmt/clippy/test 全绿；is_protected、restore、clean 统计均有测试；
PR 说明：保护路径表清单、撤销在本平台是否可用、junk 分组体积统计的耗时。
```

### A3 — 跨平台：重复文件 / 应用卸载 / 启动项

```
任务：把 duplicates/uninstaller/startup 三个模块的跨平台覆盖补齐到可用。文件归属：
src-tauri/src/cleaning/{duplicates,uninstaller,startup}.rs。
（可只读复用 paths.rs 的 application_dirs() 等，但不要改 paths.rs——它归 A2。）

目标：
1. duplicates.rs：blake3 内容哈希去重（先按大小分桶、再哈希），确认对 0 字节文件、
   同 inode 硬链接、权限不足的处理合理；大目录并行且可被 cancel；可回收空间统计正确。补测试
   （造若干重复/非重复临时文件 → 断言分组与可回收字节）。
2. uninstaller.rs：list_applications 在 macOS（/Applications 的 .app + Info.plist 读名称/版本/体积/
   最近使用）、Windows（注册表 Uninstall 键或 Program Files 扫描 + 体积）、Linux（.desktop + /opt）
   三平台都返回合理结果；uninstall_app 连带清理残留（缓存/偏好/Application Support/日志），
   默认走回收站，受 A2 的保护路径约束（调用约定见 PR 协调；如需共享 is_protected，从 cleaning::safety 引入）。
   每平台至少给出「能列出应用」的冒烟测试或在无法单测时给手动验证步骤。
3. startup.rs：list_startup_items / set_startup_item 在 macOS（LaunchAgents/LaunchDaemons + 登录项）、
   Windows（注册表 Run 键 + 启动文件夹 + 计划任务子集）、Linux（~/.config/autostart + systemd user）
   返回名称/路径/启用态，并能启停。危险写操作要稳妥（失败回滚、不破坏系统文件）。
4. 三个模块在「非本机平台」分支也要能编译（cfg 完整），未实现的平台明确返回空列表或
   AppError::Unsupported 文案，绝不 panic。

验收：cargo fmt/clippy/test 全绿（注意 cfg 分支在本机编译路径覆盖）；
PR 说明：本机平台实测 list_* 的条数与样例、uninstall/startup 的手动验证步骤、其他平台的降级行为。
```

### A4 — AI Agent 引擎：健壮性 / 工具安全 / 确认流

```
任务：把 Agent 引擎打磨到生产级。文件归属：src-tauri/src/agent/*
（prompt.rs、tools.rs、runner.rs、providers/{claude,openai,ollama,traits,mod}.rs）。

目标：
1. Provider 健壮性：三个 provider 的 HTTP 请求加超时（连接+整体）、对 429/5xx 做有限次指数退避重试、
   把 HTTP 错误体/网络错误映射成清晰的 AppError 文案（中文，给用户可读提示，不泄露 key）。
   流式解析对半包/异常 chunk 容错，不 panic。缺 key / key 无效要给明确提示。
2. 工具安全：tools::dispatch 里 clean_paths/empty_trash 这类破坏性工具，必须经过 cleaning::safety
   的保护路径校验（与 A2 协调引入 is_protected）；默认 toTrash=true。工具结果裁剪成对 LLM 友好的
   紧凑 JSON（体积人类可读、列表截断 topN），避免把超长结果灌回模型。
3. 确认流：破坏性工具调用在执行前，runner 通过 AgentEvent 让前端有机会确认（按 CONTRACT 的事件模型；
   不改 AgentEvent 变体字段——用现有 ToolCall 事件 + 前端确认；如确需新增确认事件变体，
   同步改 model.rs+types.ts+CONTRACT 并在 PR 说明）。MAX_ROUNDS 防失控已有，保留。
4. prompt.rs：强化中文系统提示词——主动用工具取真实数据再下结论、绝不编造路径/体积、
   区分「绝对安全可删 / 需用户确认」、输出按「可立即清理 / 建议复核 / 不要动」分组并给预计释放空间合计、
   安全红线（绝不建议删系统关键路径）。
5. 测试：为 tools::dispatch 的参数解析/结果裁剪、provider 的错误映射与流式解析（可用 mock/本地字符串
   构造，不真连网）写单元测试。

验收：cargo fmt/clippy/test 全绿；dispatch、错误映射、流式解析有测试；
PR 说明：三 provider 的超时/重试参数、破坏性工具的确认与保护链路、提示词改动要点。
```

### B1 — 前端骨架 / 设计系统 / i18n 框架 / 错误兜底（前端地基，先行）

```
任务：建立前端「设计系统 + 全局兜底 + 国际化框架」，并打磨外壳。文件归属：
src/App.tsx、src/components/layout/*（Sidebar/TopBar/Overview/layout.css）、
src/components/ui/*（Button/IconButton/Segmented/SurfaceCard/ProgressRing/EmptyState/ui.css）、
src/hooks/useTheme.ts、新建 src/i18n/*、新建 ui/locale 的 zustand store。
（不要改 styles/tokens.css、global.css——它们冻结；可在组件内新增 css。）

目标：
1. i18n 框架（B2/B3/B4 依赖它，先做）：新建 src/i18n/index.ts（轻量，零额外依赖即可，
   或用 i18next，自行决定但保持 <150KB 预算）；定义 locale = 'zh' | 'en'；提供 useI18n() / t(key)；
   语言存到 localStorage + 一个 zustand locale store；目录约定：
   src/i18n/locales/{zh,en}/{shell,scan,cleanup,agent}.ts，你负责 shell.ts（外壳/布局/通用按钮文案）
   和把 zh/en 聚合导出；其余命名空间留空骨架给 B2/B3/B4 填。TopBar 加语言切换。
2. 全局兜底：加 React ErrorBoundary（崩溃时友好中文兜底 + 重试），加一个轻量 Toast/通知系统
   （成功/失败/进行中），供各面板复用（导出 hook，如 useToast）。
3. 设计质量：对照 design-quality（反模板）：层次、留白节奏、深浅双主题都要像样、hover/focus/active
   有设计感、Overview 概览页有信息层级而非千篇一律卡片。不要破坏 tokens.css 的设计令牌，基于它扩展。
4. 首次运行引导：首启时的空态/引导（介绍三步：扫描→识别→AI 清理 + 一键开始），以及未配置 AI key 时
   的友好提示入口（跳设置）。
5. 无障碍：语义化标签、aria-label、键盘可达（侧边导航、抽屉开合、按钮）。

验收：pnpm build 全绿；中英切换在外壳生效；制造一个抛错组件验证 ErrorBoundary；
PR 说明：i18n 用法约定（给 B2/B3/B4 看）、Toast/ErrorBoundary 的 API、设计改动截图。
```

### B2 — 扫描可视化 UI：空态 / 错误态 / 交互

```
任务：打磨扫描与可视化界面。文件归属：src/components/scan/*（ScanView/Treemap/Sunburst/
CategoryBar/FileTree/ScanProgress/scan.css）、src/hooks/useScan.ts、src/store/scanStore.ts、
src/i18n/locales/{zh,en}/scan.ts（用 B1 的 i18n 框架，填你这一命名空间）。

目标：
1. 状态完整：未扫描（引导选盘/目录）、扫描中（实时进度：文件数/字节/当前路径 + 取消按钮）、
   完成、空结果、出错（友好中文 + 重试）五态都要有，用 B1 的 EmptyState/Toast。
2. 可视化体验：Treemap 与 Sunburst 配色按类别一致、hover 显详情 tooltip、点击下钻、
   FileTree 面包屑返回；超多节点时性能不卡（必要时虚拟化或限制渲染层级/数量）。
3. 取消与进度：取消要即时反馈；进度事件已被后端节流（A1），前端平滑展示不要自己再高频 setState 抖动。
4. 交互细节：选择磁盘/目录用 @tauri-apps/plugin-dialog；大数字用 src/lib/format.ts 的人类可读格式；
   响应式（窗口 960~1920 宽不溢出）。
5. i18n：本面板所有中文文案走 t('scan.xxx')，同时补 zh/en 两份。

验收：pnpm build 全绿；五态可手动复现；中英文都正常；
PR 说明：可视化交互说明 + 大目录下的渲染表现 + 截图（深浅各一）。
```

### B3 — 清理面板 + 设置 UX：确认 / 校验

```
任务：打磨系统垃圾/大文件/重复/卸载/启动项五个面板与设置页。文件归属：
src/components/cleanup/*（JunkPanel/LargeOldFiles/DuplicatesPanel/UninstallerPanel/StartupItems/cleanup.css）、
src/components/settings/*（SettingsPanel/settings.css）、src/store/settingsStore.ts、
src/i18n/locales/{zh,en}/cleanup.ts（用 B1 的 i18n 框架）。

目标：
1. 二次确认：所有破坏性操作（清理/卸载/清空回收站/删大文件/删重复）必须弹确认框，显示
   「将删除 N 项 / 释放 X 空间 / 是否进回收站」，默认进回收站；操作后用 Toast 报结果（成功/失败/释放量）。
2. 五态：每个面板都要有 加载中 / 空 / 出错 / 结果 / 操作进行中 态，用 B1 组件。
3. 勾选与汇总：组级/项级勾选联动、底部实时汇总「已选 N 项 / 预计释放 X」；大文件按筛选（最小大小+
   多少天未改）；重复按组保留 1 删其余；卸载显示残留项；启动项一键启停。
4. 设置校验：Provider(claude/openai/ollama) 切换联动 Model 占位与 Key/地址字段；保存前做基本校验
   （ollama 地址格式、key 非空提示但不强制）；Key 输入用密码态、可显示/隐藏；明确告知「Key 仅存本地」。
   提供「测试连接」按钮（调一次最小请求或 Ollama /api/tags）给出成功/失败反馈（经 ipc 调用，不直接 fetch）。
5. i18n：全部文案走 t('cleanup.xxx')，补 zh/en。

验收：pnpm build 全绿；确认框/校验/Toast 可手动复现；中英正常；
PR 说明：确认与校验交互说明 + 截图。
```

### B4 — AI 助手面板 UX：流式 / 工具卡 / 确认

```
任务：打磨右侧 AI 助手抽屉。文件归属：src/components/agent/*（AgentPanel/MessageList/
ToolCallCard/Composer/agent.css）、src/hooks/useAgent.ts、src/store/agentStore.ts、
src/i18n/locales/{zh,en}/agent.ts（用 B1 的 i18n 框架）。

目标：
1. 流式体验：监听 agent://event/{sessionId} 的 AgentEvent（Text/ToolCall/ToolResult/Done/Error），
   增量渲染助手文字、自动滚动、可中断（agent_cancel）；Error/Done/cancelled 都要清晰收尾。
2. 工具调用可视化：ToolCallCard 展示「调了哪个工具 + 参数 + 结果摘要」，可折叠；区分进行中/完成/失败。
3. 破坏性确认：当 Agent 想执行清理类工具时（与 A4 约定的确认流），前端弹确认/拒绝，结果回传；
   未确认不执行。展示 Agent 的结构化建议（可立即清理/建议复核/不要动 + 预计释放合计）时排版清晰。
4. 体验细节：未配置 key 时引导去设置；输入框支持多行/Enter 发送/Shift+Enter 换行/发送中禁用；
   空会话有示例提问引导；长对话性能可接受。
5. i18n：全部文案走 t('agent.xxx')，补 zh/en。

验收：pnpm build 全绿；用任一真实/mock provider 跑通一轮「提问→工具调用→结果→建议」；
中英正常；PR 说明：事件处理与确认流说明 + 截图/录屏要点。
```

### C1 — 品牌视觉与图标（全平台图标集）

```
任务：为 TrueClean 设计并产出全平台应用图标与基础品牌视觉。文件归属：
src-tauri/icons/*、新建 assets/branding/*。不要改 tauri.conf.json（归 C2，你只产出图标并把
最终图标清单/路径写进 PR 交给 C2 接线）。

目标：
1. 一个有辨识度的「磁盘清理 + 智能」主题图标（简洁、桌面级质感，深浅背景都清晰），
   导出 Tauri 需要的全套尺寸：32x32.png、128x128.png、128x128@2x.png、icon.png（512+）、
   icon.icns（macOS）、icon.ico（Windows，多尺寸）。可用脚本/工具从一张 1024 母图生成。
2. 基础品牌：主色（与 tokens.css 的设计令牌协调）、Logo 文件、一句话 slogan，放 assets/branding/
   并写一页 brand 说明（颜色值、用法）。
3. 不引入构建期重依赖；图标为静态产物。

验收：图标文件齐全且能被 Tauri 识别（C2 接入后 tauri build 出包带正确图标）；
PR 交付：图标清单（文件名→尺寸/平台）+ 母图 + brand 说明，明确告诉 C2 要在 tauri.conf 的 bundle.icon 填哪些。
```

### C2 — CI/CD + 代码规范 + 打包/签名/自动更新

```
任务：建立工程化与发布基建。文件归属：.github/workflows/*、根目录 lint/format 配置
（.eslintrc.cjs/.prettierrc/rustfmt.toml/clippy.toml）、src-tauri/tauri.conf.json、
src-tauri/Cargo.toml（仅追加 updater 插件依赖）、package.json（仅追加 lint 脚本与 devDeps）。
依赖 C1 已产出的图标清单。

目标：
1. 代码规范：加 ESLint + Prettier（React/TS 规则，禁 console.log/any 提醒）、rustfmt.toml、
   clippy.toml；package.json 增 "lint"/"format" 脚本；保证现有代码能过（必要的最小修在你的配置范围内，
   涉及他人源码的 lint 报错只汇总不擅改，列进 PR 给对应 Agent）。
2. CI（GitHub Actions）：PR 触发的 ci.yml —— 前端 tsc+vite+eslint，后端 cargo fmt --check +
   clippy -D warnings + cargo test，三平台（macos/windows/ubuntu）矩阵编译。缓存 cargo/pnpm 加速。
3. 发布：release.yml —— 打 tag 时用 tauri-action 在 macos/windows/ubuntu 构建安装包
   （.dmg/.app、.msi/.exe、.AppImage/.deb），上传到 GitHub Release。签名/公证用占位的 secrets
   （APPLE_*、WINDOWS_* 留 TODO 文档说明，不内置任何真密钥）。
4. 自动更新：接入 tauri-plugin-updater——tauri.conf 配 updater（endpoints 占位 + pubkey 占位），
   Cargo.toml 加依赖，capabilities 加权限，并产出 latest.json 生成说明。把 C1 的图标接进 bundle.icon。
   确保 bundle.targets 合理、productName/identifier/version 正确。
5. 文档：在 PR 写清「如何配置签名 secrets、如何发版、如何生成 updater 密钥」。

验收：本地 pnpm lint 通过；ci.yml/release.yml 语法正确（可 act 或 dry-run 校验）；
tauri build 在本机出带正确图标的安装包；PR 说明发布与签名流程。
```

### C3 — 立项文档与产品材料

```
任务：产出「可立项」所需的产品与工程文档。文件归属：docs/*（新建多份）、README.md、CONTRIBUTING.md。
不改任何源码。

目标（每份 markdown，中文为主，关键处中英）：
1. docs/PRD.md：产品需求文档——问题/目标用户/核心场景/功能清单（对照已实现）/非目标/
   成功指标（北极星 + 关键指标）/竞品（CleanMyMac/DaisyDisk/BleachBit）差异化。
2. docs/ARCHITECTURE.md：系统架构——前后端分层、IPC 契约、扫描/清理/Agent 数据流图（用 mermaid）、
   关键模块职责、技术选型理由。可引用 CONTRACT.md。
3. docs/SECURITY.md（威胁模型）：删除/卸载的风险面、保护路径机制、回收站/撤销、Key 本地存储、
   AI 工具调用的破坏性操作确认链；隐私声明（不上传用户数据/路径）。
4. docs/ROADMAP.md：从当前基线到 1.0 的里程碑（对照本任务书的 A~D），以及 1.0 后展望。
5. docs/USER_GUIDE.md：面向用户的使用手册（安装、扫描、各清理功能、配置 AI、常见问题、安全提示）。
6. README.md：把「WIP」更新为真实状态，加徽章、功能截图占位、快速开始、架构一图、贡献指引链接。
7. CONTRIBUTING.md：分支/提交规范、CONTRACT 约束、验收门禁、如何跑测试。
8.（可选）docs/PITCH.md：一页式立项陈述——一句话定位、市场、差异化、技术壁垒、路线图、所需资源。

验收：所有文档结构完整、链接有效、mermaid 图能渲染；README 与真实功能一致；
PR 说明：文档清单与各自定位。
```

### D1 — 集成验收 / E2E / 全平台出包终验

```
任务：作为最后一波，做端到端测试与跨 Agent 集成终验，确保「可立项清单」全绿。文件归属：
新建 tests/e2e/*、playwright.config.ts；集成期可跨文件修 bug，但每处小改、PR 注明、必要时回告对应 Agent。
前置：A/B/C 均已合并到 master。

目标：
1. E2E（Playwright + Tauri，或对 Vite dev 前端做关键路径冒烟）：
   覆盖关键用户流——启动→概览渲染→选目录扫描→看可视化→进系统垃圾→勾选→（mock）确认清理→Toast；
   打开 AI 面板→（mock provider）提问→工具卡→建议；设置切 Provider/语言。flaky 用确定性等待。
2. 全量构建：pnpm tauri build 在本机出包成功；记录三平台出包矩阵（能本机验的本机验，其余靠 C2 的 CI）。
3. 集成 bug：跑通后修复跨模块联调问题（事件名/字段/时序/空态），保持契约不破。
4. 可立项终验清单（逐项核对并在 PR 打勾）：
   □ cargo fmt/clippy/test 全绿  □ pnpm build + lint 全绿  □ tauri build 出包带图标
   □ 删除默认走回收站 + 保护路径生效 + 撤销可用  □ 破坏性操作前端二次确认 + Agent 确认流
   □ 五态 UI（空/载/错/结果/进行）齐全  □ 中英 i18n 完整  □ AI 三 provider 可配置 + 超时重试
   □ CI/release workflow 存在  □ PRD/架构/威胁模型/路线图/用户手册/README/CONTRIBUTING 齐全
   □ 无硬编码密钥、无 console.log、无 TODO/stub 残留
5. 产出 docs/RELEASE_CHECKLIST.md 与一页《验收报告》（测了什么、结果、已知问题、是否达「可立项」）。

验收：E2E 绿、tauri build 成功、终验清单逐项有据；PR 给出验收报告。
```

---

## 4. 合并与协调要点

- **契约冲突**：任何 Agent 想动 model.rs/types.ts/CONTRACT 必须开「契约变更」专项 PR，先合它，其余 Agent rebase。
- **共享读依赖**：A3 读 A2 的 `paths.rs`、A4 与 A2 共用 `cleaning::safety::is_protected`——A2 先合并，A3/A4 再 rebase。
- **i18n**：B1 先合并 i18n 框架，B2/B3/B4 再填命名空间，避免 locales 聚合文件冲突。
- **tauri.conf.json**：仅 C2 可改；C1 只交付图标清单。C1 先合，C2 再接线。
- **每个 PR**：标题 `feat(<AGENT-ID>): …`，正文贴验收门禁输出，列出动过的文件确认未越界。

> 全部合并后，仓库即从「可编译基线」升级为「可立项、可发布、可信赖」的 1.0 候选。
