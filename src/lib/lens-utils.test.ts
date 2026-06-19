import { describe, expect, it } from "vitest";

import {
  CAT_META,
  effChildren,
  effCount,
  effSize,
  findByPath,
  findTrail,
  fmtBytes,
  fmtNum,
  LEGEND_ORDER,
  rgba,
} from "./lens-utils";
import type { DirNode } from "./types";

// 构造测试用的 DirNode 树
const mockTree: DirNode = {
  name: "root",
  path: "/root",
  sizeBytes: 1000,
  fileCount: 10,
  category: "other",
  isDir: true,
  children: [
    {
      name: "a",
      path: "/root/a",
      sizeBytes: 300,
      fileCount: 3,
      category: "documents",
      isDir: false,
      children: [],
      truncatedChildren: 0,
    },
    {
      name: "b",
      path: "/root/b",
      sizeBytes: 700,
      fileCount: 7,
      category: "media",
      isDir: false,
      children: [],
      truncatedChildren: 0,
    },
  ],
  truncatedChildren: 0,
};

describe("effSize", () => {
  it("无移除时返回完整大小", () => {
    expect(effSize(mockTree, {})).toBe(1000);
  });

  it("移除根节点返回 0", () => {
    expect(effSize(mockTree, { "/root": true })).toBe(0);
  });

  it("移除子节点时减去对应大小", () => {
    expect(effSize(mockTree, { "/root/a": true })).toBe(700);
  });

  it("移除所有子节点返回 0", () => {
    expect(
      effSize(mockTree, { "/root/a": true, "/root/b": true }),
    ).toBe(0);
  });

  it("对叶子节点返回自身大小", () => {
    expect(effSize(mockTree.children[0], {})).toBe(300);
  });
});

describe("effCount", () => {
  it("无移除时返回完整文件数", () => {
    expect(effCount(mockTree, {})).toBe(10);
  });

  it("移除根节点返回 0", () => {
    expect(effCount(mockTree, { "/root": true })).toBe(0);
  });

  it("移除子节点时减去对应文件数", () => {
    expect(effCount(mockTree, { "/root/a": true })).toBe(7);
  });

  it("对叶子节点返回自身 fileCount", () => {
    expect(effCount(mockTree.children[0], {})).toBe(3);
  });
});

describe("effChildren", () => {
  it("无移除时返回全部子节点", () => {
    expect(effChildren(mockTree, {})).toHaveLength(2);
  });

  it("过滤已移除的子节点", () => {
    const result = effChildren(mockTree, { "/root/a": true });
    expect(result).toHaveLength(1);
    expect(result[0].name).toBe("b");
  });

  it("全部移除返回空数组", () => {
    expect(
      effChildren(mockTree, { "/root/a": true, "/root/b": true }),
    ).toHaveLength(0);
  });
});

describe("findByPath", () => {
  it("找到根节点", () => {
    expect(findByPath(mockTree, "/root")?.path).toBe("/root");
  });

  it("找到子节点", () => {
    expect(findByPath(mockTree, "/root/a")?.name).toBe("a");
  });

  it("路径不存在返回 null", () => {
    expect(findByPath(mockTree, "/nonexistent")).toBeNull();
  });
});

describe("findTrail", () => {
  it("根节点 trail 仅包含自身", () => {
    const trail = findTrail(mockTree, "/root");
    expect(trail).toHaveLength(1);
    expect(trail[0].path).toBe("/root");
  });

  it("子节点 trail 从根开始", () => {
    const trail = findTrail(mockTree, "/root/a");
    expect(trail).toHaveLength(2);
    expect(trail[0].path).toBe("/root");
    expect(trail[1].path).toBe("/root/a");
  });

  it("路径不存在返回空数组", () => {
    expect(findTrail(mockTree, "/nonexistent")).toHaveLength(0);
  });
});

describe("CAT_META", () => {
  it("包含所有类别", () => {
    expect(CAT_META.system.categoryKey).toBe("system");
    expect(CAT_META.applications.categoryKey).toBe("applications");
    expect(CAT_META.developer.categoryKey).toBe("developer");
    expect(CAT_META.documents.categoryKey).toBe("documents");
    expect(CAT_META.media.categoryKey).toBe("media");
    expect(CAT_META.caches.categoryKey).toBe("caches");
    expect(CAT_META.logs.categoryKey).toBe("logs");
    expect(CAT_META.trash.categoryKey).toBe("trash");
    expect(CAT_META.downloads.categoryKey).toBe("downloads");
    expect(CAT_META.archives.categoryKey).toBe("archives");
    expect(CAT_META.other.categoryKey).toBe("other");
  });

  it("每个类别都有 categoryKey、color、iconPath", () => {
    for (const key of Object.keys(CAT_META) as Array<keyof typeof CAT_META>) {
      const meta = CAT_META[key];
      expect(typeof meta.categoryKey).toBe("string");
      expect(typeof meta.color).toBe("string");
      expect(meta.color.startsWith("#")).toBe(true);
      expect(typeof meta.iconPath).toBe("string");
      expect(meta.iconPath.length).toBeGreaterThan(0);
    }
  });
});

describe("LEGEND_ORDER", () => {
  it("包含 10 个类别且无重复（不含 trash）", () => {
    expect(LEGEND_ORDER).toHaveLength(10);
    const set = new Set(LEGEND_ORDER);
    expect(set.size).toBe(LEGEND_ORDER.length);
  });

  it("以 system 开头，以 other 结尾", () => {
    expect(LEGEND_ORDER[0]).toBe("system");
    expect(LEGEND_ORDER[LEGEND_ORDER.length - 1]).toBe("other");
  });

  it("不包含 trash（废纸篓不参与图例展示）", () => {
    expect(LEGEND_ORDER).not.toContain("trash");
  });
});

describe("rgba", () => {
  it("将 hex 转为 rgba 字符串", () => {
    expect(rgba("#ff0000", 0.5)).toBe("rgba(255,0,0,0.5)");
  });

  it("处理大写 hex", () => {
    expect(rgba("#00FF00", 1)).toBe("rgba(0,255,0,1)");
  });

  it("处理不带 # 的 hex", () => {
    expect(rgba("0000ff", 0.2)).toBe("rgba(0,0,255,0.2)");
  });
});

describe("fmtBytes", () => {
  it("字节", () => {
    expect(fmtBytes(0)).toBe("0 B");
    expect(fmtBytes(500)).toBe("500 B");
    expect(fmtBytes(1023)).toBe("1023 B");
  });

  it("KB", () => {
    expect(fmtBytes(1024)).toBe("1 KB");
    expect(fmtBytes(2048)).toBe("2 KB");
  });

  it("MB", () => {
    expect(fmtBytes(1048576)).toBe("1 MB");
    expect(fmtBytes(5242880)).toBe("5 MB");
  });

  it("GB（小于 100 保留一位小数）", () => {
    expect(fmtBytes(1073741824)).toBe("1.0 GB");
    expect(fmtBytes(1610612736)).toBe("1.5 GB");
  });

  it("GB（大于等于 100 取整）", () => {
    expect(fmtBytes(107374182400)).toBe("100 GB");
  });
});

describe("fmtNum", () => {
  it("小数取整", () => {
    expect(fmtNum(1234.56)).toBe("1,235");
  });

  it("千位分隔符", () => {
    expect(fmtNum(1234567)).toBe("1,234,567");
  });

  it("小数字", () => {
    expect(fmtNum(0)).toBe("0");
    expect(fmtNum(42)).toBe("42");
  });
});
