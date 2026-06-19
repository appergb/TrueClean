import { beforeEach, describe, expect, it, vi } from "vitest";

// Mock IPC 模块，避免依赖 Tauri 运行时
vi.mock("../lib/ipc", () => ({
  cancelScan: vi.fn().mockResolvedValue(undefined),
  getVolumes: vi.fn().mockResolvedValue([]),
  onScanProgress: vi.fn().mockResolvedValue(() => {}),
  scanPath: vi.fn().mockResolvedValue(null),
}));

import { useScanStore } from "./scanStore";

describe("scanStore", () => {
  beforeEach(() => {
    useScanStore.getState().reset();
  });

  it("初始状态为 idle", () => {
    expect(useScanStore.getState().status).toBe("idle");
  });

  it("初始 result 为 null", () => {
    expect(useScanStore.getState().result).toBeNull();
  });

  it("初始 scanTarget 为 null", () => {
    expect(useScanStore.getState().scanTarget).toBeNull();
  });

  it("初始 drillPath 为 null", () => {
    expect(useScanStore.getState().drillPath).toBeNull();
  });

  it("reset 清空状态", () => {
    useScanStore.setState({ status: "done", scanTarget: "/test" });
    useScanStore.getState().reset();
    expect(useScanStore.getState().status).toBe("idle");
    expect(useScanStore.getState().result).toBeNull();
    expect(useScanStore.getState().scanTarget).toBeNull();
    expect(useScanStore.getState().drillPath).toBeNull();
  });

  it("setDrillPath 更新下钻路径", () => {
    useScanStore.getState().setDrillPath("/test/sub");
    expect(useScanStore.getState().drillPath).toBe("/test/sub");
  });

  it("setDrillPath 接受 null 回到根", () => {
    useScanStore.getState().setDrillPath("/test/sub");
    useScanStore.getState().setDrillPath(null);
    expect(useScanStore.getState().drillPath).toBeNull();
  });
});
