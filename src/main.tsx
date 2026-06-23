// 字体打包引入：Inter Variable（UI 文本）+ JetBrains Mono（数字/路径/代码）。
// 通过 @fontsource 打包 woff2，保证跨平台一致渲染，不依赖系统本地安装。
import "@fontsource-variable/inter";
import "@fontsource/jetbrains-mono/400.css";
import "@fontsource/jetbrains-mono/500.css";
import "@fontsource/jetbrains-mono/600.css";
import "./styles/global.css";

import React from "react";
import ReactDOM from "react-dom/client";

import App from "./App";

// 平台检测：在首屏前为 <html> 添加平台 class，供 CSS 按平台适配窗口控件布局
// （macOS 左侧红绿灯空间 / Windows 右侧 decorum 控件空间）。同步执行避免布局闪烁。
(function detectPlatform() {
  const uaData = (navigator as unknown as { userAgentData?: { platform?: string } })
    .userAgentData;
  const ua = uaData?.platform ?? navigator.platform ?? "";
  const platform = /mac/i.test(ua)
    ? "macos"
    : /win/i.test(ua)
      ? "windows"
      : "linux";
  document.documentElement.classList.add(`platform-${platform}`);
})();

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
  <React.StrictMode>
    <App />
  </React.StrictMode>,
);
