import { useEffect, useState } from "react";
import { ArrowDown, ArrowLeft, ArrowUp, Plus, Trash2 } from "lucide-react";
import type { AiStatus, AppInfo, Config, Rule } from "../types";
import { Badge, Button, CategoryBadge, Panel, Toggle } from "../ui";

const listToText = (l: string[]) => l.join(", ");
const textToList = (t: string) =>
  t
    .split(",")
    .map((s) => s.trim())
    .filter(Boolean);

const input =
  "h-10 w-full rounded-lg border border-line bg-surface px-3 text-[15px] text-ink transition-colors placeholder:text-ink-3 hover:border-tint-strong focus:border-brand focus:outline-none focus:ring-2 focus:ring-tint-strong disabled:bg-bg disabled:text-ink-3";

function Field({ label, children, hint }: { label: string; children: React.ReactNode; hint?: string }) {
  return (
    <label className="block">
      <span className="mb-1.5 block text-sm font-medium text-ink-2">{label}</span>
      {children}
      {hint && <span className="mt-1.5 block text-sm text-ink-3">{hint}</span>}
    </label>
  );
}

function IconButton({ onClick, disabled, title, children }: { onClick: () => void; disabled?: boolean; title: string; children: React.ReactNode }) {
  return (
    <button
      type="button"
      onClick={onClick}
      disabled={disabled}
      title={title}
      aria-label={title}
      className="rounded-lg p-2 text-ink-3 transition-colors hover:bg-tint hover:text-accent disabled:pointer-events-none disabled:opacity-30"
    >
      {children}
    </button>
  );
}

