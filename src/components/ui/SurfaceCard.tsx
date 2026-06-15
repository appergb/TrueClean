import type { HTMLAttributes, ReactNode } from "react";

export type SurfaceElevation = "flat" | "sm" | "md" | "lg";

interface SurfaceCardProps extends HTMLAttributes<HTMLDivElement> {
  elevation?: SurfaceElevation;
  /** Adds interactive hover/active affordance (lift + ring on focus). */
  interactive?: boolean;
  /** Removes the default inner padding when you need full-bleed content. */
  flush?: boolean;
  children: ReactNode;
}

export function SurfaceCard({
  elevation = "sm",
  interactive = false,
  flush = false,
  className,
  children,
  ...rest
}: SurfaceCardProps) {
  const classes = [
    "tc-surface",
    `tc-surface--${elevation}`,
    interactive ? "tc-surface--interactive" : "",
    flush ? "tc-surface--flush" : "",
    className ?? "",
  ]
    .filter(Boolean)
    .join(" ");

  return (
    <div className={classes} {...rest}>
      {children}
    </div>
  );
}

export default SurfaceCard;
