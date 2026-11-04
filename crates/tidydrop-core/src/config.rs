//! User-editable configuration: categories, rules, AI settings.
//!
//! Stored as pretty JSON in the platform config dir (`config.json`).

use crate::error::{Error, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

/// Name of the fallback category. Always present.
pub const OTHER: &str = "Other";

/// A destination category (one folder inside the cleaned directory).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Category {
    /// Display name, also the identifier used by rules and the AI.
    pub name: String,
    /// Folder name created inside the cleaned directory. Sanitized before use.
    pub folder: String,
    /// One-line description, given to the AI model as a hint.
    #[serde(default)]
    pub description: String,
}

/// A rule maps a file to a category. Rules are evaluated in order; first match wins.
///
/// A rule matches when **both** hold:
/// - `extensions` is empty, or the file extension (lowercase, no dot) is listed;
/// - `keywords` is empty, or at least one keyword matches the filename.
///
/// Keywords match whole words of the filename (case-insensitive, Unicode-aware;
/// `GST_Invoice-2024.pdf` has words `gst`, `invoice`, `2024`). A keyword may
/// contain spaces (`screen shot`) to match consecutive words. A keyword that
/// starts with `re:` is a case-insensitive regular expression over the whole name.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Rule {
    pub category: String,
    #[serde(default)]
    pub extensions: Vec<String>,
    #[serde(default)]
    pub keywords: Vec<String>,
    /// If false, a match is considered *ambiguous* and may be refined by the AI tier.
    #[serde(default = "yes")]
    pub confident: bool,
}

