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
