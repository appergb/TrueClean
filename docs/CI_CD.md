# TrueClean CI/CD 指南

本文档说明 TrueClean 的持续集成、发布流程、代码签名与自动更新配置。

---

## 1. CI 工作流（`.github/workflows/ci.yml`）

### 触发条件
- **push 到 master**
- **所有 Pull Request**

### 矩阵
| 平台 | Runner |
|------|--------|
| Linux | `ubuntu-22.04` |
| macOS | `macos-14` (Apple Silicon) |
| Windows | `windows-latest` |

### 检查项（三平台均执行）
1. **前端 Lint** — `pnpm lint`（ESLint flat config）
2. **前端构建** — `pnpm build`（`tsc --noEmit && vite build`）
3. **Rust 格式检查** — `cargo fmt --all -- --check`
4. **Rust Clippy** — `cargo clippy --all-targets -- -D warnings`
5. **Rust 测试** — `cargo test`

### 缓存
- **pnpm store** — 通过 `actions/setup-node` 的 `cache: pnpm`
- **Cargo registry + target** — 通过 `Swatinem/rust-cache@v2`

### Linux 系统依赖
Linux runner 需要安装 Tauri 2 编译所需的系统库：
```
libwebkit2gtk-4.1-dev build-essential curl wget file
libxdo-dev libssl-dev libayatana-appindicator3-dev librsvg2-dev
```

---

## 2. Release 工作流（`.github/workflows/release.yml`）

### 触发条件
- **推送 `v*` 格式的 tag**（如 `v0.1.0`、`v1.0.0-beta`）

### 发版步骤
```bash
# 1. 确保 master 分支 CI 全绿
# 2. 创建并推送 tag
git tag v0.1.0
git push origin v0.1.0
# 3. Release 工作流自动触发，构建三平台安装包
# 4. 构建完成后，在 GitHub Releases 页面查看 Draft Release
# 5. 检查无误后点击 "Publish release" 正式发布
```

### 产物
| 平台 | 产物格式 |
|------|----------|
| macOS | `.dmg` |
| Windows | `.msi` / `.exe` (NSIS) |
| Linux | `.AppImage` / `.deb` |

同时生成 `latest.json`（自动更新清单），随 Release 一起发布。

### tauri-action
使用 `tauri-apps/tauri-action@v0` 自动完成：构建 → 签名 → 上传 Release Assets → 生成 `latest.json`。

---

## 3. 签名 Secrets 配置

在 GitHub 仓库 **Settings → Secrets and variables → Actions** 中配置以下 Secrets：

### 3.1 自动更新签名（必需）

| Secret 名称 | 说明 |
|-------------|------|
| `TAURI_PRIVATE_KEY` | Tauri updater 签名私钥（`tauri signer generate` 生成的 `.key` 文件内容） |
| `TAURI_KEY_PASSWORD` | 私钥密码（如用 `--ci` 无密码生成则为空字符串） |

### 3.2 macOS 代码签名 + 公证（macOS 发版必需）

| Secret 名称 | 说明 |
|-------------|------|
| `APPLE_CERTIFICATE` | Base64 编码的 `.p12` 开发者证书 |
| `APPLE_CERTIFICATE_PASSWORD` | `.p12` 证书导出密码 |
| `APPLE_SIGNING_IDENTITY` | 签名身份名称（如 `Developer ID Application: Your Name (XXXXXXXXXX)`） |
| `APPLE_ID` | Apple ID 邮箱（用于公证） |
| `APPLE_PASSWORD` | App-Specific Password（在 appleid.apple.com 生成） |
| `APPLE_TEAM_ID` | Apple 开发者团队 ID |

### 3.3 Windows 代码签名（可选）

Windows 代码签名需要额外的 Authenticode 证书。如暂无证书，Release 仍可正常构建，但用户安装时会看到 SmartScreen 警告。配置方式见 [Tauri 2 Windows 签名文档](https://v2.tauri.app/distribute/sign-windows)。

---

## 4. 自动更新签名密钥

### 生成密钥对

```bash
# 生成 updater 签名密钥（交互式，推荐设置密码）
pnpm tauri signer generate -w ~/.tauri/trueclean-updater.key

# 或在 CI 环境中无密码生成
pnpm tauri signer generate --ci -w ~/.tauri/trueclean-updater.key --force
```

生成后会得到两个文件：
- **私钥**（`.key`）→ 内容设为 GitHub Secret `TAURI_PRIVATE_KEY`，**切勿提交到仓库**
- **公钥**（`.key.pub`）→ 内容已写入 `src-tauri/tauri.conf.json` 的 `plugins.updater.pubkey`

> ⚠️ **重要**：如果私钥丢失，已发布的更新将无法被验证，需要重新生成密钥对并发布新版本。请安全备份私钥。

### 当前配置

- **公钥**：已嵌入 `src-tauri/tauri.conf.json` → `plugins.updater.pubkey`
- **更新端点**：`https://github.com/appergb/TrueClean/releases/latest/download/latest.json`
- **Tauri 插件依赖**：`tauri-plugin-updater = "2"`（见 `src-tauri/Cargo.toml`）
- **权限**：`updater:default`（见 `src-tauri/capabilities/default.json`）

### 运行时注册（待完成）

> **注意**：`tauri-plugin-updater` 需要在 `src-tauri/src/lib.rs` 的 `tauri::Builder` 链中注册：
> ```rust
> .plugin(tauri_plugin_updater::init())
> ```
> 由于 `lib.rs` 是冻结文件（归编排者管理），此注册需由编排者或 D1 集成阶段添加。
> 在此之前，updater 配置和签名已就绪，但运行时检查更新功能尚未激活。

---

## 5. 代码规范

### 前端（ESLint + Prettier）

- **配置文件**：`eslint.config.js`（flat config）、`.prettierrc.json`
- **脚本**：
  - `pnpm lint` — 检查所有 lint 问题
  - `pnpm lint:fix` — 自动修复可修复的问题
  - `pnpm format` — Prettier 格式化
  - `pnpm format:check` — 检查格式

**核心规则**：
| 规则 | 级别 | 说明 |
|------|------|------|
| `no-console` | error | 禁止 `console.log`（允许 `console.warn`/`console.error`） |
| `@typescript-eslint/no-explicit-any` | error | 禁止使用 `any` |
| `react-hooks/rules-of-hooks` | error | 强制 Hooks 规则 |
| `react-hooks/exhaustive-deps` | warn | 依赖完整性检查 |
| `simple-import-sort/imports` | warn | import 排序（待全量修复后升级为 error） |
| `react-refresh/only-export-components` | warn | Fast Refresh 兼容性 |

### Rust（rustfmt + clippy）

- **配置文件**：`clippy.toml`（MSRV = 1.77）
- **CI 检查**：`cargo fmt --all -- --check` + `cargo clippy --all-targets -- -D warnings`

---

## 6. Issue / PR 模板

- **Bug 报告**：`.github/ISSUE_TEMPLATE/bug_report.yml`
- **功能建议**：`.github/ISSUE_TEMPLATE/feature_request.yml`
- **PR 模板**：`.github/pull_request_template.md`（含契约合规与验收门禁清单）
