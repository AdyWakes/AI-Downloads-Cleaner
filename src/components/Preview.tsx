import { useState } from "react";
import { ArrowLeft, ArrowRight, ChevronDown, Search, Sparkles, UserRound } from "lucide-react";
import type { Config, ItemOverride, Plan, PlanItem } from "../types";
import { Button, CategoryBadge, Panel, Toggle } from "../ui";
import { formatBytes, groupBy, plural } from "../format";

const CUSTOM = "__custom__";

const field =
  "rounded-lg border border-line bg-surface px-2.5 text-sm text-ink transition-colors hover:border-tint-strong focus:border-brand focus:outline-none focus:ring-2 focus:ring-tint-strong";

function Row({ item, config, onChange }: { item: PlanItem; config: Config; onChange: (o: ItemOverride) => void }) {
  const [editing, setEditing] = useState(false);
  const [custom, setCustom] = useState(item.custom_folder ?? "");
  return (
    <li className={`group flex flex-wrap items-center gap-x-4 gap-y-2 px-6 py-3 transition-colors hover:bg-tint sm:flex-nowrap ${item.enabled ? "" : "opacity-50"}`}>
      <input
        type="checkbox"
        className="h-4 w-4 shrink-0 cursor-pointer"
        checked={item.enabled}
        onChange={(e) => onChange({ id: item.id, enabled: e.target.checked })}
        aria-label={`Move ${item.name}`}
      />
      <div className="min-w-0 flex-1">
        <div className="truncate text-[15px] text-ink" title={item.source}>
          {item.name}
        </div>
        <div className="flex items-center gap-2 text-sm text-ink-3 transition-colors group-hover:text-ink-2">
          <span className="font-mono">{formatBytes(item.size)}</span>
          <span aria-hidden>·</span>
          {item.source_tier === "ai" && <Sparkles size={13} className="shrink-0 text-accent" aria-label="AI" />}
          {item.source_tier === "user" && <UserRound size={13} className="shrink-0 text-accent" aria-label="You" />}
          <span className={`truncate ${item.source_tier === "rules" ? "" : "text-accent"}`}>{item.reason}</span>
        </div>
      </div>
      <div className="flex w-full items-center gap-2 pl-8 sm:w-auto sm:pl-0">
        {editing ? (
          <form
            className="flex gap-2"
            onSubmit={(e) => {
              e.preventDefault();
              onChange({ id: item.id, custom_folder: custom });
              setEditing(false);
            }}
          >
            <input autoFocus value={custom} onChange={(e) => setCustom(e.target.value)} placeholder="e.g. Work/Taxes" className={`${field} h-9 w-48 font-mono`} />
            <Button type="submit" variant="primary" size="sm">
              OK
            </Button>
            <Button variant="quiet" size="sm" onClick={() => setEditing(false)}>
              Cancel
            </Button>
          </form>
        ) : (
          <select
            className={`${field} h-9 w-full cursor-pointer sm:w-52`}
            value={item.custom_folder ? CUSTOM : item.category}
            onChange={(e) => {
              if (e.target.value === CUSTOM) setEditing(true);
              else onChange({ id: item.id, category: e.target.value, custom_folder: "" });
            }}
            title={item.dest_rel}
            aria-label={`Destination for ${item.name}`}
          >
            {config.categories.map((c) => (
              <option key={c.name} value={c.name}>
                {c.name}
              </option>
            ))}
            <option value={CUSTOM}>{item.custom_folder ? `Custom: ${item.custom_folder}` : "Custom folder…"}</option>
          </select>
        )}
      </div>
    </li>
  );
}

