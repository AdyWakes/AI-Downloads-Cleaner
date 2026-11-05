//! tidydrop-cli: the same engine as the desktop app, from a terminal.

use clap::{Parser, Subcommand};
use std::io::{self, BufRead, Write};
use std::path::PathBuf;
use std::process::ExitCode;
use tidydrop_core::config::{default_downloads_dir, AppPaths, Config};
use tidydrop_core::journal::JournalStatus;
use tidydrop_core::pipeline::{self, Prepared, Stage};
use tidydrop_core::{ai, apply, journal, undo};

#[derive(Parser)]
#[command(
    name = "tidydrop-cli",
    version,
    about = "Sort a messy Downloads folder. Never deletes; every cleanup can be undone."
)]
struct Cli {
    /// Use this folder for settings and undo history instead of the default (same as TIDYDROP_HOME).
    #[arg(long, global = true, value_name = "DIR")]
    home: Option<PathBuf>,
    #[command(subcommand)]
    cmd: Cmd,
}

#[derive(clap::Args, Clone)]
struct PlanArgs {
    /// Folder to clean (default: your Downloads folder).
    folder: Option<PathBuf>,
    /// Put files in month sub-folders (e.g. Images/2024-05).
    #[arg(long)]
    by_month: bool,
    /// Don't use the local AI even if Ollama is running.
    #[arg(long)]
    no_ai: bool,
    /// Only move files in these categories (repeatable), e.g. --only Installers --only Archives.
    #[arg(long, value_name = "CATEGORY")]
    only: Vec<String>,
}

#[derive(Subcommand)]
enum Cmd {
    /// Show stats: file count, size, largest files, duplicates. Changes nothing.
    Scan {
        folder: Option<PathBuf>,
        #[arg(long)]
        json: bool,
    },
    /// Show what would be moved where, and why. Changes nothing.
    Plan {
        #[command(flatten)]
        args: PlanArgs,
        #[arg(long)]
        json: bool,
    },
    /// Show the plan, ask for confirmation, then move files.
    Clean {
        #[command(flatten)]
        args: PlanArgs,
        /// Don't ask for confirmation.
        #[arg(long, short = 'y')]
        yes: bool,
    },
    /// Undo the most recent cleanup (or a specific one with --id).
    Undo {
        #[arg(long)]
        id: Option<String>,
    },
    /// List past cleanups.
    History {
        #[arg(long)]
        json: bool,
    },
    /// Check whether the local AI (Ollama) is available.
    AiStatus,
    /// Settings file helpers.
    Config {
        #[command(subcommand)]
        action: ConfigCmd,
    },
}

#[derive(Subcommand)]
enum ConfigCmd {
    /// Print where the settings file and undo history live.
    Path,
    /// Print the current settings as JSON.
    Show,
    /// Restore default categories and rules.
    Reset,
}

fn human(n: u64) -> String {
    let units = ["B", "KB", "MB", "GB", "TB"];
    let mut v = n as f64;
    let mut i = 0;
    while v >= 1024.0 && i < units.len() - 1 {
        v /= 1024.0;
        i += 1;
    }
    if i == 0 {
        format!("{n} B")
    } else {
        format!("{v:.1} {}", units[i])
    }
}

fn folder_or_default(f: Option<PathBuf>) -> Result<PathBuf, String> {
    f.or_else(default_downloads_dir)
        .ok_or_else(|| "Couldn't find your Downloads folder; pass a folder path.".into())
}

