# TrueClean 品牌视觉指南（Brand Guide）

> 版本 1.0 · 2026-06-18 · 维护：C1 品牌/视觉
> 设计源：`src/assets/branding/icon.svg`（单一真相源，栅格化由 `generate_icons.py` 复现）

---

## 1. 品牌理念

TrueClean = **磁盘清理 + AI Agent**。图标用一组极简几何同时表达两件事：

| 元素 | 含义 |
|---|---|
| **圆角方块 + teal→green 渐变底** | 现代桌面应用标识；teal（清新）→ green（健康/已清理），寓意「清理后恢复健康」 |
| **白色扫掠环 + 右下缺口** | 一只「磁盘」被扫出一道缺口 = 正在清理 / 已清理出空间；缺口也暗示「腾出的空间」 |
| **白色 4 角 AI 火花（主+副）** | AI 智能判断；副火花靠近清扫缺口，寓意「AI 完成清理」 |

整体风格：简洁、现代、在小尺寸（16×16）下仍可辨识——粗描边圆环 + 高对比白色火花是辨识核心。

## 2. 标志变体

| 文件 | 用途 | 说明 |
|---|---|---|
| `icon.svg` | 应用图标设计源 | 512×512 viewBox，含圆角方块底 |
| `logo.svg` | 横版品牌锁图 | mark + 「TrueClean」字标，深色「True」+ 渐变「Clean」 |
| `favicon.svg` | 网站 favicon | 简化 mark，64×64 |
| `splash.svg` | 启动画面（可选） | 1280×832，居中 mark + 字标 + tagline |

## 3. 配色（Color）

设计令牌采用 OKLCH（与 `proposal/trueclean-proposal.html` 一致）。栅格化/不支持 OKLCH 的渲染器使用下列 hex 近似值。

| 令牌 | OKLCH | Hex（近似） | 用途 |
|---|---|---|---|
| accent（teal） | `oklch(64% 0.15 195)` | `#13A6B8` | 主色，渐变起点 |
| good（green） | `oklch(66% 0.17 150)` | `#1CB46A` | 渐变终点，健康/已清理 |
| accent-strong | `oklch(56% 0.16 195)` | `#0E8092` | 深青，强调/悬停 |
| warn | `oklch(74% 0.16 75)` | `#D9A22E` | 建议复核 |
| danger | `oklch(58% 0.2 25)` | `#D8432B` | 不要动 |
| text-dark | — | `#0C3740` | 字标「True」深色 |
| white | — | `#FFFFFF` | 圆环 / 火花 |

**主渐变**：`linear-gradient(135deg, #13A6B8 0%, #1CB46A 100%)`（左上→右下）。

## 4. 字体（Typography）

- **UI / 字标**：`Inter, -apple-system, BlinkMacSystemFont, "Segoe UI", "PingFang SC", "Microsoft YaHei", system-ui, sans-serif`
- **等宽**：`"SF Mono", "JetBrains Mono", ui-monospace, Menlo, Consolas, monospace`
- 字标字重 700，字距 -1.5~-2px。

## 5. 用法与安全边距

- 应用图标圆角方块已自带，**不要再套圆角**。
- 在纯色背景上使用 mark 时，四周留 ≥ mark 边长 25% 的留白。
- mark 渐变底已保证对比度，**不要把 mark 放在浅色背景上而不加底**——它自带底。
- 深色背景下使用字标时，「True」可改为 `#E8F0F2`。
- 禁止拉伸、倾斜、加描边、改色。

## 6. 图标生成

栅格化由 `generate_icons.py` 完成（依赖 Pillow；macOS 需 `iconutil`）：

```bash
cd <repo>
python3 src/assets/branding/generate_icons.py
```

脚本会：
1. 写出 4 个 SVG 到 `src/assets/branding/`；
2. 用 Pillow 按 `icon.svg` 相同几何渲染 1024 母版（对角渐变底 + 圆角蒙版 + 圆环+缺口+圆头端帽 + AI 火花）；
3. 缩放出各尺寸 PNG、用 `iconutil` 打包 `icon.icns`、用 Pillow 写多尺寸 `icon.ico`。

> 注：因环境无 SVG 渲染器（rsvg/cairo/sharp 不可用），采用 Pillow 直接绘制相同几何，保证 SVG 源与 PNG 输出一致。如需重渲染 SVG，可装 `librsvg`/`sharp` 后改造脚本。
