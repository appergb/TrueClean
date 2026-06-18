# TrueClean — 实现契约（单一真源 / Source of Truth）

> 所有子系统 Agent **必须**严格遵守本契约：Rust 结构体在 `src-tauri/src/model.rs`（我已写好，**不要改**），
> TS 类型在 `src/lib/types.ts`（已写好，**不要改**）。命令名、函数签名、事件名必须逐字一致，否则无法编译/联调。

## 0. 项目总览

跨平台磁盘清理 + AI Agent 桌面应用（类 CleanMyMac）。
- 后端：Rust + Tauri 2.x（`src-tauri/`）
- 前端：React 18 + TypeScript + Vite 6（`src/`）
- 状态：zustand。可视化：d3-hierarchy（treemap / sunburst）
- Agent：多 Provider（Claude / OpenAI / Ollama），工具调用可读取扫描结果并执行清理

## 1. 数据模型（已在 model.rs / types.ts 定稿，禁止修改字段）

所有结构体 `#[serde(rename_all = "camelCase")]`。枚举同样 camelCase。
（完整定义见 `src-tauri/src/model.rs` 与 `src/lib/types.ts`，二者字段一一对应。）

核心结构：`VolumeInfo`、`Category`(枚举)、`DirNode`、`CategoryBreakdown`/`CategoryEntry`、
`FileEntry`、`ScanOptions`、`ScanProgress`、`ScanResult`、`JunkItem`/`JunkGroup`/`JunkKind`、
`DuplicateGroup`、`AppInfo`、`UninstallReport`、`StartupItem`、`CleanReport`、`AppSettings`、
`ChatMessage`、`AgentEvent`(tagged enum, `type` 字段: text/toolCall/toolResult/confirmationRequest/done/error)。

## 2. Tauri 命令（IPC 契约 — 命令名与签名必须逐字一致）

命令实现放在 `src-tauri/src/commands/*.rs`，均 `#[tauri::command]`。返回 `Result<T, AppError>`。
`AppError` 实现 `serde::Serialize`（见 error.rs），前端 catch 到的是 `{ message: string }`。

### commands/scan.rs  —— owner: SCAN-RS
- `get_volumes() -> Result<Vec<VolumeInfo>>`
- `scan_path(path: String, options: ScanOptions, app: tauri::AppHandle, state: State<AppState>) -> Result<ScanResult>`
  - 全盘/目录递归扫描；过程中 `app.emit("scan://progress", ScanProgress)`；结束写入 `state.last_scan`。
- `cancel_scan(scan_id: String, state: State<AppState>) -> Result<()>`

### commands/cleanup.rs  —— owner: JUNK-RS
- `scan_junk(app: tauri::AppHandle, state: State<AppState>) -> Result<Vec<JunkGroup>>`
- `find_large_old_files(path: String, min_size_bytes: u64, older_than_days: u64) -> Result<Vec<FileEntry>>`
- `clean_paths(paths: Vec<String>, to_trash: bool) -> Result<CleanReport>`
- `empty_trash() -> Result<CleanReport>`

### commands/system.rs  —— owner: EXTRA-RS
- `find_duplicates(path: String, min_size_bytes: u64) -> Result<Vec<DuplicateGroup>>`
- `list_applications() -> Result<Vec<AppInfo>>`
- `uninstall_app(app_id: String, to_trash: bool) -> Result<UninstallReport>`
- `list_startup_items() -> Result<Vec<StartupItem>>`
- `set_startup_item(id: String, enabled: bool) -> Result<()>`

### commands/agent.rs  —— owner: AGENT-RS
- `agent_chat(session_id: String, messages: Vec<ChatMessage>, app: tauri::AppHandle, state: State<AppState>) -> Result<()>`
  - 流式：对每个增量 `app.emit(&format!("agent://event/{session_id}"), AgentEvent)`。
  - 支持工具调用循环（见 §4）。
