import { useCallback, useMemo, useState } from "react";
import { confirm, open } from "@tauri-apps/plugin-dialog";
import Button from "../ui/Button";
import { findLargeOldFiles, cleanPaths } from "../../lib/ipc";
import { useSettingsStore } from "../../store/settingsStore";
import type { FileEntry } from "../../lib/types";
import { formatBytes, formatRelativeTime } from "../../lib/format";
import "./cleanup.css";

const MB = 1024 * 1024;

function errMsg(error: unknown): string {
  if (error instanceof Error) return error.message;
  if (typeof error === "object" && error && "message" in error) {
    return String((error as { message: unknown }).message);
  }
  return "查找失败";
}

type Status = "idle" | "loading" | "ready" | "error";

export default function LargeOldFiles() {
  const [path, setPath] = useState("");
  const [minSizeMb, setMinSizeMb] = useState(100);
  const [olderThanDays, setOlderThanDays] = useState(180);
  const [status, setStatus] = useState<Status>("idle");
  const [error, setError] = useState<string | null>(null);
  const [files, setFiles] = useState<FileEntry[]>([]);
  const [selected, setSelected] = useState<Set<string>>(new Set());
  const [cleaning, setCleaning] = useState(false);

  const defaultToTrash = useSettingsStore(
    (s) => s.settings?.defaultToTrash ?? true,
  );

  const pickDir = useCallback(async () => {
    try {
      const dir = await open({ directory: true, multiple: false });
      if (typeof dir === "string") setPath(dir);
    } catch (e: unknown) {
      setError(errMsg(e));
    }
  }, []);

  const runSearch = useCallback(async () => {
    if (!path.trim()) {
      setError("请先选择要搜索的目录");
      setStatus("error");
      return;
    }
    setStatus("loading");
    setError(null);
    try {
      const result = await findLargeOldFiles(
        path,
        Math.max(0, minSizeMb) * MB,
        Math.max(0, olderThanDays),
      );
      setFiles(result);
      setSelected(new Set());
      setStatus("ready");
    } catch (e: unknown) {
      setError(errMsg(e));
      setStatus("error");
    }
  }, [path, minSizeMb, olderThanDays]);

  const toggle = useCallback((p: string) => {
    setSelected((prev) => {
      const next = new Set(prev);
      if (next.has(p)) next.delete(p);
      else next.add(p);
      return next;
    });
  }, []);

  const { selectedPaths, selectedBytes } = useMemo(() => {
    let bytes = 0;
    const paths: string[] = [];
    for (const f of files) {
      if (selected.has(f.path)) {
        bytes += f.sizeBytes;
        paths.push(f.path);
      }
    }
    return { selectedPaths: paths, selectedBytes: bytes };
  }, [files, selected]);

  const handleDelete = useCallback(async () => {
    if (selectedPaths.length === 0) return;
    const dest = defaultToTrash ? "移至废纸篓" : "永久删除";
    const ok = await confirm(
      `将对 ${selectedPaths.length} 个文件执行「${dest}」，预计释放 ${formatBytes(
        selectedBytes,
      )}。删除前请确认这些文件不再需要。`,
      { title: "确认删除", kind: "warning" },
    );
    if (!ok) return;

    setCleaning(true);
    try {
      const report = await cleanPaths(selectedPaths, defaultToTrash);
      setFiles((prev) => prev.filter((f) => !selected.has(f.path)));
      setSelected(new Set());
      const note =
        report.failed.length > 0 ? `，${report.failed.length} 项失败` : "";
      await confirm(
        `已删除 ${report.removedCount} 项，释放 ${formatBytes(
          report.freedBytes,
        )}${note}。`,
        { title: "删除完成", kind: "info" },
      );
    } catch (e: unknown) {
      setError(errMsg(e));
      setStatus("error");
    } finally {
      setCleaning(false);
    }
  }, [selectedPaths, selectedBytes, defaultToTrash, selected]);

  return (
    <section className="cln">
      <header className="cln-head">
        <div className="cln-head__titles">
          <h2 className="cln-title">大文件 & 旧文件</h2>
          <p className="cln-sub">按最小体积与未修改天数查找占空间的文件。</p>
        </div>
      </header>

      <div className="cln-tools">
        <div className="cln-field">
          <span className="cln-field__label">目录</span>
          <div className="cln-pathrow">
            <input
              className="cln-input cln-input--path"
              value={path}
              placeholder="选择要扫描的目录"
              onChange={(e) => setPath(e.target.value)}
            />
            <Button variant="subtle" onClick={() => void pickDir()}>
              浏览…
            </Button>
          </div>
        </div>
        <div className="cln-field">
          <span className="cln-field__label">最小大小 (MB)</span>
          <input
            type="number"
            min={0}
            className="cln-input cln-input--num"
            value={minSizeMb}
            onChange={(e) => setMinSizeMb(Number(e.target.value) || 0)}
          />
        </div>
        <div className="cln-field">
          <span className="cln-field__label">早于 (天)</span>
          <input
            type="number"
            min={0}
            className="cln-input cln-input--num"
            value={olderThanDays}
            onChange={(e) => setOlderThanDays(Number(e.target.value) || 0)}
          />
        </div>
        <div className="cln-field">
          <span className="cln-field__label">&nbsp;</span>
          <Button
            variant="primary"
            onClick={() => void runSearch()}
            disabled={status === "loading"}
          >
            查找
          </Button>
        </div>
      </div>

      {status === "loading" && (
        <div className="cln-state">
          <div className="cln-spinner" />
          <p className="cln-state__title">正在查找文件…</p>
        </div>
      )}

      {status === "error" && (
        <div className="cln-state cln-state--error">
          <p className="cln-state__title">出错了</p>
          <p className="cln-state__msg">{error}</p>
        </div>
      )}

      {status === "idle" && (
        <div className="cln-state">
          <p className="cln-state__title">选择目录并设置条件</p>
          <p className="cln-state__msg">
            设定最小大小与未修改天数，点击「查找」开始。
          </p>
        </div>
      )}

      {status === "ready" && files.length === 0 && (
        <div className="cln-state">
          <p className="cln-state__title">没有匹配的文件</p>
          <p className="cln-state__msg">尝试降低最小大小或缩短天数阈值。</p>
        </div>
      )}

      {status === "ready" && files.length > 0 && (
        <>
          <div className="cln-list">
            {files.map((f) => (
              <label className="cln-card" key={f.path}>
                <input
                  type="checkbox"
                  className="cln-check"
                  checked={selected.has(f.path)}
                  onChange={() => toggle(f.path)}
                />
                <div className="cln-card__main">
                  <span className="cln-card__name">{f.name}</span>
                  <span className="cln-card__path" title={f.path}>
                    {f.path}
                  </span>
                </div>
                <div className="cln-card__aside">
                  <span className="cln-card__size">
                    {formatBytes(f.sizeBytes)}
                  </span>
                  <span className="cln-card__time">
                    {formatRelativeTime(f.modified)}
                  </span>
                </div>
              </label>
            ))}
          </div>

          <div className="cln-bar">
            <div className="cln-bar__info">
              <span className="cln-bar__count">
                共 {files.length} 个 · 已选 {selectedPaths.length} 个
              </span>
              <span className="cln-bar__size">{formatBytes(selectedBytes)}</span>
            </div>
            <div className="cln-bar__actions">
              <Button
                variant="primary"
                onClick={() => void handleDelete()}
                disabled={selectedPaths.length === 0 || cleaning}
              >
                {cleaning ? "删除中…" : "删除所选"}
              </Button>
            </div>
          </div>
        </>
      )}
    </section>
  );
}
