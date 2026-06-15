import { useCallback, useEffect, useState } from "react";
import Button from "../ui/Button";
import { listStartupItems, setStartupItem } from "../../lib/ipc";
import type { StartupItem } from "../../lib/types";
import "./cleanup.css";

function errMsg(error: unknown): string {
  if (error instanceof Error) return error.message;
  if (typeof error === "object" && error && "message" in error) {
    return String((error as { message: unknown }).message);
  }
  return "加载失败";
}

type Status = "idle" | "loading" | "ready" | "error";

export default function StartupItems() {
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
      // Optimistic update.
      setItems((prev) =>
        prev.map((it) => (it.id === item.id ? { ...it, enabled: next } : it)),
      );
      try {
        await setStartupItem(item.id, next);
      } catch (e: unknown) {
        // Roll back on failure.
        setItems((prev) =>
          prev.map((it) =>
            it.id === item.id ? { ...it, enabled: item.enabled } : it,
          ),
        );
        setError(errMsg(e));
      } finally {
        setBusyId(null);
      }
    },
    [],
  );

  return (
    <section className="cln">
      <header className="cln-head">
        <div className="cln-head__titles">
          <h2 className="cln-title">启动项</h2>
          <p className="cln-sub">关闭不需要的开机自启项可加快开机速度。</p>
        </div>
        <div className="cln-tools">
          <Button
            variant="ghost"
            onClick={() => void load()}
            disabled={status === "loading"}
          >
            刷新
          </Button>
        </div>
      </header>

      {error && status === "ready" && (
        <p className="cln-sub" style={{ color: "var(--danger)" }}>
          操作失败：{error}
        </p>
      )}

      {status === "loading" && (
        <div className="cln-state">
          <div className="cln-spinner" />
          <p className="cln-state__title">正在读取启动项…</p>
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

      {status === "ready" && items.length === 0 && (
        <div className="cln-state">
          <p className="cln-state__title">没有启动项</p>
          <p className="cln-state__msg">当前没有检测到开机自启的项目。</p>
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
                aria-label={`${item.enabled ? "停用" : "启用"} ${item.name}`}
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
