import { ArrowLeft, RotateCcw } from "lucide-react";
import type { JournalSummary } from "../types";
import { Badge, Button, Count, Panel } from "../ui";
import { formatWhen } from "../format";

const STATUS: Record<string, { label: string; tone: "brand" | "neutral" | "error" }> = {
  applied: { label: "Applied", tone: "brand" },
  interrupted: { label: "Interrupted", tone: "error" },
  undone: { label: "Undone", tone: "neutral" },
  partially_undone: { label: "Partly undone", tone: "error" },
  empty: { label: "Nothing moved", tone: "neutral" },
};

export function History({ items, onUndo, busy, onBack }: { items: JournalSummary[]; onUndo: (id: string) => void; busy: boolean; onBack: () => void }) {
  const visible = items.filter((i) => i.status !== "empty");
  return (
    <div className="mx-auto max-w-3xl">
      <h2 className="text-[34px] font-bold leading-tight tracking-tight text-ink">History</h2>
      <p className="mt-1 text-lg text-ink-2">Every cleanup is recorded on this computer, so you can undo it any time.</p>

      <Panel className="mt-8 overflow-hidden">
        {visible.length === 0 && <p className="px-6 py-10 text-center text-ink-2">No cleanups yet.</p>}
        <ul className="divide-y divide-line">
          {visible.map((j) => {
            const s = STATUS[j.status] ?? STATUS.applied;
            const canUndo = j.status === "applied" || j.status === "interrupted" || j.status === "partially_undone";
            return (
              <li key={j.id} className="group flex items-center justify-between gap-4 px-6 py-4 transition-colors hover:bg-tint">
                <div className="min-w-0">
                  <div className="flex items-center gap-3">
                    <span className="text-[15px] font-semibold text-ink">{formatWhen(j.created_at)}</span>
                    <Badge tone={s.tone}>{s.label}</Badge>
                  </div>
                  <div className="mt-0.5 truncate text-sm text-ink-3 transition-colors group-hover:text-ink-2">
                    <Count n={j.moved} one="file" /> moved in <span className="font-mono">{j.root}</span>
                    {j.restored > 0 && (
                      <>
                        {" · "}
                        <span className="font-mono">{j.restored}</span> restored
                      </>
                    )}
                  </div>
                </div>
                {canUndo && (
                  <Button size="sm" disabled={busy} onClick={() => onUndo(j.id)}>
                    <RotateCcw size={14} /> Undo
                  </Button>
                )}
              </li>
            );
          })}
        </ul>
      </Panel>
      <Button variant="quiet" className="mt-8" onClick={onBack}>
        <ArrowLeft size={16} /> Back
      </Button>
    </div>
  );
}
