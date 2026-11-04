// Small shared UI building blocks.
import type { ReactNode } from "react";

export const CATEGORY_STYLE: Record<string, { icon: string; color: string }> = {
  Documents: { icon: "📄", color: "#3b82f6" },
  PDFs: { icon: "📕", color: "#ef4444" },
  Images: { icon: "🖼️", color: "#ec4899" },
  Screenshots: { icon: "📸", color: "#a855f7" },
  Videos: { icon: "🎬", color: "#f97316" },
  Audio: { icon: "🎵", color: "#14b8a6" },
  Installers: { icon: "💿", color: "#64748b" },
  Archives: { icon: "🗜️", color: "#eab308" },
  Code: { icon: "💻", color: "#0ea5e9" },
  Spreadsheets: { icon: "📊", color: "#22c55e" },
  Presentations: { icon: "📽️", color: "#f43f5e" },
  "Bills & Receipts": { icon: "🧾", color: "#10b981" },
  "College/Study": { icon: "🎓", color: "#6366f1" },
  Other: { icon: "📦", color: "#94a3b8" },
};

const FALLBACK_COLORS = ["#06b6d4", "#84cc16", "#d946ef", "#f59e0b", "#8b5cf6"];

export function catStyle(name: string) {
  if (CATEGORY_STYLE[name]) return CATEGORY_STYLE[name];
  let h = 0;
  for (const c of name) h = (h * 31 + c.charCodeAt(0)) >>> 0;
  return { icon: "📁", color: FALLBACK_COLORS[h % FALLBACK_COLORS.length] };
}

export function Button({
  children,
  onClick,
  variant = "secondary",
  disabled,
  className = "",
  title,
  type = "button",
}: {
  children: ReactNode;
  onClick?: () => void;
  variant?: "primary" | "secondary" | "ghost" | "danger";
  disabled?: boolean;
  className?: string;
  title?: string;
  type?: "button" | "submit";
}) {
  const base =
    "inline-flex items-center justify-center gap-2 rounded-xl px-4 py-2 text-sm font-semibold transition focus:outline-none focus-visible:ring-2 focus-visible:ring-brand-400 disabled:cursor-not-allowed disabled:opacity-50";
  const styles = {
    primary: "bg-brand-600 text-white shadow-sm hover:bg-brand-700 active:scale-[.98]",
    secondary:
      "bg-white text-slate-800 ring-1 ring-slate-200 hover:bg-slate-50 dark:bg-slate-800 dark:text-slate-100 dark:ring-slate-700 dark:hover:bg-slate-700",
    ghost: "text-slate-600 hover:bg-slate-100 dark:text-slate-300 dark:hover:bg-slate-800",
    danger: "bg-amber-500 text-white hover:bg-amber-600",
  }[variant];
  return (
    <button type={type} title={title} disabled={disabled} onClick={onClick} className={`${base} ${styles} ${className}`}>
      {children}
    </button>
  );
}

export function Card({ children, className = "" }: { children: ReactNode; className?: string }) {
  return (
    <div className={`rounded-2xl bg-white p-5 shadow-sm ring-1 ring-slate-200/70 dark:bg-slate-900 dark:ring-slate-800 ${className}`}>
      {children}
    </div>
  );
}

export function Toggle({
  checked,
  onChange,
  label,
  hint,
  disabled,
}: {
  checked: boolean;
  onChange: (v: boolean) => void;
  label: ReactNode;
  hint?: ReactNode;
  disabled?: boolean;
}) {
  return (
    <label className={`flex cursor-pointer items-start gap-3 ${disabled ? "opacity-50" : ""}`}>
      <button
        type="button"
        role="switch"
        aria-checked={checked}
        disabled={disabled}
        onClick={() => onChange(!checked)}
        className={`relative mt-0.5 h-6 w-11 shrink-0 rounded-full transition ${checked ? "bg-brand-500" : "bg-slate-300 dark:bg-slate-600"}`}
      >
        <span
          className={`absolute top-0.5 h-5 w-5 rounded-full bg-white shadow transition-all ${checked ? "left-[22px]" : "left-0.5"}`}
        />
      </button>
      <span>
        <span className="block text-sm font-medium">{label}</span>
        {hint && <span className="block text-xs text-slate-500 dark:text-slate-400">{hint}</span>}
      </span>
    </label>
  );
}

export function ProgressBar({ value, max }: { value: number; max: number }) {
  const pct = max > 0 ? Math.min(100, Math.round((value / max) * 100)) : 0;
  return (
    <div className="h-2.5 w-full overflow-hidden rounded-full bg-slate-200 dark:bg-slate-800">
      <div className="h-full rounded-full bg-brand-500 transition-all duration-300" style={{ width: `${pct}%` }} />
    </div>
  );
}

export function Modal({ children, onClose, title }: { children: ReactNode; onClose: () => void; title: string }) {
  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center bg-slate-900/50 p-4 backdrop-blur-sm" onClick={onClose}>
      <div
        role="dialog"
        aria-label={title}
        className="max-h-[90vh] w-full max-w-lg animate-pop overflow-auto rounded-2xl bg-white p-6 shadow-xl dark:bg-slate-900"
        onClick={(e) => e.stopPropagation()}
      >
        <div className="mb-4 flex items-center justify-between">
          <h2 className="text-lg font-bold">{title}</h2>
          <button className="rounded-lg p-1 text-slate-500 hover:bg-slate-100 dark:hover:bg-slate-800" onClick={onClose} aria-label="Close">
            ✕
          </button>
        </div>
        {children}
      </div>
    </div>
  );
}

export function Pill({ children, tone = "slate" }: { children: ReactNode; tone?: "slate" | "brand" | "violet" | "amber" | "blue" }) {
  const tones = {
    slate: "bg-slate-100 text-slate-600 dark:bg-slate-800 dark:text-slate-300",
    brand: "bg-brand-100 text-brand-800 dark:bg-brand-900/60 dark:text-brand-200",
    violet: "bg-violet-100 text-violet-700 dark:bg-violet-900/50 dark:text-violet-200",
    amber: "bg-amber-100 text-amber-800 dark:bg-amber-900/50 dark:text-amber-200",
    blue: "bg-blue-100 text-blue-700 dark:bg-blue-900/50 dark:text-blue-200",
  }[tone];
  return <span className={`inline-flex items-center gap-1 rounded-full px-2 py-0.5 text-xs font-medium ${tones}`}>{children}</span>;
}
