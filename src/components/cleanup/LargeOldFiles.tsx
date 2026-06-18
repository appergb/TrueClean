import { useCallback, useMemo, useState } from "react";
import { confirm, open } from "@tauri-apps/plugin-dialog";
import Button from "../ui/Button";
import { useToast } from "../ui/Toast";
import { findLargeOldFiles, cleanPaths } from "../../lib/ipc";
import { useSettingsStore } from "../../store/settingsStore";
import { useI18n } from "../../i18n";
import type { FileEntry } from "../../lib/types";
import { formatBytes, formatRelativeTime } from "../../lib/format";
import "./cleanup.css";

const MB = 1024 * 1024;
const DAY_MS = 86_400_000;

function errMsg(error: unknown): string {
  if (error instanceof Error) return error.message;
  if (typeof error === "object" && error && "message" in error) {
    return String((error as { message: unknown }).message);
  }
  return "";
}

type Status = "idle" | "loading" | "ready" | "error";

export default function LargeOldFiles() {
  const { t } = useI18n();
  const toast = useToast();
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
      setError(t("cleanup.large.dirRequired"));
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
  }, [path, minSizeMb, olderThanDays, t]);

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
    const dest = defaultToTrash
      ? t("cleanup.common.toTrash")
      : t("cleanup.common.permanent");
    let msg = t("cleanup.large.confirmBody", {
      count: selectedPaths.length,
      dest,
      size: formatBytes(selectedBytes),
    });
    if (!defaultToTrash) msg += "\n\n" + t("cleanup.large.confirmPermanent");
    const ok = await confirm(msg, {
      title: t("cleanup.large.confirmTitle"),
      kind: "warning",
    });
    if (!ok) return;

    setCleaning(true);
    const loadId = toast.loading(t("cleanup.large.deleting"));
    try {
      const report = await cleanPaths(selectedPaths, defaultToTrash);
      toast.dismiss(loadId);
      const failedNote =
        report.failed.length > 0
          ? " · " + t("cleanup.junk.failedNote", { count: report.failed.length })
          : "";
      toast.success(
        t("cleanup.large.successTitle"),
        t("cleanup.large.successDesc", {
          count: report.removedCount,
          size: formatBytes(report.freedBytes),
        }) + failedNote,
      );
      setFiles((prev) => prev.filter((f) => !selected.has(f.path)));
      setSelected(new Set());
      if (defaultToTrash) {
        toast.info(t("cleanup.junk.undoHint"), undefined, 6000);
      }
    } catch (e: unknown) {
      toast.dismiss(loadId);
      toast.error(t("cleanup.large.error"), errMsg(e));
      setError(errMsg(e));
      setStatus("error");
    } finally {
      setCleaning(false);
    }
  }, [selectedPaths, selectedBytes, defaultToTrash, selected, t, toast]);

  return (
    <section className="cln">
      <header className="cln-head">
        <div className="cln-head__titles">
          <h2 className="cln-title">{t("cleanup.large.title")}</h2>
          <p className="cln-sub">{t("cleanup.large.sub")}</p>
        </div>
      </header>

      <div className="cln-tools">
        <div className="cln-field">
          <span className="cln-field__label">{t("cleanup.large.dir")}</span>
          <div className="cln-pathrow">
            <input
              className="cln-input cln-input--path"
              value={path}
              placeholder={t("cleanup.large.dirPlaceholder")}
              onChange={(e) => setPath(e.target.value)}
            />
            <Button variant="subtle" onClick={() => void pickDir()}>
              {t("cleanup.large.browse")}
            </Button>
          </div>
        </div>
        <div className="cln-field">
          <span className="cln-field__label">{t("cleanup.large.minSize")}</span>
          <input
            type="number"
            min={0}
            className="cln-input cln-input--num"
            value={minSizeMb}
            onChange={(e) => setMinSizeMb(Number(e.target.value) || 0)}
          />
        </div>
        <div className="cln-field">
          <span className="cln-field__label">{t("cleanup.large.olderThan")}</span>
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
            {t("cleanup.large.search")}
          </Button>
        </div>
      </div>

      {status === "loading" && (
        <div className="cln-state">
          <div className="cln-spinner" />
          <p className="cln-state__title">{t("cleanup.large.searching")}</p>
        </div>
      )}

      {status === "error" && (
        <div className="cln-state cln-state--error">
          <p className="cln-state__title">{t("cleanup.large.error")}</p>
          <p className="cln-state__msg">{error}</p>
          <Button variant="subtle" onClick={() => setStatus("idle")}>
            {t("cleanup.common.retry")}
          </Button>
        </div>
      )}

      {status === "idle" && (
        <div className="cln-state">
          <p className="cln-state__title">{t("cleanup.large.idle")}</p>
          <p className="cln-state__msg">{t("cleanup.large.idleDesc")}</p>
        </div>
      )}

      {status === "ready" && files.length === 0 && (
        <div className="cln-state">
          <p className="cln-state__title">{t("cleanup.large.noResult")}</p>
          <p className="cln-state__msg">{t("cleanup.large.noResultDesc")}</p>
        </div>
      )}

      {status === "ready" && files.length > 0 && (
        <>
          <div className="cln-list">
            {files.map((f) => {
              const isOld =
                f.modified != null &&
                Date.now() - f.modified * 1000 >= olderThanDays * DAY_MS;
              return (
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
                    {isOld && (
                      <span className="cln-badge cln-badge--review">
                        {t("cleanup.large.daysOld", { days: olderThanDays })}
                      </span>
                    )}
                    <span className="cln-card__size">
                      {formatBytes(f.sizeBytes)}
                    </span>
                    <span className="cln-card__time">
                      {formatRelativeTime(f.modified)}
                    </span>
                  </div>
                </label>
              );
            })}
          </div>

          <div className="cln-bar">
            <div className="cln-bar__info">
              <span className="cln-bar__count">
                {t("cleanup.large.totalSelected", {
                  total: files.length,
                  selected: selectedPaths.length,
                })}
              </span>
              <span className="cln-bar__size">{formatBytes(selectedBytes)}</span>
            </div>
            <div className="cln-bar__actions">
              <Button
                variant="primary"
                onClick={() => void handleDelete()}
                disabled={selectedPaths.length === 0 || cleaning}
              >
                {cleaning
                  ? t("cleanup.large.deleting")
                  : t("cleanup.large.deleteSelected")}
              </Button>
            </div>
          </div>
        </>
      )}
    </section>
  );
}
