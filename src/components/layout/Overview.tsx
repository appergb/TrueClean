import type { ReactNode } from "react";
import { useCallback, useEffect, useState } from "react";

import { useI18n } from "../../i18n";
import { formatBytes } from "../../lib/format";
import { getVolumes } from "../../lib/ipc";
import type { AppSettings, VolumeInfo } from "../../lib/types";
import { useSettingsStore } from "../../store/settingsStore";
import { Button } from "../ui/Button";
import { EmptyState } from "../ui/EmptyState";
import { ProgressRing } from "../ui/ProgressRing";
import { SurfaceCard } from "../ui/SurfaceCard";
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

const ONBOARDED_KEY = "trueclean.onboarded";

function readOnboarded(): boolean {
  try {
    return localStorage.getItem(ONBOARDED_KEY) === "1";
  } catch {
    return false;
  }
}

function writeOnboarded(): void {
  try {
    localStorage.setItem(ONBOARDED_KEY, "1");
  } catch {
    /* ignore persistence failures */
  }
}

/** A provider is considered ready when its required credential is set. */
function isAiConfigured(settings: AppSettings | null): boolean {
  if (!settings) return true; // hide the hint while settings are still loading
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

function usageRatio(v: VolumeInfo): number {
  if (v.totalBytes <= 0) return 0;
  return Math.max(0, Math.min(1, v.usedBytes / v.totalBytes));
}

function ringColor(ratio: number): string {
  if (ratio >= 0.9) return "var(--danger)";
  if (ratio >= 0.75) return "var(--warn)";
  return "var(--accent)";
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
    titleKey: "shell.overview.guide.junkTitle",
    descKey: "shell.overview.guide.junkDesc",
    icon: (
      <svg viewBox="0 0 24 24" width="20" height="20" fill="none" stroke="currentColor" strokeWidth="1.8" strokeLinecap="round" strokeLinejoin="round">
        <path d="M3 6h18M8 6V4h8v2M6 6l1 14h10l1-14" />
      </svg>
    ),
  },
  {
    view: "large",
    titleKey: "shell.overview.guide.largeTitle",
    descKey: "shell.overview.guide.largeDesc",
    icon: (
      <svg viewBox="0 0 24 24" width="20" height="20" fill="none" stroke="currentColor" strokeWidth="1.8" strokeLinecap="round" strokeLinejoin="round">
        <path d="M14 3H6a2 2 0 0 0-2 2v14a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V9zM14 3v6h6" />
      </svg>
    ),
  },
  {
    view: "duplicates",
    titleKey: "shell.overview.guide.dupTitle",
    descKey: "shell.overview.guide.dupDesc",
    icon: (
      <svg viewBox="0 0 24 24" width="20" height="20" fill="none" stroke="currentColor" strokeWidth="1.8" strokeLinecap="round" strokeLinejoin="round">
        <rect x="9" y="9" width="11" height="11" rx="2" />
        <path d="M5 15V5a2 2 0 0 1 2-2h10" />
      </svg>
    ),
  },
  {
    view: "apps",
    titleKey: "shell.overview.guide.appsTitle",
    descKey: "shell.overview.guide.appsDesc",
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

interface OnboardingStep {
  titleKey: string;
  descKey: string;
}

const ONBOARD_STEPS: OnboardingStep[] = [
  { titleKey: "shell.onboarding.step1Title", descKey: "shell.onboarding.step1Desc" },
  { titleKey: "shell.onboarding.step2Title", descKey: "shell.onboarding.step2Desc" },
  { titleKey: "shell.onboarding.step3Title", descKey: "shell.onboarding.step3Desc" },
];

export function Overview({ onStartScan, onNavigate }: OverviewProps) {
  const { t } = useI18n();
  const [state, setState] = useState<LoadState>({ status: "loading" });
  const [onboarded, setOnboarded] = useState<boolean>(readOnboarded);
  const settings = useSettingsStore((s) => s.settings);

  const load = useCallback(async () => {
    setState({ status: "loading" });
    try {
      const volumes = await getVolumes();
      setState({ status: "ready", volumes });
    } catch (err: unknown) {
      const message =
        err && typeof err === "object" && "message" in err
          ? String((err as { message: unknown }).message)
          : t("shell.overview.loadError");
      setState({ status: "error", message });
    }
  }, [t]);

  useEffect(() => {
    void load();
  }, [load]);

  const finishOnboarding = useCallback(() => {
    writeOnboarded();
    setOnboarded(true);
  }, []);

  const handleStart = useCallback(() => {
    finishOnboarding();
    onStartScan();
  }, [finishOnboarding, onStartScan]);

  const showAiHint = !isAiConfigured(settings);

  return (
    <div className="tc-overview">
      {!onboarded ? (
        <section className="tc-onboard" aria-labelledby="tc-onboard-title">
          <div className="tc-onboard__copy">
            <p className="tc-hero__eyebrow">{t("shell.onboarding.eyebrow")}</p>
            <h2 id="tc-onboard-title" className="tc-hero__title">
              {t("shell.onboarding.title")}
            </h2>
            <p className="tc-hero__lead">{t("shell.onboarding.lead")}</p>
          </div>
          <ol className="tc-onboard__steps">
            {ONBOARD_STEPS.map((step, i) => (
              <li key={step.titleKey} className="tc-onboard__step">
                <span className="tc-onboard__num" aria-hidden="true">
                  {i + 1}
                </span>
                <div>
                  <h3 className="tc-onboard__step-title">
                    {t(step.titleKey)}
                  </h3>
                  <p className="tc-onboard__step-desc">{t(step.descKey)}</p>
                </div>
              </li>
            ))}
          </ol>
          <div className="tc-hero__cta">
            <Button variant="primary" size="lg" onClick={handleStart}>
              {t("shell.onboarding.start")}
            </Button>
            <Button variant="ghost" size="lg" onClick={finishOnboarding}>
              {t("shell.onboarding.skip")}
            </Button>
          </div>
        </section>
      ) : (
        <section className="tc-hero">
          <div className="tc-hero__copy">
            <p className="tc-hero__eyebrow">{t("shell.overview.eyebrow")}</p>
            <h2 className="tc-hero__title">{t("shell.overview.title")}</h2>
            <p className="tc-hero__lead">{t("shell.overview.lead")}</p>
            <div className="tc-hero__cta">
              <Button variant="primary" size="lg" onClick={onStartScan}>
                {t("shell.overview.startScan")}
              </Button>
              <Button variant="ghost" size="lg" onClick={() => onNavigate("junk")}>
                {t("shell.overview.quickClean")}
              </Button>
            </div>
          </div>
          <div className="tc-hero__glow" aria-hidden="true" />
        </section>
      )}

      {showAiHint && (
        <SurfaceCard elevation="sm" className="tc-aihint" role="note">
          <span className="tc-aihint__icon" aria-hidden="true">
            <svg viewBox="0 0 24 24" width="20" height="20" fill="none" stroke="currentColor" strokeWidth="1.8" strokeLinecap="round" strokeLinejoin="round">
              <path d="M12 3v4M12 17v4M3 12h4M17 12h4M5.6 5.6l2.8 2.8M15.6 15.6l2.8 2.8M18.4 5.6l-2.8 2.8M8.4 15.6l-2.8 2.8" />
            </svg>
          </span>
          <div className="tc-aihint__body">
            <h3 className="tc-aihint__title">{t("shell.aiKeyHint.title")}</h3>
            <p className="tc-aihint__desc">{t("shell.aiKeyHint.desc")}</p>
          </div>
          <Button variant="subtle" size="sm" onClick={() => onNavigate("settings")}>
            {t("shell.aiKeyHint.goSettings")}
          </Button>
        </SurfaceCard>
      )}

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
              title={t("shell.overview.loadError")}
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
                          {t("shell.overview.removable")}
                        </span>
                      )}
                    </div>
                    <div className="tc-vol-card__bytes tabular">
                      <span className="tc-vol-card__used">
                        {formatBytes(v.usedBytes)}
                      </span>
                      <span className="tc-vol-card__total">
                        / {formatBytes(v.totalBytes)}
                      </span>
                    </div>
                    <div className="tc-vol-card__free tabular">
                      {t("shell.overview.free", { size: formatBytes(v.availableBytes) })}
                      {" · "}
                      {v.fileSystem}
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
