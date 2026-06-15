import { useCallback, useMemo, useState } from "react";
import { confirm, open } from "@tauri-apps/plugin-dialog";
import Button from "../ui/Button";
import { findDuplicates, cleanPaths } from "../../lib/ipc";
import { useSettingsStore } from "../../store/settingsStore";
import type { DuplicateGroup } from "../../lib/types";
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

export default function DuplicatesPanel() {
  const [path, setPath] = useState("");
  const [minSizeMb, setMinSizeMb] = useState(1);
  const [status, setStatus] = useState<Status>("idle");
  const [error, setError] = useState<string | null>(null);
  const [groups, setGroups] = useState<DuplicateGroup[]>([]);
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
      const result = await findDuplicates(path, Math.max(0, minSizeMb) * MB);
      setGroups(result);
      // Pre-select all but the first file in every group.
      const preset = new Set<string>();
      for (const g of result) {
        g.files.slice(1).forEach((f) => preset.add(f.path));
      }
      setSelected(preset);
      setStatus("ready");
    } catch (e: unknown) {
      setError(errMsg(e));
      setStatus("error");
    }
  }, [path, minSizeMb]);

  const toggle = useCallback((p: string) => {
    setSelected((prev) => {
      const next = new Set(prev);
      if (next.has(p)) next.delete(p);
      else next.add(p);
      return next;
    });
  }, []);

  const totalWasted = useMemo(
    () => groups.reduce((sum, g) => sum + g.wastedBytes, 0),
    [groups],
  );

  const { selectedPaths, selectedBytes } = useMemo(() => {
    let bytes = 0;
    const paths: string[] = [];
    for (const g of groups) {
      for (const f of g.files) {
        if (selected.has(f.path)) {
          bytes += f.sizeBytes;
          paths.push(f.path);
        }
      }
    }
    return { selectedPaths: paths, selectedBytes: bytes };
  }, [groups, selected]);

  const handleDelete = useCallback(async () => {
    if (selectedPaths.length === 0) return;
    const dest = defaultToTrash ? "移至废纸篓" : "永久删除";
    const ok = await confirm(
      `将对 ${selectedPaths.length} 个重复文件执行「${dest}」，预计释放 ${formatBytes(
        selectedBytes,
      )}。每组至少应保留一个副本。`,
      { title: "确认删除重复项", kind: "warning" },
    );
    if (!ok) return;

    setCleaning(true);
    try {
      const report = await cleanPaths(selectedPaths, defaultToTrash);
      const removed = new Set(selectedPaths);
      setGroups((prev) =>
        prev
          .map((g) => ({
            ...g,
            files: g.files.filter((f) => !removed.has(f.path)),
          }))
          .filter((g) => g.files.length > 1),
      );
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
  }, [selectedPaths, selectedBytes, defaultToTrash]);

  return (
    <section className="cln">
      <header className="cln-head">
        <div className="cln-head__titles">
          <h2 className="cln-title">重复文件</h2>
          <p className="cln-sub">
            按内容哈希查找完全相同的文件，每组默认保留第一个。
          </p>
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
          <p className="cln-state__title">正在比对文件内容…</p>
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
          <p className="cln-state__title">选择目录开始查重</p>
          <p className="cln-state__msg">
            较小的最小大小会发现更多重复，但扫描更慢。
          </p>
        </div>
      )}

      {status === "ready" && groups.length === 0 && (
        <div className="cln-state">
          <p className="cln-state__title">未发现重复文件</p>
          <p className="cln-state__msg">该目录下没有体积达标的重复内容。</p>
        </div>
      )}

      {status === "ready" && groups.length > 0 && (
        <>
          <div className="cln-list">
            {groups.map((g) => (
              <div className="cln-dup" key={g.hash}>
                <div className="cln-dup__head">
                  <div>
                    <div className="cln-dup__title">
                      {g.files.length} 个相同文件 · 各 {formatBytes(g.sizeBytes)}
                    </div>
                    <div className="cln-dup__hash">{g.hash.slice(0, 16)}…</div>
                  </div>
                  <span className="cln-dup__waste">
                    可回收 {formatBytes(g.wastedBytes)}
                  </span>
                </div>
                <div className="cln-group__items">
                  {g.files.map((f, idx) => (
                    <label className="cln-row" key={f.path}>
                      <input
                        type="checkbox"
                        className="cln-check"
                        checked={selected.has(f.path)}
                        onChange={() => toggle(f.path)}
                      />
                      <span className="cln-row__path" title={f.path}>
                        {f.path}
                      </span>
                      {idx === 0 && !selected.has(f.path) && (
                        <span className="cln-badge cln-badge--keep">保留</span>
                      )}
                      <span className="cln-card__time">
                        {formatRelativeTime(f.modified)}
                      </span>
                    </label>
                  ))}
                </div>
              </div>
            ))}
          </div>

          <div className="cln-bar">
            <div className="cln-bar__info">
              <span className="cln-bar__count">
                {groups.length} 组 · 可回收合计 {formatBytes(totalWasted)} · 已选{" "}
                {selectedPaths.length} 项
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
