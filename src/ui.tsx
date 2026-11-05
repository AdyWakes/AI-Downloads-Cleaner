// Shared UI building blocks. All colours come from the design tokens in index.css.
import type { ReactNode } from "react";
import {
  Archive,
  CodeXml,
  FileText,
  FileType,
  Film,
  Folder,
  GraduationCap,
  Image,
  Music,
  Package,
  Presentation,
  Receipt,
  ScreenShare,
  Shapes,
  Sheet,
  X,
  type LucideIcon,
} from "lucide-react";

export const CATEGORY_ICON: Record<string, LucideIcon> = {
  Documents: FileText,
  PDFs: FileType,
  Images: Image,
  Screenshots: ScreenShare,
  Videos: Film,
  Audio: Music,
  Installers: Package,
  Archives: Archive,
  Code: CodeXml,
  Spreadsheets: Sheet,
  Presentations: Presentation,
  "Bills & Receipts": Receipt,
  "College/Study": GraduationCap,
  Other: Shapes,
};

export function categoryIcon(name: string): LucideIcon {
  return CATEGORY_ICON[name] ?? Folder;
}

/** Category icon on a soft orange tile. */
export function CategoryBadge({ name, size = "md" }: { name: string; size?: "sm" | "md" | "lg" }) {
  const Icon = categoryIcon(name);
  const box = { sm: "h-7 w-7 rounded-lg", md: "h-9 w-9 rounded-xl", lg: "h-12 w-12 rounded-2xl" }[size];
  const icon = { sm: 15, md: 18, lg: 24 }[size];
  return (
    <span className={`inline-flex shrink-0 items-center justify-center bg-tint text-accent ${box}`}>
      <Icon size={icon} strokeWidth={1.8} aria-hidden />
    </span>
  );
}

type Variant = "primary" | "outline" | "quiet";

export function Button({
  children,
  onClick,
  variant = "outline",
  disabled,
  className = "",
  title,
  type = "button",
  size = "md",
}: {
  children: ReactNode;
  onClick?: () => void;
  variant?: Variant;
  disabled?: boolean;
  className?: string;
  title?: string;
  type?: "button" | "submit";
  size?: "sm" | "md" | "lg";
}) {
  const base =
    "inline-flex items-center justify-center gap-2 font-semibold transition-colors duration-150 focus:outline-none focus-visible:ring-2 focus-visible:ring-brand focus-visible:ring-offset-2 focus-visible:ring-offset-bg disabled:cursor-not-allowed disabled:opacity-40";
  const sizes = {
    sm: "h-8 rounded-lg px-3 text-sm",
    md: "h-10 rounded-xl px-4 text-sm",
    lg: "h-12 rounded-xl px-6 text-base",
  }[size];
  const styles = {
    primary:
      "bg-action text-on-action shadow-soft hover:bg-action-hover active:translate-y-px disabled:bg-line disabled:text-ink-3 disabled:shadow-none disabled:opacity-100",
    outline: "border border-line bg-surface text-ink hover:border-tint-strong hover:bg-tint",
    quiet: "text-ink-2 hover:bg-tint hover:text-ink",
  }[variant];
  return (
    <button type={type} title={title} disabled={disabled} onClick={onClick} className={`${base} ${sizes} ${styles} ${className}`}>
      {children}
    </button>
  );
}

/** The one level of container. Never nest these. */
export function Panel({ children, className = "" }: { children: ReactNode; className?: string }) {
  return <section className={`rounded-2xl bg-surface shadow-soft ${className}`}>{children}</section>;
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
    <div className={`flex items-start gap-3 ${disabled ? "opacity-50" : ""}`}>
      <button
        type="button"
        role="switch"
        aria-checked={checked}
        disabled={disabled}
        onClick={() => onChange(!checked)}
        className={`relative mt-0.5 h-6 w-10 shrink-0 rounded-full transition-colors duration-200 focus:outline-none focus-visible:ring-2 focus-visible:ring-brand focus-visible:ring-offset-2 focus-visible:ring-offset-surface ${
          checked ? "bg-brand" : "bg-line"
        }`}
      >
        <span
          className={`absolute top-1 h-4 w-4 rounded-full bg-surface shadow-soft transition-all duration-200 ${checked ? "left-5" : "left-1"}`}
        />
      </button>
      <div className="min-w-0">
        <div className="text-[15px] font-medium leading-6 text-ink">{label}</div>
        {hint && <div className="text-sm text-ink-3">{hint}</div>}
      </div>
    </div>
  );
}

export function ProgressBar({ value, max }: { value: number; max: number }) {
  const pct = max > 0 ? Math.min(100, Math.round((value / max) * 100)) : 0;
  return (
    <div className="h-2 w-full overflow-hidden rounded-full bg-line" role="progressbar" aria-valuenow={pct} aria-valuemin={0} aria-valuemax={100}>
      <div className="h-full rounded-full bg-brand transition-all duration-300 ease-out" style={{ width: `${pct}%` }} />
    </div>
  );
}

/** Indeterminate bar for steps without a count. */
export function BusyBar() {
  return (
    <div className="relative h-2 w-full overflow-hidden rounded-full bg-line">
      <div className="absolute inset-y-0 w-2/5 animate-shimmer rounded-full bg-brand" />
    </div>
  );
}

export function Modal({ children, onClose, title }: { children: ReactNode; onClose: () => void; title: string }) {
  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center bg-ink/30 p-4" onClick={onClose}>
      <div
        role="dialog"
        aria-label={title}
        className="max-h-[90vh] w-full max-w-lg animate-pop overflow-auto rounded-2xl bg-surface p-7 shadow-lift"
        onClick={(e) => e.stopPropagation()}
      >
        <div className="mb-3 flex items-start justify-between gap-4">
          <h2 className="text-xl font-bold leading-7 text-ink">{title}</h2>
          <button className="-mr-2 -mt-1 rounded-lg p-2 text-ink-3 transition-colors hover:bg-tint hover:text-ink" onClick={onClose} aria-label="Close">
            <X size={18} />
          </button>
        </div>
        {children}
      </div>
    </div>
  );
}

export function Badge({ children, tone = "neutral" }: { children: ReactNode; tone?: "neutral" | "brand" | "success" | "error" }) {
  const tones = {
    neutral: "bg-bg text-ink-2",
    brand: "bg-tint text-accent",
    success: "bg-bg text-success",
    error: "bg-bg text-error",
  }[tone];
  return <span className={`inline-flex items-center gap-1 whitespace-nowrap rounded-full px-2 py-0.5 text-xs font-semibold ${tones}`}>{children}</span>;
}

export function Notice({ children, tone = "neutral" }: { children: ReactNode; tone?: "neutral" | "brand" | "error" }) {
  const tones = {
    neutral: "bg-surface text-ink-2",
    brand: "bg-tint text-ink",
    error: "bg-surface text-error",
  }[tone];
  return <div className={`rounded-xl px-4 py-3 text-sm leading-6 shadow-soft ${tones}`}>{children}</div>;
}

/** "30 files": the number in mono, the word in the UI font. */
export function Count({ n, one, many }: { n: number; one: string; many?: string }) {
  return (
    <>
      <span className="font-mono">{n.toLocaleString()}</span> {n === 1 ? one : (many ?? one + "s")}
    </>
  );
}
