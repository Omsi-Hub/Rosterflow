//! The companies on disk: each a file of its own in the data folder (`companies/<id>.json`),
//! written whole through a file beside it, with a copy of every day it was changed kept in
//! `backups/<id>/` (the last `BACKUPS_KEPT`). A company deleted goes to `trash/` first. The
//! program's own settings (its language, the company opened last) are `settings.json`.

use anyhow::{Context, Result};
use rosterflow_core::company::{self as co, Company};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};

/// How many days' copies of a company are kept.
pub const BACKUPS_KEPT: usize = 30;
/// The environment variable that puts the data folder elsewhere (tests, a second copy).
pub const DATA_DIR_ENV: &str = "ROSTERFLOW_DATA";

fn dir_cell() -> &'static Mutex<Option<PathBuf>> {
    static D: OnceLock<Mutex<Option<PathBuf>>> = OnceLock::new();
    D.get_or_init(|| Mutex::new(None))
}

/// The data folder from now on (the app's own folder of its platform).
pub fn set_data_dir(p: PathBuf) {
    *dir_cell().lock().unwrap_or_else(|e| e.into_inner()) = Some(p);
}

/// The data folder: `ROSTERFLOW_DATA`, else the one set, else one of the home folder.
pub fn data_dir() -> PathBuf {
    if let Some(p) = std::env::var_os(DATA_DIR_ENV).filter(|p| !p.is_empty()) {
        return PathBuf::from(p);
    }
    if let Some(p) = dir_cell().lock().unwrap_or_else(|e| e.into_inner()).clone() {
        return p;
    }
    let home = std::env::var_os("HOME").or_else(|| std::env::var_os("USERPROFILE")).map(PathBuf::from).unwrap_or_else(std::env::temp_dir);
    home.join(".rosterflow")
}

