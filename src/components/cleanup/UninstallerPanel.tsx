import "./cleanup.css";

import { confirm } from "@tauri-apps/plugin-dialog";
import { useCallback, useEffect, useMemo, useState } from "react";

import { useI18n } from "../../i18n";
import { formatBytes, formatRelativeTime } from "../../lib/format";
import { listApplications, uninstallApp } from "../../lib/ipc";
import type { AppInfo, UninstallReport } from "../../lib/types";
import { useSettingsStore } from "../../store/settingsStore";
import Button from "../ui/Button";
import { useToast } from "../ui/Toast";

function errMsg(error: unknown): string {
  if (error instanceof Error) return error.message;
  if (typeof error === "object" && error && "message" in error) {
    return String((error as { message: unknown }).message);
  }
  return "";
}

type Status = "idle" | "loading" | "ready" | "error";

export default function UninstallerPanel() {
  const { t } = useI18n();
  const toast = useToast();
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
      const dest = defaultToTrash
        ? t("cleanup.common.toTrash")
        : t("cleanup.common.permanent");
      let msg = t("cleanup.apps.confirmBody", {
        name: app.name,
        dest,
        size: formatBytes(app.sizeBytes),
      });
      if (!defaultToTrash) msg += "\n\n" + t("cleanup.apps.confirmPermanent");
      const ok = await confirm(msg, {
        title: t("cleanup.apps.confirmTitle"),
        kind: "warning",
      });
      if (!ok) return;

      setBusyId(app.id);
      setReport(null);
      const loadId = toast.loading(
        t("cleanup.apps.uninstalling"),
        app.name,
      );
      try {
        const result = await uninstallApp(app.id, defaultToTrash);
        toast.dismiss(loadId);
        toast.success(
          t("cleanup.apps.successTitle"),
          t("cleanup.apps.successDesc", {
            name: result.app,
            size: formatBytes(result.freedBytes),
          }),
        );
        setReport(result);
        setApps((prev) => prev.filter((a) => a.id !== app.id));
      } catch (e: unknown) {
        toast.dismiss(loadId);
        toast.error(t("cleanup.apps.failedTitle"), errMsg(e));
        setError(errMsg(e));
        setStatus("error");
      } finally {
        setBusyId(null);
      }
    },
    [defaultToTrash, t, toast],
  );

  return (
    <section className="cln">
      <header className="cln-head">
        <div className="cln-head__titles">
          <h2 className="cln-title">{t("cleanup.apps.title")}</h2>
          <p className="cln-sub">{t("cleanup.apps.sub")}</p>
        </div>
        <div className="cln-tools">
          <input
            className="cln-input"
            placeholder={t("cleanup.apps.searchPlaceholder")}
            value={query}
            onChange={(e) => setQuery(e.target.value)}
            style={{ width: "12rem" }}
          />
          <Button
            variant="ghost"
            onClick={() => void load()}
            disabled={status === "loading"}
          >
            {t("cleanup.apps.refresh")}
          </Button>
        </div>
      </header>

      {report && (
        <div className="cln-group" style={{ padding: "var(--space-4)" }}>
          <div className="cln-group__label" style={{ marginBottom: 4 }}>
            {t("cleanup.apps.reportTitle", { name: report.app })}
            <span className="cln-badge cln-badge--safe">
              {t("cleanup.apps.reportFreed", {
                size: formatBytes(report.freedBytes),
              })}
            </span>
          </div>
          <div className="cln-sub">
            {t("cleanup.apps.removedCount", { count: report.removedPaths.length })}
            {report.leftoverPaths.length > 0
              ? " · " +
                t("cleanup.apps.leftoverCount", {
                  count: report.leftoverPaths.length,
                })
              : " · " + t("cleanup.apps.noLeftover")}
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
          <p className="cln-state__title">{t("cleanup.apps.loading")}</p>
        </div>
      )}

      {status === "error" && (
        <div className="cln-state cln-state--error">
          <p className="cln-state__title">{t("cleanup.apps.error")}</p>
          <p className="cln-state__msg">{error}</p>
          <Button variant="subtle" onClick={() => void load()}>
            {t("cleanup.common.retry")}
          </Button>
        </div>
      )}

      {status === "ready" && filtered.length === 0 && (
        <div className="cln-state">
          <p className="cln-state__title">
            {query ? t("cleanup.apps.emptySearch") : t("cleanup.apps.empty")}
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
                      {t("cleanup.apps.version", { version: app.version })}
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
                  {t("cleanup.apps.lastUsed", {
                    time: formatRelativeTime(app.lastUsed),
                  })}
                </span>
              </div>
              <Button
                variant="danger"
                onClick={() => void handleUninstall(app)}
                disabled={busyId !== null}
              >
                {busyId === app.id
                  ? t("cleanup.apps.uninstalling")
                  : t("cleanup.apps.uninstall")}
              </Button>
            </div>
          ))}
        </div>
      )}
    </section>
  );
}
