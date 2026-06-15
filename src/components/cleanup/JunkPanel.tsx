import { useCallback, useEffect, useMemo, useState } from "react";
import { confirm } from "@tauri-apps/plugin-dialog";
import Button from "../ui/Button";
import { scanJunk, cleanPaths } from "../../lib/ipc";
import { useSettingsStore } from "../../store/settingsStore";
import type { JunkGroup } from "../../lib/types";
import { formatBytes } from "../../lib/format";
import "./cleanup.css";

function errMsg(error: unknown): string {
  if (error instanceof Error) return error.message;
  if (typeof error === "object" && error && "message" in error) {
    return String((error as { message: unknown }).message);
  }
  return "扫描失败";
}

type Status = "idle" | "loading" | "ready" | "error";

export default function JunkPanel() {
  const [status, setStatus] = useState<Status>("idle");
  const [error, setError] = useState<string | null>(null);
  const [groups, setGroups] = useState<JunkGroup[]>([]);
  const [selected, setSelected] = useState<Set<string>>(new Set());
  const [expanded, setExpanded] = useState<Set<string>>(new Set());
  const [cleaning, setCleaning] = useState(false);

  const defaultToTrash = useSettingsStore(
    (s) => s.settings?.defaultToTrash ?? true,
  );

  const runScan = useCallback(async () => {
    setStatus("loading");
    setError(null);
    try {
      const result = await scanJunk();
      setGroups(result);
      // Pre-select all items in recommended groups.
      const preset = new Set<string>();
      for (const g of result) {
        if (g.recommended) for (const it of g.items) preset.add(it.path);
      }
      setSelected(preset);
      setExpanded(new Set());
      setStatus("ready");
    } catch (e: unknown) {
      setError(errMsg(e));
      setStatus("error");
    }
  }, []);

  useEffect(() => {
    void runScan();
  }, [runScan]);

  const toggleItem = useCallback((path: string) => {
    setSelected((prev) => {
      const next = new Set(prev);
      if (next.has(path)) next.delete(path);
      else next.add(path);
      return next;
    });
  }, []);

  const toggleGroup = useCallback(
    (group: JunkGroup, checked: boolean) => {
      setSelected((prev) => {
        const next = new Set(prev);
        for (const it of group.items) {
          if (checked) next.add(it.path);
          else next.delete(it.path);
        }
        return next;
      });
    },
    [],
  );

  const toggleExpanded = useCallback((id: string) => {
    setExpanded((prev) => {
      const next = new Set(prev);
      if (next.has(id)) next.delete(id);
      else next.add(id);
      return next;
    });
  }, []);

  const { selectedPaths, selectedBytes } = useMemo(() => {
    let bytes = 0;
    const paths: string[] = [];
    for (const g of groups) {
      for (const it of g.items) {
        if (selected.has(it.path)) {
          bytes += it.sizeBytes;
          paths.push(it.path);
        }
      }
    }
    return { selectedPaths: paths, selectedBytes: bytes };
  }, [groups, selected]);

  const handleClean = useCallback(async () => {
    if (selectedPaths.length === 0) return;
    const dest = defaultToTrash ? "移至废纸篓" : "永久删除";
    const ok = await confirm(
      `将对 ${selectedPaths.length} 项执行「${dest}」，预计释放 ${formatBytes(
        selectedBytes,
      )}。是否继续？`,
      { title: "确认清理", kind: "warning" },
    );
    if (!ok) return;

    setCleaning(true);
    try {
      const report = await cleanPaths(selectedPaths, defaultToTrash);
      const note =
        report.failed.length > 0
          ? `，${report.failed.length} 项失败`
          : "";
      await confirm(
        `已清理 ${report.removedCount} 项，释放 ${formatBytes(
          report.freedBytes,
        )}${note}。`,
        { title: "清理完成", kind: "info" },
      );
      await runScan();
    } catch (e: unknown) {
      setError(errMsg(e));
      setStatus("error");
    } finally {
      setCleaning(false);
    }
  }, [selectedPaths, selectedBytes, defaultToTrash, runScan]);

  return (
    <section className="cln">
      <header className="cln-head">
        <div className="cln-head__titles">
          <h2 className="cln-title">系统垃圾</h2>
          <p className="cln-sub">
            缓存、日志、临时文件与废纸篓。推荐项默认已勾选。
          </p>
        </div>
        <div className="cln-tools">
          <Button
            variant="ghost"
            onClick={() => void runScan()}
            disabled={status === "loading" || cleaning}
          >
            重新扫描
          </Button>
        </div>
      </header>

      {status === "loading" && (
        <div className="cln-state">
          <div className="cln-spinner" />
          <p className="cln-state__title">正在扫描垃圾文件…</p>
        </div>
      )}

      {status === "error" && (
        <div className="cln-state cln-state--error">
          <p className="cln-state__title">扫描出错</p>
          <p className="cln-state__msg">{error}</p>
          <Button variant="subtle" onClick={() => void runScan()}>
            重试
          </Button>
        </div>
      )}

      {status === "ready" && groups.length === 0 && (
        <div className="cln-state">
          <p className="cln-state__title">没有发现可清理的垃圾</p>
          <p className="cln-state__msg">系统很干净，暂无缓存或临时文件。</p>
        </div>
      )}

      {status === "ready" && groups.length > 0 && (
        <>
          <div className="cln-list">
            {groups.map((g) => {
              const sel = g.items.filter((it) => selected.has(it.path)).length;
              const allChecked = sel === g.items.length && g.items.length > 0;
              const someChecked = sel > 0 && !allChecked;
              const isOpen = expanded.has(g.id);
              return (
                <div className="cln-group" key={g.id}>
                  <div className="cln-group__head">
                    <input
                      type="checkbox"
                      className="cln-check"
                      checked={allChecked}
                      ref={(el) => {
                        if (el) el.indeterminate = someChecked;
                      }}
                      onChange={(e) => toggleGroup(g, e.target.checked)}
                      aria-label={`选择「${g.label}」全部`}
                    />
                    <button
                      type="button"
                      className="cln-group__meta"
                      onClick={() => toggleExpanded(g.id)}
                      style={{
                        background: "none",
                        border: "none",
                        textAlign: "left",
                        cursor: "pointer",
                        padding: 0,
                      }}
                    >
                      <span className="cln-group__label">
                        <svg
                          className={`cln-group__caret${
                            isOpen ? " cln-group__caret--open" : ""
                          }`}
                          viewBox="0 0 16 16"
                          fill="none"
                          aria-hidden
                        >
                          <path
                            d="M6 4l4 4-4 4"
                            stroke="currentColor"
                            strokeWidth="1.5"
                            strokeLinecap="round"
                            strokeLinejoin="round"
                          />
                        </svg>
                        {g.label}
                        {g.recommended && (
                          <span className="cln-badge cln-badge--safe">
                            推荐
                          </span>
                        )}
                      </span>
                      <span className="cln-group__desc">{g.description}</span>
                    </button>
                    <span className="cln-group__size">
                      {formatBytes(g.totalBytes)}
                    </span>
                  </div>

                  {isOpen && (
                    <div className="cln-group__items">
                      {g.items.length === 0 && (
                        <div className="cln-row">
                          <span className="cln-row__path">（空）</span>
                        </div>
                      )}
                      {g.items.map((it) => (
                        <label className="cln-row" key={it.path}>
                          <input
                            type="checkbox"
                            className="cln-check"
                            checked={selected.has(it.path)}
                            onChange={() => toggleItem(it.path)}
                          />
                          <span className="cln-row__path" title={it.path}>
                            {it.path}
                          </span>
                          {!it.safe && (
                            <span className="cln-badge cln-badge--review">
                              复核
                            </span>
                          )}
                          <span className="cln-row__size">
                            {formatBytes(it.sizeBytes)}
                          </span>
                        </label>
                      ))}
                    </div>
                  )}
                </div>
              );
            })}
          </div>

          <div className="cln-bar">
            <div className="cln-bar__info">
              <span className="cln-bar__count">
                已选 {selectedPaths.length} 项
              </span>
              <span className="cln-bar__size">{formatBytes(selectedBytes)}</span>
            </div>
            <div className="cln-bar__actions">
              <Button
                variant="primary"
                onClick={() => void handleClean()}
                disabled={selectedPaths.length === 0 || cleaning}
              >
                {cleaning ? "清理中…" : "清理所选"}
              </Button>
            </div>
          </div>
        </>
      )}
    </section>
  );
}
