export function formatBytes(n: number): string {
  if (!Number.isFinite(n) || n < 0) return "–";
  const units = ["B", "KB", "MB", "GB", "TB"];
  let i = 0;
  let v = n;
  while (v >= 1024 && i < units.length - 1) {
    v /= 1024;
    i++;
  }
  const digits = i === 0 || v >= 100 ? 0 : v >= 10 ? 1 : 1;
  return `${v.toFixed(digits)} ${units[i]}`;
}

export function plural(n: number, one: string, many = one + "s"): string {
  return `${n.toLocaleString()} ${n === 1 ? one : many}`;
}

/** "20241104-213000-123" -> Date-ish label; falls back to the raw string. */
export function formatWhen(iso: string): string {
  const d = new Date(iso);
  if (Number.isNaN(d.getTime())) return iso;
  return d.toLocaleString(undefined, { dateStyle: "medium", timeStyle: "short" });
}

/** Last path component, for both / and \ separators. */
export function baseName(p: string): string {
  const parts = p.split(/[\\/]/).filter(Boolean);
  return parts[parts.length - 1] ?? p;
}

export const SKIP_LABELS: Record<string, string> = {
  directory: "Folder (left as is)",
  symlink: "Shortcut / link",
  hidden: "Hidden file",
  system: "System file",
  partial_download: "Download still in progress",
  recently_modified: "Changed in the last few minutes",
  in_use: "Open in another program",
  bad_name: "Filename can't be handled safely",
  unreadable: "Can't read file info",
};

/** Group items by category, keeping category order from config. */
export function groupBy<T>(items: T[], key: (t: T) => string, order: string[]): [string, T[]][] {
  const map = new Map<string, T[]>();
  for (const it of items) {
    const k = key(it);
    if (!map.has(k)) map.set(k, []);
    map.get(k)!.push(it);
  }
  const rank = (k: string) => {
    const i = order.indexOf(k);
    return i === -1 ? order.length : i;
  };
  return [...map.entries()].sort((a, b) => rank(a[0]) - rank(b[0]) || a[0].localeCompare(b[0]));
}
