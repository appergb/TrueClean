import { beforeEach, describe, expect, it, vi } from "vitest";

// Mock IPC 模块，避免依赖 Tauri 运行时
vi.mock("../lib/ipc", () => ({
  cleanPaths: vi.fn().mockResolvedValue({
    removedCount: 0,
    freedBytes: 0,
    failed: [],
    toTrash: false,
  }),
}));

import type { DirNode } from "../lib/types";
import { useCleanStore } from "./cleanStore";

const mockNode = (path: string): DirNode => ({
  name: path.split("/").pop() ?? path,
  path,
  sizeBytes: 100,
  fileCount: 1,
  category: "other",
  isDir: false,
  children: [],
  truncatedChildren: 0,
});

describe("cleanStore", () => {
  beforeEach(() => {
    useCleanStore.getState().reset();
  });

  it("初始状态 checked 为空", () => {
    expect(Object.keys(useCleanStore.getState().checked)).toHaveLength(0);
  });

  it("初始状态 removed 为空", () => {
    expect(Object.keys(useCleanStore.getState().removed)).toHaveLength(0);
  });

  it("初始状态 showConfirm 为 false", () => {
    expect(useCleanStore.getState().showConfirm).toBe(false);
  });

  it("初始状态 cleaning 为 false", () => {
    expect(useCleanStore.getState().cleaning).toBe(false);
  });

  it("toggleCheck 勾选节点", () => {
    useCleanStore.getState().toggleCheck(mockNode("/test"));
    expect(useCleanStore.getState().checked["/test"]).toBe(true);
    expect(useCleanStore.getState().isChecked("/test")).toBe(true);
  });

  it("toggleCheck 再次调用取消勾选", () => {
    useCleanStore.getState().toggleCheck(mockNode("/test"));
    useCleanStore.getState().toggleCheck(mockNode("/test"));
    expect(useCleanStore.getState().checked["/test"]).toBeUndefined();
    expect(useCleanStore.getState().isChecked("/test")).toBe(false);
  });

  it("selectNone 清空 checked", () => {
    useCleanStore.getState().toggleCheck(mockNode("/test"));
    useCleanStore.getState().selectNone();
    expect(Object.keys(useCleanStore.getState().checked)).toHaveLength(0);
  });

  it("selectAll 替换 checked", () => {
    useCleanStore.getState().toggleCheck(mockNode("/old"));
    useCleanStore.getState().selectAll([mockNode("/a"), mockNode("/b")]);
    expect(useCleanStore.getState().checked["/a"]).toBe(true);
    expect(useCleanStore.getState().checked["/b"]).toBe(true);
    expect(useCleanStore.getState().checked["/old"]).toBeUndefined();
  });

  it("checkMany 批量勾选（保留已有）", () => {
    useCleanStore.getState().toggleCheck(mockNode("/existing"));
    useCleanStore.getState().checkMany([mockNode("/a"), mockNode("/b")]);
    expect(useCleanStore.getState().checked["/existing"]).toBe(true);
    expect(useCleanStore.getState().checked["/a"]).toBe(true);
    expect(useCleanStore.getState().checked["/b"]).toBe(true);
  });

  it("openConfirm 无勾选时不打开", () => {
    useCleanStore.getState().openConfirm();
    expect(useCleanStore.getState().showConfirm).toBe(false);
  });

  it("openConfirm 有勾选时打开", () => {
    useCleanStore.getState().toggleCheck(mockNode("/test"));
    useCleanStore.getState().openConfirm();
    expect(useCleanStore.getState().showConfirm).toBe(true);
  });

  it("closeConfirm 关闭确认", () => {
    useCleanStore.getState().toggleCheck(mockNode("/test"));
    useCleanStore.getState().openConfirm();
    useCleanStore.getState().closeConfirm();
    expect(useCleanStore.getState().showConfirm).toBe(false);
  });

  it("reset 清空所有状态", () => {
    useCleanStore.getState().toggleCheck(mockNode("/test"));
    useCleanStore.getState().openConfirm();
    useCleanStore.getState().reset();
    expect(Object.keys(useCleanStore.getState().checked)).toHaveLength(0);
    expect(useCleanStore.getState().showConfirm).toBe(false);
    expect(useCleanStore.getState().cleaning).toBe(false);
  });

  it("isRemoved 初始返回 false", () => {
    expect(useCleanStore.getState().isRemoved("/test")).toBe(false);
  });

  it("clearToast 清空 toast", () => {
    useCleanStore.getState().clearToast();
    expect(useCleanStore.getState().toast.show).toBe(false);
  });
});
