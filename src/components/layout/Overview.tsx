import { useCallback, useEffect, useState } from "react";
import type { ReactNode } from "react";
import { getVolumes } from "../../lib/ipc";
import type { VolumeInfo, AppSettings } from "../../lib/types";
import { formatBytes } from "../../lib/format";
import { SurfaceCard } from "../ui/SurfaceCard";
import { ProgressRing } from "../ui/ProgressRing";
import { Button } from "../ui/Button";
import { EmptyState } from "../ui/EmptyState";
import { Onboarding } from "./Onboarding";
import { useI18n } from "../../i18n";
import { useSettingsStore } from "../../store/settingsStore";
import type { ViewId } from "./Sidebar";

interface OverviewProps {
  /** Switch the app to the scan view. */
  onStartScan: () => void;
  /** Navigate to an arbitrary view (used by the guide cards). */
  onNavigate: (view: ViewId) => void;
}

type LoadState =
  | { status: "loading" }
  | { status: "error"; message: string }
  | { status: "ready"; volumes: VolumeInfo[] };

function usageRatio(v: VolumeInfo): number {
  if (v.totalBytes <= 0) return 0;
  return Math.max(0, Math.min(1, v.usedBytes / v.totalBytes));
}

function ringColor(ratio: number): string {
  if (ratio >= 0.9) return "var(--danger)";
  if (ratio >= 0.75) return "var(--warn)";
  return "var(--accent)";
}

/** Returns true when the configured provider has the credentials it needs. */
function isAiConfigured(settings: AppSettings | null): boolean {
  if (!settings) return true; // don't prompt before settings have loaded
  switch (settings.provider) {
    case "claude":
      return settings.claudeApiKey.trim().length > 0;
    case "openai":
      return settings.openaiApiKey.trim().length > 0;
    case "ollama":
      return settings.ollamaBaseUrl.trim().length > 0;
    default:
      return true;
  }
}

interface GuideCard {
  view: ViewId;
  titleKey: string;
  descKey: string;
  icon: ReactNode;
}

const GUIDE_CARDS: GuideCard[] = [
  {
    view: "junk",
    titleKey: "shell.overview.guideJunkTitle",
    descKey: "shell.overview.guideJunkDesc",
    icon: (
      <svg viewBox="0 0 24 24" width="20" height="20" fill="none" stroke="currentColor" strokeWidth="1.8" strokeLinecap="round" strokeLinejoin="round">
        <path d="M3 6h18M8 6V4h8v2M6 6l1 14h10l1-14" />
      </svg>
    ),
  },
  {
    view: "large",
    titleKey: "shell.overview.guideLargeTitle",
    descKey: "shell.overview.guideLargeDesc",
    icon: (
      <svg viewBox="0 0 24 24" width="20" height="20" fill="none" stroke="currentColor" strokeWidth="1.8" strokeLinecap="round" strokeLinejoin="round">
        <path d="M14 3H6a2 2 0 0 0-2 2v14a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V9zM14 3v6h6" />
      </svg>
    ),
  },
  {
    view: "duplicates",
    titleKey: "shell.overview.guideDupTitle",
    descKey: "shell.overview.guideDupDesc",
    icon: (
      <svg viewBox="0 0 24 24" width="20" height="20" fill="none" stroke="currentColor" strokeWidth="1.8" strokeLinecap="round" strokeLinejoin="round">
        <rect x="9" y="9" width="11" height="11" rx="2" />
        <path d="M5 15V5a2 2 0 0 1 2-2h10" />
      </svg>
    ),
  },
  {
    view: "apps",
    titleKey: "shell.overview.guideAppsTitle",
    descKey: "shell.overview.guideAppsDesc",
    icon: (
      <svg viewBox="0 0 24 24" width="20" height="20" fill="none" stroke="currentColor" strokeWidth="1.8" strokeLinecap="round" strokeLinejoin="round">
        <rect x="3" y="3" width="7" height="7" rx="1.5" />
        <rect x="14" y="3" width="7" height="7" rx="1.5" />
        <rect x="3" y="14" width="7" height="7" rx="1.5" />
        <rect x="14" y="14" width="7" height="7" rx="1.5" />
      </svg>
    ),
  },
];

