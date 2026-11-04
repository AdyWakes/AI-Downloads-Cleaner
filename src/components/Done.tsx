import { useEffect, useMemo, useState } from "react";
import type { ApplyReport, PlanItem, UndoReport } from "../types";
import { Button, Card, catStyle } from "../ui";
import { baseName, groupBy, plural } from "../format";

function rand(seed: number) {
  const x = Math.sin(seed * 9301 + 49297) * 233280;
  return x - Math.floor(x);
}

/** The demo-GIF moment: scattered files fly into their folders. */
export function BeforeAfter({ items, order }: { items: PlanItem[]; order: string[] }) {
  const [phase, setPhase] = useState<"before" | "after">("before");
  const bins = useMemo(() => {
    const g = groupBy(items, (i) => i.category, order).sort((a, b) => b[1].length - a[1].length);
    if (g.length <= 7) return g;
    const rest = g.slice(6).flatMap(([, v]) => v);
    return [...g.slice(0, 6), [`+${g.length - 6} more`, rest] as [string, PlanItem[]]];
  }, [items, order]);
  const binIndex = new Map<string, number>();
  bins.forEach(([, list], i) => list.forEach((it) => binIndex.set(it.category, i)));
  const sample = items.slice(0, 36);

  useEffect(() => {
    const t = setTimeout(() => setPhase("after"), 700);
    return () => clearTimeout(t);
  }, []);

  return (
    <div className="relative h-72 w-full overflow-hidden rounded-2xl bg-gradient-to-b from-slate-100 to-white ring-1 ring-slate-200 dark:from-slate-900 dark:to-slate-950 dark:ring-slate-800">
      <div
        className={`absolute left-4 top-3 text-xs font-semibold uppercase tracking-wide transition-opacity duration-500 ${
          phase === "before" ? "text-red-500 opacity-100" : "opacity-0"
        }`}
      >
        Before: chaos
      </div>
      <div
        className={`absolute left-4 top-3 text-xs font-semibold uppercase tracking-wide text-brand-600 transition-opacity delay-1000 duration-500 ${
          phase === "after" ? "opacity-100" : "opacity-0"
        }`}
      >
        After: clean ✨
      </div>
      {sample.map((it, i) => {
        const bi = binIndex.get(it.category) ?? bins.length - 1;
        const s = catStyle(it.category);
        const bx = ((bi + 0.5) / bins.length) * 100;
        const before = { left: 4 + rand(i) * 72, top: 10 + rand(i + 99) * 48, rot: (rand(i + 7) - 0.5) * 50 };
        const style =
          phase === "before"
            ? { left: `${before.left}%`, top: `${before.top}%`, transform: `rotate(${before.rot}deg) scale(1)`, opacity: 1 }
            : { left: `calc(${bx}% - 40px)`, top: "72%", transform: "rotate(0deg) scale(.35)", opacity: 0 };
        return (
          <div
            key={it.id}
            className="absolute flex max-w-[11rem] items-center gap-1.5 rounded-lg bg-white px-2 py-1 text-xs shadow-md ring-1 ring-slate-200 dark:bg-slate-800 dark:ring-slate-700"
            style={{
              ...style,
              transition: "all 900ms cubic-bezier(.6,-0.2,.3,1.2)",
              transitionDelay: `${i * 35}ms`,
            }}
          >
            <span>{s.icon}</span>
            <span className="truncate">{it.name}</span>
          </div>
        );
      })}
      <div className="absolute inset-x-0 bottom-3 flex justify-around px-2">
        {bins.map(([cat, list], i) => {
          const s = cat.startsWith("+") ? { icon: "🗂️", color: "#94a3b8" } : catStyle(cat);
          return (
            <div
              key={cat}
              className={`flex w-20 flex-col items-center transition-all duration-500 ${phase === "after" ? "translate-y-0 opacity-100" : "translate-y-6 opacity-40"}`}
              style={{ transitionDelay: `${300 + i * 60}ms` }}
            >
              <div className="relative flex h-12 w-14 items-center justify-center rounded-xl text-2xl shadow-sm" style={{ background: s.color + "26", border: `2px solid ${s.color}` }}>
                {s.icon}
                {phase === "after" && (
                  <span
                    className="absolute -right-2 -top-2 animate-pop rounded-full px-1.5 text-[10px] font-bold text-white"
                    style={{ background: s.color, animationDelay: `${1100 + i * 80}ms` }}
                  >
                    {list.length}
                  </span>
                )}
              </div>
              <span className="mt-1 w-full truncate text-center text-[10px] text-slate-600 dark:text-slate-400">{cat}</span>
            </div>
          );
        })}
      </div>
    </div>
  );
}

