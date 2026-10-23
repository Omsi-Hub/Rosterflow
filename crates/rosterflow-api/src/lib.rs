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

#[cfg(test)]
mod tests {
    /// The texts `tr("…")`, `trf("…")` (Rust) and `tr('…')` (the page) of `files` say.
    fn literals(files: &[std::path::PathBuf]) -> Vec<(String, String)> {
        let mut out = Vec::new();
        for f in files {
            let src = std::fs::read_to_string(f).unwrap();
            let name = f.file_name().unwrap().to_string_lossy().to_string();
            for (lead, quote) in [("tr(\"", '"'), ("trf(\"", '"'), ("tr('", '\'')] {
                let mut rest = src.as_str();
                while let Some(k) = rest.find(lead) {
                    let after = &rest[k + lead.len()..];
                    rest = after;
                    let mut text = String::new();
                    let mut chars = after.chars();
                    while let Some(c) = chars.next() {
                        match c {
                            '\\' => text.extend(chars.next()),
                            c if c == quote => break,
                            c => text.push(c),
                        }
                    }
                    out.push((name.clone(), text));
                }
            }
        }
        out
    }

    #[test]
    fn every_text_of_the_program_and_its_pages_is_translated() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
        let mut files = vec![root.join("src/app.rs"), root.join("src/master.rs"), root.join("src/company/kit.rs"), root.join("../../ui/js/app.js"), root.join("../../ui/js/company/dutycard.js")];
        for dir in ["../../ui/js/pages", "../../ui/js/rosterflow"] {
            for e in std::fs::read_dir(root.join(dir)).unwrap().flatten() {
                files.push(e.path());
            }
        }
        let texts = literals(&files);
        assert!(texts.len() > 300, "only {} texts found", texts.len());
        let mut missing = Vec::new();
        for lang in ["nl", "de", "fr", "pl", "ru", "uk"] {
            let table = crate::locales::texts(lang);
            for (file, t) in &texts {
                if !t.is_empty() && !table.contains_key(t) {
                    missing.push(format!("{file} {lang}: {t:?}"));
                }
            }
        }
        missing.sort();
        missing.dedup();
        assert!(missing.is_empty(), "{} missing:\n{}", missing.len(), missing.join("\n"));
        assert_eq!(crate::locales::texts("nl").get("Overview").map(String::as_str), Some("Overzicht"));
    }
}
