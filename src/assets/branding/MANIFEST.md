# TrueClean 图标清单（MANIFEST for C2 / CI）

> 由 C1 品牌/视觉生成 · 2026-06-18 · 分支 `agent/C1-branding`
> 所有路径相对仓库根。生成命令：`python3 src/assets/branding/generate_icons.py`

## 应用图标（打包用，位于 `src-tauri/icons/`）

| 文件 | 尺寸 | 格式 | 用途 |
|---|---|---|---|
| `src-tauri/icons/icon.png` | 512×512 | PNG | 通用 / 源 / store 列表 |
| `src-tauri/icons/32x32.png` | 32×32 | PNG | Linux / Tauri 标准 |
| `src-tauri/icons/128x128.png` | 128×128 | PNG | Linux / Tauri 标准 |
| `src-tauri/icons/128x128@2x.png` | 256×256 | PNG | Linux HiDPI / Tauri 标准 |
| `src-tauri/icons/[email protected]` | 64×64 | PNG | Tauri 标准 |
| `src-tauri/icons/icon.icns` | 16/32/64/128/256/512/1024 | macOS icns | macOS 打包（`bundle.macOS`） |
| `src-tauri/icons/icon.ico` | 16/32/48/64/128/256 | Windows ico | Windows 打包（`bundle.windows`） |

## 品牌视觉资产（位于 `src/assets/branding/`）

| 文件 | 说明 |
|---|---|
| `src/assets/branding/icon.svg` | 应用图标矢量源（512 viewBox，单一真相源） |
| `src/assets/branding/logo.svg` | 横版品牌锁图（mark + 字标） |
| `src/assets/branding/favicon.svg` | 网站 favicon |
| `src/assets/branding/splash.svg` | 启动画面（1280×832，可选） |
| `src/assets/branding/brand-guide.md` | 品牌视觉指南（配色/字体/用法） |
| `src/assets/branding/generate_icons.py` | 图标生成脚本（Pillow + iconutil） |
| `src/assets/branding/MANIFEST.md` | 本清单 |

## tauri.conf.json 改动

`bundle.icon` 数组已更新为：

```json
"icon": [
  "icons/32x32.png",
  "icons/128x128.png",
  "[email protected]",
  "icons/128x128@2x.png",
  "icons/icon.icns",
  "icons/icon.ico",
  "icons/icon.png"
]
```

## C2 CI 注意事项

- macOS 打包前确认 `icon.icns` 存在（含 1024 条目，已满足 Apple notarization 要求）。
- Windows 打包前确认 `icon.ico` 存在（含 256 条目）。
- 如需重新生成，运行 `python3 src/assets/branding/generate_icons.py`（依赖 `Pillow`；macOS 需系统自带 `iconutil`）。
- 不要手动修改 `src-tauri/icons/` 下的位图，以 `icon.svg` + 脚本为准。
