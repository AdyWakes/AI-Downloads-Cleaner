import type { JournalSummary } from "../types";
import { Button, Card, Pill } from "../ui";
import { formatWhen, plural } from "../format";

const STATUS: Record<string, { label: string; tone: "brand" | "slate" | "amber" | "blue" }> = {
  applied: { label: "Applied", tone: "brand" },
  interrupted: { label: "Interrupted", tone: "amber" },
  undone: { label: "Undone", tone: "slate" },
  partially_undone: { label: "Partly undone", tone: "amber" },
  empty: { label: "Nothing moved", tone: "slate" },
};

export function History({ items, onUndo, busy, onBack }: { items: JournalSummary[]; onUndo: (id: string) => void; busy: boolean; onBack: () => void }) {
  const visible = items.filter((i) => i.status !== "empty");
  return (
    <div className="mx-auto max-w-3xl">
      <h2 className="mb-1 text-2xl font-bold">History</h2>
      <p className="mb-4 text-sm text-slate-500">Every cleanup is recorded on this computer, so you can undo it any time.</p>
      {visible.length === 0 && <Card>No cleanups yet.</Card>}
      <div className="space-y-2">
        {visible.map((j) => {
          const s = STATUS[j.status] ?? STATUS.applied;
          const canUndo = j.status === "applied" || j.status === "interrupted" || j.status === "partially_undone";
          return (
            <Card key={j.id} className="flex items-center justify-between gap-3 !py-3">
              <div className="min-w-0">
                <div className="flex items-center gap-2 font-medium">
                  {formatWhen(j.created_at)} <Pill tone={s.tone}>{s.label}</Pill>
                </div>
                <div className="truncate text-xs text-slate-500">
                  {plural(j.moved, "file")} moved in {j.root}
                  {j.restored > 0 && ` · ${j.restored} restored`}
                </div>
              </div>
              {canUndo && (
                <Button disabled={busy} onClick={() => onUndo(j.id)}>
                  ↩ Undo
                </Button>
              )}
            </Card>
          );
        })}
      </div>
      <Button variant="ghost" className="mt-6" onClick={onBack}>
        ← Back
      </Button>
    </div>
  );
}
