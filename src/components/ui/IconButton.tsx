import type { ButtonHTMLAttributes, ReactNode } from "react";

export type IconButtonVariant = "ghost" | "subtle" | "primary" | "danger";
export type IconButtonSize = "sm" | "md" | "lg";

interface IconButtonProps extends ButtonHTMLAttributes<HTMLButtonElement> {
  /** Required for accessibility — describes the action. */
  label: string;
  icon: ReactNode;
  variant?: IconButtonVariant;
  size?: IconButtonSize;
  active?: boolean;
}

export function IconButton({
  label,
  icon,
  variant = "ghost",
  size = "md",
  active = false,
  className,
  type = "button",
  ...rest
}: IconButtonProps) {
  const classes = [
    "tc-icon-btn",
    `tc-icon-btn--${variant}`,
    `tc-icon-btn--${size}`,
    active ? "is-active" : "",
    className ?? "",
  ]
    .filter(Boolean)
    .join(" ");

  return (
    <button
      type={type}
      className={classes}
      aria-label={label}
      title={label}
      aria-pressed={active || undefined}
      {...rest}
    >
      <span className="tc-icon-btn__glyph" aria-hidden="true">
        {icon}
      </span>
    </button>
  );
}

export default IconButton;
