# -*- coding: utf-8 -*-
"""TrueClean 品牌资产生成器
- 写出 SVG 矢量源（icon/logo/favicon/splash）到 src/assets/branding/
- 用 Pillow 按 icon.svg 相同几何栅格化，生成全平台图标到 src-tauri/icons/
几何与 icon.svg 完全一致（512 设计空间，圆环+缺口+AI 火花）。
"""
import math, os, subprocess, sys

REPO = "/Users/lvbaiqing/TRUE 开发/TrueClean-C1"
BRAND = REPO + "/src/assets/branding"
ICONS = REPO + "/src-tauri/icons"

# ---- 设计令牌（oklch → hex 近似，供栅格化用；oklch 原值见 brand-guide.md）----
TEAL  = (0x13, 0xA6, 0xB8)   # oklch(64% 0.15 195)
GREEN = (0x1C, 0xB4, 0x6A)   # oklch(66% 0.17 150)
WHITE = (0xFF, 0xFF, 0xFF)
RING_BOT = (0xEA, 0xFF, 0xF5)

# ---- SVG 源文件内容 ----
SVG_ICON = '''<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 512 512" width="512" height="512" role="img" aria-label="TrueClean icon">
  <title>TrueClean</title>
  <defs>
    <linearGradient id="tc-bg" x1="0" y1="0" x2="1" y2="1">
      <stop offset="0" stop-color="#13A6B8"/>
      <stop offset="1" stop-color="#1CB46A"/>
    </linearGradient>
    <radialGradient id="tc-sheen" cx="0.28" cy="0.22" r="0.85">
      <stop offset="0" stop-color="#ffffff" stop-opacity="0.22"/>
      <stop offset="0.55" stop-color="#ffffff" stop-opacity="0"/>
    </radialGradient>
    <linearGradient id="tc-ring" x1="0" y1="0" x2="0" y2="1">
      <stop offset="0" stop-color="#ffffff"/>
      <stop offset="1" stop-color="#eafff5"/>
    </linearGradient>
  </defs>
  <rect x="0" y="0" width="512" height="512" rx="112" ry="112" fill="url(#tc-bg)"/>
  <rect x="0" y="0" width="512" height="512" rx="112" ry="112" fill="url(#tc-sheen)"/>
  <path d="M 268 394 A 138 138 0 1 0 394 268" fill="none" stroke="url(#tc-ring)" stroke-width="54" stroke-linecap="round"/>
  <path d="M 210,138 L 221,189 L 272,200 L 221,211 L 210,262 L 199,211 L 148,200 L 199,189 Z" fill="#ffffff"/>
  <path d="M 322,292 L 327,313 L 348,318 L 327,323 L 322,344 L 317,323 L 296,318 L 317,313 Z" fill="#ffffff" opacity="0.92"/>
</svg>
'''

SVG_LOGO = '''<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 560 140" width="560" height="140" role="img" aria-label="TrueClean logo">
  <title>TrueClean</title>
  <defs>
    <linearGradient id="tc-bg" x1="0" y1="0" x2="1" y2="1">
      <stop offset="0" stop-color="#13A6B8"/>
      <stop offset="1" stop-color="#1CB46A"/>
    </linearGradient>
    <linearGradient id="tc-word" x1="0" y1="0" x2="1" y2="0">
      <stop offset="0" stop-color="#13A6B8"/>
      <stop offset="1" stop-color="#1CB46A"/>
    </linearGradient>
  </defs>
  <g transform="translate(20,22) scale(0.1875)">
    <rect x="0" y="0" width="512" height="512" rx="112" ry="112" fill="url(#tc-bg)"/>
    <path d="M 268 394 A 138 138 0 1 0 394 268" fill="none" stroke="#ffffff" stroke-width="54" stroke-linecap="round"/>
    <path d="M 210,138 L 221,189 L 272,200 L 221,211 L 210,262 L 199,211 L 148,200 L 199,189 Z" fill="#ffffff"/>
    <path d="M 322,292 L 327,313 L 348,318 L 327,323 L 322,344 L 317,323 L 296,318 L 317,313 Z" fill="#ffffff" opacity="0.92"/>
  </g>
  <text x="140" y="93" font-family="Inter, -apple-system, BlinkMacSystemFont, 'Segoe UI', system-ui, sans-serif" font-size="62" font-weight="700" letter-spacing="-1.5">
    <tspan fill="#0C3740">True</tspan><tspan fill="url(#tc-word)">Clean</tspan>
  </text>
</svg>
'''

