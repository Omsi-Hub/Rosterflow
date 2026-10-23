//! The program and its companies (`app_`): what the page starts with (`app_boot`), its texts in
//! the planner's language (`app_texts`), its settings, and the companies - made, opened, closed,
//! deleted, exported to a file and imported from one, put back as a day's copy had them - and the
//! open company's dashboard (`app_summary`).

use crate::company::words::{day_label, tr, trf};
use crate::company::{self, Session};
use crate::{store, Answer, Platform};
use rosterflow_core::company::{self as co, plan as pl};
use serde_json::{json, Value};

pub const VERSION: &str = env!("CARGO_PKG_VERSION");

fn s_arg(args: &Value, name: &str) -> String {
    args.get(name).and_then(Value::as_str).unwrap_or("").trim().to_string()
}

pub fn call(cmd: &str, args: &Value, p: &dyn Platform) -> Answer {
    match cmd {
        "app_boot" => Ok(boot()),
        "app_texts" => Ok(json!(crate::locales::texts(&s_arg(args, "lang")))),
        "app_settings" => settings(args),
        "app_companies" => Ok(json!(store::list())),
        "app_create" => create(args),
        "app_open" => open(&s_arg(args, "id")),
        "app_close" => {
            company::with(|s| company::close(s));
            remember_last("");
            Ok(json!({}))
        }
        "app_delete" => {
            let id = s_arg(args, "id");
            company::with(|s| {
                if s.company.as_ref().is_some_and(|c| c.id == id) {
                    company::close(s);
                }
            });
            store::delete(&id).map_err(|e| format!("{e:#}"))?;
            Ok(json!({ "companies": store::list() }))
        }
        "app_export" => export(p),
        "app_import" => import(args, p),
        "app_backups" => Ok(company::with(|s| json!(s.company.as_ref().map(|c| store::backups(&c.id)).unwrap_or_default()))),
        "app_restore" => restore(&s_arg(args, "day")),
        "app_summary" => Ok(company::with(summary)),
        "app_shell" => Ok(company::with(|s| match &s.company {
            Some(c) => json!({ "gen": s.generation, "id": c.id, "name": c.name, "short": c.short, "colours": c.colours, "money": crate::company::words::money_json(), "notes": s.take_notes() }),
            None => json!({ "gen": s.generation, "id": null }),
        })),
        "app_rename" => company::with(|s| {
            let name = s_arg(args, "name");
            if name.is_empty() {
                return Err(tr("Give the company a name."));
            }
            company::act(s, |c| {
                c.name = name.clone();
                Ok(())
            })
            .map_err(|p| p.to_string())?;
            Ok(json!({}))
        }),
        _ => Err(format!("unknown command: {cmd}")),
    }
}

/// What the page starts with: the program's version and settings, the languages, the companies,
/// the one opened (the one opened last is opened again).
fn boot() -> Value {
    let set = store::settings();
    if !set.last.is_empty() && company::with(|s| s.company.is_none()) {
        if let Ok(c) = store::load(&set.last) {
            company::with(|s| company::open(s, c));
        }
    }
    let open = company::with(|s| s.company.as_ref().map(|c| json!({ "id": c.id, "name": c.name, "short": c.short, "colours": c.colours })));
    json!({
        "version": VERSION,
        "settings": set,
        "languages": crate::locales::LANGUAGES.iter().map(|(k, n)| json!([k, n])).collect::<Vec<_>>(),
        "countries": rosterflow_core::calendar::COUNTRIES.iter().map(|(k, n)| json!([k, tr(n)])).collect::<Vec<_>>(),
        "currencies": crate::company::words::CURRENCIES.iter().map(|c| json!([c.0, format!("{} ({})", c.0, c.1)])).collect::<Vec<_>>(),
        "companies": store::list(),
        "open": open,
        "data": store::data_dir().to_string_lossy(),
        "today": crate::company::words::today(),
    })
}

fn remember_last(id: &str) {
    let mut set = store::settings();
    if set.last != id {
        set.last = id.to_string();
        let _ = store::save_settings(&set);
    }
}

/// The program's settings changed (`patch`: language, theme, toured).
fn settings(args: &Value) -> Answer {
    let mut set = store::settings();
    if let Some(p) = args.get("patch") {
        if let Some(l) = p.get("language").and_then(Value::as_str) {
            set.language = l.to_string();
        }
        if let Some(t) = p.get("theme").and_then(Value::as_str) {
            set.theme = t.to_string();
        }
        if let Some(t) = p.get("toured").and_then(Value::as_bool) {
            set.toured = t;
        }
        store::save_settings(&set).map_err(|e| format!("{e:#}"))?;
    }
    Ok(json!(set))
}

/// A new company (`demo`: with an example network, fleet and staff to try everything on), opened.
fn create(args: &Value) -> Answer {
    let f = co::Founding {
        name: s_arg(args, "name"),
        short: s_arg(args, "short"),
        colours: [s_arg(args, "colour"), s_arg(args, "colour2")],
        depot: s_arg(args, "depot"),
        date: String::new(),
        country: s_arg(args, "country"),
        currency: s_arg(args, "currency"),
    };
    if f.name.is_empty() {
        return Err(tr("Give the company a name."));
    }
    let mut c = store::create(&f).map_err(|e| tr(&format!("{e:#}")))?;
    if args.get("demo").and_then(Value::as_bool).unwrap_or(false) {
        crate::demo::fill(&mut c);
        store::save(&c).map_err(|e| format!("{e:#}"))?;
    }
    let id = c.id.clone();
    company::with(|s| company::open(s, c));
    remember_last(&id);
    Ok(json!({ "id": id }))
}

