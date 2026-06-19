import { describe, expect, it } from "vitest";

import { formatBytes, formatPercent, formatRelativeTime } from "./format";

describe("formatBytes", () => {
  it("0 字节返回 0 B", () => {
    expect(formatBytes(0)).toBe("0 B");
  });

  it("负数返回 0 B", () => {
    expect(formatBytes(-100)).toBe("0 B");
  });

  it("NaN 返回 0 B", () => {
    expect(formatBytes(Number.NaN)).toBe("0 B");
  });

  it("格式化字节", () => {
    expect(formatBytes(500)).toBe("500 B");
  });

  it("格式化 KB", () => {
    expect(formatBytes(1024)).toBe("1.0 KB");
  });

  it("格式化 MB", () => {
    expect(formatBytes(1024 * 1024)).toBe("1.0 MB");
  });

  it("格式化 GB", () => {
    expect(formatBytes(1024 * 1024 * 1024)).toBe("1.0 GB");
  });

  it("格式化 TB", () => {
    expect(formatBytes(1024 * 1024 * 1024 * 1024)).toBe("1.0 TB");
  });

  it("支持自定义小数位", () => {
    expect(formatBytes(1536, 2)).toBe("1.50 KB");
  });

  it("字节单位不显示小数", () => {
    expect(formatBytes(0)).toBe("0 B");
    expect(formatBytes(999)).toBe("999 B");
  });
});

describe("formatRelativeTime", () => {
  it("null 返回未知", () => {
    expect(formatRelativeTime(null)).toBe("未知");
  });

  it("今天", () => {
    const now = Math.floor(Date.now() / 1000);
    expect(formatRelativeTime(now)).toBe("今天");
  });

  it("未来时间返回未来", () => {
    const future = Math.floor(Date.now() / 1000) + 86400;
    expect(formatRelativeTime(future)).toBe("未来");
  });

  it("天数小于 30 返回 X 天前", () => {
    const past = Math.floor(Date.now() / 1000) - 5 * 86400;
    expect(formatRelativeTime(past)).toBe("5 天前");
  });

  it("天数小于 365 返回 X 个月前", () => {
    const past = Math.floor(Date.now() / 1000) - 60 * 86400;
    expect(formatRelativeTime(past)).toBe("2 个月前");
  });

  it("天数大于等于 365 返回 X 年前", () => {
    const past = Math.floor(Date.now() / 1000) - 400 * 86400;
    expect(formatRelativeTime(past)).toBe("1 年前");
  });
});

describe("formatPercent", () => {
  it("格式化百分比", () => {
    expect(formatPercent(12.34)).toBe("12.3%");
  });

  it("0%", () => {
    expect(formatPercent(0)).toBe("0.0%");
  });

  it("100%", () => {
    expect(formatPercent(100)).toBe("100.0%");
  });
});
