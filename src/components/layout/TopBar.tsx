import { IconButton } from "../ui/IconButton";
import { Button } from "../ui/Button";
import { Segmented } from "../ui/Segmented";
import type { ViewId } from "./Sidebar";
import { NAV_LABEL_KEYS } from "./Sidebar";
import type { Theme } from "../../hooks/useTheme";
import { useI18n } from "../../i18n";
import type { Locale } from "../../i18n";

interface TopBarProps {
  current: ViewId;
  theme: Theme;
  onToggleTheme: () => void;
  agentOpen: boolean;
  onToggleAgent: () => void;
}

const SunIcon = (
  <svg viewBox="0 0 24 24" width="18" height="18" fill="none" stroke="currentColor" strokeWidth="1.8" strokeLinecap="round" strokeLinejoin="round" aria-hidden="true">
    <circle cx="12" cy="12" r="4" />
    <path d="M12 2v2M12 20v2M4.9 4.9l1.4 1.4M17.7 17.7l1.4 1.4M2 12h2M20 12h2M4.9 19.1l1.4-1.4M17.7 6.3l1.4-1.4" />
  </svg>
);

const MoonIcon = (
  <svg viewBox="0 0 24 24" width="18" height="18" fill="none" stroke="currentColor" strokeWidth="1.8" strokeLinecap="round" strokeLinejoin="round" aria-hidden="true">
    <path d="M21 12.8A9 9 0 1 1 11.2 3a7 7 0 0 0 9.8 9.8Z" />
  </svg>
);

const SparkIcon = (
  <svg viewBox="0 0 24 24" width="16" height="16" fill="none" stroke="currentColor" strokeWidth="1.8" strokeLinecap="round" strokeLinejoin="round" aria-hidden="true">
    <path d="M12 3v4M12 17v4M3 12h4M17 12h4M5.6 5.6l2.8 2.8M15.6 15.6l2.8 2.8M18.4 5.6l-2.8 2.8M8.4 15.6l-2.8 2.8" />
  </svg>
);

export function TopBar({
  current,
  theme,
  onToggleTheme,
  agentOpen,
  onToggleAgent,
}: TopBarProps) {
  const { t, locale, setLocale } = useI18n();
  return (
    <header className="tc-topbar">
      <div className="tc-topbar__heading">
        <span className="tc-topbar__crumb">{t("shell.topbar.crumb")}</span>
        <span className="tc-topbar__sep" aria-hidden="true">
          /
        </span>
        <h1 className="tc-topbar__title">{t(NAV_LABEL_KEYS[current])}</h1>
      </div>

      <div className="tc-topbar__actions">
        <Segmented<Locale>
          size="sm"
          ariaLabel={t("shell.topbar.language")}
          value={locale}
          onChange={setLocale}
          options={[
            { value: "zh", label: t("shell.topbar.langZh") },
            { value: "en", label: t("shell.topbar.langEn") },
          ]}
        />
        <IconButton
          label={theme === "dark" ? t("shell.topbar.themeToLight") : t("shell.topbar.themeToDark")}
          icon={theme === "dark" ? SunIcon : MoonIcon}
          onClick={onToggleTheme}
        />
        <Button
          variant={agentOpen ? "subtle" : "primary"}
          iconLeading={SparkIcon}
          onClick={onToggleAgent}
          aria-pressed={agentOpen}
        >
          {t("shell.topbar.aiAssistant")}
        </Button>
      </div>
    </header>
  );
}

export default TopBar;
