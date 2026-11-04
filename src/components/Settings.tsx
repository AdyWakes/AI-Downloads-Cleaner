import { useEffect, useState } from "react";
import type { AiStatus, AppInfo, Config, Rule } from "../types";
import { Button, Card, Toggle, catStyle } from "../ui";

const listToText = (l: string[]) => l.join(", ");
const textToList = (t: string) =>
  t
    .split(",")
    .map((s) => s.trim())
    .filter(Boolean);

function Field({ label, children, hint }: { label: string; children: React.ReactNode; hint?: string }) {
  return (
    <label className="block text-sm">
      <span className="mb-1 block font-medium">{label}</span>
      {children}
      {hint && <span className="mt-1 block text-xs text-slate-500">{hint}</span>}
    </label>
  );
}

const input = "w-full rounded-lg border border-slate-200 bg-white px-2.5 py-1.5 text-sm dark:border-slate-700 dark:bg-slate-800";

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
      <div className="mb-4 flex items-center justify-between">
        <h2 className="text-2xl font-bold">Settings</h2>
        <div className="flex gap-2">
          <Button
            variant="ghost"
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
      {msg && (
        <div className={`mb-3 rounded-xl px-4 py-2 text-sm ${msg.ok ? "bg-brand-50 text-brand-800 dark:bg-brand-900/30 dark:text-brand-200" : "bg-red-50 text-red-700 dark:bg-red-900/20 dark:text-red-300"}`}>
          {msg.text}
        </div>
      )}
      <div className="mb-4 flex gap-1 rounded-xl bg-slate-100 p-1 dark:bg-slate-800">
        {tabs.map(([id, label]) => (
          <button
            key={id}
            onClick={() => setTab(id)}
            className={`flex-1 rounded-lg px-3 py-1.5 text-sm font-medium ${tab === id ? "bg-white shadow-sm dark:bg-slate-900" : "text-slate-600 dark:text-slate-300"}`}
          >
            {label}
          </button>
        ))}
      </div>

      {tab === "categories" && (
        <Card>
          <p className="mb-3 text-sm text-slate-500">Each category is a folder created inside the folder you clean. “Other” is always kept as the fallback.</p>
          <div className="space-y-2">
            {draft.categories.map((c, i) => (
              <div key={i} className="grid grid-cols-12 items-center gap-2">
                <span className="col-span-1 text-center text-xl">{catStyle(c.name).icon}</span>
                <input
                  className={`${input} col-span-3`}
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
                <input className={`${input} col-span-3`} value={c.folder} aria-label="Folder name" placeholder="Folder name" onChange={(e) => set((d) => void (d.categories[i].folder = e.target.value))} />
                <input
                  className={`${input} col-span-4`}
                  value={c.description}
                  aria-label="Description"
                  placeholder="Description (helps the AI)"
                  onChange={(e) => set((d) => void (d.categories[i].description = e.target.value))}
                />
                <button
                  className="col-span-1 text-slate-400 hover:text-red-500 disabled:opacity-30"
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
                  🗑
                </button>
              </div>
            ))}
          </div>
          <Button
            className="mt-4"
            onClick={() =>
              set((d) => {
                let n = 1;
                while (d.categories.some((c) => c.name === `New folder ${n}`)) n++;
                d.categories.splice(d.categories.length - 1, 0, { name: `New folder ${n}`, folder: `New folder ${n}`, description: "" });
              })
            }
          >
            + Add category
          </Button>
        </Card>
      )}

      {tab === "rules" && (
        <Card>
          <p className="mb-3 text-sm text-slate-500">
            Rules are checked top to bottom; the first match wins. A rule matches when the file type is listed (or the list is empty)
            <em> and</em> the name contains one of the keywords (or the list is empty). Keywords match whole words; start one with{" "}
            <code>re:</code> for a regular expression.
          </p>
          <div className="space-y-3">
            {draft.rules.map((r, i) => (
              <div key={`${i}|${r.category}|${r.extensions.join()}|${r.keywords.join()}`} className="rounded-xl bg-slate-50 p-3 ring-1 ring-slate-200 dark:bg-slate-800/50 dark:ring-slate-700">
                <div className="flex flex-wrap items-center gap-2">
                  <span className="w-6 text-xs text-slate-400">#{i + 1}</span>
                  <select className={`${input} !w-48`} value={r.category} onChange={(e) => setRule(i, (x) => void (x.category = e.target.value))} aria-label="Category">
                    {names.map((n) => (
                      <option key={n}>{n}</option>
                    ))}
                  </select>
                  <label className="flex items-center gap-1 text-xs text-slate-600 dark:text-slate-300" title="If off, the AI may re-check files matched by this rule">
                    <input type="checkbox" className="accent-brand-600" checked={r.confident} onChange={(e) => setRule(i, (x) => void (x.confident = e.target.checked))} />
                    confident
                  </label>
                  <span className="flex-1" />
                  <Button variant="ghost" className="!px-2 !py-1" disabled={i === 0} onClick={() => set((d) => void d.rules.splice(i - 1, 0, d.rules.splice(i, 1)[0]))} title="Move up">
                    ↑
                  </Button>
                  <Button variant="ghost" className="!px-2 !py-1" disabled={i === draft.rules.length - 1} onClick={() => set((d) => void d.rules.splice(i + 1, 0, d.rules.splice(i, 1)[0]))} title="Move down">
                    ↓
                  </Button>
                  <Button variant="ghost" className="!px-2 !py-1" onClick={() => set((d) => void d.rules.splice(i, 1))} title="Delete rule">
                    🗑
                  </Button>
                </div>
                <div className="mt-2 grid gap-2 md:grid-cols-2">
                  <Field label="File types">
                    <input className={input} defaultValue={listToText(r.extensions)} placeholder="pdf, docx (empty = any)" onBlur={(e) => setRule(i, (x) => void (x.extensions = textToList(e.target.value)))} />
                  </Field>
                  <Field label="Name keywords">
                    <input className={input} defaultValue={listToText(r.keywords)} placeholder="invoice, receipt (empty = any)" onBlur={(e) => setRule(i, (x) => void (x.keywords = textToList(e.target.value)))} />
                  </Field>
                </div>
              </div>
            ))}
          </div>
          <Button className="mt-4" onClick={() => set((d) => void d.rules.unshift({ category: names[0], extensions: [], keywords: [], confident: true }))}>
            + Add rule at top
          </Button>
        </Card>
      )}

      {tab === "ai" && (
        <Card className="space-y-4">
          <div className="flex items-center justify-between rounded-xl bg-slate-50 px-4 py-3 text-sm dark:bg-slate-800">
            <span>
              Status: <strong>{ai?.active ? "AI on" : "AI off"}</strong> — {ai?.message ?? "checking…"}
            </span>
            <Button onClick={onAiHelp}>Setup guide</Button>
          </div>
          <Toggle checked={draft.ai.enabled} onChange={(v) => set((d) => void (d.ai.enabled = v))} label="Use local AI when available" hint="Only unclear files are sent to the model, and only to Ollama on this computer." />
          <div className="grid gap-4 md:grid-cols-2">
            <Field label="Model" hint="Small models work best: llama3.2:3b or qwen2.5:3b">
              <input className={input} list="models" value={draft.ai.model} onChange={(e) => set((d) => void (d.ai.model = e.target.value))} />
              <datalist id="models">
                {[...(ai?.installed_models ?? []), "llama3.2:3b", "qwen2.5:3b"].map((m) => (
                  <option key={m} value={m} />
                ))}
              </datalist>
            </Field>
            <Field label="Ollama address" hint="Must be on this computer (localhost)">
              <input className={input} value={draft.ai.endpoint} onChange={(e) => set((d) => void (d.ai.endpoint = e.target.value))} />
            </Field>
            <Field label="Text read per file (characters)" hint="Bigger = smarter but slower">
              <input type="number" min={0} max={4000} className={input} value={draft.ai.max_snippet_chars} onChange={(e) => set((d) => void (d.ai.max_snippet_chars = Math.max(0, Math.min(4000, Number(e.target.value) || 0))))} />
            </Field>
            <Field label="Files per request">
              <input type="number" min={1} max={50} className={input} value={draft.ai.batch_size} onChange={(e) => set((d) => void (d.ai.batch_size = Math.max(1, Math.min(50, Number(e.target.value) || 1))))} />
            </Field>
          </div>
        </Card>
      )}

      {tab === "general" && (
        <Card className="space-y-4">
          <Toggle checked={draft.organize_by_month} onChange={(v) => set((d) => void (d.organize_by_month = v))} label="Organize by month by default" hint="Creates e.g. Images/2024-05 inside each folder" />
          <Field label="Skip files changed in the last … minutes" hint="Protects downloads that are still finishing. Default 2.">
            <input
              type="number"
              min={0}
              max={1440}
              className={`${input} !w-32`}
              value={Math.round(draft.skip_recent_secs / 60)}
              onChange={(e) => set((d) => void (d.skip_recent_secs = Math.max(0, Math.min(1440, Number(e.target.value) || 0)) * 60))}
            />
          </Field>
          <Field label="Theme">
            <select className={`${input} !w-40`} value={draft.theme} onChange={(e) => set((d) => void (d.theme = e.target.value))}>
              <option value="system">Match system</option>
              <option value="light">Light</option>
              <option value="dark">Dark</option>
            </select>
          </Field>
          {info && (
            <div className="text-xs text-slate-500">
              <div>Version {info.version}</div>
              <div>Settings file: {info.config_file}</div>
              <div>Undo history: {info.journal_dir}</div>
            </div>
          )}
        </Card>
      )}

      <Button variant="ghost" className="mt-6" onClick={onBack}>
        ← Back
      </Button>
    </div>
  );
}
