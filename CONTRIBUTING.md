# TrueClean — 贡献指南（CONTRIBUTING）

> 欢迎为 TrueClean 贡献代码、文档或反馈！请先阅读本文档。
> 配套：[CONTRACT.md](docs/CONTRACT.md)（数据契约） · [AGENT_TASKS.md](docs/AGENT_TASKS.md)（任务书） · [SECURITY.md](docs/SECURITY.md)（安全模型）

---

## 1. 快速上手

### 环境准备

| 工具 | 版本 | 说明 |
|---|---|---|
| Rust | stable（≥ 1.77） | 后端 |
| Node.js | ≥ 18 | 前端 |
| pnpm | 最新 | 包管理器（**不要用 npm/yarn**） |
| 系统依赖 | 见 [Tauri 前置](https://tauri.app/start/prerequisites/) | Linux 需 webkit2gtk 等 |

### 克隆与运行

```bash
git clone https://github.com/appergb/TrueClean.git
cd TrueClean
pnpm install
pnpm tauri dev      # 开发模式（首次编译约 1-2 分钟）
```

### 验证（不弹窗）

```bash
pnpm build                          # 前端 tsc --noEmit && vite build
cd src-tauri && cargo check         # 后端编译检查
cd src-tauri && cargo test --lib    # 后端单元测试
```

---

## 2. 分支与提交规范

### 分支命名

```
agent/<AGENT-ID>-<简述>     # 如 agent/A1-scanner、agent/C3-docs
fix/<简述>                  # bug 修复
feat/<简述>                 # 新功能
docs/<简述>                 # 文档
```

> 从 `master` 切分支，不要直接在 `master` 上提交。

### 提交信息格式

```
<type>(<scope>): <简述>

<可选正文，说明 why>
```

**type**：`feat`（新功能） · `fix`（修复） · `docs`（文档） · `refactor`（重构） · `test`（测试） · `chore`（工程） · `style`（格式）

**scope**：子系统标识，如 `A1`/`A2`/`scanner`/`cleaning`/`agent`/`ui`/`ci`

示例：
```
feat(A2): 清理核心 + 安全快照/撤销 — 信任的基石
fix(scan): 修复大目录扫描进度事件过频
docs(C3): 立项文档与产品材料
```

### PR 标题

```
<type>(<AGENT-ID>): <简述>
```

---

## 3. 数据契约约束（铁律）

TrueClean 的所有子系统通过**冻结的数据契约**协作。违反契约会导致编译失败或联调失败。

### 冻结文件（禁止修改）

以下文件是单一真源，**未经协调不得修改**：

| 文件 | 作用 |
|---|---|
| `src-tauri/src/model.rs` | 全部 IPC 数据结构（Rust 侧） |
| `src/lib/types.ts` | 镜像 model.rs（TS 侧） |
| `src-tauri/src/error.rs` | 统一错误类型 |
| `src-tauri/src/state.rs` | 全局状态 |
| `src-tauri/src/lib.rs` | 模块装配 + 命令注册 |
| `src-tauri/src/main.rs` | 入口 |
| 所有 `mod.rs` | 模块导出签名 |
| `src-tauri/src/commands/settings.rs` | 设置命令 |
| `src/lib/{ipc.ts, format.ts}` | 前端 IPC 封装 |
| `src/main.tsx` · `src/styles/{tokens.css, global.css}` | 前端地基 |

详见 [CONTRACT.md §8](docs/CONTRACT.md)。

### 如需变更契约

1. 开「契约变更」专项 PR，先合并它，其余 Agent rebase。
2. 必须同步修改 `model.rs` + `types.ts` + `CONTRACT.md`。
3. PR 描述显式说明变更原因与影响。

### 命令名与事件名逐字一致

- Tauri 命令名（如 `scan_path`、`agent_chat`）必须与 `lib.rs` 的 `invoke_handler` 注册一致。
- 事件名（`scan://progress`、`agent://event/{sessionId}`）必须逐字一致。

---

## 4. 代码风格

### Rust

- `cargo fmt --all` 格式化（提交前必跑）。
- `cargo clippy --all-targets -- -D warnings` 零警告（CI 强制）。
- **不使用 `unwrap()`/`expect()` 于生产代码**——用 `?` + `AppError`。
- 错误统一用 `AppError` / `AppResult<T>`。
- 文件保持小而内聚（< 400 行，硬上限 800）。
- 新代码注释密度对齐现有代码。

### TypeScript / React

- **不用 `any`**——公共 API 显式类型。
- 组件 props 命名 `interface`。
- **不留 `console.log`**（CI 会检查）。
- 状态管理用 zustand，不引入 Redux。
- 前端只调 `src/lib/ipc.ts`，**不直接 `invoke`**。

### 通用

- 改动要**外科手术式**：每一行改动能追溯到任务目标，不顺手重构无关代码。
- 不添加不必要的注释/文档到未改动的代码。
- 不为不会发生的场景加错误处理。

---

## 5. 验收门禁（必须全绿）

每个 PR 必须贴出以下命令的输出，全绿才算完成：

### 后端

```bash
cd src-tauri
cargo fmt --all -- --check
cargo clippy --all-targets -- -D warnings
cargo test
```

### 前端

```bash
pnpm install
pnpm build      # = tsc --noEmit && vite build
```

### 新功能要求

- 新增功能必须有对应测试。
- PR 描述说明如何手动验证。
- 涉及删除/清理的改动，必须补充安全测试（保护路径、撤销等）。

---

## 6. 如何跑测试

### 后端单元测试

```bash
cd src-tauri
cargo test --lib                    # 所有单元测试
cargo test --lib -- --nocapture     # 显示 println 输出
cargo test --lib -- --ignored       # 跑标记 #[ignore] 的测试（如需 trash 后端）
cargo test safety                   # 只跑 safety 模块
cargo test -- --ignored scan_perf   # 跑性能基线测试
```

### 前端

```bash
pnpm build                          # 类型检查 + 构建（当前主要验证手段）
```

> E2E 测试（Playwright）在波次 D1 引入，详见 [ROADMAP.md](docs/ROADMAP.md)。

### 手动验证扫描性能

```bash
cd src-tauri && cargo test --lib -- --ignored scan_perf --nocapture
# 输出示例：scan_perf_baseline: 1000 files in 0.12s (8333 files/sec)
```

---

## 7. 安全相关贡献的额外要求

TrueClean 会删除文件，安全是产品存在前提。涉及删除/清理/卸载的改动请格外谨慎：

1. **新增删除路径**：必须经过 `cleaning::safety::is_protected` 过滤。
2. **破坏性操作**：默认 `to_trash=true`；UI 二次确认。
3. **保护路径表**：如发现新的系统关键路径需保护，更新 `safety.rs` 的 `protected_roots()` 并补测试。
4. **测试覆盖**：保护路径拒绝、撤销往返、单条失败不中断等场景必须有测试。
5. **不硬编码密钥**：绝不提交任何 API Key / 凭据。
6. **工具结果裁剪**：Agent 工具返回的 JSON 要紧凑（列表截断 topN），避免泄露过多路径。

详见 [SECURITY.md](docs/SECURITY.md)。

---

## 8. 文件归属与并行协作

TrueClean 采用多 Agent 并行开发模式，文件按子系统严格分区以避免冲突。详见 [AGENT_TASKS.md §2](docs/AGENT_TASKS.md) 的文件归属矩阵。

### 贡献者须知

- 只动属于你子系统的文件，**绝不改别人的文件**。
- 如冻结文件挡路，停下来在 PR 描述里提出，不要擅自改。
- 共享读依赖（如 A3 读 A2 的 `paths.rs`）只读复用，不改签名/语义。

### 当前子系统划分

| Agent | 拥有 |
|---|---|
| A1 | `scanner/*` |
| A2 | `cleaning/{paths,junk,large_old,trash,safety}.rs` |
| A3 | `cleaning/{duplicates,uninstaller,startup}.rs` |
| A4 | `agent/*` |
| B1 | `App.tsx`、`layout/*`、`ui/*`、`i18n/*`、`useTheme.ts` |
| B2 | `scan/*`、`useScan.ts`、`scanStore.ts` |
| B3 | `cleanup/*`、`settings/*`、`settingsStore.ts` |
| B4 | `agent/*`（前端）、`useAgent.ts`、`agentStore.ts` |
| C1 | `icons/*`、`assets/branding/*` |
| C2 | `.github/workflows/*`、lint 配置、`tauri.conf.json` |
| C3 | `docs/*`、`README.md`、`CONTRIBUTING.md` |
| D1 | `tests/e2e/*`、`playwright.config.ts` |

---

## 9. 报告 Bug

提 Issue 时请包含：

1. **平台与版本**：macOS/Windows/Linux + TrueClean 版本。
2. **复现步骤**：一步步说明。
3. **预期 vs 实际**。
4. **截图/日志**（如有）。
5. **是否涉及数据丢失**（如是，标注「严重」）。

### 安全漏洞

**请勿公开 Issue 报告安全漏洞**。私信维护者，确认修复后再公开披露。

---

## 10. 行为准则

- 尊重所有贡献者，不论经验水平。
- 聚焦技术讨论，不人身攻击。
- 欢迎新手提问，耐心解答。
- 代码审查对事不对人，给出具体可执行的建议。

---

## 11. 许可

贡献的代码遵循项目 [MIT 许可证](LICENSE)。提交即表示你同意以 MIT 许可发布你的贡献。