fn yes() -> bool {
    true
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AiConfig {
    /// User toggle. AI is only used when enabled *and* Ollama is reachable.
    pub enabled: bool,
    /// Must be a loopback address (localhost / 127.0.0.1 / ::1). Enforced at runtime.
    pub endpoint: String,
    pub model: String,
    /// Max characters of file text sent to the model per file.
    pub max_snippet_chars: usize,
    /// Files per request.
    pub batch_size: usize,
    /// Per-request timeout in seconds.
    pub timeout_secs: u64,
}

impl Default for AiConfig {
    fn default() -> Self {
        AiConfig {
            enabled: true,
            endpoint: "http://127.0.0.1:11434".into(),
            model: "llama3.2:3b".into(),
            max_snippet_chars: 400,
            batch_size: 8,
            timeout_secs: 60,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Config {
    #[serde(default = "default_version")]
    pub version: u32,
    pub categories: Vec<Category>,
    pub rules: Vec<Rule>,
    #[serde(default)]
    pub organize_by_month: bool,
    /// Files modified (or, on Windows, created) more recently than this are skipped.
    #[serde(default = "default_recent")]
    pub skip_recent_secs: u64,
    #[serde(default)]
    pub ai: AiConfig,
    /// "system" | "light" | "dark" (UI only).
    #[serde(default = "default_theme")]
    pub theme: String,
}

fn default_version() -> u32 {
    1
}
fn default_recent() -> u64 {
    120
}
fn default_theme() -> String {
    "system".into()
}

fn cat(name: &str, folder: &str, description: &str) -> Category {
    Category {
        name: name.into(),
        folder: folder.into(),
        description: description.into(),
    }
}

fn rule(category: &str, exts: &[&str], keywords: &[&str], confident: bool) -> Rule {
    Rule {
        category: category.into(),
        extensions: exts.iter().map(|s| s.to_string()).collect(),
        keywords: keywords.iter().map(|s| s.to_string()).collect(),
        confident,
    }
}

pub const DOC_EXTS: &[&str] = &[
    "doc", "docx", "odt", "rtf", "txt", "md", "pages", "tex", "epub", "wpd",
];
pub const PDF_EXTS: &[&str] = &["pdf"];
pub const IMAGE_EXTS: &[&str] = &[
    "jpg", "jpeg", "png", "gif", "webp", "bmp", "tif", "tiff", "svg", "heic", "heif", "avif",
    "ico", "raw", "cr2", "cr3", "nef", "arw", "dng", "psd", "jfif",
];
pub const VIDEO_EXTS: &[&str] = &[
    "mp4", "mkv", "mov", "avi", "wmv", "webm", "flv", "m4v", "3gp", "mpeg", "mpg", "mts", "m2ts",
    "vob",
];
pub const AUDIO_EXTS: &[&str] = &[
    "mp3", "wav", "flac", "aac", "ogg", "oga", "m4a", "wma", "opus", "aiff", "aif", "mid", "midi",
    "amr",
];
pub const INSTALLER_EXTS: &[&str] = &[
    "exe",
    "msi",
    "msix",
    "msixbundle",
    "appx",
    "appxbundle",
    "dmg",
    "pkg",
    "mpkg",
    "deb",
    "rpm",
    "appimage",
    "apk",
    "xapk",
    "flatpakref",
    "snap",
    "iso",
    "img",
];
pub const ARCHIVE_EXTS: &[&str] = &[
    "zip", "rar", "7z", "tar", "gz", "tgz", "bz2", "tbz", "xz", "txz", "zst", "lz", "lzma", "cab",
];
pub const CODE_EXTS: &[&str] = &[
    "py", "ipynb", "js", "mjs", "cjs", "ts", "tsx", "jsx", "java", "kt", "c", "h", "cpp", "cc",
    "hpp", "cs", "go", "rs", "rb", "php", "swift", "html", "htm", "css", "scss", "json", "xml",
    "yaml", "yml", "toml", "ini", "sh", "bash", "ps1", "bat", "cmd", "sql", "r", "m", "lua",
    "dart", "vue", "svelte",
];
pub const SHEET_EXTS: &[&str] = &[
    "xls", "xlsx", "xlsm", "xlsb", "csv", "tsv", "ods", "numbers",
];
pub const SLIDE_EXTS: &[&str] = &["ppt", "pptx", "pps", "ppsx", "odp", "key"];

impl Default for Config {
    fn default() -> Self {
        let categories = vec![
            cat("Documents", "Documents", "Letters, Word documents, text files, ebooks"),
            cat("PDFs", "PDFs", "PDF files that are not bills or study material"),
            cat("Images", "Images", "Photos and pictures"),
            cat("Screenshots", "Screenshots", "Screen captures"),
            cat("Videos", "Videos", "Video files"),
            cat("Audio", "Audio", "Music, voice notes, podcasts"),
            cat("Installers", "Installers", "Program installers and disk images"),
            cat("Archives", "Archives", "Zip, rar, 7z and other compressed files"),
            cat("Code", "Code", "Source code, scripts, data/config files"),
            cat("Spreadsheets", "Spreadsheets", "Excel, CSV and other tables"),
            cat("Presentations", "Presentations", "Slide decks"),
            cat(
                "Bills & Receipts",
                "Bills & Receipts",
                "Invoices, receipts, bills, order confirmations, tax/GST documents, bank statements",
            ),
            cat(
                "College/Study",
                "College & Study",
                "Assignments, lecture notes, syllabus, question papers, study material",
            ),
            cat(OTHER, "Other", "Anything that does not fit elsewhere"),
        ];

        let billable: Vec<&str> = [
            PDF_EXTS,
            DOC_EXTS,
            IMAGE_EXTS,
            SHEET_EXTS,
            &["html", "htm", "eml"],
        ]
        .concat();
        let studyable: Vec<&str> =
            [PDF_EXTS, DOC_EXTS, SLIDE_EXTS, SHEET_EXTS, &["ipynb"]].concat();

        let rules = vec![
            rule(
                "Bills & Receipts",
                &billable,
                &[
                    "invoice",
                    "invoices",
                    "receipt",
                    "receipts",
                    "bill",
                    "bills",
                    "billing",
                    "gst",
                    "gstin",
                    "order",
                    "orders",
                    "payment",
                    "paid",
                    "statement",
                    "estatement",
                    "e-statement",
                    "tax invoice",
                    "txn",
                    "transaction",
                    "purchase",
                    "ebill",
                    "e-bill",
                    "challan",
                    "voucher",
                    "quotation",
                    "booking",
                    "ticket",
                    "eticket",
                    "e-ticket",
                    "itr",
                    "form16",
                    "form 16",
                    "salary slip",
                    "payslip",
                    "re:^inv[-_ ]?\\d",
                    "बिल",
                    "रसीद",
                    "चालान",
                    "भुगतान",
                ],
                true,
            ),
            rule(
                "College/Study",
                &studyable,
                &[
                    "assignment",
                    "assignments",
                    "lecture",
                    "lectures",
                    "notes",
                    "syllabus",
                    "semester",
                    "sem",
                    "exam",
                    "exams",
                    "question paper",
                    "questions",
                    "homework",
                    "hw",
                    "lab",
                    "tutorial",
                    "thesis",
                    "dissertation",
                    "unit",
                    "chapter",
                    "course",
                    "college",
                    "university",
                    "midterm",
                    "mid-term",
                    "quiz",
                    "practical",
                    "project report",
                    "marksheet",
                    "admit card",
                    "timetable",
                    "time table",
                    "module",
                    "study",
                    "pyq",
                    "solutions",
                    "worksheet",
                    "nptel",
                    "cbse",
                    "ncert",
                    "नोट्स",
                    "अध्याय",
                    "परीक्षा",
                ],
                true,
            ),
            rule(
                "Screenshots",
                IMAGE_EXTS,
                &[
                    "screenshot",
                    "screenshots",
                    "screen shot",
                    "screencapture",
                    "screen capture",
                    "snip",
                    "snipping",
                    "capture",
                    "scrnli",
                    "re:^scr_?\\d",
                    "re:^ss_?\\d",
                ],
                true,
            ),
            rule("Installers", INSTALLER_EXTS, &[], true),
            rule("Archives", ARCHIVE_EXTS, &[], true),
            rule("PDFs", PDF_EXTS, &[], false),
            rule("Spreadsheets", SHEET_EXTS, &[], true),
            rule("Presentations", SLIDE_EXTS, &[], true),
            rule("Documents", &["txt", "md", "rtf"], &[], false),
            rule("Documents", DOC_EXTS, &[], false),
            rule("Images", IMAGE_EXTS, &[], true),
            rule("Videos", VIDEO_EXTS, &[], true),
            rule("Audio", AUDIO_EXTS, &[], true),
            rule("Code", CODE_EXTS, &[], true),
        ];

        Config {
            version: 1,
            categories,
            rules,
            organize_by_month: false,
            skip_recent_secs: 120,
            ai: AiConfig::default(),
            theme: default_theme(),
        }
    }
}

impl Config {
    /// Load from a JSON file. Missing file => defaults.
    pub fn load(path: &Path) -> Result<Config> {
        match fs::read_to_string(path) {
            Ok(text) => {
                let mut cfg: Config = serde_json::from_str(&text)
                    .map_err(|e| Error::Config(format!("{}: {e}", path.display())))?;
                cfg.normalize();
                Ok(cfg)
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(Config::default()),
            Err(e) => Err(e.into()),
        }
    }

    /// Save atomically (write temp file, then rename over).
    pub fn save(&self, path: &Path) -> Result<()> {
        let mut cfg = self.clone();
        cfg.normalize();
        cfg.validate()?;
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let tmp = path.with_extension("json.tmp");
        fs::write(&tmp, serde_json::to_string_pretty(&cfg)?)?;
        fs::rename(&tmp, path)?;
        Ok(())
    }

    /// Lowercase/strip extensions, ensure the fallback category exists.
    pub fn normalize(&mut self) {
        for r in &mut self.rules {
            for e in &mut r.extensions {
                *e = e.trim().trim_start_matches('.').to_lowercase();
            }
            r.extensions.retain(|e| !e.is_empty());
            for k in &mut r.keywords {
                *k = k.trim().to_string();
            }
            r.keywords.retain(|k| !k.is_empty());
        }
        for c in &mut self.categories {
            c.name = c.name.trim().to_string();
            if c.folder.trim().is_empty() {
                c.folder = c.name.clone();
            }
        }
        if !self.categories.iter().any(|c| c.name == OTHER) {
            self.categories
                .push(cat(OTHER, "Other", "Anything that does not fit elsewhere"));
        }
        if self.ai.batch_size == 0 {
            self.ai.batch_size = 1;
        }
    }

    /// Check rules reference known categories, names are unique, regexes compile.
    pub fn validate(&self) -> Result<()> {
        let mut seen = std::collections::HashSet::new();
        let mut folders = std::collections::HashSet::new();
        for c in &self.categories {
            if c.name.is_empty() {
                return Err(Error::Config("category name cannot be empty".into()));
            }
            if !seen.insert(c.name.to_lowercase()) {
                return Err(Error::Config(format!("duplicate category '{}'", c.name)));
            }
            let f = crate::paths::sanitize_component(&c.folder);
            if !folders.insert(f.to_lowercase()) {
                return Err(Error::Config(format!(
                    "two categories use the folder '{f}'"
                )));
            }
        }
        for r in &self.rules {
            if self.category(&r.category).is_none() {
                return Err(Error::Config(format!(
                    "rule refers to unknown category '{}'",
                    r.category
                )));
            }
            for k in &r.keywords {
                if let Some(re) = k.strip_prefix("re:") {
                    regex::RegexBuilder::new(re)
                        .case_insensitive(true)
                        .build()
                        .map_err(|e| Error::Config(format!("bad regex '{re}': {e}")))?;
                }
            }
        }
        Ok(())
    }

    pub fn category(&self, name: &str) -> Option<&Category> {
        self.categories.iter().find(|c| c.name == name)
    }

    pub fn category_names(&self) -> Vec<String> {
        self.categories.iter().map(|c| c.name.clone()).collect()
    }
}

/// Where tidydrop keeps its config and journals.
///
/// `TIDYDROP_HOME` overrides everything (useful for tests and portable installs).
#[derive(Debug, Clone)]
pub struct AppPaths {
    pub config_file: PathBuf,
    pub journal_dir: PathBuf,
}

impl AppPaths {
    pub fn detect() -> AppPaths {
        if let Some(home) = std::env::var_os("TIDYDROP_HOME") {
            return AppPaths::under(Path::new(&home));
        }
        let config = dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("tidydrop");
        let data = dirs::data_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("tidydrop");
        AppPaths {
            config_file: config.join("config.json"),
            journal_dir: data.join("journals"),
        }
    }

    pub fn under(base: &Path) -> AppPaths {
        AppPaths {
            config_file: base.join("config.json"),
            journal_dir: base.join("journals"),
        }
    }
}

/// System Downloads folder, if known.
pub fn default_downloads_dir() -> Option<PathBuf> {
    dirs::download_dir().or_else(|| dirs::home_dir().map(|h| h.join("Downloads")))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_config_is_valid() {
        Config::default().validate().unwrap();
    }

    #[test]
    fn roundtrip_save_load() {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join("sub").join("config.json");
        let mut c = Config::default();
        c.organize_by_month = true;
        c.save(&p).unwrap();
        let back = Config::load(&p).unwrap();
        assert_eq!(c, back);
    }

    #[test]
    fn missing_file_gives_defaults() {
        let dir = tempfile::tempdir().unwrap();
        assert_eq!(
            Config::load(&dir.path().join("nope.json")).unwrap(),
            Config::default()
        );
    }

    #[test]
    fn rejects_unknown_category_and_bad_regex() {
        let mut c = Config::default();
        c.rules.push(rule("Nope", &["x"], &[], true));
        assert!(c.validate().is_err());
        let mut c = Config::default();
        c.rules.push(rule("Other", &[], &["re:(unclosed"], true));
        assert!(c.validate().is_err());
    }

    #[test]
    fn normalize_cleans_extensions_and_adds_other() {
        let mut c = Config::default();
        c.categories.retain(|c| c.name != OTHER);
        c.rules[0].extensions.push(" .PDF ".into());
        c.normalize();
        assert!(c.category(OTHER).is_some());
        assert!(c.rules[0].extensions.contains(&"pdf".to_string()));
    }
}