fn prepare(paths: &AppPaths, args: &PlanArgs, quiet: bool) -> Result<(Config, Prepared), String> {
    let mut config = Config::load(&paths.config_file).map_err(|e| e.to_string())?;
    if args.by_month {
        config.organize_by_month = true;
    }
    for o in &args.only {
        if config.category(o).is_none() {
            return Err(format!(
                "unknown category '{o}'. Categories: {}",
                config.category_names().join(", ")
            ));
        }
    }
    let folder = folder_or_default(args.folder.clone())?;
    let mut p = pipeline::prepare(&folder, &config, !args.no_ai, &mut |s| {
        if quiet {
            return;
        }
        match s {
            Stage::Scanning => eprintln!("Scanning {} …", folder.display()),
            Stage::Ai { done, total } => eprint!("\rLocal AI: {done}/{total} files"),
            Stage::Ready => eprint!(""),
            _ => {}
        }
    })
    .map_err(|e| e.to_string())?;
    if !quiet && p.ai.used {
        eprintln!();
    }
    if !args.only.is_empty() {
        for it in &mut p.plan.items {
            it.enabled = args.only.contains(&it.category);
        }
    }
    Ok((config, p))
}

fn print_plan(p: &Prepared, config: &Config) {
    let plan = &p.plan;
    println!("Folder: {}", plan.root.display());
    for w in &plan.warnings {
        println!("WARNING: {w}");
    }
    println!("AI: {}", p.ai.message);
    println!();
    for cat in &config.categories {
        let items: Vec<_> = plan
            .items
            .iter()
            .filter(|i| i.enabled && i.category == cat.name)
            .collect();
        if items.is_empty() {
            continue;
        }
        println!("{}  ({} files)", cat.name, items.len());
        for i in items {
            let tag = match i.source_tier {
                tidydrop_core::rules::Source::Ai => " [AI]",
                _ => "",
            };
            println!("  {}  ->  {}/   ({}){tag}", i.name, i.dest_rel, i.reason);
        }
    }
    let off = plan.items.iter().filter(|i| !i.enabled).count();
    if off > 0 {
        println!("\n{off} files not selected (--only).");
    }
    if !plan.skipped.is_empty() {
        println!("\nLeft alone for safety ({}):", plan.skipped.len());
        for s in &plan.skipped {
            println!("  {}  — {}", s.name, s.reason.describe());
        }
    }
}

fn confirm(question: &str) -> bool {
    print!("{question} [y/N] ");
    let _ = io::stdout().flush();
    let mut line = String::new();
    if io::stdin().lock().read_line(&mut line).is_err() {
        return false;
    }
    matches!(line.trim().to_lowercase().as_str(), "y" | "yes")
}