SVG_FAVICON = '''<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 512 512" width="64" height="64" role="img" aria-label="TrueClean">
  <title>TrueClean</title>
  <defs>
    <linearGradient id="tc-bg" x1="0" y1="0" x2="1" y2="1">
      <stop offset="0" stop-color="#13A6B8"/>
      <stop offset="1" stop-color="#1CB46A"/>
    </linearGradient>
  </defs>
  <rect x="0" y="0" width="512" height="512" rx="112" ry="112" fill="url(#tc-bg)"/>
  <path d="M 268 394 A 138 138 0 1 0 394 268" fill="none" stroke="#ffffff" stroke-width="54" stroke-linecap="round"/>
  <path d="M 210,138 L 221,189 L 272,200 L 221,211 L 210,262 L 199,211 L 148,200 L 199,189 Z" fill="#ffffff"/>
  <path d="M 322,292 L 327,313 L 348,318 L 327,323 L 322,344 L 317,323 L 296,318 L 317,313 Z" fill="#ffffff" opacity="0.92"/>
</svg>
'''

SVG_SPLASH = '''<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 1280 832" width="1280" height="832" role="img" aria-label="TrueClean splash">
  <title>TrueClean</title>
  <defs>
    <linearGradient id="tc-bg" x1="0" y1="0" x2="1" y2="1">
      <stop offset="0" stop-color="#13A6B8"/>
      <stop offset="1" stop-color="#1CB46A"/>
    </linearGradient>
    <linearGradient id="tc-word" x1="0" y1="0" x2="1" y2="0">
      <stop offset="0" stop-color="#13A6B8"/>
      <stop offset="1" stop-color="#1CB46A"/>
    </linearGradient>
    <radialGradient id="tc-glow" cx="0.5" cy="0.4" r="0.6">
      <stop offset="0" stop-color="#13A6B8" stop-opacity="0.10"/>
      <stop offset="1" stop-color="#13A6B8" stop-opacity="0"/>
    </radialGradient>
  </defs>
  <rect x="0" y="0" width="1280" height="832" fill="#F7F9FA"/>
  <rect x="0" y="0" width="1280" height="832" fill="url(#tc-glow)"/>
  <g transform="translate(560,236) scale(0.3125)">
    <rect x="0" y="0" width="512" height="512" rx="112" ry="112" fill="url(#tc-bg)"/>
    <path d="M 268 394 A 138 138 0 1 0 394 268" fill="none" stroke="#ffffff" stroke-width="54" stroke-linecap="round"/>
    <path d="M 210,138 L 221,189 L 272,200 L 221,211 L 210,262 L 199,211 L 148,200 L 199,189 Z" fill="#ffffff"/>
    <path d="M 322,292 L 327,313 L 348,318 L 327,323 L 322,344 L 317,323 L 296,318 L 317,313 Z" fill="#ffffff" opacity="0.92"/>
  </g>
  <text x="640" y="500" text-anchor="middle" font-family="Inter, -apple-system, BlinkMacSystemFont, 'Segoe UI', system-ui, sans-serif" font-size="64" font-weight="700" letter-spacing="-2">
    <tspan fill="#0C3740">True</tspan><tspan fill="url(#tc-word)">Clean</tspan>
  </text>
  <text x="640" y="556" text-anchor="middle" font-family="Inter, -apple-system, BlinkMacSystemFont, 'Segoe UI', system-ui, sans-serif" font-size="22" font-weight="500" fill="#5A6B72" letter-spacing="0.5">
    看清楚 · 清得安全 · 让 AI 帮你判断
  </text>
  <text x="640" y="588" text-anchor="middle" font-family="Inter, -apple-system, BlinkMacSystemFont, 'Segoe UI', system-ui, sans-serif" font-size="14" font-weight="500" fill="#94A3A8" letter-spacing="2">
    AI-POWERED DISK CLEANER
  </text>
</svg>
'''