export function Done({
  report,
  items,
  order,
  root,
  onUndo,
  undoReport,
  undoing,
  onOpenFolder,
  onHome,
}: {
  report: ApplyReport;
  items: PlanItem[];
  order: string[];
  root: string;
  onUndo: () => void;
  undoReport: UndoReport | null;
  undoing: boolean;
  onOpenFolder: () => void;
  onHome: () => void;
}) {
  const movedIds = new Set(report.moved.map((m) => m.id));
  const moved = items.filter((i) => movedIds.has(i.id));
  const folders = new Set(moved.map((i) => i.dest_rel.split("/")[0]));
  const notMoved = [...report.skipped, ...report.failed];

  if (undoReport) {
    return (
      <div className="mx-auto max-w-2xl py-10 text-center">
        <div className="text-6xl">↩️</div>
        <h2 className="mt-4 text-2xl font-bold">Everything is back where it was</h2>
        <p className="mt-2 text-slate-600 dark:text-slate-300">
          Restored {plural(undoReport.restored.length, "file")}
          {undoReport.folders_removed > 0 && ` and removed ${plural(undoReport.folders_removed, "empty folder")} tidydrop had created`}.
        </p>
        {undoReport.restored.some((r) => r.renamed) && (
          <p className="mt-2 text-sm text-amber-700">Some files came back with “(1)” added because a new file with the same name was already there.</p>
        )}
        {undoReport.problems.length > 0 && (
          <Card className="mt-4 text-left">
            <h3 className="mb-2 font-semibold">Couldn't restore {plural(undoReport.problems.length, "file")}</h3>
            <ul className="space-y-1 text-sm">
              {undoReport.problems.map((p) => (
                <li key={p.path}>
                  <strong>{baseName(p.path)}</strong> — {p.reason}
                </li>
              ))}
            </ul>
            <p className="mt-2 text-xs text-slate-500">You can try Undo again later from History.</p>
          </Card>
        )}
        <Button variant="primary" className="mt-6" onClick={onHome}>
          Done
        </Button>
      </div>
    );
  }

  return (
    <div className="mx-auto max-w-3xl py-4">
      <BeforeAfter items={moved} order={order} />
      <div className="mt-6 text-center">
        <h2 className="text-3xl font-extrabold">
          {moved.length > 0 ? `Tidy! ${plural(moved.length, "file")} sorted into ${plural(folders.size, "folder")}.` : "Nothing was moved."}
        </h2>
        <p className="mt-2 text-slate-600 dark:text-slate-300">Changed your mind? One click puts everything back, even after you restart.</p>
        <div className="mt-5 flex flex-wrap justify-center gap-3">
          <Button variant="primary" onClick={onOpenFolder}>
            📂 Open {baseName(root)}
          </Button>
          <Button onClick={onUndo} disabled={undoing || moved.length === 0}>
            {undoing ? "Undoing…" : "↩ Undo this cleanup"}
          </Button>
          <Button variant="ghost" onClick={onHome}>
            Done
          </Button>
        </div>
      </div>
      {notMoved.length > 0 && (
        <Card className="mt-6">
          <h3 className="mb-2 font-semibold">Left in place ({notMoved.length})</h3>
          <ul className="space-y-1 text-sm">
            {notMoved.map((n) => (
              <li key={n.id} className="flex justify-between gap-3">
                <span className="truncate">{n.name}</span>
                <span className="shrink-0 text-slate-500">{n.reason}</span>
              </li>
            ))}
          </ul>
        </Card>
      )}
    </div>
  );
}
