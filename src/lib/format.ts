/** Human-readable sizes for machine facts. `undefined` must be rendered as a
 * gap by the caller, never as `0 B`. */
export function formatBytes(bytes: number): string {
  const units = ["B", "KB", "MB", "GB", "TB"];
  let value = bytes;
  let unit = 0;

  while (value >= 1024 && unit < units.length - 1) {
    value /= 1024;
    unit += 1;
  }

  return `${value.toFixed(unit === 0 ? 0 : 1)} ${units[unit]}`;
}

/** Time of day only, for "last updated" captions. */
export function formatTime(milliseconds: number): string {
  return new Date(milliseconds).toLocaleTimeString([], {
    hour: "2-digit",
    minute: "2-digit",
    second: "2-digit",
  });
}

/** Date and time, for values that are not from today. */
export function formatDateTime(milliseconds: number): string {
  return new Date(milliseconds).toLocaleString([], {
    dateStyle: "short",
    timeStyle: "medium",
  });
}