- `agent_cancel(session_id: String, state: State<AppState>) -> Result<()>`
- `agent_confirm(confirmation_id: String, approved: bool) -> Result<bool>`
  - 前端收到 `ConfirmationRequest` 事件后调用此命令回传确认结果。`approved=true` 则执行破坏性工具，`false` 则跳过。返回 `true` 表示确认 ID 已被解析（会话仍在等待）；`false` 表示 ID 已失效（超时/取消/不存在）。
  - 备用路径：前端也可直接 `app.emit('agent://confirm', { id, approved })`，runner 内置的事件监听器会同样路由到 `resolve_confirmation`。

### commands/settings.rs  —— owner: ME（已写好，勿改）
- `get_settings(state) -> Result<AppSettings>`，`save_settings(settings, state) -> Result<()>`（持久化到配置文件）。

## 3. 核心库函数（被 commands 与 agent 工具复用 — 函数名/签名必须一致）

> 业务逻辑写成普通函数放在各模块；commands 是薄包装；agent 工具直接调用这些函数。

### scanner/ (owner: SCAN-RS)
- `scanner::scan_tree(root: &std::path::Path, options: &ScanOptions, cancel: &std::sync::atomic::AtomicBool, on_progress: &(dyn Fn(ScanProgress) + Sync)) -> Result<ScanResult, AppError>`
- `scanner::categories::classify(path: &std::path::Path, is_dir: bool) -> Category`

### cleaning/ (owner: JUNK-RS)
- `cleaning::junk::scan_junk(cancel: &std::sync::atomic::AtomicBool) -> Result<Vec<JunkGroup>, AppError>`
- `cleaning::large_old::find_large_old(root: &std::path::Path, min_size: u64, older_than_days: u64) -> Result<Vec<FileEntry>, AppError>`
- `cleaning::trash::clean_paths(paths: &[String], to_trash: bool) -> Result<CleanReport, AppError>`
- `cleaning::trash::empty_trash() -> Result<CleanReport, AppError>`
- `cleaning::paths` 提供平台路径表（**公开给 EXTRA-RS 复用**）：
  - `pub fn user_cache_dirs() -> Vec<PathBuf>`、`system_cache_dirs()`、`log_dirs()`、`temp_dirs()`、
    `trash_dirs()`、`browser_cache_dirs()`、`developer_junk_dirs()`、`language_cache_dirs()`、
    `application_dirs() -> Vec<PathBuf>`（应用安装目录）。全部用 `cfg!(target_os=...)` 区分 macOS/Windows/Linux。

### cleaning/ (owner: EXTRA-RS — 同目录但不同文件)
- `cleaning::duplicates::find_duplicates(root: &Path, min_size: u64) -> Result<Vec<DuplicateGroup>, AppError>`（blake3 内容哈希，先按大小分桶再哈希）
- `cleaning::uninstaller::list_applications() -> Result<Vec<AppInfo>, AppError>`
- `cleaning::uninstaller::uninstall_app(app_id: &str, to_trash: bool) -> Result<UninstallReport, AppError>`
- `cleaning::startup::list_startup_items() -> Result<Vec<StartupItem>, AppError>`
- `cleaning::startup::set_startup_item(id: &str, enabled: bool) -> Result<(), AppError>`

### agent/ (owner: AGENT-RS)
- `agent::prompt::SYSTEM_PROMPT: &str`（强力中文系统提示词，见 §5）
- `agent::tools::tool_specs() -> Vec<serde_json::Value>`（统一中立 schema，见 §4）
- `agent::tools::dispatch(name: &str, args: &serde_json::Value, state: &AppState) -> Result<serde_json::Value, AppError>`
- `agent::providers::Provider` trait + `ClaudeProvider`/`OpenAiProvider`/`OllamaProvider`
- `agent::runner::run_chat(session_id, messages, settings, app, state)`：Provider 流式 + 工具循环，发 `AgentEvent`。

## 4. Agent 工具（LLM 可调用）

