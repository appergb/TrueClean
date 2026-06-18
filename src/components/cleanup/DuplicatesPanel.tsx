import "./cleanup.css";

import { confirm, open } from "@tauri-apps/plugin-dialog";
import { useCallback, useMemo, useState } from "react";

import { useI18n } from "../../i18n";
import { formatBytes, formatRelativeTime } from "../../lib/format";
import { cleanPaths,findDuplicates } from "../../lib/ipc";
import type { DuplicateGroup } from "../../lib/types";
import { useSettingsStore } from "../../store/settingsStore";
import Button from "../ui/Button";
import { useToast } from "../ui/Toast";

const MB = 1024 * 1024;

function errMsg(error: unknown): string {
  if (error instanceof Error) return error.message;
  if (typeof error === "object" && error && "message" in error) {
    return String((error as { message: unknown }).message);
  }
  return "";
}

type Status = "idle" | "loading" | "ready" | "error";

export default function DuplicatesPanel() {
  const { t } = useI18n();
  const toast = useToast();
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
      setError(t("cleanup.dup.dirRequired"));
      setStatus("error");
      return;
    }
    setStatus("loading");
    setError(null);
    try {
      const result = await findDuplicates(path, Math.max(0, minSizeMb) * MB);
      setGroups(result);
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
  }, [path, minSizeMb, t]);

  const toggle = useCallback((p: string) => {
    setSelected((prev) => {
      const next = new Set(prev);
      if (next.has(p)) next.delete(p);
      else next.add(p);
      return next;
    });
  }, []);

  // Group checkbox: checked = select all but first (keep one); unchecked = clear group.
  const toggleGroup = useCallback((g: DuplicateGroup, checked: boolean) => {
    setSelected((prev) => {
      const next = new Set(prev);
      const targets = g.files.slice(1);
      if (checked) {
        targets.forEach((f) => next.add(f.path));
      } else {
        g.files.forEach((f) => next.delete(f.path));
      }
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
    const dest = defaultToTrash
      ? t("cleanup.common.toTrash")
      : t("cleanup.common.permanent");
    let msg = t("cleanup.dup.confirmBody", {
      count: selectedPaths.length,
      dest,
      size: formatBytes(selectedBytes),
    });
    if (!defaultToTrash) msg += "\n\n" + t("cleanup.dup.confirmPermanent");
    const ok = await confirm(msg, {
      title: t("cleanup.dup.confirmTitle"),
      kind: "warning",
    });
    if (!ok) return;

    setCleaning(true);
    const loadId = toast.loading(t("cleanup.dup.deleting"));
    try {
      const report = await cleanPaths(selectedPaths, defaultToTrash);
      toast.dismiss(loadId);
      const failedNote =
        report.failed.length > 0
          ? " · " + t("cleanup.junk.failedNote", { count: report.failed.length })
          : "";
      toast.success(
        t("cleanup.dup.successTitle"),
        t("cleanup.dup.successDesc", {
          count: report.removedCount,
          size: formatBytes(report.freedBytes),
        }) + failedNote,
      );
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
      if (defaultToTrash) {
        toast.info(t("cleanup.junk.undoHint"), undefined, 6000);
      }
    } catch (e: unknown) {
      toast.dismiss(loadId);
      toast.error(t("cleanup.dup.error"), errMsg(e));
      setError(errMsg(e));
      setStatus("error");
    } finally {
      setCleaning(false);
    }
  }, [selectedPaths, selectedBytes, defaultToTrash, t, toast]);

  return (
    <section className="cln">
      <header className="cln-head">
        <div className="cln-head__titles">
          <h2 className="cln-title">{t("cleanup.dup.title")}</h2>
          <p className="cln-sub">{t("cleanup.dup.sub")}</p>
        </div>
      </header>

      <div className="cln-tools">
        <div className="cln-field">
          <span className="cln-field__label">{t("cleanup.dup.dir")}</span>
          <div className="cln-pathrow">
            <input
              className="cln-input cln-input--path"
              value={path}
              placeholder={t("cleanup.dup.dirPlaceholder")}
              onChange={(e) => setPath(e.target.value)}
            />
            <Button variant="subtle" onClick={() => void pickDir()}>
              {t("cleanup.dup.browse")}
            </Button>
          </div>
        </div>
        <div className="cln-field">
          <span className="cln-field__label">{t("cleanup.dup.minSize")}</span>
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
            {t("cleanup.dup.search")}
          </Button>
        </div>
      </div>

      {status === "loading" && (
        <div className="cln-state">
          <div className="cln-spinner" />
          <p className="cln-state__title">{t("cleanup.dup.searching")}</p>
        </div>
      )}

      {status === "error" && (
        <div className="cln-state cln-state--error">
          <p className="cln-state__title">{t("cleanup.dup.error")}</p>
          <p className="cln-state__msg">{error}</p>
          <Button variant="subtle" onClick={() => setStatus("idle")}>
            {t("cleanup.common.retry")}
          </Button>
        </div>
      )}

      {status === "idle" && (
        <div className="cln-state">
          <p className="cln-state__title">{t("cleanup.dup.idle")}</p>
          <p className="cln-state__msg">{t("cleanup.dup.idleDesc")}</p>
        </div>
      )}

      {status === "ready" && groups.length === 0 && (
        <div className="cln-state">
          <p className="cln-state__title">{t("cleanup.dup.noResult")}</p>
          <p className="cln-state__msg">{t("cleanup.dup.noResultDesc")}</p>
        </div>
      )}

      {status === "ready" && groups.length > 0 && (
        <>
          <div className="cln-list">
            {groups.map((g) => {
              const deletable = g.files.slice(1);
              const selCount = deletable.filter((f) =>
                selected.has(f.path),
              ).length;
              const groupChecked =
                deletable.length > 0 && selCount === deletable.length;
              const groupSome = selCount > 0 && !groupChecked;
              return (
                <div className="cln-dup" key={g.hash}>
                  <div className="cln-dup__head">
                    <div className="cln-dup__headleft">
                      <input
                        type="checkbox"
                        className="cln-check"
                        checked={groupChecked}
                        ref={(el) => {
                          if (el) el.indeterminate = groupSome;
                        }}
                        onChange={(e) => toggleGroup(g, e.target.checked)}
                        aria-label={t("cleanup.dup.selectGroup")}
                      />
                      <div>
                        <div className="cln-dup__title">
                          {t("cleanup.dup.filesCount", {
                            count: g.files.length,
                            size: formatBytes(g.sizeBytes),
                          })}
                        </div>
                        <div className="cln-dup__hash">{g.hash.slice(0, 16)}…</div>
                      </div>
                    </div>
                    <span className="cln-dup__waste">
                      {t("cleanup.dup.recyclable", {
                        size: formatBytes(g.wastedBytes),
                      })}
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
                          <span className="cln-badge cln-badge--keep">
                            {t("cleanup.dup.keep")}
                          </span>
                        )}
                        <span className="cln-card__time">
                          {formatRelativeTime(f.modified)}
                        </span>
                      </label>
                    ))}
                  </div>
                </div>
              );
            })}
          </div>

          <div className="cln-bar">
            <div className="cln-bar__info">
              <span className="cln-bar__count">
                {t("cleanup.dup.summary", {
                  groups: groups.length,
                  total: formatBytes(totalWasted),
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
                  ? t("cleanup.dup.deleting")
                  : t("cleanup.dup.deleteSelected")}
              </Button>
            </div>
          </div>
        </>
      )}
    </section>
  );
}
