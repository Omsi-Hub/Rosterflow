//! The company open in the program: its session (`with`), the change of it saved to its file
//! (`changed`), and the day's timetables and plans worked out from it (`lines_of`, `plan_of`).

pub mod kit;
pub mod words;

use rosterflow_core::company::{self as co, Company};
use rosterflow_core::LineInfo;
use serde_json::{json, Value};
use std::sync::{Arc, Mutex, OnceLock};
use words::tr;

pub use rosterflow_core::company::{found, Founding};

/// The open company and what is worked out from it.
#[derive(Default)]
pub struct Session {
    pub company: Option<Company>,
    /// Raised whenever the company changed: the page asks for its view again, the day's
    /// timetables and plans are worked out anew.
    pub generation: u64,
    /// The timetable of a date, worked out at a generation.
    lines: Vec<(String, u64, Arc<Vec<LineInfo>>)>,
    /// The plan of a date, worked out at a generation.
    plans: Vec<(String, u64, co::plan::DayPlan)>,
    /// What went wrong saving the company last (None: it is saved).
    pub save_error: Option<String>,
    /// What the program has to tell the planner (`say`), until the page takes it.
    notes: Vec<(String, bool)>,
    /// The tests' timetable of every day (`test_week`), instead of the network's.
    fixed: Option<Arc<Vec<LineInfo>>>,
}

impl Session {
    /// A session of `c`, not saved anywhere (the tests', a lesson's).
    pub fn of(c: Company) -> Session {
        Session { company: Some(c), ..Default::default() }
    }

    /// Something to tell the planner (`warn`: a warning) with the page's next view.
    pub fn say(&mut self, text: String, warn: bool) {
        self.notes.push((text, warn));
    }

    /// What there is to tell, taken: `[{text, warn}]`.
    pub fn take_notes(&mut self) -> Value {
        let mut v: Vec<Value> = std::mem::take(&mut self.notes).into_iter().map(|(text, warn)| json!({ "text": text, "warn": warn })).collect();
        if let Some(e) = &self.save_error {
            v.push(json!({ "text": format!("{}: {e}", tr("The company could not be saved")), "warn": true }));
        }
        Value::Array(v)
    }
}

fn session() -> &'static Mutex<Session> {
    static S: OnceLock<Mutex<Session>> = OnceLock::new();
    S.get_or_init(|| Mutex::new(Session::default()))
}

/// Run `f` on the session: the company's day made today's (the planning's "today" is the
/// computer's), money written in its currency.
pub fn with<T>(f: impl FnOnce(&mut Session) -> T) -> T {
    let mut s = session().lock().unwrap_or_else(|e| e.into_inner());
    let today = words::today();
    if let Some(c) = s.company.as_mut() {
        if c.date != today {
            c.date = today;
            s.generation += 1;
        }
    }
    words::set_currency(s.company.as_ref().map(|c| c.currency.as_str()).unwrap_or("EUR"));
    f(&mut s)
}

/// After a change: the company saved, and what is worked out from it made again.
pub fn changed(s: &mut Session) {
    s.generation += 1;
    if let Some(c) = s.company.as_mut() {
        let today = c.date.clone();
        rosterflow_core::timetable::sync_lines(c, &today);
    }
    if let Some(c) = s.company.as_ref() {
        if !crate::store::dry() {
            s.save_error = crate::store::save(c).err().map(|e| format!("{e:#}"));
        }
    }
}

/// The timetable changed (a line's design or its Timeplan): every day worked out anew.
pub fn reload_timetable(s: &mut Session) {
    s.lines.clear();
    s.plans.clear();
    changed(s);
}

/// Do something to the company; its refusal comes back as the popup that says why.
pub fn act<T>(s: &mut Session, f: impl FnOnce(&mut Company) -> Result<T, &'static str>) -> Result<T, Value> {
    let Some(c) = s.company.as_mut() else { return Err(json!(null)) };
    match f(c) {
        Ok(v) => {
            changed(s);
            Ok(v)
        }
        Err(e) => Err(refuse(s, e)),
    }
}

/// A refusal as its popup.
pub fn refuse(_s: &Session, reason: &str) -> Value {
    kit::Popup::new("error", "This cannot be done", tr(reason), "", None).json()
}

