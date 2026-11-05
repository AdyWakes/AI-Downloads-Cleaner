//! The preview plan: which file goes where and why. Nothing on disk changes here.

use crate::config::{Config, OTHER};
use crate::paths::{sanitize_component, sanitize_relative};
use crate::rules::{Classification, RuleEngine, Source};
use crate::scanner::{FileEntry, ScanResult, Skipped};
use chrono::{Local, TimeZone};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PlanItem {
    pub id: usize,
    pub source: PathBuf,
    pub name: String,
    pub size: u64,
    pub modified: i64,
    pub ext: String,
    pub category: String,
    /// User-entered folder (relative to the cleaned folder) overriding the category folder.
    #[serde(default)]
    pub custom_folder: Option<String>,
    /// Destination folder relative to the root, with `/` separators (for display).
    pub dest_rel: String,
    pub reason: String,
    pub source_tier: Source,
    pub ambiguous: bool,
    pub enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Plan {
    pub root: PathBuf,
    pub organize_by_month: bool,
    pub items: Vec<PlanItem>,
    pub skipped: Vec<Skipped>,
    pub warnings: Vec<String>,
    pub created_at: String,
}

/// A user edit from the preview screen. `None` fields are left unchanged.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ItemOverride {
    pub id: usize,
    #[serde(default)]
    pub enabled: Option<bool>,
    #[serde(default)]
    pub category: Option<String>,
    /// `Some("")` clears a custom folder.
    #[serde(default)]
    pub custom_folder: Option<String>,
}

/// `YYYY-MM` in local time for a unix timestamp.
pub fn month_folder(unix: i64) -> String {
    Local
        .timestamp_opt(unix, 0)
        .single()
        .map(|d| d.format("%Y-%m").to_string())
        .unwrap_or_else(|| "Unknown date".into())
}

/// Destination folder (relative to root) for an item.
pub fn dest_rel_path(
    config: &Config,
    category: &str,
    custom: Option<&str>,
    modified: i64,
    by_month: bool,
) -> PathBuf {
    let mut rel = match custom
        .map(sanitize_relative)
        .filter(|p| p.components().next().is_some())
    {
        Some(p) => p,
        None => {
            let folder = config
                .category(category)
                .map(|c| c.folder.as_str())
                .unwrap_or(OTHER);
            PathBuf::from(sanitize_component(folder))
        }
    };
    if by_month && custom.map(|c| c.trim().is_empty()).unwrap_or(true) {
        rel.push(month_folder(modified));
    }
    rel
}

pub fn rel_to_display(p: &Path) -> String {
    p.components()
        .map(|c| c.as_os_str().to_string_lossy().into_owned())
        .collect::<Vec<_>>()
        .join("/")
}

/// Rules-only classification for every file in the scan.
pub fn classify_with_rules(files: &[FileEntry], config: &Config) -> Vec<Classification> {
    let engine = RuleEngine::new(config);
    files.iter().map(|f| engine.classify(&f.name)).collect()
}

/// Build a plan from a scan and one classification per file (same order).
pub fn build_plan(
    scan: &ScanResult,
    classes: &[Classification],
    config: &Config,
    by_month: bool,
) -> Plan {
    assert_eq!(
        scan.files.len(),
        classes.len(),
        "one classification per file"
    );
    let items = scan
        .files
        .iter()
        .zip(classes)
        .enumerate()
        .map(|(id, (f, c))| {
            let category = if config.category(&c.category).is_some() {
                c.category.clone()
            } else {
                OTHER.to_string()
            };
            let rel = dest_rel_path(config, &category, None, f.modified, by_month);
            PlanItem {
                id,
                source: f.path.clone(),
                name: f.name.clone(),
                size: f.size,
                modified: f.modified,
                ext: f.ext.clone(),
                category,
                custom_folder: None,
                dest_rel: rel_to_display(&rel),
                reason: c.reason.clone(),
                source_tier: c.source,
                ambiguous: c.ambiguous,
                enabled: true,
            }
        })
        .collect();
    Plan {
        root: scan.root.clone(),
        organize_by_month: by_month,
        items,
        skipped: scan.skipped.clone(),
        warnings: scan.warnings.clone(),
        created_at: Local::now().to_rfc3339(),
    }
}

