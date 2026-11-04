// Browser-only mock backend for UI development and demo recordings.
// It never touches real files. The real logic is in crates/tidydrop-core.

import type {
  AiStatus,
  AppInfo,
  ApplyReport,
  Config,
  ItemOverride,
  JournalSummary,
  Plan,
  PlanItem,
  Prepared,
  Stage,
  UndoReport,
} from "./types";

const listeners = new Map<string, Set<(p: unknown) => void>>();
export function mockListen(event: string, cb: (p: unknown) => void) {
  if (!listeners.has(event)) listeners.set(event, new Set());
  listeners.get(event)!.add(cb);
  return () => listeners.get(event)!.delete(cb);
}
function emit(event: string, payload: unknown) {
  listeners.get(event)?.forEach((cb) => cb(payload));
}
const sleep = (ms: number) => new Promise((r) => setTimeout(r, ms));

const cats: [string, string, string][] = [
  ["Documents", "Documents", "Letters, Word documents, text files, ebooks"],
  ["PDFs", "PDFs", "PDF files that are not bills or study material"],
  ["Images", "Images", "Photos and pictures"],
  ["Screenshots", "Screenshots", "Screen captures"],
  ["Videos", "Videos", "Video files"],
  ["Audio", "Audio", "Music, voice notes, podcasts"],
  ["Installers", "Installers", "Program installers and disk images"],
  ["Archives", "Archives", "Zip, rar, 7z and other compressed files"],
  ["Code", "Code", "Source code, scripts, data/config files"],
  ["Spreadsheets", "Spreadsheets", "Excel, CSV and other tables"],
  ["Presentations", "Presentations", "Slide decks"],
  ["Bills & Receipts", "Bills & Receipts", "Invoices, receipts, bills, order confirmations"],
  ["College/Study", "College & Study", "Assignments, lecture notes, syllabus"],
  ["Other", "Other", "Anything that does not fit elsewhere"],
];

const defaultConfig = (): Config => ({
  version: 1,
  categories: cats.map(([name, folder, description]) => ({ name, folder, description })),
  rules: [
    { category: "Bills & Receipts", extensions: ["pdf", "jpg", "png"], keywords: ["invoice", "receipt", "bill", "gst", "order"], confident: true },
    { category: "College/Study", extensions: ["pdf", "docx", "pptx"], keywords: ["assignment", "lecture", "notes", "syllabus"], confident: true },
    { category: "Screenshots", extensions: ["png", "jpg"], keywords: ["screenshot", "screen shot"], confident: true },
    { category: "Installers", extensions: ["exe", "msi", "dmg", "deb", "appimage"], keywords: [], confident: true },
    { category: "Archives", extensions: ["zip", "rar", "7z", "gz"], keywords: [], confident: true },
    { category: "PDFs", extensions: ["pdf"], keywords: [], confident: false },
    { category: "Images", extensions: ["jpg", "jpeg", "png", "heic", "webp"], keywords: [], confident: true },
    { category: "Videos", extensions: ["mp4", "mkv", "mov"], keywords: [], confident: true },
    { category: "Audio", extensions: ["mp3", "m4a", "wav"], keywords: [], confident: true },
  ],
  organize_by_month: false,
  skip_recent_secs: 120,
  ai: { enabled: true, endpoint: "http://127.0.0.1:11434", model: "llama3.2:3b", max_snippet_chars: 400, batch_size: 8, timeout_secs: 60 },
  theme: "system",
});

let config = defaultConfig();
const ROOT = "C:\\Users\\Asha\\Downloads";

