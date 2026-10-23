//! RosterFlow, the desktop app: a Tauri 2 window with the page in `ui/`, and the commands the
//! page calls.
//!
//! * `call {cmd, args}`, `call_bytes {cmd, args}` - `rosterflow-api`'s commands, run off the main
//!   thread;
//! * `pick_folder`, `pick_file` - the system's pickers; `open_url`, `open_path` - the system's
//!   browser and file manager.
//!
//! The companies are kept in the app's data folder (`%APPDATA%\app.rosterflow.desktop` on
//! Windows, `~/Library/Application Support/app.rosterflow.desktop` on macOS), unless
//! `ROSTERFLOW_DATA` names another. Files the planner saves or opens (a company file, a
//! spreadsheet, a print) are chosen with the system's own dialogs.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use serde::Deserialize;
use serde_json::Value;
use std::path::PathBuf;
use tauri::{AppHandle, Emitter, Manager};
use tauri_plugin_dialog::DialogExt;
use tauri_plugin_opener::OpenerExt;

/// The commands' view of the app: its events, and the system's dialogs for the files the planner
/// saves and opens.
struct App(AppHandle);

impl rosterflow_api::Platform for App {
    fn emit(&self, event: &str, payload: Value) {
        let _ = self.0.emit(event, payload);
    }

    fn save_path(&self, title: &str, name: &str, kind: &str, exts: &[&str]) -> Option<PathBuf> {
        let mut d = self.0.dialog().file().set_title(title).set_file_name(name);
        if !exts.is_empty() {
            d = d.add_filter(kind, exts);
        }
        d.blocking_save_file().and_then(|p| p.into_path().ok())
    }

    fn open_path(&self, title: &str, kind: &str, exts: &[&str]) -> Option<PathBuf> {
        let mut d = self.0.dialog().file().set_title(title);
        if !exts.is_empty() {
            d = d.add_filter(kind, exts);
        }
        d.blocking_pick_file().and_then(|p| p.into_path().ok())
    }
}

/// A command slower than this is written in the log (ms).
const SLOW_MS: u128 = 500;

#[tauri::command]
async fn call(app: AppHandle, cmd: String, args: Option<Value>) -> Result<Value, String> {
    let args = args.unwrap_or(Value::Null);
    tauri::async_runtime::spawn_blocking(move || {
        let t0 = std::time::Instant::now();
        let answer = rosterflow_api::call(&cmd, &args, &App(app));
        let ms = t0.elapsed().as_millis();
        if ms >= SLOW_MS {
            rosterflow_api::files::log(&format!("slow: {cmd} took {ms} ms"));
        }
        answer
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
async fn call_bytes(app: AppHandle, cmd: String, args: Option<Value>) -> Result<tauri::ipc::Response, String> {
    let args = args.unwrap_or(Value::Null);
    let bytes = tauri::async_runtime::spawn_blocking(move || rosterflow_api::call_bytes(&cmd, &args, &App(app))).await.map_err(|e| e.to_string())??;
    Ok(tauri::ipc::Response::new(bytes))
}

#[tauri::command]
async fn pick_folder(app: AppHandle, title: Option<String>) -> Option<String> {
    let mut d = app.dialog().file();
    if let Some(t) = title.filter(|t| !t.is_empty()) {
        d = d.set_title(t);
    }
    d.blocking_pick_folder().map(|p| p.to_string())
}

#[derive(Deserialize)]
struct Filter {
    name: String,
    extensions: Vec<String>,
}

#[tauri::command]
async fn pick_file(app: AppHandle, title: Option<String>, filters: Option<Vec<Filter>>) -> Option<String> {
    let mut d = app.dialog().file();
    if let Some(t) = title.filter(|t| !t.is_empty()) {
        d = d.set_title(t);
    }
    for f in filters.unwrap_or_default() {
        let ext: Vec<&str> = f.extensions.iter().map(String::as_str).collect();
        d = d.add_filter(f.name, &ext);
    }
    d.blocking_pick_file().map(|p| p.to_string())
}

#[tauri::command]
fn open_url(app: AppHandle, url: String) -> Result<(), String> {
    // (web pages and mail only: never a file or a program by way of a URL)
    if !(url.starts_with("https://") || url.starts_with("http://") || url.starts_with("mailto:")) {
        return Err(format!("not a web address: {url}"));
    }
    app.opener().open_url(url, None::<&str>).map_err(|e| e.to_string())
}

#[tauri::command]
fn open_path(app: AppHandle, path: String) -> Result<(), String> {
    if !std::path::Path::new(&path).exists() {
        return Err(format!("{path} is not there"));
    }
    app.opener().open_path(path, None::<&str>).map_err(|e| e.to_string())
}

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![call, call_bytes, pick_folder, pick_file, open_url, open_path])
        .setup(|app| {
            // (the companies in the app's own data folder: `ROSTERFLOW_DATA` still goes first)
            if let Ok(dir) = app.path().app_data_dir() {
                rosterflow_api::store::set_data_dir(dir);
            }
            tauri::WebviewWindowBuilder::new(app, "main", tauri::WebviewUrl::App("index.html".into()))
                .title("RosterFlow")
                .inner_size(1440.0, 900.0)
                .min_inner_size(1100.0, 700.0)
                .center()
                .build()?;
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("RosterFlow could not start");
}