/// A popup of the session.
pub fn popup(_s: &Session, p: kit::Popup) -> Value {
    p.json()
}

/// The timetable of `date` (worked out once a generation).
pub fn lines_arc(s: &mut Session, date: &str) -> Option<Arc<Vec<LineInfo>>> {
    let c = s.company.as_ref()?;
    if let Some(f) = &s.fixed {
        return Some(f.clone());
    }
    let g = s.generation;
    if let Some(x) = s.lines.iter().find(|x| x.0 == date && x.1 == g) {
        return Some(x.2.clone());
    }
    let lines = Arc::new(rosterflow_core::timetable::lines_on(c, date));
    s.lines.retain(|x| x.1 == g && x.0 != date);
    s.lines.push((date.to_string(), g, lines.clone()));
    Some(lines)
}

/// The timetable of `date`.
pub fn lines_of(s: &Session, date: &str) -> Option<Vec<LineInfo>> {
    let c = s.company.as_ref()?;
    if let Some(f) = &s.fixed {
        return Some(f.as_ref().clone());
    }
    if let Some(x) = s.lines.iter().find(|x| x.0 == date && x.1 == s.generation) {
        return Some(x.2.as_ref().clone());
    }
    Some(rosterflow_core::timetable::lines_on(c, date))
}

/// The timetable of `date` lent to `f`.
pub fn with_lines_of<T>(s: &Session, date: &str, f: impl FnOnce(&[LineInfo]) -> T) -> Option<T> {
    lines_of(s, date).map(|l| f(&l))
}

/// Tests: every day's timetable `lines`, whatever the network says.
#[cfg(test)]
pub(crate) fn test_week(s: &mut Session, lines: Vec<LineInfo>) {
    s.fixed = Some(Arc::new(lines));
    s.lines.clear();
    s.plans.clear();
}

/// The week's timetables (they are worked out when asked: nothing to read ahead).
pub fn week_work(_s: &mut Session) {}

/// The plan of `date`: its tours given buses and drivers by the roster and the rules, each
/// driver's days of the week and the night's rest after the day before counted in.
pub fn plan_of(s: &mut Session, date: &str) -> Option<Result<co::plan::DayPlan, String>> {
    s.company.as_ref()?;
    let g = s.generation;
    if let Some(p) = s.plans.iter().find(|x| x.0 == date && x.1 == g) {
        return Some(Ok(p.2.clone()));
    }
    // (the day before: when each driver's last duty ended, for their night's rest)
    let before = co::dates::add(date, -1);
    let prev = {
        let lines = lines_arc(s, &before)?;
        let c = s.company.as_ref()?;
        let p = co::plan::prepared(c, &before, &Default::default());
        let tours = co::network::tours_of_day(&p, &lines, &before);
        co::plan::last_ends(&co::plan::day_plan(&p, &before, tours))
    };
    let lines = lines_arc(s, date)?;
    let c = s.company.as_ref()?;
    let p = co::plan::prepared(c, date, &prev);
    let tours = co::network::tours_of_day(&p, &lines, date);
    let plan = co::plan::day_plan(&p, date, tours);
    s.plans.retain(|x| x.1 == g && x.0 != date);
    s.plans.push((date.to_string(), g, plan.clone()));
    Some(Ok(plan))
}

/// The company `c` opened in the session (another one closed).
pub fn open(s: &mut Session, mut c: Company) {
    c.date = words::today();
    let today = c.date.clone();
    rosterflow_core::timetable::sync_lines(&mut c, &today);
    s.company = Some(c);
    s.lines.clear();
    s.plans.clear();
    s.generation += 1;
    s.save_error = None;
}

/// The company closed.
pub fn close(s: &mut Session) {
    s.company = None;
    s.lines.clear();
    s.plans.clear();
    s.generation += 1;
}

/// A label for a spare bus (the roster's own words).
pub fn spare_label(bus: &str, from: &str, until: Option<i32>) -> String {
    match until {
        Some(u) => format!("{bus} {from}–{}", co::rosterflow::clock(u)),
        None => format!("{bus} {from}"),
    }
}
