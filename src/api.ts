// Thin wrapper over Tauri commands. When opened in a plain browser (`npm run dev`)
// a mock backend is used so the UI can be developed and demoed without Rust.

import type {
  AiStatus,
  AppInfo,
  ApplyReport,
  Config,
  ItemOverride,
  JournalSummary,
  Plan,
  Prepared,
  Stage,
  UndoReport,
} from "./types";

export const isTauri = typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;

type Unlisten = () => void;

async function call<T>(cmd: string, args?: Record<string, unknown>): Promise<T> {
  if (!isTauri) {
    const { mockInvoke } = await import("./mock");
    return mockInvoke<T>(cmd, args ?? {});
  }
  const { invoke } = await import("@tauri-apps/api/core");
  return invoke<T>(cmd, args);
}

async function on<T>(event: string, cb: (payload: T) => void): Promise<Unlisten> {
  if (!isTauri) {
    const { mockListen } = await import("./mock");
    return mockListen(event, cb as (p: unknown) => void);
  }
  const { listen } = await import("@tauri-apps/api/event");
  return listen<T>(event, (e) => cb(e.payload));
}

export const api = {
  appInfo: () => call<AppInfo>("app_info"),
  getConfig: () => call<Config>("get_config"),
  saveConfig: (config: Config) => call<Config>("save_config", { config }),
  resetConfig: () => call<Config>("reset_config"),
  aiStatus: () => call<AiStatus>("ai_status"),
  prepare: (folder: string, useAi: boolean) => call<Prepared>("prepare", { folder, useAi }),
  updatePlan: (overrides: ItemOverride[], organizeByMonth?: boolean) =>
    call<Plan>("update_plan", { overrides, organizeByMonth }),
  apply: () => call<ApplyReport>("apply_plan"),
  undo: (id?: string) => call<UndoReport>("undo_last", { id: id ?? null }),
  history: () => call<JournalSummary[]>("history"),
  openFolder: (path: string) => call<void>("open_folder", { path }),
  openOllamaDownload: () => call<void>("open_ollama_download"),
  onProgress: (cb: (s: Stage) => void) => on<Stage>("tidydrop://progress", cb),
  onApplyProgress: (cb: (p: { done: number; total: number }) => void) => on("tidydrop://apply-progress", cb),

  async pickFolder(defaultPath?: string): Promise<string | null> {
    if (!isTauri) {
      const { mockPickFolder } = await import("./mock");
      return mockPickFolder();
    }
    const { open } = await import("@tauri-apps/plugin-dialog");
    const r = await open({ directory: true, multiple: false, defaultPath: defaultPath ?? undefined });
    return typeof r === "string" ? r : null;
  },
};

export function errorText(e: unknown): string {
  if (typeof e === "string") return e;
  if (e instanceof Error) return e.message;
  return JSON.stringify(e);
}
