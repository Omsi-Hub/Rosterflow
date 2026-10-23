//! RosterFlow's commands: what the page asks (`call(cmd, args)`, JSON in and out) routed by
//! prefix - the program and its companies (`app_`), the company's master data (`md_`) and the
//! planning itself (`rf_`). The desktop app hands every call of the page here; nothing here
//! knows of the window around it.

pub mod app;
pub mod company;
pub mod csv;
pub mod demo;
pub mod files;
pub mod locales;
pub mod master;
pub mod rosterflow;
pub mod store;

use serde_json::Value;

/// A command's answer: its JSON, or the words of what went wrong.
pub type Answer = Result<Value, String>;

/// What the commands need of the app around them (the window, or a test).
pub trait Platform: Send + Sync {
    /// Tell the page something, with a JSON payload.
    fn emit(&self, _event: &str, _payload: Value) {}
    /// Where to save a file the planner saves (`name` proposed, of `kind` with `exts`): the
    /// window's save dialog; None: cancelled. Without a window: the data folder's `exports`.
    fn save_path(&self, _title: &str, name: &str, _kind: &str, _exts: &[&str]) -> Option<std::path::PathBuf> {
        files::export_path(name)
    }
    /// A file to open (`kind` with `exts`): the window's open dialog; None: none chosen.
    fn open_path(&self, _title: &str, _kind: &str, _exts: &[&str]) -> Option<std::path::PathBuf> {
        None
    }
}

/// A platform that does nothing (tests).
pub struct Quiet;

impl Platform for Quiet {}

/// One argument of `args` (`args.name`), read as `T`; a missing one reads as `null`.
pub fn arg<T: serde::de::DeserializeOwned>(args: &Value, name: &str) -> Result<T, String> {
    serde_json::from_value(args.get(name).cloned().unwrap_or(Value::Null)).map_err(|e| format!("{name}: {e}"))
}

/// Any value as an answer.
pub fn to_json<T: serde::Serialize>(v: T) -> Answer {
    serde_json::to_value(v).map_err(|e| e.to_string())
}

/// A command of the page.
pub fn call(cmd: &str, args: &Value, p: &dyn Platform) -> Answer {
    company::words::set_language(args.get("lang").and_then(Value::as_str).unwrap_or(""));
    match cmd {
        c if c.starts_with("app_") => app::call(c, args, p),
        c if c.starts_with("md_") => master::call(c, args, p),
        c if c.starts_with("rf_") => rosterflow::call(c, args, p),
        _ => Err(format!("unknown command: {cmd}")),
    }
}

/// A command that answers bytes (a file).
pub fn call_bytes(cmd: &str, args: &Value, p: &dyn Platform) -> Result<Vec<u8>, String> {
    match cmd {
        c if c.starts_with("rf_") => rosterflow::call_bytes(c, args, p),
        c if c.starts_with("md_") => master::call_bytes(c, args, p),
        _ => Err(format!("unknown command: {cmd}")),
    }
}
