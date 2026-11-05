import { useId } from "react";

// Small hand-drawn illustration pieces, all in token colours.

/**
 * An open orange folder. `papers` sheets peek out of it; they rise into view when `filled`
 * turns true (used on the Done screen as files settle in).
 */
export function FolderArt({ size = 64, papers = 0, filled = true, delayMs = 0, className = "" }: { size?: number; papers?: number; filled?: boolean; delayMs?: number; className?: string }) {
  const sheets = Array.from({ length: Math.min(papers, 3) });
  const clip = useId();
  return (
    <svg width={size} height={size * 0.82} viewBox="0 0 100 82" className={className} aria-hidden overflow="visible">
      <defs>
        <clipPath id={clip}>
          <rect x="-20" y="-40" width="140" height="110" />
        </clipPath>
      </defs>
      <path d="M6 14a6 6 0 0 1 6-6h22l7 8h47a6 6 0 0 1 6 6v50a6 6 0 0 1-6 6H12a6 6 0 0 1-6-6z" fill="rgb(var(--chart-2))" />
      <g clipPath={`url(#${clip})`}>
      {sheets.map((_, i) => (
        <g
          key={i}
          style={{
            transform: filled ? `translateY(0) rotate(${(i - 1) * 7}deg)` : "translateY(40px) rotate(0deg)",
            transformOrigin: "50px 60px",
            transition: `transform .45s cubic-bezier(.3,1.5,.5,1) ${delayMs + i * 90}ms`,
          }}
        >
          <rect x={26 + i * 6} y={6 + (2 - i) * 3} width="42" height="52" rx="3" fill="rgb(var(--paper))" stroke="rgb(var(--line))" strokeWidth="1.2" />
          <rect x={32 + i * 6} y={16 + (2 - i) * 3} width="24" height="3" rx="1.5" fill="rgb(var(--paper-line))" />
          <rect x={32 + i * 6} y={23 + (2 - i) * 3} width="16" height="3" rx="1.5" fill="rgb(var(--paper-line))" />
        </g>
      ))}
      </g>
      {papers === 0 && <rect x="14" y="20" width="72" height="40" rx="3" fill="rgb(var(--paper))" opacity=".9" />}
      <path d="M3 34a5 5 0 0 1 5-5h84a5 5 0 0 1 5 5l-5 38a6 6 0 0 1-6 5H14a6 6 0 0 1-6-5z" fill="rgb(var(--brand))" />
    </svg>
  );
}

/** A paper file with a folded corner. */
export function FileArt({ size = 28, className = "" }: { size?: number; className?: string }) {
  return (
    <svg width={size} height={size * 1.25} viewBox="0 0 40 50" className={className} aria-hidden>
      <path d="M4 2h22l10 10v34a2 2 0 0 1-2 2H4a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2z" fill="rgb(var(--paper))" stroke="rgb(var(--line))" strokeWidth="2" />
      <path d="M26 2v8a2 2 0 0 0 2 2h8" fill="rgb(var(--paper-line))" />
      <rect x="8" y="22" width="20" height="3" rx="1.5" fill="rgb(var(--paper-line))" />
      <rect x="8" y="29" width="14" height="3" rx="1.5" fill="rgb(var(--paper-line))" />
    </svg>
  );
}

/** Files dropping into a folder, looped. Used on the "working" screen. */
export function DroppingFiles() {
  return (
    <div className="relative mx-auto h-36 w-40">
      {[0, 1, 2].map((i) => (
        <div key={i} className="absolute left-1/2 top-2 animate-drop" style={{ marginLeft: -14 + (i - 1) * 18, animationDelay: `${i * 0.45}s` }}>
          <FileArt size={26} />
        </div>
      ))}
      <div className="absolute bottom-0 left-1/2 -translate-x-1/2">
        <FolderArt size={120} />
      </div>
    </div>
  );
}

/** Decorative hero for the home screen: a few tilted files beside a folder. */
export function HeroArt() {
  const files = [
    { x: 6, y: 30, r: -16 },
    { x: 34, y: 8, r: 9 },
    { x: 62, y: 34, r: -5 },
  ];
  return (
    <div className="relative h-28 w-44" aria-hidden>
      {files.map((f, i) => (
        <div key={i} className="absolute" style={{ left: f.x, top: f.y, transform: `rotate(${f.r}deg)` }}>
          <FileArt size={30} />
        </div>
      ))}
      <div className="absolute bottom-0 right-0">
        <FolderArt size={92} />
      </div>
    </div>
  );
}