const demoFiles: [string, number, string, string, PlanItem["source_tier"]][] = [
  ["Amazon_Invoice_2024.pdf", 182_000, "Bills & Receipts", 'name contains "invoice"', "rules"],
  ["GST bill March.pdf", 96_000, "Bills & Receipts", 'name contains "gst"', "rules"],
  ["बिजली का बिल.pdf", 120_000, "Bills & Receipts", 'name contains "बिल"', "rules"],
  ["scan0042.pdf", 2_400_000, "Bills & Receipts", "AI: electricity bill from BESCOM", "ai"],
  ["Unit 3 Lecture Notes.pdf", 4_800_000, "College/Study", 'name contains "lecture"', "rules"],
  ["DBMS_Assignment2.docx", 64_000, "College/Study", 'name contains "assignment"', "rules"],
  ["document(3).pdf", 1_100_000, "College/Study", "AI: semester exam timetable", "ai"],
  ["Screenshot 2024-05-01 101010.png", 840_000, "Screenshots", 'name contains "screenshot"', "rules"],
  ["Screenshot 2024-06-11 220301.png", 610_000, "Screenshots", 'name contains "screenshot"', "rules"],
  ["IMG_20240101_101010.jpg", 3_200_000, "Images", ".jpg file", "rules"],
  ["holiday.heic", 2_100_000, "Images", ".heic file", "rules"],
  ["vlc-3.0.20-win64.exe", 42_000_000, "Installers", ".exe file", "rules"],
  ["ChromeSetup.exe", 1_300_000, "Installers", ".exe file", "rules"],
  ["python-3.12.3-amd64.exe", 26_000_000, "Installers", ".exe file", "rules"],
  ["photos.zip", 210_000_000, "Archives", ".zip file", "rules"],
  ["project-final.rar", 18_000_000, "Archives", ".rar file", "rules"],
  ["movie.mkv", 1_400_000_000, "Videos", ".mkv file", "rules"],
  ["WhatsApp Video 2024-03-01.mp4", 22_000_000, "Videos", ".mp4 file", "rules"],
  ["song.mp3", 5_200_000, "Audio", ".mp3 file", "rules"],
  ["voice note.m4a", 800_000, "Audio", ".m4a file", "rules"],
  ["budget.xlsx", 44_000, "Spreadsheets", ".xlsx file", "rules"],
  ["contacts.csv", 12_000, "Spreadsheets", ".csv file", "rules"],
  ["pitch deck.pptx", 7_800_000, "Presentations", ".pptx file", "rules"],
  ["letter.docx", 22_000, "Documents", ".docx file", "rules"],
  ["resume_final_v3.docx", 38_000, "Documents", "AI: personal resume", "ai"],
  ["script.py", 3_000, "Code", ".py file", "rules"],
  ["random.pdf", 340_000, "PDFs", ".pdf file", "rules"],
  ["manual_EN.pdf", 5_600_000, "PDFs", "AI: product manual", "ai"],
  ["mystery.xyz", 1_000, "Other", "unknown type .xyz", "rules"],
  ["IMG_20240101_101010 (1).jpg", 3_200_000, "Images", ".jpg file", "rules"],
];

let plan: Plan | null = null;
let history: JournalSummary[] = [];

function makePlan(byMonth: boolean): Plan {
  const items: PlanItem[] = demoFiles.map(([name, size, category, reason, tier], id) => ({
    id,
    source: `${ROOT}\\${name}`,
    name,
    size,
    modified: 1_715_000_000 - id * 86_400 * 9,
    ext: name.split(".").pop()!.toLowerCase(),
    category,
    custom_folder: null,
    dest_rel: "",
    reason,
    source_tier: tier,
    ambiguous: tier === "ai",
    enabled: true,
  }));
  const p: Plan = {
    root: ROOT,
    organize_by_month: byMonth,
    items,
    skipped: [
      { name: "big.iso.crdownload", reason: "partial_download", detail: "" },
      { name: "desktop.ini", reason: "system", detail: "" },
      { name: "report-draft.docx", reason: "recently_modified", detail: "" },
    ],
    warnings: [],
    created_at: new Date().toISOString(),
  };
  recompute(p);
  return p;
}

function recompute(p: Plan) {
  for (const it of p.items) {
    const folder = it.custom_folder ?? config.categories.find((c) => c.name === it.category)?.folder ?? "Other";
    const d = new Date(it.modified * 1000);
    const month = `${d.getFullYear()}-${String(d.getMonth() + 1).padStart(2, "0")}`;
    it.dest_rel = p.organize_by_month && !it.custom_folder ? `${folder}/${month}` : folder;
  }
}

export async function mockPickFolder() {
  await sleep(150);
  return ROOT;
}

export async function mockInvoke<T>(cmd: string, args: Record<string, unknown>): Promise<T> {
  const r = await handle(cmd, args);
  return r as T;
}