fn open(id: &str) -> Answer {
    let c = store::load(id).map_err(|e| format!("{}: {e:#}", tr("The company could not be opened")))?;
    company::with(|s| company::open(s, c));
    remember_last(id);
    Ok(json!({ "id": id }))
}

/// The open company saved as a file where the planner chooses.
fn export(p: &dyn Platform) -> Answer {
    let (name, text) = company::with(|s| {
        let c = s.company.as_ref().ok_or_else(|| tr("No company is open."))?;
        Ok::<_, String>((format!("{}.rosterflow.json", c.id), store::to_text(c).map_err(|e| e.to_string())?))
    })?;
    let Some(path) = p.save_path(&tr("Save the company as a file"), &name, "RosterFlow", &["json"]) else { return Ok(json!({})) };
    std::fs::write(&path, text).map_err(|e| format!("{}: {e}", tr("Could not save the file")))?;
    Ok(json!({ "say": { "text": trf("Saved: %{file}", &[("file", &path.display().to_string())]), "error": false } }))
}

/// A company file taken in (`text`, else one the planner chooses), opened.
fn import(args: &Value, p: &dyn Platform) -> Answer {
    let text = match args.get("text").and_then(Value::as_str) {
        Some(t) => t.to_string(),
        None => {
            let Some(path) = p.open_path(&tr("Open a company file"), "RosterFlow", &["json"]) else { return Ok(json!({})) };
            std::fs::read_to_string(&path).map_err(|e| format!("{}: {e}", path.display()))?
        }
    };
    let c = store::import(&text).map_err(|e| format!("{}: {e:#}", tr("Not a RosterFlow company file")))?;
    let id = c.id.clone();
    company::with(|s| company::open(s, c));
    remember_last(&id);
    Ok(json!({ "id": id }))
}

/// The open company put back as its copy of `day` had it (the company as it is now kept as a
/// copy of its own first).
fn restore(day: &str) -> Answer {
    company::with(|s| {
        let c = s.company.as_ref().ok_or_else(|| tr("No company is open."))?;
        let back = store::backup(&c.id, day).map_err(|e| format!("{e:#}"))?;
        store::keep_copy(c).map_err(|e| format!("{e:#}"))?;
        s.company = Some(back);
        company::reload_timetable(s);
        let c = s.company.clone().unwrap();
        company::open(s, c);
        Ok(json!({ "say": { "text": trf("The company is back as it was on %{day}.", &[("day", &day_label(day))]), "error": false } }))
    })
}

/// The open company's dashboard: what it has, today's and the next days' planning, what needs
/// doing.
fn summary(s: &mut Session) -> Value {
    let Some(c) = s.company.clone() else { return json!({ "none": true }) };
    let drivers = c.staff.iter().filter(|e| co::staff::drives(e) && e.employed_on(&c.date)).count();
    let absent = c.staff.iter().filter(|e| co::staff::drives(e) && e.employed_on(&c.date) && e.absent(&c.date)).count();
    let buses = c.fleet.iter().filter(|v| v.held_on(&c.date)).count();
    let workshop = c.fleet.iter().filter(|v| v.held_on(&c.date) && v.in_workshop(&c.date)).count();
    let lines = c.network.lines.len();
    let runnable = c.network.lines.iter().filter(|l| rosterflow_core::lines::written(l)).count();
    let mut days = Vec::new();
    for k in 0..7 {
        let date = co::dates::add(&c.date, k);
        let p = company::plan_of(s, &date).and_then(Result::ok);
        let (tours, covered, open, no_bus) = p.as_ref().map(pl::DayPlan::counts).unwrap_or_default();
        days.push(json!({ "date": date, "label": day_label(&date), "tours": tours, "covered": covered, "open": open, "no_bus": no_bus }));
    }
    let mut todo: Vec<Value> = Vec::new();
    let mut say = |text: String, go: &str, tone: &str| todo.push(json!({ "text": text, "go": go, "tone": tone }));
    if c.network.stops.is_empty() {
        say(tr("Enter the stops of your network."), "stops", "warn");
    }
    if lines == 0 {
        say(tr("Make your first line: its stops in each direction and the minutes between them."), "lines", "warn");
    } else if runnable < lines {
        say(trf("%{n} lines are not complete yet (stops or departures missing).", &[("n", &(lines - runnable).to_string())]), "lines", "warn");
    }
    if buses == 0 {
        say(tr("Enter your buses."), "fleet", "warn");
    }
    if drivers == 0 {
        say(tr("Enter your drivers."), "staff", "warn");
    }
    let waiting = co::holidays::waiting(&c).len();
    if waiting > 0 {
        say(trf("%{n} holiday requests wait for an answer.", &[("n", &waiting.to_string())]), "leave", "warn");
    }
    if let Some(d) = days.first() {
        let open = d["open"].as_u64().unwrap_or(0) + d["no_bus"].as_u64().unwrap_or(0);
        if open > 0 {
            say(trf("Today %{n} duties or tours are still open.", &[("n", &open.to_string())]), "roster", "err");
        }
    }
    json!({
        "name": c.name, "short": c.short, "colours": c.colours, "date": c.date, "today": day_label(&c.date),
        "counts": { "stops": c.network.stops.len(), "lines": lines, "buses": buses, "workshop": workshop, "drivers": drivers, "absent": absent },
        "days": days, "todo": todo,
        "draft": co::rosterflow::has_draft(&c),
        "save_error": s.save_error,
    })
}
