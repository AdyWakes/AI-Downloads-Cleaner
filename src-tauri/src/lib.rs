//! Tauri shell: thin commands over `tidydrop-core`. No file logic lives here.

use serde::Serialize;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;
use tauri::{AppHandle, Emitter, Manager, State};
use tauri_plugin_opener::OpenerExt;
use tidydrop_core::ai::{self, AiStatus};
use tidydrop_core::apply::{self, ApplyReport};
use tidydrop_core::config::{default_downloads_dir, AppPaths, Config};
use tidydrop_core::journal::{self, JournalSummary};
use tidydrop_core::pipeline::{self, Prepared};
use tidydrop_core::plan::{ItemOverride, Plan};
use tidydrop_core::undo::{self, UndoReport};

struct AppState {
    paths: AppPaths,
    config: Mutex<Config>,
    plan: Mutex<Option<Plan>>,
    busy: AtomicBool,
}

/// Clears the busy flag when dropped, so a panic can't leave the app stuck.
struct BusyGuard<'a>(&'a AtomicBool);
impl Drop for BusyGuard<'_> {
    fn drop(&mut self) {
        self.0.store(false, Ordering::SeqCst);
    }
}

fn claim(state: &AppState) -> Result<BusyGuard<'_>, String> {
    if state.busy.swap(true, Ordering::SeqCst) {
        return Err(
            "tidydrop is already working on something. Please wait for it to finish.".into(),
        );
    }
    Ok(BusyGuard(&state.busy))
}

#[derive(Serialize)]
struct AppInfo {
    version: String,
    default_folder: Option<PathBuf>,
    config_file: PathBuf,
    journal_dir: PathBuf,
    os: String,
}

#[tauri::command]
fn app_info(state: State<'_, AppState>) -> AppInfo {
    AppInfo {
        version: tidydrop_core::version().into(),
        default_folder: default_downloads_dir(),
        config_file: state.paths.config_file.clone(),
        journal_dir: state.paths.journal_dir.clone(),
        os: std::env::consts::OS.into(),
    }
}

#[tauri::command]
fn get_config(state: State<'_, AppState>) -> Config {
    state.config.lock().unwrap().clone()
}

#[tauri::command]
fn save_config(state: State<'_, AppState>, config: Config) -> Result<Config, String> {
    let mut config = config;
    config.normalize();
    config.validate().map_err(|e| e.to_string())?;
    config
        .save(&state.paths.config_file)
        .map_err(|e| e.to_string())?;
    *state.config.lock().unwrap() = config.clone();
    // Category names may have changed; recompute destinations of a pending plan.
    if let Some(p) = state.plan.lock().unwrap().as_mut() {
        for item in &mut p.items {
            if config.category(&item.category).is_none() {
                item.category = tidydrop_core::config::OTHER.into();
            }
        }
        p.recompute(&config);
    }
    Ok(config)
}

#[tauri::command]
fn reset_config(state: State<'_, AppState>) -> Result<Config, String> {
    // keep UI theme preference
    let theme = state.config.lock().unwrap().theme.clone();
    let c = Config {
        theme,
        ..Default::default()
    };
    save_config(state, c)
}

#[tauri::command]
async fn ai_status(state: State<'_, AppState>) -> Result<AiStatus, String> {
    let cfg = state.config.lock().unwrap().ai.clone();
    tauri::async_runtime::spawn_blocking(move || ai::status(&cfg))
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn prepare(
    app: AppHandle,
    state: State<'_, AppState>,
    folder: String,
    use_ai: bool,
) -> Result<Prepared, String> {
    let _guard = claim(&state)?;
    let config = state.config.lock().unwrap().clone();
    let app2 = app.clone();
    let prepared = tauri::async_runtime::spawn_blocking(move || {
        pipeline::prepare(
            std::path::Path::new(&folder),
            &config,
            use_ai,
            &mut |stage| {
                let _ = app2.emit("tidydrop://progress", &stage);
            },
        )
    })
    .await
    .map_err(|e| e.to_string())?
    .map_err(|e| e.to_string())?;
    *state.plan.lock().unwrap() = Some(prepared.plan.clone());
    Ok(prepared)
}

#[tauri::command]
fn update_plan(
    state: State<'_, AppState>,
    overrides: Vec<ItemOverride>,
    organize_by_month: Option<bool>,
) -> Result<Plan, String> {
    let config = state.config.lock().unwrap().clone();
    let mut guard = state.plan.lock().unwrap();
    let plan = guard
        .as_mut()
        .ok_or("No preview yet. Scan a folder first.")?;
    if let Some(m) = organize_by_month {
        plan.set_organize_by_month(m, &config);
    }
    plan.apply_overrides(&overrides, &config);
    Ok(plan.clone())
}

#[tauri::command]
async fn apply_plan(app: AppHandle, state: State<'_, AppState>) -> Result<ApplyReport, String> {
    let _guard = claim(&state)?;
    let plan = state
        .plan
        .lock()
        .unwrap()
        .clone()
        .ok_or("No preview yet. Scan a folder first.")?;
    let config = state.config.lock().unwrap().clone();
    let jdir = state.paths.journal_dir.clone();
    let app2 = app.clone();
    let report = tauri::async_runtime::spawn_blocking(move || {
        apply::apply_plan(&plan, &config, &jdir, &mut |p| {
            let _ = app2.emit("tidydrop://apply-progress", &p);
        })
    })
    .await
    .map_err(|e| e.to_string())?
    .map_err(|e| e.to_string())?;
    // A plan is single-use: the folder has changed.
    *state.plan.lock().unwrap() = None;
    Ok(report)
}

#[tauri::command]
async fn undo_last(state: State<'_, AppState>, id: Option<String>) -> Result<UndoReport, String> {
    let _guard = claim(&state)?;
    let jdir = state.paths.journal_dir.clone();
    *state.plan.lock().unwrap() = None;
    tauri::async_runtime::spawn_blocking(move || match id {
        Some(id) => undo::undo_by_id(&jdir, &id),
        None => undo::undo_last(&jdir),
    })
    .await
    .map_err(|e| e.to_string())?
    .map_err(|e| e.to_string())
}

#[tauri::command]
async fn history(state: State<'_, AppState>) -> Result<Vec<JournalSummary>, String> {
    let jdir = state.paths.journal_dir.clone();
    tauri::async_runtime::spawn_blocking(move || journal::list(&jdir))
        .await
        .map_err(|e| e.to_string())?
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn open_folder(app: AppHandle, path: String) -> Result<(), String> {
    let p = PathBuf::from(&path);
    if !p.is_dir() {
        return Err(format!("{path} is not a folder"));
    }
    app.opener()
        .open_path(path, None::<&str>)
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn open_ollama_download(app: AppHandle) -> Result<(), String> {
    app.opener()
        .open_url("https://ollama.com/download", None::<&str>)
        .map_err(|e| e.to_string())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            let paths = AppPaths::detect();
            let config = Config::load(&paths.config_file).unwrap_or_else(|e| {
                eprintln!("config unreadable ({e}); using defaults");
                Config::default()
            });
            app.manage(AppState {
                paths,
                config: Mutex::new(config),
                plan: Mutex::new(None),
                busy: AtomicBool::new(false),
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            app_info,
            get_config,
            save_config,
            reset_config,
            ai_status,
            prepare,
            update_plan,
            apply_plan,
            undo_last,
            history,
            open_folder,
            open_ollama_download
        ])
        .run(tauri::generate_context!())
        .expect("error while running tidydrop");
}