def write_svg(name, content):
    p = os.path.join(BRAND, name)
    with open(p, "w", encoding="utf-8") as f:
        f.write(content)
    print("wrote", p, os.path.getsize(p), "bytes")

def lerp_c(c1, c2, t):
    return tuple(int(c1[i] + (c2[i]-c1[i])*t) for i in range(3))

def in_rounded(x, y, r, w=512, h=512):
    if not (0 <= x <= w and 0 <= y <= h):
        return False
    nx = x if x < w/2 else w - x
    ny = y if y < h/2 else h - y
    if nx >= r or ny >= r:
        return True
    cx = r if x < w/2 else w - r
    cy = r if y < h/2 else h - r
    return math.hypot(x-cx, y-cy) <= r

def build_master(size=1024):
    from PIL import Image, ImageDraw
    RES = Image.Resampling.LANCZOS
    S = size / 512.0

    # --- 背景对角渐变 teal->green（256² 渲染再放大，平滑且快）---
    small = Image.new("RGB", (256, 256))
    px = small.load()
    for y in range(256):
        for x in range(256):
            t = (x + y) / 510.0
            px[x, y] = lerp_c(TEAL, GREEN, t)
    grad = small.resize((size, size), RES)

    # --- 顶部高光 sheen（径向白，256² 再放大）---
    sheen = Image.new("L", (256, 256))
    spx = sheen.load()
    for y in range(256):
        for x in range(256):
            ux = x/256*512; uy = y/256*512
            dx = (ux - 143)/350.0; dy = (uy - 113)/350.0
            d = math.hypot(dx, dy)
            spx[x, y] = int(max(0.0, 1.0 - d) * 0.22 * 255)
    sheen = sheen.resize((size, size), RES)
    white = Image.new("RGB", (size, size), (255, 255, 255))
    grad = Image.composite(white, grad, sheen)

    # --- 圆角方块 alpha 蒙版（256² 再放大，抗锯齿圆角）---
    mask = Image.new("L", (256, 256), 0)
    mpx = mask.load()
    for y in range(256):
        for x in range(256):
            ux = x/256*512; uy = y/256*512
            mpx[x, y] = 255 if in_rounded(ux, uy, 112) else 0
    mask = mask.resize((size, size), RES)

    bg = Image.new("RGBA", (size, size), (0, 0, 0, 0))
    bg.paste(grad, (0, 0), mask)

    # --- 扫掠环（磁盘+清扫缺口）---
    cx, cy = 256*S, 256*S
    Rout = (138 + 27)*S   # 330
    Rin  = (138 - 27)*S   # 222
    rmask = Image.new("L", (size, size), 0)
    rd = ImageDraw.Draw(rmask)
    rd.ellipse([cx-Rout, cy-Rout, cx+Rout, cy+Rout], fill=255)
    rd.ellipse([cx-Rin, cy-Rin, cx+Rin, cy+Rin], fill=0)
    # 缺口扇形（右下，5°..85°，屏幕坐标 y-down 顺时针）
    pts = [(cx, cy)]
    for ang in range(5, 86):
        a = math.radians(ang)
        pts.append((cx + Rout*math.cos(a), cy + Rout*math.sin(a)))
    rd.polygon(pts, fill=0)
    # 圆头端帽（匹配 SVG stroke-linecap=round）
    for ang in (5, 85):
        a = math.radians(ang)
        ex = cx + 138*S*math.cos(a)
        ey = cy + 138*S*math.sin(a)
        cap = 27*S
        rd.ellipse([ex-cap, ey-cap, ex+cap, ey+cap], fill=255)

    # 环色：垂直白->#eafff5（1 像素条放大）
    strip = Image.new("RGB", (1, size))
    sp = strip.load()
    for y in range(size):
        t = y/float(size-1)
        sp[0, y] = lerp_c(WHITE, RING_BOT, t)
    ring_rgb = strip.resize((size, size), RES)
    ring = Image.new("RGBA", (size, size), (0, 0, 0, 0))
    ring.paste(ring_rgb, (0, 0), rmask)

    # --- AI 火花 ---
    spk = Image.new("RGBA", (size, size), (0, 0, 0, 0))
    sd = ImageDraw.Draw(spk)
    def star(cx_, cy_, R, r):
        out = []
        for i in range(8):
            ang = math.radians(45*i - 90)
            rad = R if i % 2 == 0 else r
            out.append((cx_ + rad*math.cos(ang), cy_ + rad*math.sin(ang)))
        return out
    sd.polygon(star(210*S, 200*S, 62*S, 16*S), fill=(255, 255, 255, 255))
    sd.polygon(star(322*S, 318*S, 26*S, 7*S), fill=(255, 255, 255, int(255*0.92)))

    master = Image.alpha_composite(bg, ring)
    master = Image.alpha_composite(master, spk)
    return master

