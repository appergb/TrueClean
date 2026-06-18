import { useCallback, useEffect, useMemo, useState } from "react";
import { confirm } from "@tauri-apps/plugin-dialog";
import Button from "../ui/Button";
import { useToast } from "../ui/Toast";
import { scanJunk, cleanPaths, emptyTrash } from "../../lib/ipc";
import { useSettingsStore } from "../../store/settingsStore";
import { useI18n } from "../../i18n";
import type { JunkGroup } from "../../lib/types";
import { formatBytes } from "../../lib/format";
import "./cleanup.css";

function errMsg(error: unknown): string {
  if (error instanceof Error) return error.message;
  if (typeof error === "object" && error && "message" in error) {
    return String((error as { message: unknown }).message);
  }
  return "";
}

type Status = "idle" | "loading" | "ready" | "error";

export default function JunkPanel() {
  const { t } = useI18n();
  const toast = useToast();
  const [status, setStatus] = useState<Status>("idle");
  const [error, setError] = useState<string | null>(null);
  const [groups, setGroups] = useState<JunkGroup[]>([]);
  const [selected, setSelected] = useState<Set<string>>(new Set());
  const [expanded, setExpanded] = useState<Set<string>>(new Set());
  const [cleaning, setCleaning] = useState(false);
  const [emptying, setEmptying] = useState(false);

  const defaultToTrash = useSettingsStore(
    (s) => s.settings?.defaultToTrash ?? true,
  );

  const runScan = useCallback(async () => {
    setStatus("loading");
    setError(null);
    try {
      const result = await scanJunk();
      setGroups(result);
      const preset = new Set<string>();
      for (const g of result) {
        if (g.recommended) for (const it of g.items) preset.add(it.path);
      }
      setSelected(preset);
      setExpanded(new Set());
      setStatus("ready");
    } catch (e: unknown) {
      setError(errMsg(e) || t("cleanup.junk.scanError"));
      setStatus("error");
    }
  }, [t]);

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

  const toggleGroup = useCallback((group: JunkGroup, checked: boolean) => {
    setSelected((prev) => {
      const next = new Set(prev);
      for (const it of group.items) {
        if (checked) next.add(it.path);
        else next.delete(it.path);
      }
      return next;
    });
  }, []);

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
    const dest = defaultToTrash
      ? t("cleanup.common.toTrash")
      : t("cleanup.common.permanent");
    let msg = t("cleanup.junk.confirmBody", {
      count: selectedPaths.length,
      dest,
      size: formatBytes(selectedBytes),
    });
    if (!defaultToTrash) msg += "\n\n" + t("cleanup.junk.confirmPermanent");
    const ok = await confirm(msg, {
      title: t("cleanup.junk.confirmTitle"),
      kind: "warning",
    });
    if (!ok) return;

    setCleaning(true);
    const loadId = toast.loading(t("cleanup.junk.cleaning"));
    try {
      const report = await cleanPaths(selectedPaths, defaultToTrash);
      toast.dismiss(loadId);
      const failedNote =
        report.failed.length > 0
          ? " · " + t("cleanup.junk.failedNote", { count: report.failed.length })
          : "";
      toast.success(
        t("cleanup.junk.successTitle"),
        t("cleanup.junk.successDesc", {
          count: report.removedCount,
          size: formatBytes(report.freedBytes),
        }) + failedNote,
      );
      if (defaultToTrash) {
        toast.info(t("cleanup.junk.undoHint"), undefined, 6000);
      }
      await runScan();
    } catch (e: unknown) {
      toast.dismiss(loadId);
      toast.error(t("cleanup.junk.scanError"), errMsg(e));
      setError(errMsg(e));
      setStatus("error");
    } finally {
      setCleaning(false);
    }
  }, [selectedPaths, selectedBytes, defaultToTrash, runScan, t, toast]);

  const handleEmptyTrash = useCallback(async () => {
    const ok = await confirm(t("cleanup.junk.emptyTrashConfirmBody"), {
      title: t("cleanup.junk.emptyTrashConfirmTitle"),
      kind: "warning",
    });
    if (!ok) return;

    setEmptying(true);
    const loadId = toast.loading(t("cleanup.junk.emptyTrash"));
    try {
      const report = await emptyTrash();
      toast.dismiss(loadId);
      toast.success(
        t("cleanup.junk.emptyTrash"),
        t("cleanup.junk.emptyTrashSuccess", {
          size: formatBytes(report.freedBytes),
        }),
      );
      await runScan();
    } catch (e: unknown) {
      toast.dismiss(loadId);
      toast.error(t("cleanup.junk.emptyTrashFailed"), errMsg(e));
    } finally {
      setEmptying(false);
    }
  }, [t, toast, runScan]);

  return (
    <section className="cln">
      <header className="cln-head">
        <div className="cln-head__titles">
          <h2 className="cln-title">{t("cleanup.junk.title")}</h2>
          <p className="cln-sub">{t("cleanup.junk.sub")}</p>
        </div>
        <div className="cln-tools">
          <Button
            variant="ghost"
            onClick={() => void runScan()}
            disabled={status === "loading" || cleaning || emptying}
          >
            {t("cleanup.junk.rescan")}
          </Button>
          <Button
            variant="danger"
            onClick={() => void handleEmptyTrash()}
            disabled={status === "loading" || cleaning || emptying}
          >
            {emptying ? t("cleanup.junk.cleaning") : t("cleanup.junk.emptyTrash")}
          </Button>
        </div>
      </header>

      {status === "loading" && (
        <div className="cln-state">
          <div className="cln-spinner" />
          <p className="cln-state__title">{t("cleanup.junk.scanning")}</p>
        </div>
      )}

      {status === "error" && (
        <div className="cln-state cln-state--error">
          <p className="cln-state__title">{t("cleanup.junk.scanError")}</p>
          <p className="cln-state__msg">{error}</p>
          <Button variant="subtle" onClick={() => void runScan()}>
            {t("cleanup.common.retry")}
          </Button>
        </div>
      )}

      {status === "ready" && groups.length === 0 && (
        <div className="cln-state">
          <p className="cln-state__title">{t("cleanup.junk.empty")}</p>
          <p className="cln-state__msg">{t("cleanup.junk.emptyDesc")}</p>
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
                      aria-label={t("cleanup.dup.selectGroup")}
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
                            {t("cleanup.junk.recommended")}
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
                          <span className="cln-row__path">—</span>
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
                              {t("cleanup.junk.review")}
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
                {t("cleanup.junk.selected", { count: selectedPaths.length })}
              </span>
              <span className="cln-bar__size">{formatBytes(selectedBytes)}</span>
            </div>
            <div className="cln-bar__actions">
              <Button
                variant="primary"
                onClick={() => void handleClean()}
                disabled={selectedPaths.length === 0 || cleaning}
              >
                {cleaning ? t("cleanup.junk.cleaning") : t("cleanup.junk.cleanSelected")}
              </Button>
            </div>
          </div>
        </>
      )}
    </section>
  );
}
