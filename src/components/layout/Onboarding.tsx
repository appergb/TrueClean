// First-run onboarding — a 3-step intro shown until the user dismisses it.
// Persisted via localStorage so it only appears once.

import type { ReactNode } from "react";
import { useState } from "react";

import { useI18n } from "../../i18n";
import { Button } from "../ui/Button";
import { PermissionGuide } from "./PermissionGuide";

const STORAGE_KEY = "trueclean.onboarded";

function hasOnboarded(): boolean {
  try {
    return localStorage.getItem(STORAGE_KEY) === "1";
  } catch {
    return false;
  }
}

function markOnboarded(): void {
  try {
    localStorage.setItem(STORAGE_KEY, "1");
  } catch {
    /* ignore */
  }
}

interface Step {
  titleKey: string;
  descKey: string;
  icon: ReactNode;
}

const STEPS: Step[] = [
  {
    titleKey: "shell.onboarding.step1Title",
    descKey: "shell.onboarding.step1Desc",
    icon: (
      <svg viewBox="0 0 24 24" width="22" height="22" fill="none" stroke="currentColor" strokeWidth="1.8" strokeLinecap="round" strokeLinejoin="round" aria-hidden="true">
        <circle cx="11" cy="11" r="7" />
        <path d="m20 20-3.5-3.5" />
      </svg>
    ),
  },
  {
    titleKey: "shell.onboarding.step2Title",
    descKey: "shell.onboarding.step2Desc",
    icon: (
      <svg viewBox="0 0 24 24" width="22" height="22" fill="none" stroke="currentColor" strokeWidth="1.8" strokeLinecap="round" strokeLinejoin="round" aria-hidden="true">
        <path d="M9 11l3 3L22 4" />
        <path d="M21 12v7a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h11" />
      </svg>
    ),
  },
  {
    titleKey: "shell.onboarding.step3Title",
    descKey: "shell.onboarding.step3Desc",
    icon: (
      <svg viewBox="0 0 24 24" width="22" height="22" fill="none" stroke="currentColor" strokeWidth="1.8" strokeLinecap="round" strokeLinejoin="round" aria-hidden="true">
        <path d="M12 3v4M12 17v4M3 12h4M17 12h4M5.6 5.6l2.8 2.8M15.6 15.6l2.8 2.8M18.4 5.6l-2.8 2.8M8.4 15.6l-2.8 2.8" />
      </svg>
    ),
  },
];

interface OnboardingProps {
  /** Called when the user clicks "Get started" — typically navigates to scan. */
  onStart: () => void;
}

export function Onboarding({ onStart }: OnboardingProps) {
  const { t } = useI18n();
  const [dismissed, setDismissed] = useState(hasOnboarded());

  if (dismissed) return null;

  const handleStart = (): void => {
    markOnboarded();
    setDismissed(true);
    onStart();
  };

  const handleSkip = (): void => {
    markOnboarded();
    setDismissed(true);
  };

  return (
    <section className="tc-onboard" aria-label={t("shell.onboarding.title")}>
      <div className="tc-onboard__head">
        <div>
          <h3 className="tc-onboard__title">{t("shell.onboarding.title")}</h3>
          <p className="tc-onboard__subtitle">{t("shell.onboarding.subtitle")}</p>
        </div>
        <button
          type="button"
          className="tc-onboard__skip"
          onClick={handleSkip}
          aria-label={t("shell.onboarding.skip")}
        >
          {t("shell.onboarding.skip")}
        </button>
      </div>
      <ol className="tc-onboard__steps">
        {STEPS.map((step, i) => (
          <li key={i} className="tc-onboard__step">
            <span className="tc-onboard__step-num" aria-hidden="true">
              {i + 1}
            </span>
            <span className="tc-onboard__step-icon" aria-hidden="true">
              {step.icon}
            </span>
            <div className="tc-onboard__step-body">
              <h4 className="tc-onboard__step-title">{t(step.titleKey)}</h4>
              <p className="tc-onboard__step-desc">{t(step.descKey)}</p>
            </div>
          </li>
        ))}
      </ol>
      <PermissionGuide />
      <div className="tc-onboard__cta">
        <Button variant="primary" size="lg" onClick={handleStart}>
          {t("shell.onboarding.start")}
        </Button>
      </div>
    </section>
  );
}

export default Onboarding;
