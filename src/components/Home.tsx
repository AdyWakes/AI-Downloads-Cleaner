import { FolderOpen, Lock, RotateCcw } from "lucide-react";
import type { AiStatus, JournalSummary } from "../types";
import { Button, Count, Panel, Toggle } from "../ui";
import { formatWhen } from "../format";
import { HeroArt } from "./Art";

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
    <div className="mx-auto max-w-2xl pt-6">
      <div className="flex items-end justify-between gap-6">
        <div>
          <h1 className="text-[44px] font-bold leading-[1.05] tracking-tight text-ink">
            Chaos to clean
            <br />
            in <span className="text-accent">10 seconds.</span>
          </h1>
          <p className="mt-4 max-w-md text-lg leading-7 text-ink-2">Nothing leaves your computer, nothing is ever deleted, and one click undoes everything.</p>
        </div>
        <div className="hidden shrink-0 sm:block">
          <HeroArt />
        </div>
      </div>

      {lastUndoable && (
        <div className="mt-8 flex items-center justify-between gap-4 rounded-xl bg-tint px-5 py-3">
          <span className="text-[15px] text-ink">
            Your last cleanup ({formatWhen(lastUndoable.created_at)}) moved <Count n={lastUndoable.moved} one="file" />.
          </span>
          <Button variant="quiet" size="sm" onClick={onUndo} className="!text-accent">
            <RotateCcw size={15} /> Undo it
          </Button>
        </div>
      )}

      <Panel className="mt-8 p-7">
        <div className="text-[15px] font-medium text-ink-2">Folder to clean</div>
        <div className="mt-3 flex items-center gap-3">
          <button
            onClick={onPick}
            className="group flex min-w-0 flex-1 items-center gap-3 rounded-xl bg-bg px-4 py-3.5 text-left transition-colors hover:bg-tint"
          >
            <FolderOpen size={20} className="shrink-0 text-accent" aria-hidden />
            <span className="truncate font-mono text-[15px] text-ink" title={folder ?? ""}>
              {folder ?? "No folder chosen"}
            </span>
          </button>
          <Button onClick={onPick}>Change…</Button>
        </div>

        <div className="mt-7 grid gap-5 sm:grid-cols-2">
          <Toggle
            checked={useAi && !!ai?.active}
            disabled={!ai?.active}
            onChange={setUseAi}
            label="Use local AI for unclear files"
            hint={
              ai?.active ? (
                <>
                  <span className="font-mono">{ai.model}</span> · runs on this computer
                </>
              ) : (
                <button className="font-medium text-accent underline-offset-2 hover:underline" onClick={onAiHelp}>
                  Set up free local AI (optional)
                </button>
              )
            }
          />
          <Toggle checked={byMonth} onChange={setByMonth} label="Organize by month" hint={<span className="font-mono">Images/2024-05</span>} />
        </div>

        {error && <div className="mt-6 rounded-xl bg-bg px-4 py-3 text-sm text-error">{error}</div>}

        <Button variant="primary" size="lg" className="mt-8 w-full" disabled={!folder} onClick={onScan}>
          Scan folder
        </Button>
        <p className="mt-3 text-center text-sm text-ink-3">Scanning only looks. You'll see a full preview before anything moves.</p>
      </Panel>

      <p className="mt-8 flex items-center justify-center gap-2 text-sm text-ink-3">
        <Lock size={14} aria-hidden /> 100% offline · no accounts · no telemetry · files are only moved, never deleted
      </p>
    </div>
  );
}
