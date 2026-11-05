//! Convenience: scan -> stats -> rules -> optional AI -> plan, used by both app and CLI.

use crate::ai::{self, AiRunReport};
use crate::config::Config;
use crate::plan::{build_plan, classify_with_rules, Plan};
use crate::stats::{self, Stats};
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "stage", rename_all = "snake_case")]
pub enum Stage {
    Scanning,
    Hashing { files: usize },
    Classifying,
    Ai { done: usize, total: usize },
    Ready,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Prepared {
    pub plan: Plan,
    pub stats: Stats,
    pub ai: AiRunReport,
}

/// `use_ai`: try the AI tier (still requires Ollama + model; falls back silently).
pub fn prepare(
    folder: &Path,
    config: &Config,
    use_ai: bool,
    progress: &mut dyn FnMut(Stage),
) -> crate::Result<Prepared> {
    progress(Stage::Scanning);
    let scan = crate::scanner::scan(folder, config)?;
    progress(Stage::Hashing {
        files: scan.files.len(),
    });
    let stats = stats::compute(&scan);
    progress(Stage::Classifying);
    let mut classes = classify_with_rules(&scan.files, config);
    let ai_report = if use_ai && config.ai.enabled {
        let st = ai::status(&config.ai);
        if st.active {
            ai::refine(&scan.files, &mut classes, config, &mut |done, total| {
                progress(Stage::Ai { done, total })
            })
        } else {
            AiRunReport {
                message: st.message,
                ..Default::default()
            }
        }
    } else {
        AiRunReport {
            message: "AI not used; smart rules only.".into(),
            ..Default::default()
        }
    };
    let plan = build_plan(&scan, &classes, config, config.organize_by_month);
    progress(Stage::Ready);
    Ok(Prepared {
        plan,
        stats,
        ai: ai_report,
    })
}
