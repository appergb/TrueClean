import type { ButtonHTMLAttributes, ReactNode } from "react";

export type ButtonVariant = "primary" | "ghost" | "danger" | "subtle";
export type ButtonSize = "sm" | "md" | "lg";

interface ButtonProps extends ButtonHTMLAttributes<HTMLButtonElement> {
  variant?: ButtonVariant;
  size?: ButtonSize;
  /** Icon rendered before the label. */
  iconLeading?: ReactNode;
  /** Icon rendered after the label. */
  iconTrailing?: ReactNode;
  /** Stretch to fill the available inline space. */
  block?: boolean;
}

export function Button({
  variant = "subtle",
  size = "md",
  iconLeading,
  iconTrailing,
  block = false,
  className,
  children,
  type = "button",
  ...rest
}: ButtonProps) {
  const classes = [
    "tc-btn",
    `tc-btn--${variant}`,
    `tc-btn--${size}`,
    block ? "tc-btn--block" : "",
    className ?? "",
  ]
    .filter(Boolean)
    .join(" ");

  return (
    <button type={type} className={classes} {...rest}>
      {iconLeading != null && (
        <span className="tc-btn__icon" aria-hidden="true">
          {iconLeading}
        </span>
      )}
      {children != null && <span className="tc-btn__label">{children}</span>}
      {iconTrailing != null && (
        <span className="tc-btn__icon" aria-hidden="true">
          {iconTrailing}
        </span>
      )}
    </button>
  );
}

export default Button;