thread_local! {
    /// A test's company changes stay in its session: nothing is written.
    static DRY: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

pub fn dry() -> bool {
    DRY.with(|d| d.get())
}

/// This thread writes nothing (a test).
pub fn set_dry(on: bool) {
    DRY.with(|d| d.set(on));
}

fn companies_dir() -> PathBuf {
    data_dir().join("companies")
}

/// The file of company `id` (None: no plain name).
pub fn path_of(id: &str) -> Option<PathBuf> {
    (!id.is_empty() && id.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')).then(|| companies_dir().join(format!("{id}.json")))
}

/// `text` written to `path` whole: through a file beside it, renamed over it.
fn write_whole(path: &Path, text: &str) -> Result<()> {
    if let Some(d) = path.parent() {
        std::fs::create_dir_all(d).with_context(|| format!("{}", d.display()))?;
    }
    let tmp = path.with_extension("json.tmp");
    std::fs::write(&tmp, text).with_context(|| format!("{}", tmp.display()))?;
    std::fs::rename(&tmp, path).with_context(|| format!("{}", path.display()))?;
    Ok(())
}

/// A company as it is saved: the file's text.
pub fn to_text(c: &Company) -> Result<String> {
    Ok(serde_json::to_string_pretty(c)?)
}

/// A company of a file's text (a file of another program refused).
pub fn from_text(text: &str) -> Result<Company> {
    let v: serde_json::Value = serde_json::from_str(text).context("not a RosterFlow file")?;
    if v.get("planning").is_none() || v.get("name").is_none() {
        anyhow::bail!("not a RosterFlow company file");
    }
    let c: Company = serde_json::from_value(v).context("not a RosterFlow company file")?;
    Ok(c)
}

/// Save `c` (and the day's copy of it).
pub fn save(c: &Company) -> Result<()> {
    let path = path_of(&c.id).context("the company has no file name")?;
    let text = to_text(c)?;
    write_whole(&path, &text)?;
    let day = co::dates::today();
    let backups = data_dir().join("backups").join(&c.id);
    write_whole(&backups.join(format!("{day}.json")), &text)?;
    prune_backups(&backups);
    Ok(())
}

fn prune_backups(dir: &Path) {
    let mut files: Vec<PathBuf> = std::fs::read_dir(dir).into_iter().flatten().flatten().map(|e| e.path()).filter(|p| p.extension().is_some_and(|x| x == "json")).collect();
    files.sort();
    let extra = files.len().saturating_sub(BACKUPS_KEPT);
    for p in files.into_iter().take(extra) {
        let _ = std::fs::remove_file(p);
    }
}

/// Company `id` read.
pub fn load(id: &str) -> Result<Company> {
    let path = path_of(id).context("no such company")?;
    let text = std::fs::read_to_string(&path).with_context(|| format!("{}", path.display()))?;
    let mut c = from_text(&text)?;
    c.id = id.to_string();
    Ok(c)
}

/// A company in the list of companies.
#[derive(Serialize, Clone, Debug, PartialEq)]
pub struct Entry {
    pub id: String,
    pub name: String,
    pub short: String,
    pub colours: [String; 2],
    pub lines: usize,
    pub buses: usize,
    pub drivers: usize,
    /// When its file was last written (Unix seconds).
    pub modified: u64,
}

/// Every company in the data folder, the one changed last first.
pub fn list() -> Vec<Entry> {
    let mut out: Vec<Entry> = std::fs::read_dir(companies_dir())
        .into_iter()
        .flatten()
        .flatten()
        .filter_map(|e| {
            let p = e.path();
            let id = p.file_name()?.to_str()?.strip_suffix(".json")?.to_string();
            let c = load(&id).ok()?;
            let modified = e.metadata().and_then(|m| m.modified()).ok().and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok()).map_or(0, |d| d.as_secs());
            Some(Entry {
                id,
                name: c.name.clone(),
                short: c.short.clone(),
                colours: c.colours.clone(),
                lines: c.network.lines.len(),
                buses: c.fleet.len(),
                drivers: c.staff.iter().filter(|e| co::staff::drives(e)).count(),
                modified,
            })
        })
        .collect();
    out.sort_by(|a, b| b.modified.cmp(&a.modified).then(a.name.cmp(&b.name)));
    out
}

/// An id for a new company of `name` no other company has.
pub fn free_id(name: &str) -> String {
    let base = co::slug(name);
    let taken = |id: &str| path_of(id).is_some_and(|p| p.exists());
    if !taken(&base) {
        return base;
    }
    (2..).map(|k| format!("{base}-{k}")).find(|id| !taken(id)).unwrap_or(base)
}

/// A new company, saved: its id.
pub fn create(f: &co::Founding) -> Result<Company> {
    if f.name.trim().is_empty() {
        anyhow::bail!("Give the company a name.");
    }
    let mut c = co::found(f, "");
    c.id = free_id(&c.name);
    if !dry() {
        save(&c)?;
    }
    Ok(c)
}

/// Company `id` taken out of the list: its file moved to `trash/`.
pub fn delete(id: &str) -> Result<()> {
    let path = path_of(id).context("no such company")?;
    let trash = data_dir().join("trash");
    std::fs::create_dir_all(&trash)?;
    let stamp = chrono::Local::now().format("%Y%m%d-%H%M%S");
    std::fs::rename(&path, trash.join(format!("{id}-{stamp}.json"))).with_context(|| format!("{}", path.display()))?;
    Ok(())
}

/// A company file from elsewhere (`text`) taken in as a company of its own: a new id when one of
/// that name is there already.
pub fn import(text: &str) -> Result<Company> {
    let mut c = from_text(text)?;
    c.id = free_id(&c.name);
    save(&c)?;
    Ok(c)
}

/// The days of company `id` a copy is kept of, the latest first.
pub fn backups(id: &str) -> Vec<String> {
    let mut v: Vec<String> =
        std::fs::read_dir(data_dir().join("backups").join(id)).into_iter().flatten().flatten().filter_map(|e| e.file_name().to_str()?.strip_suffix(".json").map(str::to_string)).collect();
    v.sort_by(|a, b| b.cmp(a));
    v
}

/// Company `id` as it was on `day` (its copy).
pub fn backup(id: &str, day: &str) -> Result<Company> {
    if !day.chars().all(|c| c.is_ascii_digit() || c == '-') {
        anyhow::bail!("no such copy");
    }
    let p = data_dir().join("backups").join(id).join(format!("{day}.json"));
    let mut c = from_text(&std::fs::read_to_string(&p).with_context(|| format!("{}", p.display()))?)?;
    c.id = id.to_string();
    Ok(c)
}

/// The program's own settings.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Default)]
#[serde(default)]
pub struct Settings {
    /// The language of the pages (ISO code; empty: the computer's).
    pub language: String,
    /// The company opened last (opened again at the start).
    pub last: String,
    /// "light", "dark" or "" (the system's).
    pub theme: String,
    /// The guided tour was offered.
    pub toured: bool,
}

pub fn settings() -> Settings {
    std::fs::read_to_string(data_dir().join("settings.json")).ok().and_then(|t| serde_json::from_str(&t).ok()).unwrap_or_default()
}

pub fn save_settings(s: &Settings) -> Result<()> {
    if dry() {
        return Ok(());
    }
    write_whole(&data_dir().join("settings.json"), &serde_json::to_string_pretty(s)?)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_company_is_saved_listed_backed_up_and_deleted() {
        let dir = std::env::temp_dir().join(format!("rosterflow-store-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        set_data_dir(dir.clone());
        let c = create(&co::Founding { name: "Stadsbus Test".into(), ..Default::default() }).unwrap();
        assert_eq!(c.id, "stadsbus-test");
        let c2 = create(&co::Founding { name: "Stadsbus Test".into(), ..Default::default() }).unwrap();
        assert_eq!(c2.id, "stadsbus-test-2");
        assert_eq!(list().len(), 2);
        assert_eq!(load("stadsbus-test").unwrap().name, "Stadsbus Test");
        assert_eq!(backups("stadsbus-test"), vec![co::dates::today()]);
        assert!(backup("stadsbus-test", &co::dates::today()).is_ok());
        let imported = import(&to_text(&c).unwrap()).unwrap();
        assert_eq!(imported.id, "stadsbus-test-3");
        assert!(from_text("{\"a\": 1}").is_err());
        delete("stadsbus-test").unwrap();
        assert_eq!(list().len(), 2);
        assert!(path_of("../x").is_none());
        let _ = std::fs::remove_dir_all(&dir);
    }
}
