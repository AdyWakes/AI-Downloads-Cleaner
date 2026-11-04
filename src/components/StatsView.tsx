import { useState } from "react";
import type { Config, Prepared } from "../types";
import { Button, Card, Pill, catStyle } from "../ui";
import { SKIP_LABELS, formatBytes, groupBy, plural } from "../format";

export function StatsView({ prepared, config, onBack, onNext }: { prepared: Prepared; config: Config; onBack: () => void; onNext: () => void }) {
  const { stats, plan, ai } = prepared;
  const [showSkipped, setShowSkipped] = useState(false);
  const groups = groupBy(plan.items, (i) => i.category, config.categories.map((c) => c.name));
  const max = Math.max(1, ...groups.map(([, v]) => v.length));

  const Tile = ({ label, value, sub }: { label: string; value: string; sub?: string }) => (
    <Card className="!p-4">
      <div className="text-xs font-semibold uppercase tracking-wide text-slate-500">{label}</div>
      <div className="mt-1 text-2xl font-extrabold">{value}</div>
      {sub && <div className="text-xs text-slate-500">{sub}</div>}
    </Card>
  );

  return (
    <div className="mx-auto max-w-4xl">
      <div className="mb-4 flex items-end justify-between">
        <div>
          <h2 className="text-2xl font-bold">Here's what's in your folder</h2>
          <p className="truncate text-sm text-slate-500">{plan.root}</p>
        </div>
      </div>

      {plan.warnings.map((w) => (
        <div key={w} className="mb-3 rounded-xl bg-amber-50 px-4 py-3 text-sm text-amber-900 ring-1 ring-amber-200 dark:bg-amber-900/20 dark:text-amber-200 dark:ring-amber-800">
          ⚠️ {w}
        </div>
      ))}

      <div className="grid grid-cols-2 gap-3 md:grid-cols-4">
        <Tile label="Files to sort" value={stats.file_count.toLocaleString()} />
        <Tile label="Total size" value={formatBytes(stats.total_size)} />
        <Tile
          label="Duplicates"
          value={stats.duplicates.length.toString()}
          sub={stats.duplicates.length ? `${formatBytes(stats.duplicate_waste)} in extra copies` : "none found"}
        />
        <Tile label="Left alone" value={stats.skipped_count.toString()} sub="in use, hidden, partial…" />
      </div>

      {ai.used || ai.message ? (
        <div className="mt-3 text-sm text-slate-600 dark:text-slate-300">
          <Pill tone={ai.used ? "violet" : "slate"}>{ai.used ? "✨ AI" : "Rules"}</Pill> <span className="ml-1">{ai.message}</span>
        </div>
      ) : null}

      <div className="mt-4 grid gap-4 md:grid-cols-2">
        <Card>
          <h3 className="mb-3 font-semibold">Where things will go</h3>
          <div className="space-y-2">
            {groups.map(([cat, items]) => {
              const s = catStyle(cat);
              return (
                <div key={cat} className="flex items-center gap-2 text-sm">
                  <span className="w-6 text-center">{s.icon}</span>
                  <span className="w-32 truncate">{cat}</span>
                  <div className="h-2 flex-1 overflow-hidden rounded-full bg-slate-100 dark:bg-slate-800">
                    <div className="h-full rounded-full" style={{ width: `${(items.length / max) * 100}%`, background: s.color }} />
                  </div>
                  <span className="w-8 text-right tabular-nums text-slate-500">{items.length}</span>
                </div>
              );
            })}
            {groups.length === 0 && <p className="text-sm text-slate-500">Nothing to sort here. Your folder is already tidy! 🎉</p>}
          </div>
        </Card>

        <Card>
          <h3 className="mb-3 font-semibold">Largest files</h3>
          <ul className="space-y-1.5 text-sm">
            {stats.largest.slice(0, 8).map((f) => (
              <li key={f.path} className="flex justify-between gap-3">
                <span className="truncate" title={f.name}>
                  {f.name}
                </span>
                <span className="shrink-0 tabular-nums text-slate-500">{formatBytes(f.size)}</span>
              </li>
            ))}
          </ul>
        </Card>
      </div>

      {stats.duplicates.length > 0 && (
        <Card className="mt-4">
          <h3 className="font-semibold">Duplicate files</h3>
          <p className="mb-3 text-xs text-slate-500">
            These files have identical contents. tidydrop never deletes anything. If you want, remove extra copies yourself after cleaning.
          </p>
          <ul className="space-y-2 text-sm">
            {stats.duplicates.slice(0, 20).map((g) => (
              <li key={g.hash} className="rounded-xl bg-slate-50 px-3 py-2 dark:bg-slate-800">
                <div className="mb-1 text-xs text-slate-500">
                  {plural(g.files.length, "copy", "copies")} · {formatBytes(g.size)} each
                </div>
                {g.files.map((f) => (
                  <div key={f.path} className="truncate">
                    {f.name}
                  </div>
                ))}
              </li>
            ))}
          </ul>
        </Card>
      )}

      {plan.skipped.length > 0 && (
        <Card className="mt-4">
          <button className="flex w-full items-center justify-between font-semibold" onClick={() => setShowSkipped(!showSkipped)}>
            <span>Files left alone for safety ({plan.skipped.length})</span>
            <span className="text-slate-400">{showSkipped ? "▲" : "▼"}</span>
          </button>
          {showSkipped && (
            <ul className="mt-3 space-y-1 text-sm">
              {plan.skipped.map((s) => (
                <li key={s.name} className="flex justify-between gap-3">
                  <span className="truncate">{s.name}</span>
                  <span className="shrink-0 text-slate-500">{SKIP_LABELS[s.reason] ?? s.reason}</span>
                </li>
              ))}
            </ul>
          )}
        </Card>
      )}

      <div className="sticky bottom-0 mt-6 flex justify-between gap-3 bg-gradient-to-t from-slate-50 via-slate-50 pb-4 pt-6 dark:from-slate-950 dark:via-slate-950">
        <Button variant="ghost" onClick={onBack}>
          ← Back
        </Button>
        <Button variant="primary" disabled={plan.items.length === 0} onClick={onNext} className="px-6">
          Review the plan →
        </Button>
      </div>
    </div>
  );
}
