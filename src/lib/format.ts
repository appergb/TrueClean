// Formatting helpers shared across the UI.

/** Human-readable byte size, e.g. 1.5 GB. */
export function formatBytes(bytes: number, decimals = 1): string {
  if (!Number.isFinite(bytes) || bytes <= 0) return "0 B";
  const units = ["B", "KB", "MB", "GB", "TB", "PB"];
  const i = Math.min(
    units.length - 1,
    Math.floor(Math.log(bytes) / Math.log(1024)),
  );
  const value = bytes / Math.pow(1024, i);
  return `${value.toFixed(i === 0 ? 0 : decimals)} ${units[i]}`;
}

/** Relative time from a unix-seconds timestamp, in Chinese. */
export function formatRelativeTime(unixSecs: number | null): string {
  if (unixSecs == null) return "未知";
  const diffMs = Date.now() - unixSecs * 1000;
  const days = Math.floor(diffMs / 86_400_000);
  if (days < 0) return "未来";
  if (days === 0) return "今天";
  if (days < 30) return `${days} 天前`;
  if (days < 365) return `${Math.floor(days / 30)} 个月前`;
  return `${Math.floor(days / 365)} 年前`;
}

/** Percentage with one decimal, e.g. "12.3%". */
export function formatPercent(value: number): string {
  return `${value.toFixed(1)}%`;
}