def main():
    os.makedirs(BRAND, exist_ok=True)
    os.makedirs(ICONS, exist_ok=True)
    write_svg("icon.svg", SVG_ICON)
    write_svg("logo.svg", SVG_LOGO)
    write_svg("favicon.svg", SVG_FAVICON)
    write_svg("splash.svg", SVG_SPLASH)

    from PIL import Image
    RES = Image.Resampling.LANCZOS
    print("rendering master 1024...")
    master = build_master(1024)
    master_rgba = master.convert("RGBA")

    def save(px, name):
        p = os.path.join(ICONS, name)
        im = master_rgba.resize((px, px), RES) if px != 1024 else master_rgba
        im.save(p, "PNG")
        print("wrote", p, px, "x", px, os.path.getsize(p), "bytes")

    # 通用 / Linux / Tauri 标准集
    save(512, "icon.png")
    save(32,  "32x32.png")
    save(128, "128x128.png")
    save(256, "128x128@2x.png")
    save(64,  "[email protected]")

    # --- macOS .icns（iconutil）---
    iconset = os.path.join(ICONS, "TrueClean.iconset")
    os.makedirs(iconset, exist_ok=True)
    icns_sizes = [
        (16,  "icon_16x16"),      (32, "icon_16x16@2x"),
        (32,  "icon_32x32"),      (64, "icon_32x32@2x"),
        (128, "icon_128x128"),    (256,"icon_128x128@2x"),
        (256, "icon_256x256"),    (512,"icon_256x256@2x"),
        (512, "icon_512x512"),    (1024,"icon_512x512@2x"),
    ]
    for px, name in icns_sizes:
        im = master_rgba.resize((px, px), RES) if px != 1024 else master_rgba
        im.save(os.path.join(iconset, name + ".png"), "PNG")
    icns_out = os.path.join(ICONS, "icon.icns")
    r = subprocess.run(["iconutil", "-c", "icns", iconset, "-o", icns_out],
                       capture_output=True, text=True)
    if r.returncode != 0:
        print("iconutil ERROR:", r.stderr)
        sys.exit(1)
    print("wrote", icns_out, os.path.getsize(icns_out), "bytes")
    # 清理 iconset
    for f in os.listdir(iconset):
        os.remove(os.path.join(iconset, f))
    os.rmdir(iconset)

    # --- Windows .ico（Pillow 多尺寸）---
    ico_out = os.path.join(ICONS, "icon.ico")
    master_rgba.save(ico_out, format="ICO",
                     sizes=[(16,16),(32,32),(48,48),(64,64),(128,128),(256,256)])
    print("wrote", ico_out, os.path.getsize(ico_out), "bytes")

    print("\nDONE.")

if __name__ == "__main__":
    main()
