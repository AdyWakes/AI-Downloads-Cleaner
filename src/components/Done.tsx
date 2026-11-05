import { useEffect, useMemo, useState } from "react";
import { FolderOpen, RotateCcw } from "lucide-react";
import type { ApplyReport, PlanItem, UndoReport } from "../types";
import { Button, Count, Panel, categoryIcon } from "../ui";
import { baseName, groupBy, plural } from "../format";
import { FolderArt } from "./Art";

function rand(seed: number) {
  const x = Math.sin(seed * 9301 + 49297) * 233280;
  return x - Math.floor(x);
}

const SWITCH_MS = 900;
const FLIGHT_MS = 850;
const STAGGER_MS = 38;

/** The demo-GIF moment: a messy pile of files flies into orange folders, which gulp them up. */
export function BeforeAfter({ items, order }: { items: PlanItem[]; order: string[] }) {
  const [phase, setPhase] = useState<"before" | "after">("before");
  const bins = useMemo(() => {
    const g = groupBy(items, (i) => i.category, order).sort((a, b) => b[1].length - a[1].length);
    if (g.length <= 6) return g;
    const rest = g.slice(5).flatMap(([, v]) => v);
    return [...g.slice(0, 5), [`${g.length - 5} more`, rest] as [string, PlanItem[]]];
  }, [items, order]);
  const binOf = new Map<number, number>();
  bins.forEach(([, list], b) => list.forEach((it) => binOf.set(it.id, b)));
  const sample = items.slice(0, 32);
  // When does the last file land in each bin? That's when the folder gulps.
  const landAt = bins.map((_, b) => {
    const last = sample.reduce((m, it, i) => (binOf.get(it.id) === b ? i : m), 0);
    return last * STAGGER_MS + FLIGHT_MS * 0.8;
  });

  useEffect(() => {
    const t = setTimeout(() => setPhase("after"), SWITCH_MS);
    return () => clearTimeout(t);
  }, []);

  return (
    <div className="relative h-[340px] w-full overflow-hidden rounded-2xl bg-surface shadow-soft">
      <div className="absolute left-6 top-5 text-[15px] font-semibold">
        <span className={`transition-opacity duration-300 ${phase === "before" ? "text-ink opacity-100" : "opacity-0"}`}>Before</span>
        <span className={`absolute left-0 whitespace-nowrap text-accent transition-opacity duration-500 ${phase === "after" ? "opacity-100" : "opacity-0"}`} style={{ transitionDelay: "1200ms" }}>
          After
        </span>
      </div>

      {sample.map((it, i) => {
        const b = binOf.get(it.id) ?? 0;
        const Icon = categoryIcon(it.category);
        const bx = ((b + 0.5) / bins.length) * 100;
        const before = { x: 10 + rand(i) * 80, y: 15 + rand(i + 99) * 33, r: (rand(i + 7) - 0.5) * 36 };
        const style =
          phase === "before"
            ? { left: `${before.x}%`, top: `${before.y}%`, transform: `translate(-50%,-50%) rotate(${before.r}deg) scale(1)`, opacity: 1 }
            : { left: `${bx}%`, top: "58%", transform: "translate(-50%,-50%) rotate(0deg) scale(.25)", opacity: 0 };
        return (
          <div
            key={it.id}
            className="absolute flex max-w-[10.5rem] items-center gap-1.5 rounded-lg border border-line bg-surface px-2.5 py-1.5 text-[13px] text-ink shadow-soft"
            style={{
              ...style,
              transitionProperty: "left, top, transform, opacity",
              transitionDuration: `${FLIGHT_MS}ms, ${FLIGHT_MS}ms, ${FLIGHT_MS}ms, ${FLIGHT_MS * 0.5}ms`,
              transitionTimingFunction: "cubic-bezier(.55,-0.15,.35,1)",
              transitionDelay: `${i * STAGGER_MS}ms, ${i * STAGGER_MS}ms, ${i * STAGGER_MS}ms, ${i * STAGGER_MS + FLIGHT_MS * 0.55}ms`,
            }}
          >
            <Icon size={14} className="shrink-0 text-accent" aria-hidden />
            <span className="truncate">{it.name}</span>
          </div>
        );
      })}

      <div
        className="absolute inset-x-0 bottom-6 flex justify-around px-4 transition-transform duration-700 ease-out"
        style={{ transform: phase === "after" ? "translateY(-64px)" : "none", transitionDelay: `${Math.max(...landAt, 0) + 700}ms` }}
      >
        {bins.map(([cat, list], b) => (
          <div key={cat} className="flex w-28 flex-col items-center">
            <div className={phase === "after" ? "animate-gulp" : ""} style={{ animationDelay: `${landAt[b]}ms` }}>
              <FolderArt size={92} papers={list.length} filled={phase === "after"} delayMs={landAt[b] - 150} />
            </div>
            <span className="mt-2 w-full truncate text-center text-sm font-medium text-ink-2">{cat}</span>
            <span
              className={`text-sm font-medium text-accent ${phase === "after" ? "animate-pop" : "opacity-0"}`}
              style={{ animationDelay: `${landAt[b] + 150}ms` }}
            >
              <Count n={list.length} one="file" />
            </span>
          </div>
        ))}
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
      <div className="mx-auto max-w-2xl pt-16 text-center">
        <div className="mx-auto flex h-20 w-20 animate-pop items-center justify-center rounded-full bg-tint text-accent">
          <RotateCcw size={34} strokeWidth={1.8} aria-hidden />
        </div>
        <h2 className="mt-6 text-[34px] font-bold leading-tight tracking-tight text-ink">Everything is back where it was</h2>
        <p className="mt-3 text-lg text-ink-2">
          Restored <Count n={undoReport.restored.length} one="file" />
          {undoReport.folders_removed > 0 && (
            <>
              {" "}
              and removed <Count n={undoReport.folders_removed} one="empty folder" /> tidydrop had created
            </>
          )}
          .
        </p>
        {undoReport.restored.some((r) => r.renamed) && (
          <p className="mt-3 text-[15px] text-ink-2">Some files came back with “(1)” added because a new file with the same name was already there.</p>
        )}
        {undoReport.problems.length > 0 && (
          <Panel className="mt-8 p-6 text-left">
            <h3 className="mb-3 text-lg font-semibold text-error">Couldn't restore {plural(undoReport.problems.length, "file")}</h3>
            <ul className="divide-y divide-line">
              {undoReport.problems.map((p) => (
                <li key={p.path} className="py-2 text-[15px] text-ink">
                  <span className="font-semibold">{baseName(p.path)}</span> <span className="text-ink-2">— {p.reason}</span>
                </li>
              ))}
            </ul>
            <p className="mt-3 text-sm text-ink-3">You can try Undo again later from History.</p>
          </Panel>
        )}
        <Button variant="primary" size="lg" className="mt-10" onClick={onHome}>
          Done
        </Button>
      </div>
    );
  }

  return (
    <div className="mx-auto max-w-4xl">
      <BeforeAfter items={moved} order={order} />
      <div className="mt-10 text-center">
        <h2 className="text-[40px] font-bold leading-tight tracking-tight text-ink">
          {moved.length > 0 ? (
            <>
              Tidy. <span className="font-mono">{moved.length}</span> files sorted into <span className="font-mono">{folders.size}</span> folders.
            </>
          ) : (
            "Nothing was moved."
          )}
        </h2>
        <p className="mt-3 text-lg text-ink-2">Changed your mind? One click puts everything back, even after a restart.</p>
        <div className="mt-8 flex flex-wrap items-center justify-center gap-3">
          <Button variant="primary" size="lg" onClick={onOpenFolder}>
            <FolderOpen size={18} /> Open {baseName(root)}
          </Button>
          <Button size="lg" onClick={onUndo} disabled={undoing || moved.length === 0}>
            <RotateCcw size={17} /> {undoing ? "Undoing…" : "Undo this cleanup"}
          </Button>
          <Button variant="quiet" size="lg" onClick={onHome}>
            Done
          </Button>
        </div>
      </div>
      {notMoved.length > 0 && (
        <section className="mx-auto mt-12 max-w-2xl">
          <h3 className="mb-2 text-lg font-semibold text-ink">Left in place ({notMoved.length})</h3>
          <ul className="divide-y divide-line">
            {notMoved.map((n) => (
              <li key={n.id} className="flex justify-between gap-4 py-2 text-[15px]">
                <span className="truncate text-ink">{n.name}</span>
                <span className="shrink-0 text-ink-3">{n.reason}</span>
              </li>
            ))}
          </ul>
        </section>
      )}
    </div>
  );
}