fn run(cli: Cli) -> Result<(), String> {
    let paths = match &cli.home {
        Some(h) => AppPaths::under(h),
        None => AppPaths::detect(),
    };
    match cli.cmd {
        Cmd::Scan { folder, json } => {
            let config = Config::load(&paths.config_file).map_err(|e| e.to_string())?;
            let folder = folder_or_default(folder)?;
            let scan = tidydrop_core::scanner::scan(&folder, &config).map_err(|e| e.to_string())?;
            let stats = tidydrop_core::stats::compute(&scan);
            if json {
                println!("{}", serde_json::to_string_pretty(&stats).unwrap());
                return Ok(());
            }
            println!("Folder: {}", scan.root.display());
            println!(
                "Files to sort: {}   Total size: {}   Left alone: {}",
                stats.file_count,
                human(stats.total_size),
                stats.skipped_count
            );
            println!("\nLargest files:");
            for f in &stats.largest {
                println!("  {:>10}  {}", human(f.size), f.name);
            }
            if stats.duplicates.is_empty() {
                println!("\nNo duplicates found.");
            } else {
                println!(
                    "\nDuplicates ({} in extra copies; tidydrop never deletes them):",
                    human(stats.duplicate_waste)
                );
                for g in &stats.duplicates {
                    let names: Vec<_> = g.files.iter().map(|f| f.name.as_str()).collect();
                    println!("  {:>10}  {}", human(g.size), names.join("  =  "));
                }
            }
        }
        Cmd::Plan { args, json } => {
            let (config, p) = prepare(&paths, &args, json)?;
            if json {
                println!("{}", serde_json::to_string_pretty(&p).unwrap());
            } else {
                print_plan(&p, &config);
                println!("\nNothing was moved. Run `tidydrop-cli clean` to apply.");
            }
        }
        Cmd::Clean { args, yes } => {
            let (config, p) = prepare(&paths, &args, false)?;
            print_plan(&p, &config);
            let n = p.plan.enabled_count();
            if n == 0 {
                println!("\nNothing to move.");
                return Ok(());
            }
            if !yes
                && !confirm(&format!(
                    "\nMove {n} files? (You can undo with `tidydrop-cli undo`.)"
                ))
            {
                println!("Cancelled. Nothing was moved.");
                return Ok(());
            }
            let report = apply::apply_plan(&p.plan, &config, &paths.journal_dir, &mut |pr| {
                if pr.total > 0 {
                    eprint!("\rMoving {}/{}", pr.done, pr.total);
                }
            })
            .map_err(|e| e.to_string())?;
            eprintln!();
            println!(
                "Moved {} files. Undo id: {}",
                report.moved.len(),
                report.journal_id
            );
            for s in report.skipped.iter().chain(&report.failed) {
                println!("  not moved: {}  — {}", s.name, s.reason);
            }
            if !report.failed.is_empty() {
                return Err(format!(
                    "{} files could not be moved (they are still in place)",
                    report.failed.len()
                ));
            }
        }
        Cmd::Undo { id } => {
            let r = match id {
                Some(id) => undo::undo_by_id(&paths.journal_dir, &id),
                None => undo::undo_last(&paths.journal_dir),
            }
            .map_err(|e| e.to_string())?;
            println!(
                "Restored {} files, removed {} empty folders.",
                r.restored.len(),
                r.folders_removed
            );
            for x in r.restored.iter().filter(|x| x.renamed) {
                println!("  restored as {} (original name was taken)", x.to.display());
            }
            for p in &r.problems {
                println!("  problem: {}  — {}", p.path.display(), p.reason);
            }
            if !r.problems.is_empty() {
                return Err(format!(
                    "{} files could not be restored; run undo again after fixing",
                    r.problems.len()
                ));
            }
        }
        Cmd::History { json } => {
            let list = journal::list(&paths.journal_dir).map_err(|e| e.to_string())?;
            if json {
                println!("{}", serde_json::to_string_pretty(&list).unwrap());
                return Ok(());
            }
            if list.is_empty() {
                println!("No cleanups yet.");
            }
            for j in list {
                let st = match j.status {
                    JournalStatus::Applied => "applied",
                    JournalStatus::Interrupted => "interrupted",
                    JournalStatus::Undone => "undone",
                    JournalStatus::PartiallyUndone => "partly undone",
                    JournalStatus::Empty => "nothing moved",
                };
                println!(
                    "{}  {:<14} {:>5} files  {}",
                    j.id,
                    st,
                    j.moved,
                    j.root.display()
                );
            }
        }
        Cmd::AiStatus => {
            let config = Config::load(&paths.config_file).map_err(|e| e.to_string())?;
            let s = ai::status(&config.ai);
            println!("AI: {}", if s.active { "on" } else { "off" });
            println!("{}", s.message);
            if !s.installed_models.is_empty() {
                println!("Installed models: {}", s.installed_models.join(", "));
            }
            if !s.reachable {
                println!(
                    "Install Ollama from https://ollama.com/download, then run: ollama pull {}",
                    config.ai.model
                );
            }
        }
        Cmd::Config { action } => match action {
            ConfigCmd::Path => {
                println!("settings: {}", paths.config_file.display());
                println!("undo history: {}", paths.journal_dir.display());
            }
            ConfigCmd::Show => {
                let c = Config::load(&paths.config_file).map_err(|e| e.to_string())?;
                println!("{}", serde_json::to_string_pretty(&c).unwrap());
            }
            ConfigCmd::Reset => {
                Config::default()
                    .save(&paths.config_file)
                    .map_err(|e| e.to_string())?;
                println!("Defaults written to {}", paths.config_file.display());
            }
        },
    }
    Ok(())
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    match run(cli) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("error: {e}");
            ExitCode::FAILURE
        }
    }
}