export function Overview({ onStartScan, onNavigate }: OverviewProps) {
  const { t } = useI18n();
  const [state, setState] = useState<LoadState>({ status: "loading" });
  const settings = useSettingsStore((s) => s.settings);
  const settingsLoading = useSettingsStore((s) => s.loading);
  const loadSettings = useSettingsStore((s) => s.load);

  // Load settings once so we can detect a missing AI key.
  useEffect(() => {
    if (!settings && !settingsLoading) void loadSettings();
  }, [settings, settingsLoading, loadSettings]);

  const load = useCallback(async () => {
    setState({ status: "loading" });
    try {
      const volumes = await getVolumes();
      setState({ status: "ready", volumes });
    } catch (err: unknown) {
      const message =
        err && typeof err === "object" && "message" in err
          ? String((err as { message: unknown }).message)
          : t("shell.overview.readVolumesFail");
      setState({ status: "error", message });
    }
  }, [t]);

  useEffect(() => {
    void load();
  }, [load]);

  const showAiPrompt = !isAiConfigured(settings);

  return (
    <div className="tc-overview">
      <Onboarding onStart={onStartScan} />

      {showAiPrompt && (
        <SurfaceCard elevation="sm" className="tc-ai-prompt" role="note">
          <span className="tc-ai-prompt__icon" aria-hidden="true">
            <svg viewBox="0 0 24 24" width="20" height="20" fill="none" stroke="currentColor" strokeWidth="1.8" strokeLinecap="round" strokeLinejoin="round">
              <path d="M12 3v4M12 17v4M3 12h4M17 12h4M5.6 5.6l2.8 2.8M15.6 15.6l2.8 2.8M18.4 5.6l-2.8 2.8M8.4 15.6l-2.8 2.8" />
            </svg>
          </span>
          <div className="tc-ai-prompt__body">
            <h3 className="tc-ai-prompt__title">{t("shell.aiKeyPrompt.title")}</h3>
            <p className="tc-ai-prompt__desc">{t("shell.aiKeyPrompt.desc")}</p>
          </div>
          <Button variant="subtle" size="sm" onClick={() => onNavigate("settings")}>
            {t("shell.aiKeyPrompt.action")}
          </Button>
        </SurfaceCard>
      )}

      <section className="tc-hero">
        <div className="tc-hero__copy">
          <p className="tc-hero__eyebrow">{t("shell.overview.heroEyebrow")}</p>
          <h2 className="tc-hero__title">{t("shell.overview.heroTitle")}</h2>
          <p className="tc-hero__lead">{t("shell.overview.heroLead")}</p>
          <div className="tc-hero__cta">
            <Button variant="primary" size="lg" onClick={onStartScan}>
              {t("shell.overview.ctaScan")}
            </Button>
            <Button variant="ghost" size="lg" onClick={() => onNavigate("junk")}>
              {t("shell.overview.ctaJunk")}
            </Button>
          </div>
        </div>
        <div className="tc-hero__glow" aria-hidden="true" />
      </section>

      <section className="tc-section">
        <header className="tc-section__head">
          <h3 className="tc-section__title">{t("shell.overview.volumesTitle")}</h3>
          {state.status === "ready" && (
            <span className="tc-section__meta">
              {t("shell.overview.volumesMeta", { count: state.volumes.length })}
            </span>
          )}
        </header>

        {state.status === "loading" && (
          <div className="tc-vol-grid">
            {[0, 1, 2].map((i) => (
              <SurfaceCard key={i} elevation="sm" className="tc-vol-card is-skeleton">
                <div className="tc-skel-ring" />
                <div className="tc-skel-lines">
                  <span />
                  <span />
                </div>
              </SurfaceCard>
            ))}
          </div>
        )}

        {state.status === "error" && (
          <SurfaceCard elevation="sm">
            <EmptyState
              compact
              title={t("shell.overview.readVolumesFail")}
              description={state.message}
              action={
                <Button variant="subtle" onClick={() => void load()}>
                  {t("shell.common.retry")}
                </Button>
              }
            />
          </SurfaceCard>
        )}

        {state.status === "ready" && state.volumes.length === 0 && (
          <SurfaceCard elevation="sm">
            <EmptyState compact title={t("shell.overview.noVolumes")} />
          </SurfaceCard>
        )}

        {state.status === "ready" && state.volumes.length > 0 && (
          <div className="tc-vol-grid">
            {state.volumes.map((v) => {
              const ratio = usageRatio(v);
              return (
                <SurfaceCard
                  key={v.mountPoint}
                  elevation="md"
                  interactive
                  className="tc-vol-card"
                  onClick={onStartScan}
                  role="button"
                  tabIndex={0}
                  onKeyDown={(e) => {
                    if (e.key === "Enter" || e.key === " ") {
                      e.preventDefault();
                      onStartScan();
                    }
                  }}
                >
                  <ProgressRing
                    value={ratio}
                    size={104}
                    thickness={9}
                    color={ringColor(ratio)}
                  />
                  <div className="tc-vol-card__info">
                    <div className="tc-vol-card__name" title={v.mountPoint}>
                      {v.name || v.mountPoint}
                      {v.isRemovable && (
                        <span className="tc-vol-card__badge">
                          {t("shell.overview.volRemovable")}
                        </span>
                      )}
                    </div>
                    <div className="tc-vol-card__bytes tabular">
                      <span className="tc-vol-card__used">
                        {formatBytes(v.usedBytes)}
                      </span>
                      <span className="tc-vol-card__total">
                        {t("shell.overview.volOf", { size: formatBytes(v.totalBytes) })}
                      </span>
                    </div>
                    <div className="tc-vol-card__free tabular">
                      {t("shell.overview.volFree", { size: formatBytes(v.availableBytes) })} · {v.fileSystem}
                    </div>
                  </div>
                </SurfaceCard>
              );
            })}
          </div>
        )}
      </section>

      <section className="tc-section">
        <header className="tc-section__head">
          <h3 className="tc-section__title">{t("shell.overview.guideTitle")}</h3>
        </header>
        <div className="tc-guide-grid">
          {GUIDE_CARDS.map((card) => (
            <SurfaceCard
              key={card.view}
              elevation="sm"
              interactive
              className="tc-guide-card"
              role="button"
              tabIndex={0}
              onClick={() => onNavigate(card.view)}
              onKeyDown={(e) => {
                if (e.key === "Enter" || e.key === " ") {
                  e.preventDefault();
                  onNavigate(card.view);
                }
              }}
            >
              <span className="tc-guide-card__icon" aria-hidden="true">
                {card.icon}
              </span>
              <div className="tc-guide-card__body">
                <h4 className="tc-guide-card__title">{t(card.titleKey)}</h4>
                <p className="tc-guide-card__desc">{t(card.descKey)}</p>
              </div>
              <span className="tc-guide-card__chev" aria-hidden="true">
                →
              </span>
            </SurfaceCard>
          ))}
        </div>
      </section>
    </div>
  );
}

export default Overview;
