import { useCallback, useEffect, useMemo, useState } from "react";
import { confirm } from "@tauri-apps/plugin-dialog";
import Button from "../ui/Button";
import { listApplications, uninstallApp } from "../../lib/ipc";
import { useSettingsStore } from "../../store/settingsStore";
import type { AppInfo, UninstallReport } from "../../lib/types";
import { formatBytes, formatRelativeTime } from "../../lib/format";
import "./cleanup.css";

function errMsg(error: unknown): string {
  if (error instanceof Error) return error.message;
  if (typeof error === "object" && error && "message" in error) {
    return String((error as { message: unknown }).message);
  }
  return "加载失败";
}

type Status = "idle" | "loading" | "ready" | "error";

export default function UninstallerPanel() {
  const [status, setStatus] = useState<Status>("idle");
  const [error, setError] = useState<string | null>(null);
  const [apps, setApps] = useState<AppInfo[]>([]);
  const [query, setQuery] = useState("");
  const [busyId, setBusyId] = useState<string | null>(null);
  const [report, setReport] = useState<UninstallReport | null>(null);

  const defaultToTrash = useSettingsStore(
    (s) => s.settings?.defaultToTrash ?? true,
  );

  const load = useCallback(async () => {
    setStatus("loading");
    setError(null);
    try {
      const result = await listApplications();
      setApps(result);
      setStatus("ready");
    } catch (e: unknown) {
      setError(errMsg(e));
      setStatus("error");
    }
  }, []);

  useEffect(() => {
    void load();
  }, [load]);

  const filtered = useMemo(() => {
    const q = query.trim().toLowerCase();
    const list = q
      ? apps.filter((a) => a.name.toLowerCase().includes(q))
      : apps;
    return [...list].sort((a, b) => b.sizeBytes - a.sizeBytes);
  }, [apps, query]);

  const handleUninstall = useCallback(
    async (app: AppInfo) => {
      const dest = defaultToTrash ? "移至废纸篓" : "永久删除";
      const ok = await confirm(
        `将卸载「${app.name}」并「${dest}」其相关文件，预计释放 ${formatBytes(
          app.sizeBytes,
        )}。是否继续？`,
        { title: "确认卸载", kind: "warning" },
      );
      if (!ok) return;

      setBusyId(app.id);
      setReport(null);
      try {
        const result = await uninstallApp(app.id, defaultToTrash);
        setReport(result);
        setApps((prev) => prev.filter((a) => a.id !== app.id));
      } catch (e: unknown) {
        setError(errMsg(e));
        setStatus("error");
      } finally {
        setBusyId(null);
      }
    },
    [defaultToTrash],
  );

  return (
    <section className="cln">
      <header className="cln-head">
        <div className="cln-head__titles">
          <h2 className="cln-title">应用卸载</h2>
          <p className="cln-sub">彻底卸载应用并清除其残留文件。</p>
        </div>
        <div className="cln-tools">
          <input
            className="cln-input"
            placeholder="搜索应用…"
            value={query}
            onChange={(e) => setQuery(e.target.value)}
            style={{ width: "12rem" }}
          />
          <Button
            variant="ghost"
            onClick={() => void load()}
            disabled={status === "loading"}
          >
            刷新
          </Button>
        </div>
      </header>

      {report && (
        <div className="cln-group" style={{ padding: "var(--space-4)" }}>
          <div className="cln-group__label" style={{ marginBottom: 4 }}>
            已卸载「{report.app}」
            <span className="cln-badge cln-badge--safe">
              释放 {formatBytes(report.freedBytes)}
            </span>
          </div>
          <div className="cln-sub">
            移除 {report.removedPaths.length} 项
            {report.leftoverPaths.length > 0
              ? ` · ${report.leftoverPaths.length} 项残留未能移除`
              : " · 无残留"}
          </div>
          {report.leftoverPaths.length > 0 && (
            <ul
              style={{
                margin: "var(--space-2) 0 0",
                paddingLeft: "var(--space-5)",
                color: "var(--text-muted)",
                fontFamily: "var(--font-mono)",
                fontSize: "var(--text-xs)",
              }}
            >
              {report.leftoverPaths.slice(0, 8).map((p) => (
                <li key={p}>{p}</li>
              ))}
            </ul>
          )}
        </div>
      )}

      {status === "loading" && (
        <div className="cln-state">
          <div className="cln-spinner" />
          <p className="cln-state__title">正在扫描已安装应用…</p>
        </div>
      )}

      {status === "error" && (
        <div className="cln-state cln-state--error">
          <p className="cln-state__title">出错了</p>
          <p className="cln-state__msg">{error}</p>
          <Button variant="subtle" onClick={() => void load()}>
            重试
          </Button>
        </div>
      )}

      {status === "ready" && filtered.length === 0 && (
        <div className="cln-state">
          <p className="cln-state__title">
            {query ? "没有匹配的应用" : "未发现可卸载的应用"}
          </p>
        </div>
      )}

      {status === "ready" && filtered.length > 0 && (
        <div className="cln-list">
          {filtered.map((app) => (
            <div className="cln-card" key={app.id}>
              <div className="cln-card__main">
                <span className="cln-card__name">
                  {app.name}
                  {app.version && (
                    <span
                      className="cln-card__time"
                      style={{ marginLeft: "var(--space-2)" }}
                    >
                      v{app.version}
                    </span>
                  )}
                </span>
                <span className="cln-card__path" title={app.path}>
                  {app.path}
                </span>
              </div>
              <div className="cln-card__aside">
                <span className="cln-card__size">
                  {formatBytes(app.sizeBytes)}
                </span>
                <span className="cln-card__time">
                  最近使用 {formatRelativeTime(app.lastUsed)}
                </span>
              </div>
              <Button
                variant="danger"
                onClick={() => void handleUninstall(app)}
                disabled={busyId !== null}
              >
                {busyId === app.id ? "卸载中…" : "卸载"}
              </Button>
            </div>
          ))}
        </div>
      )}
    </section>
  );
}
