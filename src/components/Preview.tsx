import { useState } from "react";
import type { Config, ItemOverride, Plan, PlanItem } from "../types";
import { Button, Card, Pill, Toggle, catStyle } from "../ui";
import { formatBytes, groupBy, plural } from "../format";

const CUSTOM = "__custom__";

function Row({ item, config, onChange }: { item: PlanItem; config: Config; onChange: (o: ItemOverride) => void }) {
  const [editing, setEditing] = useState(false);
  const [custom, setCustom] = useState(item.custom_folder ?? "");
  const tier =
    item.source_tier === "ai" ? (
      <Pill tone="violet">✨ AI</Pill>
    ) : item.source_tier === "user" ? (
      <Pill tone="blue">You</Pill>
    ) : null;
  return (
    <li className={`flex flex-wrap items-center gap-3 px-4 py-2.5 sm:flex-nowrap ${item.enabled ? "" : "opacity-50"}`}>
      <input
        type="checkbox"
        className="h-4 w-4 shrink-0 accent-brand-600"
        checked={item.enabled}
        onChange={(e) => onChange({ id: item.id, enabled: e.target.checked })}
        aria-label={`Move ${item.name}`}
      />
      <div className="min-w-0 flex-1">
        <div className="truncate text-sm font-medium" title={item.source}>
          {item.name}
        </div>
        <div className="flex items-center gap-2 text-xs text-slate-500">
          <span className="tabular-nums">{formatBytes(item.size)}</span>
          <span>·</span>
          <span className="truncate">{item.reason}</span>
          {tier}
        </div>
      </div>
      <div className="flex w-full items-center gap-2 sm:w-auto">
        <span className="text-slate-400">→</span>
        {editing ? (
          <form
            className="flex gap-1"
            onSubmit={(e) => {
              e.preventDefault();
              onChange({ id: item.id, custom_folder: custom });
              setEditing(false);
            }}
          >
            <input
              autoFocus
              value={custom}
              onChange={(e) => setCustom(e.target.value)}
              placeholder="Folder name, e.g. Work/Taxes"
              className="w-48 rounded-lg border border-slate-300 bg-white px-2 py-1 text-sm dark:border-slate-600 dark:bg-slate-800"
            />
            <Button type="submit" variant="primary" className="!px-2 !py-1">
              OK
            </Button>
            <Button variant="ghost" className="!px-2 !py-1" onClick={() => setEditing(false)}>
              ✕
            </Button>
          </form>
        ) : (
          <select
            className="w-full max-w-[14rem] rounded-lg border border-slate-200 bg-white px-2 py-1 text-sm dark:border-slate-700 dark:bg-slate-800 sm:w-56"
            value={item.custom_folder ? CUSTOM : item.category}
            onChange={(e) => {
              if (e.target.value === CUSTOM) {
                setEditing(true);
              } else {
                onChange({ id: item.id, category: e.target.value, custom_folder: "" });
              }
            }}
            title={item.dest_rel}
          >
            {config.categories.map((c) => (
              <option key={c.name} value={c.name}>
                {catStyle(c.name).icon} {c.name}
              </option>
            ))}
            <option value={CUSTOM}>{item.custom_folder ? `📁 ${item.custom_folder}` : "📁 Custom folder…"}</option>
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
      <div className="mb-4 flex flex-wrap items-end justify-between gap-3">
        <div>
          <h2 className="text-2xl font-bold">Preview the plan</h2>
          <p className="text-sm text-slate-500">Untick anything you want to leave where it is, or pick a different folder. Nothing has moved yet.</p>
        </div>
        <Toggle checked={plan.organize_by_month} onChange={onByMonth} label="Organize by month" />
      </div>

      <div className="mb-3 flex flex-wrap items-center gap-2">
        <input
          value={filter}
          onChange={(e) => setFilter(e.target.value)}
          placeholder="Filter files…"
          className="min-w-[12rem] flex-1 rounded-xl border border-slate-200 bg-white px-3 py-2 text-sm dark:border-slate-700 dark:bg-slate-900"
        />
        <Button variant="ghost" onClick={() => onChange(plan.items.map((i) => ({ id: i.id, enabled: true })))}>
          Select all
        </Button>
        <Button variant="ghost" onClick={() => onChange(plan.items.map((i) => ({ id: i.id, enabled: false })))}>
          Select none
        </Button>
      </div>

      <div className="space-y-3">
        {groups.map(([group, items]) => {
          const s = catStyle(group);
          const on = items.filter((i) => i.enabled).length;
          const isCollapsed = collapsed[group];
          return (
            <Card key={group} className="!p-0 overflow-hidden">
              <div className="flex items-center gap-3 border-b border-slate-100 px-4 py-3 dark:border-slate-800">
                <input
                  type="checkbox"
                  className="h-4 w-4 accent-brand-600"
                  checked={on === items.length}
                  ref={(el) => {
                    if (el) el.indeterminate = on > 0 && on < items.length;
                  }}
                  onChange={(e) => onChange(items.map((i) => ({ id: i.id, enabled: e.target.checked })))}
                  aria-label={`Move all in ${group}`}
                />
                <span className="flex h-8 w-8 items-center justify-center rounded-lg text-lg" style={{ background: s.color + "22" }}>
                  {s.icon}
                </span>
                <button className="flex flex-1 items-center justify-between text-left" onClick={() => setCollapsed({ ...collapsed, [group]: !isCollapsed })}>
                  <span>
                    <span className="font-semibold">{group}</span>
                    <span className="ml-2 text-xs text-slate-500">
                      {on}/{items.length} · into <code>{items[0]?.dest_rel.split("/")[0]}/</code>
                    </span>
                  </span>
                  <span className="text-slate-400">{isCollapsed ? "▼" : "▲"}</span>
                </button>
              </div>
              {!isCollapsed && (
                <ul className="divide-y divide-slate-100 dark:divide-slate-800">
                  {items.map((it) => (
                    <Row key={it.id + it.category + (it.custom_folder ?? "")} item={it} config={config} onChange={(o) => onChange([o])} />
                  ))}
                </ul>
              )}
            </Card>
          );
        })}
      </div>

      <div className="sticky bottom-0 mt-6 flex items-center justify-between gap-3 bg-gradient-to-t from-slate-50 via-slate-50 pb-4 pt-6 dark:from-slate-950 dark:via-slate-950">
        <Button variant="ghost" onClick={onBack}>
          ← Back
        </Button>
        <div className="flex items-center gap-3">
          <span className="hidden text-sm text-slate-500 sm:inline">
            {plural(enabled.length, "file")} into {plural(folders.size, "folder")} · undo anytime
          </span>
          <Button variant="primary" className="px-6 py-3 text-base" disabled={enabled.length === 0} onClick={onApply}>
            Clean up {plural(enabled.length, "file")}
          </Button>
        </div>
      </div>
    </div>
  );
}
