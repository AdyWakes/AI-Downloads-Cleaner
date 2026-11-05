import { useCallback, useEffect, useState } from "react";
import { api, errorText, isTauri } from "./api";
import type { AiStatus, AppInfo, ApplyReport, Config, ItemOverride, JournalSummary, Plan, Prepared, Stage, UndoReport } from "./types";
import { Button, Card, ProgressBar } from "./ui";
import { Home } from "./components/Home";
import { StatsView } from "./components/StatsView";
import { Preview } from "./components/Preview";
import { Done } from "./components/Done";
import { Settings } from "./components/Settings";
import { History } from "./components/History";
import { OllamaGuide } from "./components/OllamaGuide";

type Screen = "home" | "working" | "stats" | "preview" | "done" | "settings" | "history";

function useTheme(theme: string | undefined) {
  useEffect(() => {
    const mq = window.matchMedia("(prefers-color-scheme: dark)");
    const apply = () => {
      const dark = theme === "dark" || ((theme === "system" || !theme) && mq.matches);
      document.documentElement.classList.toggle("dark", dark);
    };
    apply();
    mq.addEventListener("change", apply);
    return () => mq.removeEventListener("change", apply);
  }, [theme]);
}

function stageText(s: Stage | null): string {
  switch (s?.stage) {
    case "scanning":
      return "Looking at your files…";
    case "hashing":
      return `Checking ${s.files.toLocaleString()} files for duplicates…`;
    case "classifying":
      return "Sorting with smart rules…";
    case "ai":
      return `Local AI is reading unclear files… ${s.done}/${s.total}`;
    case "ready":
      return "Ready!";
    default:
      return "Working…";
  }
}

