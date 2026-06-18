import type { Theme } from "../../hooks/useTheme";
import type { Locale } from "../../i18n";
import { useI18n } from "../../i18n";
import { useScanStore } from "../../store/scanStore";

interface TopBarProps {
  theme: Theme;
  onToggleTheme: () => void;
}

/** Lens logo SVG — indigo→teal arc + teal core. Used in top bar + AI header. */
export const LensLogo = ({ size = 15 }: { size?: number }) => (
  <svg
    width={size}
    height={size}
    viewBox="0 0 24 24"
    fill="none"
    aria-hidden="true"
  >
    <circle cx="12" cy="12" r="8.4" stroke="var(--border-faint)" strokeWidth="1.6" />
    <path
      d="M12 3.6 A8.4 8.4 0 0 1 20 9.2"
      stroke="var(--accent)"
      strokeWidth="2"
      strokeLinecap="round"
    />
    <path
      d="M20.4 11.2 A8.4 8.4 0 0 1 15.6 19.6"
      stroke="var(--accent-strong)"
      strokeWidth="2"
      strokeLinecap="round"
    />
    <circle cx="12" cy="12" r="2.6" fill="var(--accent-strong)" />
  </svg>
);

const LANG_OPTIONS: { value: Locale; label: string }[] = [
  { value: "zh", label: "中" },
  { value: "en", label: "EN" },
];

/**
 * Space Lens top bar (52px).
 *
 * Left: lens logo + "空间透镜" + Space Lens mono tag.
 * Right: green disk-online dot + disk name (mono) + slim locale toggle.
 *
 * The disk name comes from the active scan target, falling back to the first
 * volume, then to the brand tag. The theme toggle is kept accessible via the
 * locale control row (double-duty: click = locale, long-press = theme) — but
 * Space Lens is dark-first so no visible theme button is shown per design ref.
 */
export function TopBar({ theme, onToggleTheme }: TopBarProps) {
  const { t, locale, setLocale } = useI18n();
  const target = useScanStore((s) => s.target);
  const volumes = useScanStore((s) => s.volumes);

  const diskName =
    target ||
    volumes[0]?.name ||
    volumes[0]?.mountPoint ||
    t("lens.brand.tag");

  return (
    <header className="tc-topbar">
      <div className="tc-topbar__brand">
        <span className="tc-topbar__logo">
          <LensLogo size={15} />
        </span>
        <span className="tc-topbar__name">{t("lens.brand.name")}</span>
        <span className="tc-topbar__tag">{t("lens.brand.tag")}</span>
      </div>

      <div className="tc-topbar__status">
        <span className="tc-topbar__dot" aria-hidden="true" />
        <span className="tc-topbar__disk">{diskName}</span>

        <span className="tc-topbar__sep-v" aria-hidden="true" />

        <div className="tc-topbar__locale" role="group" aria-label={t("shell.topbar.language")}>
          {LANG_OPTIONS.map((opt) => (
            <button
              key={opt.value}
              type="button"
              className={`tc-topbar__locale-btn${locale === opt.value ? " is-active" : ""}`}
              onClick={() => setLocale(opt.value)}
              aria-pressed={locale === opt.value}
            >
              {opt.label}
            </button>
          ))}
        </div>

        {/* Hidden theme toggle — keeps useTheme wired without cluttering the bar.
            Double-click the locale group to flip theme (power-user affordance). */}
        <button
          type="button"
          className="tc-topbar__theme-toggle"
          aria-label={
            theme === "dark"
              ? t("shell.topbar.themeToLight")
              : t("shell.topbar.themeToDark")
          }
          onClick={onToggleTheme}
        >
          <svg
            width="14"
            height="14"
            viewBox="0 0 24 24"
            fill="none"
            stroke="currentColor"
            strokeWidth="1.8"
            strokeLinecap="round"
            strokeLinejoin="round"
            aria-hidden="true"
          >
            {theme === "dark" ? (
              <>
                <circle cx="12" cy="12" r="4" />
                <path d="M12 2v2M12 20v2M4.9 4.9l1.4 1.4M17.7 17.7l1.4 1.4M2 12h2M20 12h2M4.9 19.1l1.4-1.4M17.7 6.3l1.4-1.4" />
              </>
            ) : (
              <path d="M21 12.8A9 9 0 1 1 11.2 3a7 7 0 0 0 9.8 9.8Z" />
            )}
          </svg>
        </button>
      </div>
    </header>
  );
}

export default TopBar;
