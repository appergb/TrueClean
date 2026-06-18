import "./cleanup.css";

import { useCallback, useEffect, useState } from "react";

import { useI18n } from "../../i18n";
import { listStartupItems, setStartupItem } from "../../lib/ipc";
import type { StartupItem } from "../../lib/types";
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

export default function StartupItems() {
  const { t } = useI18n();
  const toast = useToast();
  const [status, setStatus] = useState<Status>("idle");
  const [error, setError] = useState<string | null>(null);
  const [items, setItems] = useState<StartupItem[]>([]);
  const [busyId, setBusyId] = useState<string | null>(null);

  const load = useCallback(async () => {
    setStatus("loading");
    setError(null);
    try {
      const result = await listStartupItems();
      setItems(result);
      setStatus("ready");
    } catch (e: unknown) {
      setError(errMsg(e));
      setStatus("error");
    }
  }, []);

  useEffect(() => {
    void load();
  }, [load]);

  const handleToggle = useCallback(
    async (item: StartupItem) => {
      const next = !item.enabled;
      setBusyId(item.id);
      setItems((prev) =>
        prev.map((it) => (it.id === item.id ? { ...it, enabled: next } : it)),
      );
      try {
        await setStartupItem(item.id, next);
        toast.success(
          next
            ? t("cleanup.startup.toggleOnSuccess", { name: item.name })
            : t("cleanup.startup.toggleOffSuccess", { name: item.name }),
        );
      } catch (e: unknown) {
        setItems((prev) =>
          prev.map((it) =>
            it.id === item.id ? { ...it, enabled: item.enabled } : it,
          ),
        );
        toast.error(
          t("cleanup.startup.toggleError", { error: errMsg(e) }),
        );
      } finally {
        setBusyId(null);
      }
    },
    [t, toast],
  );

  return (
    <section className="cln">
      <header className="cln-head">
        <div className="cln-head__titles">
          <h2 className="cln-title">{t("cleanup.startup.title")}</h2>
          <p className="cln-sub">{t("cleanup.startup.sub")}</p>
        </div>
        <div className="cln-tools">
          <Button
            variant="ghost"
            onClick={() => void load()}
            disabled={status === "loading"}
          >
            {t("cleanup.startup.refresh")}
          </Button>
        </div>
      </header>

      {status === "loading" && (
        <div className="cln-state">
          <div className="cln-spinner" />
          <p className="cln-state__title">{t("cleanup.startup.loading")}</p>
        </div>
      )}

      {status === "error" && (
        <div className="cln-state cln-state--error">
          <p className="cln-state__title">{t("cleanup.startup.error")}</p>
          <p className="cln-state__msg">{error}</p>
          <Button variant="subtle" onClick={() => void load()}>
            {t("cleanup.common.retry")}
          </Button>
        </div>
      )}

      {status === "ready" && items.length === 0 && (
        <div className="cln-state">
          <p className="cln-state__title">{t("cleanup.startup.empty")}</p>
          <p className="cln-state__msg">{t("cleanup.startup.emptyDesc")}</p>
        </div>
      )}

      {status === "ready" && items.length > 0 && (
        <div className="cln-list">
          {items.map((item) => (
            <div className="cln-card" key={item.id}>
              <div className="cln-card__main">
                <span className="cln-card__name">{item.name}</span>
                <span className="cln-card__path" title={item.path}>
                  {item.path}
                </span>
              </div>
              <span className="cln-badge cln-badge--keep">{item.kind}</span>
              <button
                type="button"
                className="cln-switch"
                role="switch"
                aria-checked={item.enabled}
                aria-label={
                  item.enabled
                    ? t("cleanup.startup.disable") + " " + item.name
                    : t("cleanup.startup.enable") + " " + item.name
                }
                disabled={busyId === item.id}
                onClick={() => void handleToggle(item)}
              />
            </div>
          ))}
        </div>
      )}
    </section>
  );
}
