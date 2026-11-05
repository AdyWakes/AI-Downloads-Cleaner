// Mirrors the serde types in crates/tidydrop-core.

export interface Category {
  name: string;
  folder: string;
  description: string;
}

export interface Rule {
  category: string;
  extensions: string[];
  keywords: string[];
  confident: boolean;
}

export interface AiConfig {
  enabled: boolean;
  endpoint: string;
  model: string;
  max_snippet_chars: number;
  batch_size: number;
  timeout_secs: number;
}

export interface Config {
  version: number;
  categories: Category[];
  rules: Rule[];
  organize_by_month: boolean;
  skip_recent_secs: number;
  ai: AiConfig;
  theme: "system" | "light" | "dark" | string;
}

export type Source = "rules" | "ai" | "user";

export interface PlanItem {
  id: number;
  source: string;
  name: string;
  size: number;
  modified: number;
  ext: string;
  category: string;
  custom_folder: string | null;
  dest_rel: string;
  reason: string;
  source_tier: Source;
  ambiguous: boolean;
  enabled: boolean;
}

export interface Skipped {
  name: string;
  reason: string;
  detail: string;
}

export interface Plan {
  root: string;
  organize_by_month: boolean;
  items: PlanItem[];
  skipped: Skipped[];
  warnings: string[];
  created_at: string;
}

export interface FileBrief {
  name: string;
  path: string;
  size: number;
}

export interface DuplicateGroup {
  hash: string;
  size: number;
  files: FileBrief[];
}

export interface Stats {
  file_count: number;
  total_size: number;
  skipped_count: number;
  largest: FileBrief[];
  duplicates: DuplicateGroup[];
  duplicate_waste: number;
}

export interface AiRunReport {
  used: boolean;
  candidates: number;
  classified: number;
  changed: number;
  failed_batches: number;
  message: string;
}

export interface Prepared {
  plan: Plan;
  stats: Stats;
  ai: AiRunReport;
}

export interface AiStatus {
  reachable: boolean;
  model_available: boolean;
  model: string;
  installed_models: string[];
  active: boolean;
  enabled: boolean;
  message: string;
}

export interface ItemOverride {
  id: number;
  enabled?: boolean;
  category?: string;
  custom_folder?: string;
}

export interface Moved {
  id: number;
  from: string;
  to: string;
  category: string;
}

export interface NotMoved {
  id: number;
  name: string;
  reason: string;
}

export interface ApplyReport {
  journal_id: string;
  moved: Moved[];
  skipped: NotMoved[];
  failed: NotMoved[];
}

export interface UndoReport {
  journal_id: string;
  restored: { from: string; to: string; renamed: boolean }[];
  problems: { path: string; reason: string }[];
  folders_removed: number;
}

export type JournalStatus = "applied" | "interrupted" | "undone" | "partially_undone" | "empty";

export interface JournalSummary {
  id: string;
  path: string;
  root: string;
  created_at: string;
  moved: number;
  restored: number;
  status: JournalStatus;
}

export interface AppInfo {
  version: string;
  default_folder: string | null;
  config_file: string;
  journal_dir: string;
  os: string;
}

export type Stage =
  | { stage: "scanning" }
  | { stage: "hashing"; files: number }
  | { stage: "classifying" }
  | { stage: "ai"; done: number; total: number }
  | { stage: "ready" };