export function Settings({
  config,
  ai,
  info,
  onSave,
  onReset,
  onBack,
  onAiHelp,
}: {
  config: Config;
  ai: AiStatus | null;
  info: AppInfo | null;
  onSave: (c: Config) => Promise<string | null>;
  onReset: () => Promise<void>;
  onBack: () => void;
  onAiHelp: () => void;
}) {
  const [draft, setDraft] = useState<Config>(() => structuredClone(config));
  const [tab, setTab] = useState<"categories" | "rules" | "ai" | "general">("categories");
  const [msg, setMsg] = useState<{ ok: boolean; text: string } | null>(null);
  const dirty = JSON.stringify(draft) !== JSON.stringify(config);
  useEffect(() => setDraft(structuredClone(config)), [config]);

  const set = (f: (c: Config) => void) => {
    const c = structuredClone(draft);
    f(c);
    setDraft(c);
    setMsg(null);
  };
  const setRule = (i: number, f: (r: Rule) => void) => set((c) => f(c.rules[i]));
  const names = draft.categories.map((c) => c.name);

  const save = async () => {
    const err = await onSave(draft);
    setMsg(err ? { ok: false, text: err } : { ok: true, text: "Saved." });
  };

  const tabs = [
    ["categories", "Folders"],
    ["rules", "Rules"],
    ["ai", "AI"],
    ["general", "General"],
  ] as const;

  return (
    <div className="mx-auto max-w-4xl">
      <div className="flex flex-wrap items-end justify-between gap-4">
        <div>
          <h2 className="text-[34px] font-bold leading-tight tracking-tight text-ink">Settings</h2>
          <p className="mt-1 text-lg text-ink-2">Folders, sorting rules and the optional local AI.</p>
        </div>
        <div className="flex items-center gap-2">
          {msg && <span className={`mr-2 text-[15px] font-medium ${msg.ok ? "text-success" : "text-error"}`}>{msg.text}</span>}
          <Button
            variant="quiet"
            onClick={async () => {
              if (confirm("Reset all folders, rules and AI settings to defaults?")) {
                await onReset();
                setMsg({ ok: true, text: "Defaults restored." });
              }
            }}
          >
            Reset to defaults
          </Button>
          <Button variant="primary" disabled={!dirty} onClick={save}>
            Save changes
          </Button>
        </div>
      </div>

      <nav className="mt-8 flex gap-8 border-b border-line" role="tablist">
        {tabs.map(([id, label]) => (
          <button
            key={id}
            role="tab"
            aria-selected={tab === id}
            onClick={() => setTab(id)}
            className={`-mb-px border-b-2 pb-3 text-[15px] font-semibold transition-colors ${tab === id ? "border-brand text-ink" : "border-transparent text-ink-3 hover:text-ink"}`}
          >
            {label}
          </button>
        ))}
      </nav>

      {tab === "categories" && (
        <Panel className="mt-6 p-7">
          <p className="mb-6 max-w-2xl text-[15px] text-ink-2">Each category is a folder created inside the folder you clean. “Other” is always kept as the fallback.</p>
          <div className="mb-2 hidden grid-cols-[2.25rem_1fr_1fr_1.4fr_2.5rem] gap-3 text-sm font-medium text-ink-3 md:grid">
            <span />
            <span>Name</span>
            <span>Folder</span>
            <span>Description (helps the AI)</span>
            <span />
          </div>
          <div className="space-y-2">
            {draft.categories.map((c, i) => (
              <div key={i} className="grid grid-cols-[2.25rem_1fr_2.5rem] items-center gap-3 md:grid-cols-[2.25rem_1fr_1fr_1.4fr_2.5rem]">
                <CategoryBadge name={c.name} />
                <input
                  className={input}
                  value={c.name}
                  disabled={c.name === "Other"}
                  aria-label="Category name"
                  onChange={(e) =>
                    set((d) => {
                      const old = d.categories[i].name;
                      d.categories[i].name = e.target.value;
                      d.rules.forEach((r) => {
                        if (r.category === old) r.category = e.target.value;
                      });
                    })
                  }
                />
                <input className={`${input} hidden font-mono text-sm md:block`} value={c.folder} aria-label="Folder name" placeholder="Folder name" onChange={(e) => set((d) => void (d.categories[i].folder = e.target.value))} />
                <input className={`${input} hidden md:block`} value={c.description} aria-label="Description" placeholder="Description" onChange={(e) => set((d) => void (d.categories[i].description = e.target.value))} />
                <IconButton
                  disabled={c.name === "Other"}
                  title="Remove category (its rules are removed too)"
                  onClick={() =>
                    set((d) => {
                      const name = d.categories[i].name;
                      d.categories.splice(i, 1);
                      d.rules = d.rules.filter((r) => r.category !== name);
                    })
                  }
                >
                  <Trash2 size={17} />
                </IconButton>
              </div>
            ))}
          </div>
          <Button
            className="mt-6"
            onClick={() =>
              set((d) => {
                let n = 1;
                while (d.categories.some((c) => c.name === `New folder ${n}`)) n++;
                d.categories.splice(d.categories.length - 1, 0, { name: `New folder ${n}`, folder: `New folder ${n}`, description: "" });
              })
            }
          >
            <Plus size={16} /> Add category
          </Button>
        </Panel>
      )}

      {tab === "rules" && (
        <Panel className="mt-6 overflow-hidden">
          <div className="flex flex-wrap items-start justify-between gap-4 p-7 pb-5">
            <p className="max-w-2xl text-[15px] leading-6 text-ink-2">
              Rules are checked top to bottom and the first match wins. A rule matches when the file type is listed (or the list is empty) <em>and</em> the name contains one of the
              keywords (or the list is empty). Keywords match whole words; start one with <code className="font-mono text-ink">re:</code> for a regular expression.
            </p>
            <Button onClick={() => set((d) => void d.rules.unshift({ category: names[0], extensions: [], keywords: [], confident: true }))}>
              <Plus size={16} /> Add rule at top
            </Button>
          </div>
          <ol className="divide-y divide-line border-t border-line">
            {draft.rules.map((r, i) => (
              <li key={`${i}|${r.category}|${r.extensions.join()}|${r.keywords.join()}`} className="group px-7 py-5 transition-colors hover:bg-tint">
                <div className="flex flex-wrap items-center gap-3">
                  <span className="w-7 font-mono text-sm text-ink-3 group-hover:text-ink-2">{i + 1}</span>
                  <CategoryBadge name={r.category} size="sm" />
                  <select className={`${input} !w-52 cursor-pointer`} value={r.category} onChange={(e) => setRule(i, (x) => void (x.category = e.target.value))} aria-label="Category">
                    {names.map((n) => (
                      <option key={n}>{n}</option>
                    ))}
                  </select>
                  <label className="flex cursor-pointer items-center gap-2 text-sm text-ink-2" title="If off, the AI may re-check files matched by this rule">
                    <input type="checkbox" className="h-4 w-4" checked={r.confident} onChange={(e) => setRule(i, (x) => void (x.confident = e.target.checked))} />
                    Confident
                  </label>
                  {!r.confident && <Badge tone="brand">AI may re-check</Badge>}
                  <span className="flex-1" />
                  <IconButton title="Move up" disabled={i === 0} onClick={() => set((d) => void d.rules.splice(i - 1, 0, d.rules.splice(i, 1)[0]))}>
                    <ArrowUp size={16} />
                  </IconButton>
                  <IconButton title="Move down" disabled={i === draft.rules.length - 1} onClick={() => set((d) => void d.rules.splice(i + 1, 0, d.rules.splice(i, 1)[0]))}>
                    <ArrowDown size={16} />
                  </IconButton>
                  <IconButton title="Delete rule" onClick={() => set((d) => void d.rules.splice(i, 1))}>
                    <Trash2 size={16} />
                  </IconButton>
                </div>
                <div className="mt-3 grid gap-3 pl-10 md:grid-cols-2">
                  <Field label="File types">
                    <input className={`${input} font-mono text-sm`} defaultValue={listToText(r.extensions)} placeholder="pdf, docx (empty = any)" onBlur={(e) => setRule(i, (x) => void (x.extensions = textToList(e.target.value)))} />
                  </Field>
                  <Field label="Name keywords">
                    <input className={`${input} font-mono text-sm`} defaultValue={listToText(r.keywords)} placeholder="invoice, receipt (empty = any)" onBlur={(e) => setRule(i, (x) => void (x.keywords = textToList(e.target.value)))} />
                  </Field>
                </div>
              </li>
            ))}
          </ol>
        </Panel>
      )}

      {tab === "ai" && (
        <Panel className="mt-6 p-7">
          <div className="flex flex-wrap items-center justify-between gap-4 rounded-xl bg-bg px-5 py-4">
            <div className="flex items-center gap-3">
              <span className={`h-2.5 w-2.5 rounded-full ${ai?.active ? "bg-brand" : "bg-ink-3"}`} aria-hidden />
              <span className="text-[15px] text-ink">
                <span className="font-semibold">{ai?.active ? "AI on" : "AI off"}</span>
                <span className="text-ink-2"> — {ai?.message ?? "checking…"}</span>
              </span>
            </div>
            <Button size="sm" onClick={onAiHelp}>
              Setup guide
            </Button>
          </div>
          <div className="mt-7">
            <Toggle checked={draft.ai.enabled} onChange={(v) => set((d) => void (d.ai.enabled = v))} label="Use local AI when available" hint="Only unclear files are sent to the model, and only to Ollama on this computer." />
          </div>
          <div className="mt-7 grid gap-6 md:grid-cols-2">
            <Field label="Model" hint="Small models work best: llama3.2:3b or qwen2.5:3b">
              <input className={`${input} font-mono text-sm`} list="models" value={draft.ai.model} onChange={(e) => set((d) => void (d.ai.model = e.target.value))} />
              <datalist id="models">
                {[...(ai?.installed_models ?? []), "llama3.2:3b", "qwen2.5:3b"].map((m) => (
                  <option key={m} value={m} />
                ))}
              </datalist>
            </Field>
            <Field label="Ollama address" hint="Must be on this computer (localhost)">
              <input className={`${input} font-mono text-sm`} value={draft.ai.endpoint} onChange={(e) => set((d) => void (d.ai.endpoint = e.target.value))} />
            </Field>
            <Field label="Text read per file (characters)" hint="Bigger is smarter but slower">
              <input type="number" min={0} max={4000} className={`${input} font-mono text-sm`} value={draft.ai.max_snippet_chars} onChange={(e) => set((d) => void (d.ai.max_snippet_chars = Math.max(0, Math.min(4000, Number(e.target.value) || 0))))} />
            </Field>
            <Field label="Files per request">
              <input type="number" min={1} max={50} className={`${input} font-mono text-sm`} value={draft.ai.batch_size} onChange={(e) => set((d) => void (d.ai.batch_size = Math.max(1, Math.min(50, Number(e.target.value) || 1))))} />
            </Field>
          </div>
        </Panel>
      )}

      {tab === "general" && (
        <Panel className="mt-6 space-y-7 p-7">
          <Toggle checked={draft.organize_by_month} onChange={(v) => set((d) => void (d.organize_by_month = v))} label="Organize by month by default" hint="Creates e.g. Images/2024-05 inside each folder" />
          <Field label="Skip files changed in the last … minutes" hint="Protects downloads that are still finishing. Default 2.">
            <input
              type="number"
              min={0}
              max={1440}
              className={`${input} !w-32 font-mono text-sm`}
              value={Math.round(draft.skip_recent_secs / 60)}
              onChange={(e) => set((d) => void (d.skip_recent_secs = Math.max(0, Math.min(1440, Number(e.target.value) || 0)) * 60))}
            />
          </Field>
          <Field label="Theme">
            <select className={`${input} !w-48 cursor-pointer`} value={draft.theme} onChange={(e) => set((d) => void (d.theme = e.target.value))}>
              <option value="system">Match system</option>
              <option value="light">Light</option>
              <option value="dark">Dark</option>
            </select>
          </Field>
          {info && (
            <dl className="grid grid-cols-[auto_1fr] gap-x-6 gap-y-1.5 border-t border-line pt-6 text-sm">
              <dt className="text-ink-3">Version</dt>
              <dd className="font-mono text-ink-2">{info.version}</dd>
              <dt className="text-ink-3">Settings file</dt>
              <dd className="break-all font-mono text-ink-2">{info.config_file}</dd>
              <dt className="text-ink-3">Undo history</dt>
              <dd className="break-all font-mono text-ink-2">{info.journal_dir}</dd>
            </dl>
          )}
        </Panel>
      )}

      <Button variant="quiet" className="mt-8" onClick={onBack}>
        <ArrowLeft size={16} /> Back
      </Button>
    </div>
  );
}
