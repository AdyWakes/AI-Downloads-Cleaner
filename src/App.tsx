import { useCallback, useEffect, useState } from "react";
import { api, errorText, isTauri } from "./api";
import type { AiStatus, AppInfo, ApplyReport, Config, ItemOverride, JournalSummary, Plan, Prepared, Stage, UndoReport } from "./types";
import { History as HistoryIcon, Moon, Settings as SettingsIcon, Sparkles, Sun } from "lucide-react";
import { Button, BusyBar, ProgressBar } from "./ui";
import { DroppingFiles } from "./components/Art";
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
    return <div className="flex h-screen items-center justify-center text-ink-3">Loading…</div>;
  }

  const busy = screen === "working";

  return (
    <div className="min-h-screen bg-bg text-ink">
      <header className="sticky top-0 z-40 border-b border-line bg-bg">
        <div className="mx-auto flex h-16 max-w-5xl items-center gap-2 px-6">
          <button className="flex items-center gap-2.5 rounded-lg focus:outline-none focus-visible:ring-2 focus-visible:ring-brand" onClick={() => !busy && goHome()} disabled={busy}>
            <img src="/logo.svg" alt="" className="h-8 w-8" />
            <span className="text-xl font-bold tracking-tight text-ink">tidydrop</span>
          </button>
          {!isTauri && <span className="ml-2 text-sm text-ink-3">browser demo · mock data</span>}
          <span className="flex-1" />
          <button
            onClick={() => setGuide(true)}
            title={ai?.message}
            className={`mr-2 flex h-8 items-center gap-1.5 rounded-full px-3 text-sm font-semibold transition-colors ${
              ai?.active ? "bg-tint text-accent hover:bg-tint-strong" : "text-ink-3 hover:bg-tint hover:text-ink"
            }`}
          >
            {ai?.active ? <Sparkles size={14} aria-hidden /> : <span className="h-2 w-2 rounded-full bg-ink-3" aria-hidden />}
            AI: {ai?.active ? "on" : "off"}
          </button>
          <Button variant="quiet" size="sm" disabled={busy} onClick={() => (refreshHistory(), setScreen("history"))}>
            <HistoryIcon size={15} aria-hidden /> History
          </Button>
          <Button variant="quiet" size="sm" disabled={busy} onClick={() => setScreen("settings")}>
            <SettingsIcon size={15} aria-hidden /> Settings
          </Button>
          <button
            onClick={toggleTheme}
            title="Toggle dark mode"
            aria-label="Toggle dark mode"
            className="ml-1 rounded-lg p-2 text-ink-3 transition-colors hover:bg-tint hover:text-ink"
          >
            {document.documentElement.classList.contains("dark") ? <Sun size={17} /> : <Moon size={17} />}
          </button>
        </div>
      </header>

      <main className="mx-auto max-w-5xl px-6 pb-10 pt-10">
        {error && screen !== "home" && (
          <div className="mx-auto mb-6 max-w-3xl rounded-xl bg-surface px-4 py-3 text-[15px] text-error shadow-soft">{error}</div>
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
          <div className="mx-auto max-w-md pt-16 text-center">
            <DroppingFiles />
            {workingLabel === "apply" && applyProgress ? (
              <>
                <h2 className="mt-8 text-[28px] font-bold tracking-tight text-ink">Moving files…</h2>
                <p className="mb-6 mt-1 font-mono text-lg text-ink-2">
                  {applyProgress.done} / {applyProgress.total}
                </p>
                <ProgressBar value={applyProgress.done} max={applyProgress.total} />
                <p className="mt-4 text-[15px] text-ink-3">Every move is written to the undo journal first.</p>
              </>
            ) : (
              <>
                <h2 className="mb-6 mt-8 text-[28px] font-bold tracking-tight text-ink">{stageText(stage)}</h2>
                {stage?.stage === "ai" ? <ProgressBar value={stage.done} max={stage.total} /> : <BusyBar />}
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
