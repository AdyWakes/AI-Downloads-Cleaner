import type { AiStatus, JournalSummary } from "../types";
import { Button, Card, Toggle } from "../ui";
import { formatWhen, plural } from "../format";

export function Home({
  folder,
  onPick,
  onScan,
  useAi,
  setUseAi,
  byMonth,
  setByMonth,
  ai,
  onAiHelp,
  lastUndoable,
  onUndo,
  error,
}: {
  folder: string | null;
  onPick: () => void;
  onScan: () => void;
  useAi: boolean;
  setUseAi: (v: boolean) => void;
  byMonth: boolean;
  setByMonth: (v: boolean) => void;
  ai: AiStatus | null;
  onAiHelp: () => void;
  lastUndoable: JournalSummary | null;
  onUndo: () => void;
  error: string | null;
}) {
  return (
    <div className="mx-auto max-w-2xl py-6">
      <div className="mb-8 text-center">
        <h1 className="text-4xl font-extrabold tracking-tight">
          Chaos to clean in <span className="text-brand-600 dark:text-brand-400">10 seconds</span>.
        </h1>
        <p className="mt-3 text-slate-600 dark:text-slate-300">
          Nothing leaves your computer, nothing is ever deleted, and one click undoes everything.
        </p>
      </div>

      {lastUndoable && (
        <div className="mb-4 flex items-center justify-between gap-3 rounded-2xl bg-amber-50 px-4 py-3 text-sm ring-1 ring-amber-200 dark:bg-amber-900/20 dark:ring-amber-800">
          <span>
            Last cleanup ({formatWhen(lastUndoable.created_at)}) moved {plural(lastUndoable.moved, "file")}.
          </span>
          <Button variant="secondary" onClick={onUndo}>
            ↩ Undo it
          </Button>
        </div>
      )}

      <Card>
        <div className="text-xs font-semibold uppercase tracking-wide text-slate-500">Folder to clean</div>
        <div className="mt-2 flex items-center gap-3">
          <div className="flex min-w-0 flex-1 items-center gap-3 rounded-xl bg-slate-50 px-4 py-3 ring-1 ring-slate-200 dark:bg-slate-800 dark:ring-slate-700">
            <span className="text-2xl">📂</span>
            <span className="truncate font-medium" title={folder ?? ""}>
              {folder ?? "No folder chosen"}
            </span>
          </div>
          <Button onClick={onPick}>Change…</Button>
        </div>

        <div className="mt-5 grid gap-4 sm:grid-cols-2">
          <Toggle
            checked={useAi && !!ai?.active}
            disabled={!ai?.active}
            onChange={setUseAi}
            label="Use local AI for unclear files"
            hint={
              ai?.active ? (
                `${ai.model} · runs on this computer`
              ) : (
                <button className="text-brand-700 underline dark:text-brand-400" onClick={onAiHelp}>
                  Set up free local AI (optional)
                </button>
              )
            }
          />
          <Toggle checked={byMonth} onChange={setByMonth} label="Organize by month" hint="e.g. Images/2024-05" />
        </div>

        {error && (
          <div className="mt-4 rounded-xl bg-red-50 px-4 py-3 text-sm text-red-700 ring-1 ring-red-200 dark:bg-red-900/20 dark:text-red-300 dark:ring-red-900">
            {error}
          </div>
        )}

        <Button variant="primary" className="mt-6 w-full py-3 text-base" disabled={!folder} onClick={onScan}>
          Scan folder
        </Button>
        <p className="mt-3 text-center text-xs text-slate-500">Scanning only looks. You'll see a full preview before anything moves.</p>
      </Card>
    </div>
  );
}