async function handle(cmd: string, args: Record<string, unknown>): Promise<unknown> {
  switch (cmd) {
    case "app_info":
      return { version: "0.1.0-mock", default_folder: ROOT, config_file: "(mock)", journal_dir: "(mock)", os: "windows" } satisfies AppInfo;
    case "get_config":
      return structuredClone(config);
    case "save_config":
      config = structuredClone(args.config as Config);
      if (plan) recompute(plan);
      return structuredClone(config);
    case "reset_config":
      config = defaultConfig();
      return structuredClone(config);
    case "ai_status":
      await sleep(200);
      return {
        reachable: true,
        model_available: true,
        model: config.ai.model,
        installed_models: ["llama3.2:3b"],
        active: config.ai.enabled,
        enabled: config.ai.enabled,
        message: config.ai.enabled ? `Using ${config.ai.model} on this computer.` : "AI is turned off in Settings.",
      } satisfies AiStatus;
    case "prepare": {
      const useAi = Boolean(args.useAi) && config.ai.enabled;
      const stages: Stage[] = [{ stage: "scanning" }, { stage: "hashing", files: demoFiles.length }, { stage: "classifying" }];
      for (const s of stages) {
        emit("tidydrop://progress", s);
        await sleep(250);
      }
      if (useAi) {
        for (let i = 0; i <= 4; i++) {
          emit("tidydrop://progress", { stage: "ai", done: i, total: 4 });
          await sleep(200);
        }
      }
      plan = makePlan(config.organize_by_month);
      if (!useAi) {
        for (const it of plan.items) {
          if (it.source_tier === "ai") {
            it.source_tier = "rules";
            it.category = it.ext === "pdf" ? "PDFs" : "Documents";
            it.reason = `.${it.ext} file`;
          }
        }
        recompute(plan);
      }
      const total = plan.items.reduce((a, b) => a + b.size, 0);
      const largest = [...plan.items].sort((a, b) => b.size - a.size).slice(0, 10).map((i) => ({ name: i.name, path: i.source, size: i.size }));
      const dupA = plan.items.find((i) => i.name === "IMG_20240101_101010.jpg")!;
      const dupB = plan.items.find((i) => i.name === "IMG_20240101_101010 (1).jpg")!;
      return {
        plan: structuredClone(plan),
        stats: {
          file_count: plan.items.length,
          total_size: total,
          skipped_count: plan.skipped.length,
          largest,
          duplicates: [{ hash: "ab12", size: dupA.size, files: [dupA, dupB].map((i) => ({ name: i.name, path: i.source, size: i.size })) }],
          duplicate_waste: dupA.size,
        },
        ai: useAi
          ? { used: true, candidates: 4, classified: 4, changed: 4, failed_batches: 0, message: "AI looked at 4 unclear files and re-sorted 4." }
          : { used: false, candidates: 0, classified: 0, changed: 0, failed_batches: 0, message: "AI not used; smart rules only." },
      } satisfies Prepared;
    }
    case "update_plan": {
      if (!plan) throw "No preview yet. Scan a folder first.";
      if (typeof args.organizeByMonth === "boolean") plan.organize_by_month = args.organizeByMonth;
      for (const o of args.overrides as ItemOverride[]) {
        const it = plan.items.find((i) => i.id === o.id);
        if (!it) continue;
        if (o.enabled !== undefined) it.enabled = o.enabled;
        if (o.category && o.category !== it.category) {
          it.category = o.category;
          it.source_tier = "user";
          it.reason = "chosen by you";
        }
        if (o.custom_folder !== undefined) {
          const c = o.custom_folder.split(/[\\/]/).map((s) => s.trim()).filter((s) => s && s !== "." && s !== "..").join("/");
          it.custom_folder = c || null;
        }
      }
      recompute(plan);
      return structuredClone(plan);
    }
    case "apply_plan": {
      if (!plan) throw "No preview yet. Scan a folder first.";
      const items = plan.items.filter((i) => i.enabled);
      for (let i = 0; i <= items.length; i++) {
        emit("tidydrop://apply-progress", { done: i, total: items.length });
        await sleep(25);
      }
      const id = new Date().toISOString().replace(/\D/g, "").slice(0, 14);
      history.unshift({ id, path: "", root: ROOT, created_at: new Date().toISOString(), moved: items.length, restored: 0, status: "applied" });
      plan = null;
      return {
        journal_id: id,
        moved: items.map((i) => ({ id: i.id, from: i.source, to: `${ROOT}\\${i.dest_rel.replace(/\//g, "\\")}\\${i.name}`, category: i.category })),
        skipped: [],
        failed: [],
      } satisfies ApplyReport;
    }
    case "undo_last": {
      const id = args.id as string | null;
      const j = history.find((h) => (id ? h.id === id : h.status === "applied"));
      if (!j) throw "journal error: nothing to undo";
      await sleep(400);
      j.status = "undone";
      j.restored = j.moved;
      return { journal_id: j.id, restored: Array.from({ length: j.moved }, () => ({ from: "", to: "", renamed: false })), problems: [], folders_removed: 12 } satisfies UndoReport;
    }
    case "history":
      return structuredClone(history);
    case "open_folder":
    case "open_ollama_download":
      return null;
    default:
      throw `mock: unknown command ${cmd}`;
  }
}
