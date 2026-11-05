import { useState } from "react";
import { ArrowLeft, ArrowRight, ChevronDown, Sparkles, TriangleAlert } from "lucide-react";
import type { Config, PlanItem, Prepared } from "../types";
import { Button, CategoryBadge, Panel } from "../ui";
import { SKIP_LABELS, formatBytes, groupBy, plural } from "../format";

const CHART = ["bg-chart-1", "bg-chart-2", "bg-chart-3", "bg-chart-4", "bg-chart-5"];

/** One stacked bar: biggest groups in orange shades, the rest in warm neutrals. */
function WhereBar({ groups }: { groups: [string, PlanItem[]][] }) {
  const total = groups.reduce((a, [, v]) => a + v.length, 0) || 1;
  const sorted = [...groups].sort((a, b) => b[1].length - a[1].length);
  const shade = (i: number) => CHART[Math.min(i, 3) + (i >= 4 && i % 2 === 1 ? 1 : 0)];
  return (
    <div>
      <div className="flex h-4 w-full gap-[3px] overflow-hidden rounded-full">
        {sorted.map(([cat, items], i) => (
          <div
            key={cat}
            title={`${cat}: ${items.length}`}
            className={`${shade(i)} h-full transition-all duration-500 first:rounded-l-full last:rounded-r-full`}
            style={{ width: `${(items.length / total) * 100}%` }}
          />
        ))}
      </div>
      <ul className="mt-6 grid grid-cols-1 gap-x-8 gap-y-3 sm:grid-cols-2">
        {sorted.map(([cat, items], i) => (
          <li key={cat} className="flex items-center gap-3">
            <CategoryBadge name={cat} size="sm" />
            <span className={`flex-1 truncate text-[15px] ${i === 0 ? "font-semibold text-ink" : "text-ink-2"}`}>{cat}</span>
            <span className={`h-2.5 w-2.5 shrink-0 rounded-full ${shade(i)}`} aria-hidden />
            <span className="w-8 text-right font-mono text-sm text-ink-2">{items.length}</span>
          </li>
        ))}
      </ul>
    </div>
  );
}

export function StatsView({ prepared, config, onBack, onNext }: { prepared: Prepared; config: Config; onBack: () => void; onNext: () => void }) {
  const { stats, plan, ai } = prepared;
  const [showSkipped, setShowSkipped] = useState(false);
  const groups = groupBy(plan.items, (i) => i.category, config.categories.map((c) => c.name));

  return (
    <div className="mx-auto max-w-4xl">
      <p className="truncate font-mono text-sm text-ink-3">{plan.root}</p>
      <h2 className="mt-2 text-[44px] font-bold leading-tight tracking-tight text-ink">
        <span className="font-mono">{stats.file_count.toLocaleString()}</span> files,{" "}
        <span className="font-mono">{formatBytes(stats.total_size).split(" ")[0]}</span> {formatBytes(stats.total_size).split(" ")[1]}
      </h2>
      <p className="mt-2 text-lg text-ink-2">
        {stats.duplicates.length > 0 ? (
          <>
            {plural(stats.duplicates.length, "set")} of duplicates ({formatBytes(stats.duplicate_waste)} in extra copies)
          </>
        ) : (
          "No duplicates"
        )}
        {" · "}
        {plural(stats.skipped_count, "file")} left alone for safety
      </p>

      {ai.used && (
        <p className="mt-4 inline-flex items-center gap-2 rounded-full bg-tint px-3 py-1 text-sm font-medium text-accent">
          <Sparkles size={14} aria-hidden /> {ai.message}
        </p>
      )}

      {plan.warnings.map((w) => (
        <p key={w} className="mt-4 flex items-start gap-2 text-[15px] text-ink">
          <TriangleAlert size={18} className="mt-0.5 shrink-0 text-accent" aria-hidden /> {w}
        </p>
      ))}

      <Panel className="mt-8 p-7">
        <h3 className="text-lg font-semibold text-ink">Where things will go</h3>
        <p className="mb-5 text-sm text-ink-3">{plural(groups.length, "folder")} inside {plan.root.split(/[\\/]/).filter(Boolean).pop()}</p>
        {groups.length ? <WhereBar groups={groups} /> : <p className="text-ink-2">Nothing to sort here. Your folder is already tidy.</p>}
      </Panel>

      <div className="mt-10 grid gap-10 md:grid-cols-2">
        <section>
          <h3 className="mb-3 text-lg font-semibold text-ink">Largest files</h3>
          <ul className="divide-y divide-line">
            {stats.largest.slice(0, 8).map((f) => (
              <li key={f.path} className="flex justify-between gap-4 py-2.5 text-[15px]">
                <span className="truncate text-ink" title={f.name}>
                  {f.name}
                </span>
                <span className="shrink-0 font-mono text-sm text-ink-2">{formatBytes(f.size)}</span>
              </li>
            ))}
          </ul>
        </section>

        <section>
          <h3 className="mb-1 text-lg font-semibold text-ink">Duplicates</h3>
          <p className="mb-3 text-sm text-ink-3">Identical contents. tidydrop never deletes them; tidy them yourself if you like.</p>
          {stats.duplicates.length === 0 && <p className="text-[15px] text-ink-2">None found.</p>}
          <ul className="divide-y divide-line">
            {stats.duplicates.slice(0, 12).map((g) => (
              <li key={g.hash} className="py-2.5">
                {g.files.map((f) => (
                  <div key={f.path} className="truncate text-[15px] text-ink">
                    {f.name}
                  </div>
                ))}
                <div className="font-mono text-xs text-ink-3">
                  {g.files.length} × {formatBytes(g.size)}
                </div>
              </li>
            ))}
          </ul>
        </section>
      </div>

      {plan.skipped.length > 0 && (
        <section className="mt-10">
          <button className="flex items-center gap-2 text-[15px] font-semibold text-ink transition-colors hover:text-accent" onClick={() => setShowSkipped(!showSkipped)} aria-expanded={showSkipped}>
            Files left alone for safety ({plan.skipped.length})
            <ChevronDown size={16} className={`transition-transform ${showSkipped ? "rotate-180" : ""}`} />
          </button>
          {showSkipped && (
            <ul className="mt-3 max-w-2xl divide-y divide-line">
              {plan.skipped.map((s) => (
                <li key={s.name} className="flex justify-between gap-4 py-2 text-[15px]">
                  <span className="truncate text-ink">{s.name}</span>
                  <span className="shrink-0 text-ink-3">{SKIP_LABELS[s.reason] ?? s.reason}</span>
                </li>
              ))}
            </ul>
          )}
        </section>
      )}

      <div className="sticky bottom-0 -mx-6 mt-10 flex items-center justify-between gap-3 border-t border-line bg-bg px-6 py-4">
        <Button variant="quiet" onClick={onBack}>
          <ArrowLeft size={16} /> Back
        </Button>
        <Button variant="primary" size="lg" disabled={plan.items.length === 0} onClick={onNext}>
          Review the plan <ArrowRight size={18} />
        </Button>
      </div>
    </div>
  );
}
