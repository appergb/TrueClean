import { beforeEach, describe, expect, it, vi } from "vitest";

// Mock IPC 模块，避免依赖 Tauri 运行时
// 注意：vi.mock 会被提升到文件顶部，工厂内不能引用外部变量
vi.mock("../lib/ipc", () => ({
  getHelperStatus: vi.fn().mockResolvedValue({
    installed: false,
    version: null,
    path: "",
  }),
  getPermissionStatus: vi.fn().mockResolvedValue({
    fullDiskAccess: true,
    isAdmin: false,
    platform: "macos",
    needsHelper: false,
    skippedPaths: [],
  }),
  openSystemPermissionSettings: vi.fn().mockResolvedValue(undefined),
}));

import { usePermissions } from "./usePermissions";

describe("usePermissions", () => {
  beforeEach(() => {
    usePermissions.setState({
      status: null,
      helper: null,
      loading: false,
      error: null,
    });
  });

  it("初始 status 为 null", () => {
    expect(usePermissions.getState().status).toBeNull();
  });

  it("初始 helper 为 null", () => {
    expect(usePermissions.getState().helper).toBeNull();
  });

  it("初始 loading 为 false", () => {
    expect(usePermissions.getState().loading).toBe(false);
  });

  it("初始 error 为 null", () => {
    expect(usePermissions.getState().error).toBeNull();
  });

  it("refresh 加载权限状态", async () => {
    await usePermissions.getState().refresh();
    const state = usePermissions.getState();
    expect(state.status).not.toBeNull();
    expect(state.status?.fullDiskAccess).toBe(true);
    expect(state.status?.platform).toBe("macos");
    expect(state.helper).not.toBeNull();
    expect(state.helper?.installed).toBe(false);
    expect(state.loading).toBe(false);
    expect(state.error).toBeNull();
  });

  it("refresh 过程中 loading 先变 true 再变 false", async () => {
    const promise = usePermissions.getState().refresh();
    // refresh 调用后立即设置 loading: true
    expect(usePermissions.getState().loading).toBe(true);
    await promise;
    expect(usePermissions.getState().loading).toBe(false);
  });

  it("refresh 失败时设置 error", async () => {
    const { getPermissionStatus } = await import("../lib/ipc");
    vi.mocked(getPermissionStatus).mockRejectedValueOnce(new Error("网络错误"));

    await usePermissions.getState().refresh();
    const state = usePermissions.getState();
    expect(state.status).toBeNull();
    expect(state.loading).toBe(false);
    expect(state.error).toContain("网络错误");
  });

  it("openSettings 调用 IPC", async () => {
    const { openSystemPermissionSettings } = await import("../lib/ipc");
    await usePermissions.getState().openSettings("fullDiskAccess");
    expect(openSystemPermissionSettings).toHaveBeenCalledWith("fullDiskAccess");
  });
});
