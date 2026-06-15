import type { ReactNode } from "react";

interface ProgressRingProps {
  /** Progress in the range 0..1 (values are clamped). */
  value: number;
  /** Outer diameter in px. */
  size?: number;
  /** Stroke thickness in px. */
  thickness?: number;
  /** Center content (e.g. a percentage). Falls back to a derived percent. */
  label?: ReactNode;
  /** Stroke color (CSS value). Defaults to the accent token. */
  color?: string;
  /** Hide the auto-generated percent label. */
  hideLabel?: boolean;
}

export function ProgressRing({
  value,
  size = 96,
  thickness = 8,
  label,
  color,
  hideLabel = false,
}: ProgressRingProps) {
  const clamped = Math.max(0, Math.min(1, Number.isFinite(value) ? value : 0));
  const radius = (size - thickness) / 2;
  const circumference = 2 * Math.PI * radius;
  const dashOffset = circumference * (1 - clamped);
  const percentText = `${Math.round(clamped * 100)}%`;

  return (
    <div
      className="tc-ring"
      style={{ width: size, height: size }}
      role="img"
      aria-label={`${percentText} 已使用`}
    >
      <svg
        className="tc-ring__svg"
        width={size}
        height={size}
        viewBox={`0 0 ${size} ${size}`}
      >
        <circle
          className="tc-ring__track"
          cx={size / 2}
          cy={size / 2}
          r={radius}
          fill="none"
          strokeWidth={thickness}
        />
        <circle
          className="tc-ring__value"
          cx={size / 2}
          cy={size / 2}
          r={radius}
          fill="none"
          strokeWidth={thickness}
          strokeLinecap="round"
          stroke={color ?? "var(--accent)"}
          strokeDasharray={circumference}
          strokeDashoffset={dashOffset}
          transform={`rotate(-90 ${size / 2} ${size / 2})`}
        />
      </svg>
      {!hideLabel && (
        <div className="tc-ring__label">{label ?? percentText}</div>
      )}
    </div>
  );
}

export default ProgressRing;