工具用中立 JSON schema 定义一次，各 Provider 适配层转成自己格式。工具集：
- `list_volumes` — 列出磁盘与可用空间
- `scan_directory` {path, topN?} — 扫描目录，返回分类占比 + top 大项（摘要，避免超长）
- `scan_junk` — 列出可清理的缓存/日志/临时/垃圾分组
- `find_large_old_files` {path, minSizeMb, olderThanDays}
- `find_duplicates` {path, minSizeMb}
- `list_applications` — 已安装应用及体积/最近使用
- `list_startup_items` — 启动项
- `analyze_disk_health` — 综合磁盘健康扫描：链式调用 scan_junk + list_volumes，返回总容量/垃圾总量/top3 可清理项/风险等级
- `clean_paths` {paths[], toTrash} — **危险操作**：执行前 runner 必须发 `ConfirmationRequest` 事件让前端确认（默认 toTrash=true 走回收站）
- `empty_trash` — **危险操作**：执行前 runner 必须发 `ConfirmationRequest` 事件让前端确认

所有返回列表的工具结果包含 `highlights` 字段（3-5 条关键发现，按可释放空间×安全等级排序）和每项的 `dataNature` 字段（system/systemCache/systemLog/userCache/userData/userMedia/developerArtifact/temp/trash/unknown）。

`dispatch` 内部直接调用 §3 的库函数，并把结果裁剪成对 LLM 友好的紧凑 JSON（大小用人类可读，列表截断 topN）。破坏性工具经 `cleaning::safety::split_protected` 校验，命中保护路径的拒绝并返回受保护清单。

## 5. 系统提示词要点（agent/prompt.rs）

中文。人格：TrueClean 的磁盘清理与系统优化专家。要求：
- 主动用工具获取真实数据再下结论，绝不编造路径/体积。
- 区分「绝对安全可删」（缓存/日志/临时/回收站）与「需用户确认」（文档/媒体/大文件/应用）。
- 清理前给出预计释放空间与风险等级；破坏性操作默认走回收站并请用户确认。
- 输出结构化、可执行：按「可立即清理 / 建议复核 / 不要动」分组，给出预计释放空间合计。
- 安全红线：绝不建议删除系统关键路径（/System, /usr, Windows, Program Files 系统部分等）。

## 6. 前端契约

- `src/lib/ipc.ts`（已写好）封装所有命令为类型安全函数 + 事件监听器。UI Agent 只调用 ipc.ts，不直接 `invoke`。
- `src/lib/types.ts`（已写好）镜像 Rust model。
- 路由/布局：左侧 `Sidebar`（概览/扫描/系统垃圾/大文件/重复/应用/启动项/设置）+ 右侧主区 + 可切出 Agent 面板（右抽屉）。
- 设计：参考 design-quality 规则，做有层次、有质感的桌面应用，不要模板感。深浅色都要像样。

### 前端文件 owner
- UI-SHELL：App.tsx、components/layout/*、components/ui/*、全局路由与 Agent 抽屉开合
- UI-SCAN：components/scan/*（ScanView/Treemap/Sunburst/CategoryBar/FileTree/ScanProgress）、hooks/useScan.ts、store/scanStore.ts
- UI-CLEAN：components/cleanup/*（JunkPanel/LargeOldFiles/DuplicatesPanel/UninstallerPanel/StartupItems）、components/settings/SettingsPanel.tsx、store/settingsStore.ts
- UI-AGENT：components/agent/*（AgentPanel/MessageList/ToolCallCard/Composer）、hooks/useAgent.ts、store/agentStore.ts

## 7. 事件名汇总
- `scan://progress` → ScanProgress
- `agent://event/{sessionId}` → AgentEvent

## 8. 我（编排者）已提供、各 Agent 勿改的文件
model.rs, error.rs, state.rs, lib.rs, main.rs, 所有 mod.rs, commands/settings.rs,
src/lib/{types.ts, ipc.ts, format.ts}, src/main.tsx, src/styles/{tokens.css, global.css}, 配置文件。
