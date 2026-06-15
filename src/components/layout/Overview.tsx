import { useCallback, useEffect, useState } from "react";
import type { ReactNode } from "react";
import { getVolumes } from "../../lib/ipc";
import type { VolumeInfo } from "../../lib/types";
import { formatBytes } from "../../lib/format";
import { SurfaceCard } from "../ui/SurfaceCard";
import { ProgressRing } from "../ui/ProgressRing";
import { Button } from "../ui/Button";
import { EmptyState } from "../ui/EmptyState";
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

interface GuideCard {
  view: ViewId;
  title: string;
  desc: string;
  icon: ReactNode;
}

const GUIDE_CARDS: GuideCard[] = [
  {
    view: "junk",
    title: "系统垃圾",
    desc: "清理缓存、日志、临时文件与废纸篓，安全释放空间。",
    icon: (
      <svg viewBox="0 0 24 24" width="20" height="20" fill="none" stroke="currentColor" strokeWidth="1.8" strokeLinecap="round" strokeLinejoin="round">
        <path d="M3 6h18M8 6V4h8v2M6 6l1 14h10l1-14" />
      </svg>
    ),
  },
  {
    view: "large",
    title: "大文件与旧文件",
    desc: "找出占用空间最多、长期未使用的文件，按需复核处理。",
    icon: (
      <svg viewBox="0 0 24 24" width="20" height="20" fill="none" stroke="currentColor" strokeWidth="1.8" strokeLinecap="round" strokeLinejoin="round">
        <path d="M14 3H6a2 2 0 0 0-2 2v14a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V9zM14 3v6h6" />
      </svg>
    ),
  },
  {
    view: "duplicates",
    title: "重复文件",
    desc: "基于内容哈希识别重复副本，去重回收冗余空间。",
    icon: (
      <svg viewBox="0 0 24 24" width="20" height="20" fill="none" stroke="currentColor" strokeWidth="1.8" strokeLinecap="round" strokeLinejoin="round">
        <rect x="9" y="9" width="11" height="11" rx="2" />
        <path d="M5 15V5a2 2 0 0 1 2-2h10" />
      </svg>
    ),
  },
  {
    view: "apps",
    title: "应用卸载",
    desc: "彻底卸载应用并清除残留配置与缓存文件。",
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
  const [state, setState] = useState<LoadState>({ status: "loading" });

  const load = useCallback(async () => {
    setState({ status: "loading" });
    try {
      const volumes = await getVolumes();
      setState({ status: "ready", volumes });
    } catch (err: unknown) {
      const message =
        err && typeof err === "object" && "message" in err
          ? String((err as { message: unknown }).message)
          : "无法读取磁盘信息";
      setState({ status: "error", message });
    }
  }, []);

  useEffect(() => {
    void load();
  }, [load]);

  return (
    <div className="tc-overview">
      <section className="tc-hero">
        <div className="tc-hero__copy">
          <p className="tc-hero__eyebrow">磁盘健康</p>
          <h2 className="tc-hero__title">让你的磁盘保持清爽</h2>
          <p className="tc-hero__lead">
            一次扫描，看清空间去向。TrueClean 会分类统计磁盘占用，
            找出可安全清理的垃圾，并让 AI 助手帮你做决策。
          </p>
          <div className="tc-hero__cta">
            <Button variant="primary" size="lg" onClick={onStartScan}>
              开始扫描
            </Button>
            <Button variant="ghost" size="lg" onClick={() => onNavigate("junk")}>
              快速清理垃圾
            </Button>
          </div>
        </div>
        <div className="tc-hero__glow" aria-hidden="true" />
      </section>

      <section className="tc-section">
        <header className="tc-section__head">
          <h3 className="tc-section__title">磁盘卷</h3>
          {state.status === "ready" && (
            <span className="tc-section__meta">
              {state.volumes.length} 个磁盘
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
              title="读取磁盘信息失败"
              description={state.message}
              action={
                <Button variant="subtle" onClick={() => void load()}>
                  重试
                </Button>
              }
            />
          </SurfaceCard>
        )}

        {state.status === "ready" && state.volumes.length === 0 && (
          <SurfaceCard elevation="sm">
            <EmptyState compact title="未发现可用磁盘" />
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
                        <span className="tc-vol-card__badge">可移动</span>
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
                      剩余 {formatBytes(v.availableBytes)} · {v.fileSystem}
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
          <h3 className="tc-section__title">从这里开始</h3>
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
                <h4 className="tc-guide-card__title">{card.title}</h4>
                <p className="tc-guide-card__desc">{card.desc}</p>
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