export default function App() {
  const [screen, setScreen] = useState<Screen>("home");
  const [info, setInfo] = useState<AppInfo | null>(null);
  const [config, setConfig] = useState<Config | null>(null);
  const [ai, setAi] = useState<AiStatus | null>(null);
  const [folder, setFolder] = useState<string | null>(null);
  const [useAi, setUseAi] = useState(true);
  const [prepared, setPrepared] = useState<Prepared | null>(null);
  const [plan, setPlan] = useState<Plan | null>(null);
  const [stage, setStage] = useState<Stage | null>(null);
  const [applyProgress, setApplyProgress] = useState<{ done: number; total: number } | null>(null);
  const [report, setReport] = useState<ApplyReport | null>(null);
  const [appliedPlan, setAppliedPlan] = useState<Plan | null>(null);
  const [undoReport, setUndoReport] = useState<UndoReport | null>(null);
  const [undoing, setUndoing] = useState(false);
  const [history, setHistory] = useState<JournalSummary[]>([]);
  const [error, setError] = useState<string | null>(null);
  const [guide, setGuide] = useState(false);
  const [workingLabel, setWorkingLabel] = useState("");

  useTheme(config?.theme);

  const refreshAi = useCallback(() => api.aiStatus().then(setAi).catch(() => setAi(null)), []);
  const refreshHistory = useCallback(() => api.history().then(setHistory).catch(() => {}), []);

  useEffect(() => {
    api.appInfo().then((i) => {
      setInfo(i);
      setFolder((f) => f ?? i.default_folder);
    });
    api.getConfig().then(setConfig);
    refreshAi();
    refreshHistory();
    const un: Promise<() => void>[] = [api.onProgress(setStage), api.onApplyProgress(setApplyProgress)];
    return () => un.forEach((p) => p.then((f) => f()));
  }, [refreshAi, refreshHistory]);

  const lastUndoable = history.find((h) => h.status === "applied" || h.status === "interrupted" || h.status === "partially_undone") ?? null;

  const scan = async () => {
    if (!folder) return;
    setError(null);
    setStage({ stage: "scanning" });
    setWorkingLabel("scan");
    setScreen("working");
    try {
      const p = await api.prepare(folder, useAi && !!ai?.active);
      setPrepared(p);
      setPlan(p.plan);
      setScreen("stats");
    } catch (e) {
      setError(errorText(e));
      setScreen("home");
    }
  };

  const change = async (overrides: ItemOverride[], byMonth?: boolean) => {
    try {
      setPlan(await api.updatePlan(overrides, byMonth));
    } catch (e) {
      setError(errorText(e));
    }
  };

  const apply = async () => {
    if (!plan) return;
    setApplyProgress({ done: 0, total: plan.items.filter((i) => i.enabled).length });
    setWorkingLabel("apply");
    setScreen("working");
    try {
      const r = await api.apply();
      setAppliedPlan(plan);
      setReport(r);
      setUndoReport(null);
      setScreen("done");
    } catch (e) {
      setError(errorText(e));
      setScreen("preview");
    } finally {
      refreshHistory();
    }
  };

  const undo = async (id?: string) => {
    setUndoing(true);
    setError(null);
    try {
      const r = await api.undo(id);
      setUndoReport(r);
      if (screen !== "done") {
        setReport({ journal_id: r.journal_id, moved: [], skipped: [], failed: [] });
        setAppliedPlan(null);
      }
      setScreen("done");
    } catch (e) {
      setError(errorText(e));
    } finally {
      setUndoing(false);
      refreshHistory();
    }
  };

  const goHome = () => {
    setPrepared(null);
    setPlan(null);
    setReport(null);
    setUndoReport(null);
    setError(null);
    setScreen("home");
  };

  const toggleTheme = async () => {
    if (!config) return;
    const dark = document.documentElement.classList.contains("dark");
    const c = { ...config, theme: dark ? "light" : "dark" };
    setConfig(c);
    api.saveConfig(c).then(setConfig).catch(() => {});
  };

  if (!config) {
    return <div className="flex h-screen items-center justify-center text-slate-500">Loading…</div>;
  }

  const busy = screen === "working";

  return (
    <div className="min-h-screen bg-slate-50 text-slate-900 dark:bg-slate-950 dark:text-slate-100">
      <header className="sticky top-0 z-40 border-b border-slate-200/70 bg-slate-50/80 backdrop-blur dark:border-slate-800 dark:bg-slate-950/80">
        <div className="mx-auto flex max-w-5xl items-center gap-3 px-5 py-3">
          <button className="flex items-center gap-2" onClick={() => !busy && goHome()} disabled={busy}>
            <img src="/logo.svg" alt="" className="h-8 w-8" />
            <span className="text-lg font-extrabold tracking-tight">tidydrop</span>
          </button>
          {!isTauri && <span className="rounded bg-amber-100 px-2 py-0.5 text-xs text-amber-800">browser demo (mock data)</span>}
          <span className="flex-1" />
          <button
            onClick={() => setGuide(true)}
            title={ai?.message}
            className={`flex items-center gap-1.5 rounded-full px-3 py-1 text-xs font-semibold ring-1 ${
              ai?.active
                ? "bg-violet-50 text-violet-700 ring-violet-200 dark:bg-violet-900/30 dark:text-violet-200 dark:ring-violet-800"
                : "bg-slate-100 text-slate-600 ring-slate-200 dark:bg-slate-800 dark:text-slate-300 dark:ring-slate-700"
            }`}
          >
            <span className={`h-2 w-2 rounded-full ${ai?.active ? "bg-violet-500" : "bg-slate-400"}`} />
            AI: {ai?.active ? "on" : "off"}
          </button>
          <Button variant="ghost" disabled={busy} onClick={() => (refreshHistory(), setScreen("history"))}>
            History
          </Button>
          <Button variant="ghost" disabled={busy} onClick={() => setScreen("settings")}>
            Settings
          </Button>
          <Button variant="ghost" onClick={toggleTheme} title="Toggle dark mode">
            {document.documentElement.classList.contains("dark") ? "☀️" : "🌙"}
          </Button>
        </div>
      </header>

      <main className="mx-auto max-w-5xl px-5 py-6">
        {error && screen !== "home" && (
          <div className="mx-auto mb-4 max-w-3xl rounded-xl bg-red-50 px-4 py-3 text-sm text-red-700 ring-1 ring-red-200 dark:bg-red-900/20 dark:text-red-300 dark:ring-red-900">
            {error}
          </div>
        )}

        {screen === "home" && (
          <Home
            folder={folder}
            onPick={async () => {
              const f = await api.pickFolder(folder ?? undefined);
              if (f) setFolder(f);
            }}
            onScan={scan}
            useAi={useAi}
            setUseAi={setUseAi}
            byMonth={config.organize_by_month}
            setByMonth={(v) => {
              const c = { ...config, organize_by_month: v };
              setConfig(c);
              api.saveConfig(c).then(setConfig).catch((e) => setError(errorText(e)));
            }}
            ai={ai}
            onAiHelp={() => setGuide(true)}
            lastUndoable={lastUndoable}
            onUndo={() => undo(lastUndoable?.id)}
            error={error}
          />
        )}

        {screen === "working" && (
          <div className="mx-auto max-w-md py-24 text-center">
            <div className="mb-6 text-5xl">{workingLabel === "apply" ? "🚚" : "🔍"}</div>
            {workingLabel === "apply" && applyProgress ? (
              <>
                <h2 className="mb-4 text-xl font-bold">
                  Moving files… {applyProgress.done}/{applyProgress.total}
                </h2>
                <ProgressBar value={applyProgress.done} max={applyProgress.total} />
                <p className="mt-3 text-sm text-slate-500">Every move is written to the undo journal first.</p>
              </>
            ) : (
              <>
                <h2 className="mb-4 text-xl font-bold">{stageText(stage)}</h2>
                {stage?.stage === "ai" ? <ProgressBar value={stage.done} max={stage.total} /> : <div className="mx-auto h-1.5 w-40 animate-pulse rounded-full bg-brand-400" />}
              </>
            )}
          </div>
        )}

        {screen === "stats" && prepared && plan && (
          <StatsView prepared={{ ...prepared, plan }} config={config} onBack={goHome} onNext={() => setScreen("preview")} />
        )}

        {screen === "preview" && plan && (
          <Preview plan={plan} config={config} onChange={(o) => change(o)} onByMonth={(v) => change([], v)} onBack={() => setScreen("stats")} onApply={apply} />
        )}

        {screen === "done" && report && (
          <Done
            report={report}
            items={appliedPlan?.items ?? []}
            order={config.categories.map((c) => c.name)}
            root={appliedPlan?.root ?? folder ?? ""}
            onUndo={() => undo(report.journal_id)}
            undoReport={undoReport}
            undoing={undoing}
            onOpenFolder={() => api.openFolder(appliedPlan?.root ?? folder ?? "").catch((e) => setError(errorText(e)))}
            onHome={goHome}
          />
        )}

        {screen === "settings" && (
          <Settings
            config={config}
            ai={ai}
            info={info}
            onSave={async (c) => {
              try {
                setConfig(await api.saveConfig(c));
                refreshAi();
                return null;
              } catch (e) {
                return errorText(e);
              }
            }}
            onReset={async () => {
              setConfig(await api.resetConfig());
              refreshAi();
            }}
            onBack={() => setScreen(plan ? "preview" : "home")}
            onAiHelp={() => setGuide(true)}
          />
        )}

        {screen === "history" && <History items={history} busy={undoing} onUndo={(id) => undo(id)} onBack={() => setScreen(plan ? "preview" : "home")} />}

        {screen === "home" && (
          <Card className="mx-auto mt-6 max-w-2xl !bg-transparent !shadow-none !ring-0 text-center text-xs text-slate-500">
            🔒 100% offline · no accounts · no telemetry · files are only ever moved, never deleted
          </Card>
        )}
      </main>

      {guide && (
        <OllamaGuide
          status={ai}
          onClose={() => setGuide(false)}
          onRecheck={() => {
            refreshAi();
          }}
        />
      )}
    </div>
  );
}