export function Preview({
  plan,
  config,
  onChange,
  onByMonth,
  onBack,
  onApply,
}: {
  plan: Plan;
  config: Config;
  onChange: (o: ItemOverride[]) => void;
  onByMonth: (v: boolean) => void;
  onBack: () => void;
  onApply: () => void;
}) {
  const [collapsed, setCollapsed] = useState<Record<string, boolean>>({});
  const [filter, setFilter] = useState("");
  const enabled = plan.items.filter((i) => i.enabled);
  const shown = filter ? plan.items.filter((i) => i.name.toLowerCase().includes(filter.toLowerCase())) : plan.items;
  const groups = groupBy(shown, (i) => i.custom_folder ?? i.category, config.categories.map((c) => c.name));
  const folders = new Set(enabled.map((i) => i.dest_rel.split("/")[0]));

  return (
    <div className="mx-auto max-w-4xl">
      <h2 className="text-[34px] font-bold leading-tight tracking-tight text-ink">Preview the plan</h2>
      <p className="mt-1 text-lg text-ink-2">Untick anything you want to leave where it is, or pick a different folder. Nothing has moved yet.</p>

      <div className="mt-8 flex flex-wrap items-center gap-x-6 gap-y-3">
        <label className="relative min-w-[14rem] flex-1">
          <Search size={16} className="pointer-events-none absolute left-3 top-1/2 -translate-y-1/2 text-ink-3" aria-hidden />
          <input value={filter} onChange={(e) => setFilter(e.target.value)} placeholder="Filter files…" className={`${field} h-10 w-full pl-9 text-[15px]`} aria-label="Filter files" />
        </label>
        <div className="flex items-center gap-1">
          <Button variant="quiet" size="sm" onClick={() => onChange(plan.items.map((i) => ({ id: i.id, enabled: true })))}>
            Select all
          </Button>
          <Button variant="quiet" size="sm" onClick={() => onChange(plan.items.map((i) => ({ id: i.id, enabled: false })))}>
            Select none
          </Button>
        </div>
        <Toggle checked={plan.organize_by_month} onChange={onByMonth} label="Organize by month" />
      </div>

      <Panel className="mt-5 overflow-hidden">
        {groups.map(([group, items], gi) => {
          const on = items.filter((i) => i.enabled).length;
          const isCollapsed = collapsed[group];
          return (
            <div key={group} className={gi > 0 ? "border-t border-line" : ""}>
              <div className="flex items-center gap-4 px-6 pb-3 pt-5">
                <input
                  type="checkbox"
                  className="h-4 w-4 cursor-pointer"
                  checked={on === items.length}
                  ref={(el) => {
                    if (el) el.indeterminate = on > 0 && on < items.length;
                  }}
                  onChange={(e) => onChange(items.map((i) => ({ id: i.id, enabled: e.target.checked })))}
                  aria-label={`Move all in ${group}`}
                />
                <CategoryBadge name={group} />
                <button className="flex min-w-0 flex-1 items-center justify-between gap-3 text-left" onClick={() => setCollapsed({ ...collapsed, [group]: !isCollapsed })} aria-expanded={!isCollapsed}>
                  <span className="min-w-0">
                    <span className="text-lg font-semibold text-ink">{group}</span>
                    <span className="ml-3 font-mono text-sm text-ink-3">
                      {on}/{items.length} → {items[0]?.dest_rel.split("/")[0]}/
                    </span>
                  </span>
                  <ChevronDown size={18} className={`shrink-0 text-ink-3 transition-transform ${isCollapsed ? "" : "rotate-180"}`} />
                </button>
              </div>
              {!isCollapsed && (
                <ul className="pb-2">
                  {items.map((it) => (
                    <Row key={it.id + it.category + (it.custom_folder ?? "")} item={it} config={config} onChange={(o) => onChange([o])} />
                  ))}
                </ul>
              )}
            </div>
          );
        })}
        {groups.length === 0 && <p className="px-6 py-8 text-center text-ink-2">No files match “{filter}”.</p>}
      </Panel>

      <div className="sticky bottom-0 -mx-6 mt-8 flex items-center justify-between gap-4 border-t border-line bg-bg px-6 py-4">
        <Button variant="quiet" onClick={onBack}>
          <ArrowLeft size={16} /> Back
        </Button>
        <div className="flex items-center gap-5">
          <span className="hidden text-[15px] text-ink-2 sm:inline">
            <span className="font-mono text-ink">{enabled.length}</span> files into <span className="font-mono text-ink">{folders.size}</span> folders · undo anytime
          </span>
          <Button variant="primary" size="lg" disabled={enabled.length === 0} onClick={onApply}>
            Clean up {plural(enabled.length, "file")} <ArrowRight size={18} />
          </Button>
        </div>
      </div>
    </div>
  );
}