impl Plan {
    /// Apply user edits. Destinations are always recomputed here (never taken from the UI),
    /// so they can only ever point inside the cleaned folder.
    pub fn apply_overrides(&mut self, overrides: &[ItemOverride], config: &Config) {
        for o in overrides {
            let Some(item) = self.items.iter_mut().find(|i| i.id == o.id) else {
                continue;
            };
            if let Some(e) = o.enabled {
                item.enabled = e;
            }
            if let Some(c) = &o.category {
                if config.category(c).is_some() && &item.category != c {
                    item.category = c.clone();
                    item.source_tier = Source::User;
                    item.reason = "chosen by you".into();
                }
            }
            if let Some(cf) = &o.custom_folder {
                let cleaned = rel_to_display(&sanitize_relative(cf));
                item.custom_folder = if cleaned.is_empty() {
                    None
                } else {
                    Some(cleaned)
                };
                if item.custom_folder.is_some() {
                    item.source_tier = Source::User;
                    item.reason = "chosen by you".into();
                }
            }
        }
        self.recompute(config);
    }

    pub fn set_organize_by_month(&mut self, by_month: bool, config: &Config) {
        self.organize_by_month = by_month;
        self.recompute(config);
    }

    pub fn recompute(&mut self, config: &Config) {
        for item in &mut self.items {
            let rel = dest_rel_path(
                config,
                &item.category,
                item.custom_folder.as_deref(),
                item.modified,
                self.organize_by_month,
            );
            item.dest_rel = rel_to_display(&rel);
        }
    }

    pub fn enabled_count(&self) -> usize {
        self.items.iter().filter(|i| i.enabled).count()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn scan_of(names: &[&str]) -> (tempfile::TempDir, ScanResult, Config) {
        let d = tempfile::tempdir().unwrap();
        for n in names {
            fs::write(d.path().join(n), n).unwrap();
        }
        let mut c = Config::default();
        c.skip_recent_secs = 0;
        let s = crate::scanner::scan(d.path(), &c).unwrap();
        (d, s, c)
    }

    #[test]
    fn plan_destinations() {
        let (_d, s, c) = scan_of(&["invoice.pdf", "setup.exe", "notes.zzz"]);
        let cls = classify_with_rules(&s.files, &c);
        let p = build_plan(&s, &cls, &c, false);
        let dest: Vec<_> = p.items.iter().map(|i| i.dest_rel.as_str()).collect();
        assert_eq!(dest, vec!["Bills & Receipts", "Other", "Installers"]);
        let p = build_plan(&s, &cls, &c, true);
        assert!(p.items[0].dest_rel.starts_with("Bills & Receipts/20"));
    }

    #[test]
    fn overrides_cannot_escape_root() {
        let (_d, s, c) = scan_of(&["a.pdf"]);
        let cls = classify_with_rules(&s.files, &c);
        let mut p = build_plan(&s, &cls, &c, false);
        p.apply_overrides(
            &[ItemOverride {
                id: 0,
                custom_folder: Some("../../Windows/System32".into()),
                ..Default::default()
            }],
            &c,
        );
        assert_eq!(p.items[0].dest_rel, "Windows/System32");
        p.apply_overrides(
            &[ItemOverride {
                id: 0,
                category: Some("Images".into()),
                custom_folder: Some("".into()),
                ..Default::default()
            }],
            &c,
        );
        assert_eq!(p.items[0].dest_rel, "Images");
        assert_eq!(p.items[0].source_tier, Source::User);
        p.apply_overrides(
            &[ItemOverride {
                id: 0,
                category: Some("Nonexistent".into()),
                enabled: Some(false),
                ..Default::default()
            }],
            &c,
        );
        assert_eq!(p.items[0].category, "Images");
        assert!(!p.items[0].enabled);
    }
}
