//! The company's master data (`md_`): the company itself and its rules, its stops, its lines
//! (their stops in each direction, the minutes between them, the days they run and how often),
//! its buses, its people, their absences and holiday requests, the agency drivers booked - each
//! listed, saved whole as the page edited it, taken out, and read from or written to a
//! spreadsheet (`md_import`, `md_export`).
//!
//! A form hands back the record as it was given with its fields changed: what the planner does
//! not edit (an id, a person's holiday requests) is kept as the company has it.

use crate::company::kit::{Go, Popup};
use crate::company::words::{day_label, eur, tr, trf, typed_money};
use crate::company::{self, Session};
use crate::csv::{self, Table};
use crate::{arg, Answer, Platform};
use rosterflow_core::calendar::{HolidayPeriod, ServiceKind, COUNTRIES};
use rosterflow_core::company::{self as co, agency, holidays as hol, licences, plan as pl, BusSize, Company, Drive, Employee, Licence, Role, ShiftPref, Vehicle};
use rosterflow_core::lines::{self as reg, LineDesign, Stop, StopRef};
use serde_json::{json, Value};

fn s_arg(args: &Value, name: &str) -> String {
    args.get(name).and_then(Value::as_str).unwrap_or("").trim().to_string()
}

fn n_arg(args: &Value, name: &str) -> u64 {
    args.get(name).and_then(|v| v.as_u64().or_else(|| v.as_str().and_then(|s| s.trim().parse().ok()))).unwrap_or(0)
}

/// What a command that changed something answers: done, with what to tell.
fn done(text: impl Into<String>) -> Value {
    json!({ "ok": true, "say": { "text": text.into(), "error": false } })
}

/// A refusal in the planner's words.
fn no(text: &str) -> String {
    tr(text)
}

/// The open company, or the words that none is.
fn need(s: &Session) -> Result<&Company, String> {
    s.company.as_ref().ok_or_else(|| tr("No company is open."))
}

/// Change the company (`f` refuses with an English text, said in the planner's language): saved
/// and its timetables worked out anew when it changed.
fn change<T>(s: &mut Session, timetable: bool, f: impl FnOnce(&mut Company) -> Result<T, String>) -> Result<T, String> {
    let c = s.company.as_mut().ok_or_else(|| tr("No company is open."))?;
    let v = f(c)?;
    if timetable {
        company::reload_timetable(s);
    } else {
        company::changed(s);
    }
    Ok(v)
}

pub fn call(cmd: &str, args: &Value, p: &dyn Platform) -> Answer {
    company::with(|s| match cmd {
        "md_options" => Ok(options()),
        "md_company" => company_view(s),
        "md_company_set" => company_set(s, args),
        "md_stops" => stops_view(s),
        "md_stop_save" => stop_save(s, args),
        "md_stop_delete" => stop_delete(s, n_arg(args, "id") as i64),
        "md_lines" => lines_view(s),
        "md_line" => line_view(s, n_arg(args, "id")),
        "md_line_new" => line_new(s, args),
        "md_line_save" => line_save(s, args),
        "md_line_delete" => line_delete(s, n_arg(args, "id")),
        "md_line_copy" => line_copy(s, n_arg(args, "id")),
        "md_fleet" => fleet_view(s),
        "md_vehicle_save" => vehicle_save(s, args),
        "md_vehicle_delete" => vehicle_delete(s, n_arg(args, "id") as u32),
        "md_staff" => staff_view(s),
        "md_employee_save" => employee_save(s, args),
        "md_employee_delete" => employee_delete(s, n_arg(args, "id") as u32),
        "md_absence" => absence(s, args),
        "md_leave" => leave_view(s),
        "md_leave_add" => leave_add(s, args),
        "md_leave_answer" => leave_answer(s, args),
        "md_leave_remove" => {
            let no = n_arg(args, "no") as u32;
            change(s, false, |c| if hol::remove_request(c, no) { Ok(()) } else { Err(no_such()) })?;
            Ok(done(tr("The request is taken out.")))
        }
        "md_agency" => agency_view(s),
        "md_agency_book" => agency_book(s, args),
        "md_agency_cancel" => agency_cancel(s, n_arg(args, "no") as u32),
        "md_export" => export(s, &s_arg(args, "what"), p),
        "md_import" => import(s, args, p),
        _ => Err(format!("unknown command: {cmd}")),
    })
}

pub fn call_bytes(cmd: &str, _args: &Value, _p: &dyn Platform) -> Result<Vec<u8>, String> {
    Err(format!("unknown command: {cmd}"))
}

fn no_such() -> String {
    tr("It is no longer there: the list changed meanwhile.")
}

// --- the choices of the forms ------------------------------------------------------------------

/// Every choice the forms offer, with its words.
fn options() -> Value {
    let pairs = |v: Vec<(String, &str)>| v.into_iter().map(|(k, l)| json!([k, tr(l)])).collect::<Vec<_>>();
    let key = |v: Value| v.as_str().unwrap_or("").to_string();
    json!({
        "sizes": pairs(BusSize::ALL.iter().map(|x| (key(json!(x)), x.label())).collect()),
        "drives": pairs(Drive::ALL.iter().map(|x| (key(json!(x)), x.label())).collect()),
        "roles": pairs(Role::ALL.iter().map(|x| (key(json!(x)), x.label())).collect()),
        "endorsements": pairs(licences::Endorsement::ALL.iter().map(|x| (key(json!(x)), x.label())).collect()),
        "licences": [["D", tr("D (every bus)")], ["D1", tr("D1 (small buses)")]],
        "shifts": [["any", tr("Early or late")], ["early", tr("Early duties only")], ["late", tr("Late duties only")]],
        "services": ServiceKind::ALL.iter().map(|x| json!([key(json!(x)), tr(x.label()), tr(x.note())])).collect::<Vec<_>>(),
        "reasons": pairs(hol::Reason::ALL.iter().map(|x| (key(json!(x)), x.label())).collect()),
        "day_groups": reg::DAY_GROUPS.iter().map(|d| tr(d.0)).collect::<Vec<_>>(),
        "weekdays": crate::company::words::WEEKDAYS.iter().map(|d| tr(d)).collect::<Vec<_>>(),
        "countries": COUNTRIES.iter().map(|(k, n)| json!([k, tr(n)])).collect::<Vec<_>>(),
        "currencies": crate::company::words::CURRENCIES.iter().map(|c| json!([c.0, format!("{} ({})", c.0, c.1)])).collect::<Vec<_>>(),
    })
}

// --- the company -------------------------------------------------------------------------------

fn company_view(s: &mut Session) -> Answer {
    let c = need(s)?;
    let rules = &c.staff_rules;
    Ok(json!({
        "name": c.name, "short": c.short, "colours": c.colours, "depot": c.depot, "depots": c.depots,
        "country": c.country, "currency": c.currency, "founded": c.founded,
        "school_holidays": c.network.school_holidays, "public_holidays": c.network.public_holidays,
        "week_days": rules.week_days, "full_week": rules.full_week, "type_training": rules.type_training,
        "max_off": rules.holidays.max_off, "max_off_default": hol::default_limit(c),
    }))
}

/// Holiday periods as the page typed them: those that cannot be read refused.
fn periods(v: &Value) -> Result<Vec<HolidayPeriod>, String> {
    let list: Vec<HolidayPeriod> = serde_json::from_value(v.clone()).map_err(|e| e.to_string())?;
    let mut out = Vec::new();
    for p in list {
        let p = HolidayPeriod { name: p.name.trim().to_string(), from: p.from.trim().to_string(), to: if p.to.trim().is_empty() { p.from.trim().to_string() } else { p.to.trim().to_string() } };
        if p.from.is_empty() && p.name.is_empty() {
            continue;
        }
        if !p.valid() {
            return Err(trf("“%{name}”: enter its days as 2026-07-18 or, for every year, as 07-18.", &[("name", &p.name)]));
        }
        out.push(p);
    }
    Ok(out)
}

fn company_set(s: &mut Session, args: &Value) -> Answer {
    let p = args.get("patch").cloned().unwrap_or(Value::Null);
    let str_of = |k: &str| p.get(k).and_then(Value::as_str).map(|x| x.trim().to_string());
    let school = p.get("school_holidays").map(periods).transpose()?;
    let public = p.get("public_holidays").map(periods).transpose()?;
    let timetable = school.is_some() || public.is_some() || p.get("country").is_some();
    change(s, timetable, |c| {
        if let Some(n) = str_of("name") {
            if n.is_empty() {
                return Err(no("Give the company a name."));
            }
            c.name = n;
        }
        if let Some(x) = str_of("short") {
            c.short = x.chars().take(4).collect::<String>().to_uppercase();
        }
        if let Some(x) = str_of("colour") {
            c.colours[0] = x;
        }
        if let Some(x) = str_of("colour2") {
            c.colours[1] = x;
        }
        if let Some(x) = str_of("depot").filter(|x| !x.is_empty()) {
            c.depot = x;
        }
        if let Some(d) = p.get("depots") {
            c.depots = serde_json::from_value::<Vec<co::Depot>>(d.clone()).map_err(|e| e.to_string())?.into_iter().filter(|d| !d.name.trim().is_empty()).collect();
        }
        if let Some(x) = str_of("country") {
            c.country = x.to_uppercase();
        }
        if let Some(x) = str_of("currency").filter(|x| !x.is_empty()) {
            c.currency = x.to_uppercase();
        }
        if let Some(v) = school {
            c.network.school_holidays = v;
        }
        if let Some(v) = public {
            c.network.public_holidays = v;
        }
        if let Some(n) = p.get("week_days").and_then(Value::as_u64) {
            c.staff_rules.week_days = (n as u32).clamp(1, 7);
        }
        if let Some(n) = p.get("full_week").and_then(Value::as_f64) {
            c.staff_rules.full_week = n.clamp(1.0, 80.0);
        }
        if let Some(b) = p.get("type_training").and_then(Value::as_bool) {
            c.staff_rules.type_training = b;
        }
        if let Some(m) = p.get("max_off") {
            hol::set_limit(c, m.as_u64().map(|n| n as u32));
        }
        Ok(())
    })?;
    Ok(done(tr("Saved.")))
}

// --- stops ------------------------------------------------------------------------------------

fn stops_view(s: &mut Session) -> Answer {
    let c = need(s)?;
    let mut v: Vec<&Stop> = c.network.stops.iter().collect();
    v.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()).then(a.code.cmp(&b.code)));
    Ok(json!({
        "stops": v.iter().map(|st| json!({ "stop": st, "lines": c.network.lines_with_stop(st.id) })).collect::<Vec<_>>(),
    }))
}

fn stop_save(s: &mut Session, args: &Value) -> Answer {
    let mut st: Stop = arg(args, "stop")?;
    st.name = st.name.trim().to_string();
    st.code = st.code.trim().to_string();
    if st.name.is_empty() {
        return Err(no("Give the stop a name."));
    }
    let id = change(s, true, |c| {
        if !st.code.is_empty() && c.network.stops.iter().any(|x| x.id != st.id && x.code.eq_ignore_ascii_case(&st.code)) {
            return Err(trf("Another stop has the code %{code} already.", &[("code", &st.code)]));
        }
        if st.id == 0 {
            st.id = c.network.add_stop(&st.code, &st.name);
        }
        let x = c.network.stops.iter_mut().find(|x| x.id == st.id).ok_or_else(no_such)?;
        *x = st.clone();
        c.network.rename_stops();
        Ok(st.id)
    })?;
    Ok(json!({ "ok": true, "id": id }))
}

fn stop_delete(s: &mut Session, id: i64) -> Answer {
    change(s, false, |c| {
        let used = c.network.lines_with_stop(id);
        if !used.is_empty() {
            return Err(trf("Lines %{lines} stop here: take the stop out of them first.", &[("lines", &used.join(", "))]));
        }
        let before = c.network.stops.len();
        c.network.stops.retain(|x| x.id != id);
        if c.network.stops.len() == before {
            return Err(no_such());
        }
        Ok(())
    })?;
    Ok(done(tr("The stop is taken out.")))
}

// --- lines ------------------------------------------------------------------------------------

/// A line in the list: what it is, where it goes, how long it takes, whether it can run.
fn line_row(c: &Company, l: &LineDesign, stems: &std::collections::HashMap<u64, String>) -> Value {
    let stem = stems.get(&l.id).cloned().unwrap_or_default();
    let tours = reg::tour_plan(l);
    let per_day: Vec<usize> = (0..reg::DAY_GROUPS.len()).map(|d| tours.iter().filter(|t| t.day == d).count()).collect();
    let roster = c.planning.week.iter().filter(|r| r.line.eq_ignore_ascii_case(&stem)).count();
    json!({
        "id": l.id, "number": l.number, "name": l.name, "title": l.title, "colour": l.colour, "caption": l.caption(),
        "service": l.service, "service_label": tr(l.service.label()),
        "dirs": l.directions.iter().filter(|d| !d.stops.is_empty()).map(|d| json!({
            "from": d.stops.first().map(|x| x.name.clone()).unwrap_or_default(),
            "to": d.destination(), "stops": d.stops.len(), "minutes": d.minutes(),
        })).collect::<Vec<_>>(),
        "problems": reg::problems(l).iter().map(|p| tr(p.text).replace("%{n}", &p.n.to_string())).collect::<Vec<_>>(),
        "tours": per_day, "table_on": l.table_on, "timeplan": l.timeplan.is_some(), "stem": stem, "planned": roster,
    })
}

fn lines_view(s: &mut Session) -> Answer {
    let c = need(s)?;
    let stems = reg::stems(&c.network);
    let mut v: Vec<&LineDesign> = c.network.lines.iter().collect();
    v.sort_by(|a, b| natural(&a.number, &b.number));
    Ok(json!({ "lines": v.iter().map(|l| line_row(c, l, &stems)).collect::<Vec<_>>() }))
}

/// Line numbers in the order people count them ("2" before "10", "10a" after "10").
fn natural(a: &str, b: &str) -> std::cmp::Ordering {
    let lead = |s: &str| -> (u64, String) {
        let d: String = s.chars().take_while(char::is_ascii_digit).collect();
        (d.parse().unwrap_or(u64::MAX), s[d.len()..].to_lowercase())
    };
    lead(a.trim()).cmp(&lead(b.trim()))
}

fn line_view(s: &mut Session, id: u64) -> Answer {
    let c = need(s)?;
    let l = c.network.line(id).ok_or_else(no_such)?;
    let stems = reg::stems(&c.network);
    let depots: Vec<String> = std::iter::once(c.depot.clone()).chain(c.depots.iter().map(|d| d.name.clone())).filter(|d| !d.trim().is_empty()).fold(Vec::new(), |mut v, d| {
        if !v.contains(&d) {
            v.push(d);
        }
        v
    });
    Ok(json!({
        "line": l,
        "row": line_row(c, l, &stems),
        "depots": depots,
        "stops": c.network.stops.iter().map(|st| json!({ "id": st.id, "code": st.code, "name": st.name })).collect::<Vec<_>>(),
        "hourly": (0..reg::DAY_GROUPS.len()).map(|d| reg::hourly_departures(l, d).to_vec()).collect::<Vec<_>>(),
    }))
}

fn line_new(s: &mut Session, args: &Value) -> Answer {
    let number = s_arg(args, "number");
    let id = change(s, true, |c| {
        let number = if number.is_empty() {
            let top = c.network.lines.iter().filter_map(|l| l.number.trim().parse::<u32>().ok()).max().unwrap_or(0);
            (top + 1).to_string()
        } else {
            number.clone()
        };
        let l = c.network.add_line(&number);
        l.depot_minutes = 8.0;
        Ok(l.id)
    })?;
    Ok(json!({ "ok": true, "id": id }))
}

fn line_copy(s: &mut Session, id: u64) -> Answer {
    let new = change(s, true, |c| {
        let old = c.network.line(id).cloned().ok_or_else(no_such)?;
        let number = format!("{}a", old.number.trim());
        let l = c.network.add_line(&number);
        let (nid, created, modified) = (l.id, l.created, l.modified);
        *l = LineDesign { id: nid, number, created, modified, ..old };
        Ok(nid)
    })?;
    Ok(json!({ "ok": true, "id": new }))
}

/// The direction's stops as the network has them (names, a stop gone taken out), its legs and
/// times made to fit them.
fn tidy_direction(c: &Company, d: &mut reg::Direction) {
    d.stops.retain(|x| c.network.stop(x.id).is_some());
    for x in &mut d.stops {
        if let Some(st) = c.network.stop(x.id) {
            *x = StopRef { id: st.id, name: st.name.clone() };
        }
    }
    d.fit_legs();
    if d.times.len() != d.stops.len() || d.times.windows(2).any(|w| w[1] < w[0]) {
        d.manual_times = !d.times.is_empty();
        d.times.truncate(d.stops.len());
        // (each time at least the one before)
        for k in 1..d.times.len() {
            if d.times[k] < d.times[k - 1] {
                d.times[k] = d.times[k - 1];
            }
        }
    }
    d.refresh_times();
    if let Some(f) = d.times.first_mut() {
        *f = 0.0;
    }
}

fn line_save(s: &mut Session, args: &Value) -> Answer {
    let mut new: LineDesign = arg(args, "line")?;
    new.number = new.number.trim().to_string();
    if new.number.is_empty() {
        return Err(no("Give the line a number."));
    }
    let stems = reg::stems(&need(s)?.network);
    let stem = stems.get(&new.id).cloned().unwrap_or_default();
    let (said, anew) = change(s, true, |c| {
        let old = c.network.line(new.id).cloned().ok_or_else(no_such)?;
        for d in &mut new.directions {
            tidy_direction(c, d);
        }
        while new.directions.len() < 2 {
            new.directions.push(Default::default());
        }
        for p in new.days.iter_mut().chain(new.back_days.iter_mut()) {
            p.headway = p.headway.max(1.0);
            p.layover = p.layover.max(0.0);
            if p.last < p.first {
                p.last = p.first;
            }
        }
        new.depot_minutes = new.depot_minutes.clamp(0.0, 180.0);
        new.depot_km = new.depot_km.clamp(0.0, 300.0);
        // (the stops of a direction changed: the Timeplan's own times of it cannot hold)
        let stops_of = |l: &LineDesign| l.directions.iter().map(|d| d.stops.iter().map(|x| x.id).collect::<Vec<_>>()).collect::<Vec<_>>();
        let mut said = Vec::new();
        if stops_of(&old) != stops_of(&new) {
            if new.timeplan.is_some() {
                new.timeplan = None;
                said.push(tr("The stops changed: the Timeplan's running times of the line start anew."));
            }
            if new.table_on {
                for day in 0..reg::DAY_GROUPS.len() {
                    new.refill_table(day, &[0, 1]);
                }
                said.push(tr("The stops changed: its timetable is made anew from its departures."));
            }
        }
        let diff = reg::tour_diff(&old, &new);
        let replan = diff.replan();
        let anew = replan.len() + diff.added.len();
        new.created = old.created;
        new.modified = reg::now_secs();
        let x = c.network.line_mut(new.id).ok_or_else(no_such)?;
        *x = new.clone();
        pl::forget_tours(c, &stem, &replan);
        Ok((said, anew))
    })?;
    let mut text = if anew > 0 { trf("Line %{n} saved: %{k} of its tours to plan anew.", &[("k", &anew.to_string())]) } else { tr("Line %{n} saved.") }.replace("%{n}", &new.number);
    for x in said {
        text = format!("{text} {x}");
    }
    Ok(done(text))
}

fn line_delete(s: &mut Session, id: u64) -> Answer {
    let stems = reg::stems(&need(s)?.network);
    let stem = stems.get(&id).cloned().unwrap_or_default();
    change(s, true, |c| {
        let before = c.network.lines.len();
        c.network.lines.retain(|l| l.id != id);
        if c.network.lines.len() == before {
            return Err(no_such());
        }
        c.planning.week.retain(|r| !r.line.eq_ignore_ascii_case(&stem));
        Ok(())
    })?;
    Ok(done(tr("The line is taken out.")))
}

// --- the fleet --------------------------------------------------------------------------------

fn fleet_view(s: &mut Session) -> Answer {
    let c = need(s)?;
    let today = c.date.clone();
    let mut v: Vec<&Vehicle> = c.fleet.iter().collect();
    v.sort_by(|a, b| natural(&a.number, &b.number));
    Ok(json!({
        "fleet": v.iter().map(|x| json!({
            "vehicle": x,
            "kind": tr(x.kind.size.label()), "drive": tr(x.kind.drive.label()),
            "held": x.held_on(&today), "workshop": x.in_workshop(&today),
            "drivers": licences::qualified_drivers(c, x.kind, &x.name).0,
        })).collect::<Vec<_>>(),
        "next_number": next_number(c),
        "depots": std::iter::once(c.depot.clone()).chain(c.depots.iter().map(|d| d.name.clone())).collect::<Vec<_>>(),
    }))
}

fn next_number(c: &Company) -> String {
    let mut c = c.clone();
    c.next_fleet_number()
}

fn vehicle_save(s: &mut Session, args: &Value) -> Answer {
    let mut v: Vehicle = arg(args, "vehicle")?;
    v.number = v.number.trim().to_string();
    if v.number.is_empty() {
        return Err(no("Give the bus its fleet number."));
    }
    for d in [&mut v.from, &mut v.until, &mut v.built] {
        *d = d.trim().to_string();
    }
    if v.workshop_until.as_deref().is_some_and(|w| w.trim().is_empty()) {
        v.workshop_until = None;
    }
    let id = change(s, false, |c| put_vehicle(c, v))?;
    Ok(json!({ "ok": true, "id": id }))
}

fn put_vehicle(c: &mut Company, mut v: Vehicle) -> Result<u32, String> {
    if c.fleet.iter().any(|x| x.id != v.id && x.number.eq_ignore_ascii_case(&v.number) && x.held_on(&c.date)) {
        return Err(trf("Another bus has the number %{n} already.", &[("n", &v.number)]));
    }
    if v.id == 0 {
        c.counters.vehicle += 1;
        v.id = c.counters.vehicle.max(c.fleet.iter().map(|x| x.id).max().unwrap_or(0) + 1);
        c.counters.vehicle = v.id;
        if let Ok(n) = v.number.parse::<u32>() {
            c.counters.fleet_number = c.counters.fleet_number.max(n);
        }
        c.fleet.push(v.clone());
    } else {
        let x = c.fleet.iter_mut().find(|x| x.id == v.id).ok_or_else(no_such)?;
        *x = v.clone();
    }
    Ok(v.id)
}

fn vehicle_delete(s: &mut Session, id: u32) -> Answer {
    change(s, false, |c| {
        let before = c.fleet.len();
        c.fleet.retain(|x| x.id != id);
        if c.fleet.len() == before {
            return Err(no_such());
        }
        for r in c.planning.week.iter_mut().filter(|r| r.bus == Some(id)) {
            r.bus = None;
        }
        Ok(())
    })?;
    Ok(done(tr("The bus is taken out of the fleet.")))
}

// --- the staff --------------------------------------------------------------------------------

/// Where someone is on `date`: at work, ill, on holiday, on a course, not (yet) employed.
fn status(e: &Employee, date: &str) -> (&'static str, String) {
    if !e.employed_on(date) {
        return ("gone", tr("Not employed"));
    }
    if e.sick_on(date) {
        return ("sick", if e.sick_open { tr("Ill") } else { trf("Ill until %{day}", &[("day", &day_label(e.sick_until.as_deref().unwrap_or("")))]) });
    }
    if let Some(u) = e.on_holiday(date) {
        return ("holiday", trf("On holiday until %{day}", &[("day", &day_label(u))]));
    }
    if e.absent(date) {
        return ("training", trf("On a course until %{day}", &[("day", &day_label(e.training_until.as_deref().unwrap_or("")))]));
    }
    ("in", tr("Available"))
}

fn staff_view(s: &mut Session) -> Answer {
    let c = need(s)?;
    let mut v: Vec<&Employee> = c.staff.iter().collect();
    v.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    let rules = &c.staff_rules;
    Ok(json!({
        "staff": v.iter().map(|e| {
            let (state, text) = status(e, &c.date);
            json!({
                "employee": e, "role": tr(e.role.label()), "state": state, "state_text": text,
                "hours": rules.hours_of(e), "week_days": rules.week_days_of(e),
                "wage_text": if e.wage > 0 { eur(e.wage) } else { String::new() },
                "marks": e.endorsements.iter().map(|x| x.short()).collect::<Vec<_>>(),
            })
        }).collect::<Vec<_>>(),
        "types": licences::fleet_types(c).iter().map(|k| json!([k, licences::type_name(c, k)])).collect::<Vec<_>>(),
        "type_training": rules.type_training,
        "full_week": rules.full_week, "week_days": rules.week_days,
    }))
}

fn employee_save(s: &mut Session, args: &Value) -> Answer {
    let mut e: Employee = arg(args, "employee")?;
    e.name = e.name.trim().to_string();
    if e.name.is_empty() {
        return Err(no("Give the employee a name."));
    }
    if let Some(w) = args.get("employee").and_then(|x| x.get("wage_text")).and_then(Value::as_str) {
        e.wage = typed_money(w).unwrap_or(0).max(0);
    }
    for d in [&mut e.notice_until, &mut e.holiday_until, &mut e.training_until] {
        if d.as_deref().is_some_and(|x| x.trim().is_empty()) {
            *d = None;
        }
    }
    e.types = e.types.iter().map(|t| licences::type_key(t)).filter(|t| !t.is_empty()).collect();
    e.days_off.retain(|d| *d < 7);
    e.days_off.sort_unstable();
    e.days_off.dedup();
    let id = change(s, false, |c| put_employee(c, e, true))?;
    Ok(json!({ "ok": true, "id": id }))
}

/// `e` into the staff (a new one numbered; one there keeps what the form does not edit: their
/// holiday requests and whether they are ill, unless `whole`).
fn put_employee(c: &mut Company, mut e: Employee, whole: bool) -> Result<u32, String> {
    if !e.number.trim().is_empty() && c.staff.iter().any(|x| x.id != e.id && x.number.trim() == e.number.trim()) {
        return Err(trf("Another employee has the personnel number %{n} already.", &[("n", e.number.trim())]));
    }
    if e.id == 0 {
        c.counters.employee += 1;
        e.id = c.counters.employee.max(c.staff.iter().map(|x| x.id).max().unwrap_or(0) + 1);
        c.counters.employee = e.id;
        if e.hired.trim().is_empty() {
            e.hired = c.date.clone();
        }
        if e.holiday_left == 0 || e.holiday_left > e.holiday_days {
            e.holiday_left = e.holiday_days;
        }
        e.leave = Default::default();
        e.agency_no = 0;
        c.staff.push(e.clone());
    } else {
        let x = c.staff.iter_mut().find(|x| x.id == e.id).ok_or_else(no_such)?;
        e.leave = x.leave.clone();
        e.agency_no = x.agency_no;
        if !whole {
            e.sick_until = x.sick_until.clone();
            e.sick_open = x.sick_open;
        }
        *x = e.clone();
    }
    Ok(e.id)
}

fn employee_delete(s: &mut Session, id: u32) -> Answer {
    change(s, false, |c| {
        let before = c.staff.len();
        c.staff.retain(|x| x.id != id);
        if c.staff.len() == before {
            return Err(no_such());
        }
        for r in &mut c.planning.week {
            for d in r.duties.iter_mut().filter(|d| **d == Some(pl::Who::Staff(id))) {
                *d = None;
            }
        }
        Ok(())
    })?;
    Ok(done(tr("The employee is taken out.")))
}

/// Someone reported ill (`until`, else until reported fit), fit again, on holiday or on a course
/// up to a day, or back (`kind`: sick, fit, holiday, training; `until` "" ends it).
fn absence(s: &mut Session, args: &Value) -> Answer {
    let id = n_arg(args, "id") as u32;
    let kind = s_arg(args, "kind");
    let until = s_arg(args, "until");
    let day = |u: &str| (!u.is_empty()).then(|| u.to_string());
    if !until.is_empty() && co::dates::parse(&until).is_none() {
        return Err(no("Enter the day as 2026-07-18."));
    }
    let name = change(s, true, |c| {
        let today = c.date.clone();
        let e = c.staff.iter_mut().find(|x| x.id == id).ok_or_else(no_such)?;
        match kind.as_str() {
            "sick" => {
                e.sick_open = until.is_empty();
                e.sick_until = if until.is_empty() { Some(today) } else { Some(until.clone()) };
            }
            "fit" => {
                e.sick_open = false;
                e.sick_until = None;
            }
            "holiday" => e.holiday_until = day(&until),
            "training" => e.training_until = day(&until),
            _ => return Err(format!("unknown absence: {kind}")),
        }
        Ok(e.name.clone())
    })?;
    let text = match kind.as_str() {
        "sick" => trf("%{name} is reported ill.", &[("name", &name)]),
        "fit" => trf("%{name} is reported fit.", &[("name", &name)]),
        _ => tr("Saved."),
    };
    Ok(done(text))
}

// --- holiday requests ---------------------------------------------------------------------------

fn leave_view(s: &mut Session) -> Answer {
    let c = need(s)?;
    let mut rows = Vec::new();
    for e in &c.staff {
        for r in &e.leave.requests {
            let advice = hol::advice(c, e.id, r);
            rows.push(json!({
                "no": r.no, "id": e.id, "name": e.name, "from": r.from, "until": r.until,
                "from_label": day_label(&r.from), "until_label": day_label(&r.until),
                "days": r.working_days(), "reason": r.reason, "reason_label": tr(r.reason.label()),
                "state": r.state, "note": r.note, "asked": r.asked,
                "advice": advice.approve(), "advice_text": tr(advice.why()),
                "left": e.holiday_left,
            }));
        }
    }
    rows.sort_by(|a, b| a["from"].as_str().cmp(&b["from"].as_str()));
    Ok(json!({
        "requests": rows,
        "limit": hol::limit(c),
        "staff": c.staff.iter().filter(|e| e.employed_on(&c.date)).map(|e| json!([e.id, e.name])).collect::<Vec<_>>(),
    }))
}

fn leave_add(s: &mut Session, args: &Value) -> Answer {
    let id = n_arg(args, "id") as u32;
    let (from, until) = (s_arg(args, "from"), s_arg(args, "until"));
    let until = if until.is_empty() { from.clone() } else { until };
    let reason: hol::Reason = serde_json::from_value(args.get("reason").cloned().unwrap_or(json!("break"))).unwrap_or_default();
    let note = s_arg(args, "note");
    change(s, false, |c| hol::add_request(c, id, &from, &until, reason, &note).map_err(no))?;
    Ok(done(tr("The holiday request is entered.")))
}

fn leave_answer(s: &mut Session, args: &Value) -> Answer {
    let no_ = n_arg(args, "no") as u32;
    let approve = args.get("approve").and_then(Value::as_bool).unwrap_or(false);
    change(s, true, |c| hol::answer(c, no_, approve, hol::By::Planner).map_err(no))?;
    Ok(done(if approve { tr("Approved.") } else { tr("Refused.") }))
}

// --- agency drivers ---------------------------------------------------------------------------

fn agency_view(s: &mut Session) -> Answer {
    let c = need(s)?;
    let agencies: Vec<String> = c.planning.agency.iter().map(|b| b.agency.clone()).filter(|a| !a.is_empty()).fold(Vec::new(), |mut v, a| {
        if !v.contains(&a) {
            v.push(a);
        }
        v
    });
    Ok(json!({
        "bookings": agency::bookings(c).iter().map(|b| json!({
            "no": b.no, "date": b.date, "label": day_label(&b.date), "count": b.count,
            "hours": b.hours.map(|(f, t)| format!("{}–{}", co::rosterflow::clock(f), co::rosterflow::clock(t))),
            "duties": b.duties, "cost": eur(b.cost), "settled": b.settled,
            "past": co::dates::between(&c.date, &b.date) < 0,
            "agency": c.planning.agency.iter().find(|x| x.no == b.no).map(|x| x.agency.clone()).unwrap_or_default(),
        })).collect::<Vec<_>>(),
        "rate": eur(agency::rate(c)),
        "agencies": agencies,
        "first": co::dates::add(&c.date, 1),
        "last": co::dates::add(&c.date, agency::DAYS_AHEAD),
    }))
}

/// `HH:MM` as minutes of the day.
fn minutes_of(t: &str) -> Option<i32> {
    let (h, m) = t.trim().split_once(':')?;
    Some(h.trim().parse::<i32>().ok()? * 60 + m.trim().parse::<i32>().ok()?)
}

fn agency_book(s: &mut Session, args: &Value) -> Answer {
    let date = s_arg(args, "date");
    let count = n_arg(args, "count").max(1) as u32;
    let hours = match (minutes_of(&s_arg(args, "from")), minutes_of(&s_arg(args, "to"))) {
        (Some(f), Some(t)) => Some((f, if t <= f { t + 1440 } else { t })),
        _ => None,
    };
    let name = s_arg(args, "agency");
    let made = change(s, true, |c| agency::book(c, &date, count, hours, &[], &name, &[]).map_err(no))?;
    let _ = made;
    Ok(done(trf("%{n} agency drivers booked for %{day}.", &[("n", &count.to_string()), ("day", &day_label(&date))])))
}

fn agency_cancel(s: &mut Session, no_: u32) -> Answer {
    let fee = change(s, true, |c| agency::cancel(c, no_).map_err(no))?;
    Ok(done(if fee > 0 { trf("The booking is cancelled: the agency charges %{fee}.", &[("fee", &eur(fee))]) } else { tr("The booking is cancelled.") }))
}

// --- spreadsheets -----------------------------------------------------------------------------

/// The columns of each list a spreadsheet holds.
const STOP_COLS: [&str; 6] = ["Code", "Name", "Latitude", "Longitude", "Relief point", "Notes"];
const FLEET_COLS: [&str; 13] = ["Number", "Plate", "Model", "Size", "Drive", "Seats", "Standing", "Built", "From", "Until", "Depot", "Workshop until", "Notes"];
const STAFF_COLS: [&str; 18] = [
    "Number",
    "Name",
    "Role",
    "Phone",
    "Email",
    "Licence",
    "Licence valid until",
    "Endorsements",
    "Type trainings",
    "Hours a week",
    "Days a week",
    "Hourly wage",
    "Hired",
    "Leaves",
    "Holiday days",
    "Days off",
    "Shift",
    "Notes",
];

fn yes(b: bool) -> String {
    if b { "1" } else { "" }.to_string()
}

fn truthy(s: &str) -> bool {
    matches!(s.trim().to_lowercase().as_str(), "1" | "x" | "yes" | "ja" | "true" | "y" | "j" | "oui" | "tak")
}

/// A choice written as its key ("articulated"), read back by its key or its English name.
fn key_of<T: serde::Serialize>(x: T) -> String {
    json!(x).as_str().unwrap_or("").to_string()
}

fn pick<T: serde::Serialize + Copy>(all: &[T], text: &str, label: impl Fn(T) -> &'static str) -> Option<T> {
    let t = text.trim().to_lowercase();
    all.iter().copied().find(|x| key_of(*x) == t || label(*x).to_lowercase() == t || tr(label(*x)).to_lowercase() == t)
}

fn export(s: &mut Session, what: &str, p: &dyn Platform) -> Answer {
    let c = need(s)?;
    let head = |cols: &[&str]| cols.iter().map(|x| x.to_string()).collect::<Vec<_>>();
    let mut rows = Vec::new();
    match what {
        "stops" => {
            rows.push(head(&STOP_COLS));
            for st in &c.network.stops {
                rows.push(vec![
                    st.code.clone(),
                    st.name.clone(),
                    if st.lat != 0.0 { st.lat.to_string() } else { String::new() },
                    if st.lon != 0.0 { st.lon.to_string() } else { String::new() },
                    yes(st.relief),
                    st.notes.clone(),
                ]);
            }
        }
        "fleet" => {
            rows.push(head(&FLEET_COLS));
            for v in &c.fleet {
                rows.push(vec![
                    v.number.clone(),
                    v.plate.clone(),
                    v.name.clone(),
                    key_of(v.kind.size),
                    key_of(v.kind.drive),
                    v.seats.to_string(),
                    v.standing.to_string(),
                    v.built.clone(),
                    v.from.clone(),
                    v.until.clone(),
                    v.depot.clone(),
                    v.workshop_until.clone().unwrap_or_default(),
                    v.notes.clone(),
                ]);
            }
        }
        "staff" => {
            rows.push(head(&STAFF_COLS));
            for e in &c.staff {
                rows.push(vec![
                    e.number.clone(),
                    e.name.clone(),
                    key_of(e.role),
                    e.phone.clone(),
                    e.email.clone(),
                    key_of(e.licence),
                    e.licence_until.clone(),
                    e.endorsements.iter().map(|x| key_of(*x)).collect::<Vec<_>>().join(", "),
                    e.types.join(", "),
                    if e.contract_hours > 0.0 { e.contract_hours.to_string() } else { String::new() },
                    if e.week_days_max > 0 { e.week_days_max.to_string() } else { String::new() },
                    if e.wage > 0 { format!("{:.2}", e.wage as f64 / 100.0) } else { String::new() },
                    e.hired.clone(),
                    e.notice_until.clone().unwrap_or_default(),
                    e.holiday_days.to_string(),
                    e.days_off.iter().map(|d| crate::company::words::WEEKDAYS[*d as usize].to_string()).collect::<Vec<_>>().join(", "),
                    key_of(e.shift),
                    e.notes.clone(),
                ]);
            }
        }
        _ => return Err(format!("unknown list: {what}")),
    }
    let name = format!(
        "{} - {}.csv",
        c.name,
        tr(match what {
            "stops" => "Stops",
            "fleet" => "Fleet",
            _ => "Staff",
        })
    );
    let Some(path) = p.save_path(&tr("Save as a spreadsheet"), &name, "CSV", &["csv"]) else { return Ok(json!({})) };
    std::fs::write(&path, csv::write(&rows)).map_err(|e| format!("{}: {e}", tr("Could not save the file")))?;
    Ok(done(trf("Saved: %{file}", &[("file", &path.display().to_string())])))
}

/// A spreadsheet taken in (`text`, else one the planner chooses): its rows added to the list
/// `what`, a row of a stop, bus or person there already (the same code, number or personnel
/// number) changing it.
fn import(s: &mut Session, args: &Value, p: &dyn Platform) -> Answer {
    let what = s_arg(args, "what");
    let text = match args.get("text").and_then(Value::as_str) {
        Some(t) => t.to_string(),
        None => {
            let Some(path) = p.open_path(&tr("Open a spreadsheet"), "CSV", &["csv", "txt"]) else { return Ok(json!({})) };
            let bytes = std::fs::read(&path).map_err(|e| format!("{}: {e}", path.display()))?;
            // (a spreadsheet of an older Windows saves Latin-1)
            String::from_utf8(bytes.clone()).unwrap_or_else(|_| bytes.iter().map(|b| *b as char).collect())
        }
    };
    let t = Table::of(&text);
    if t.rows.is_empty() {
        return Err(no("The file has no rows under its header."));
    }
    let (added, changed, skipped) = change(s, what == "stops", |c| match what.as_str() {
        "stops" => Ok(import_stops(c, &t)),
        "fleet" => import_fleet(c, &t),
        "staff" => import_staff(c, &t),
        _ => Err(format!("unknown list: {what}")),
    })?;
    let mut text = trf("%{added} added, %{changed} changed.", &[("added", &added.to_string()), ("changed", &changed.to_string())]);
    if skipped > 0 {
        text = format!("{text} {}", trf("%{n} rows skipped (no name or number).", &[("n", &skipped.to_string())]));
    }
    Ok(done(text))
}

fn import_stops(c: &mut Company, t: &Table) -> (usize, usize, usize) {
    let (mut added, mut changed, mut skipped) = (0, 0, 0);
    for r in &t.rows {
        let name = t.get(r, &["name", "naam", "stop_name", "haltenaam"]).to_string();
        let code = t.get(r, &["code", "stop_code", "haltecode", "stop_id"]).to_string();
        if name.is_empty() {
            skipped += 1;
            continue;
        }
        let id = match c.network.stops.iter().find(|x| (!code.is_empty() && x.code.eq_ignore_ascii_case(&code)) || (code.is_empty() && x.name.eq_ignore_ascii_case(&name))) {
            Some(x) => {
                changed += 1;
                x.id
            }
            None => {
                added += 1;
                c.network.add_stop(&code, &name)
            }
        };
        let x = c.network.stops.iter_mut().find(|x| x.id == id).unwrap();
        x.name = name;
        x.code = code;
        x.lat = csv::number(t.get(r, &["latitude", "lat", "stop_lat", "breedtegraad"])).unwrap_or(x.lat);
        x.lon = csv::number(t.get(r, &["longitude", "lon", "lng", "stop_lon", "lengtegraad"])).unwrap_or(x.lon);
        if t.col(&["relief point", "relief", "aflospunt"]).is_some() {
            x.relief = truthy(t.get(r, &["relief point", "relief", "aflospunt"]));
        }
        let notes = t.get(r, &["notes", "opmerkingen"]);
        if !notes.is_empty() {
            x.notes = notes.to_string();
        }
    }
    c.network.rename_stops();
    (added, changed, skipped)
}

fn import_fleet(c: &mut Company, t: &Table) -> Result<(usize, usize, usize), String> {
    let (mut added, mut changed, mut skipped) = (0, 0, 0);
    for r in &t.rows {
        let number = t.get(r, &["number", "fleet number", "nummer", "wagennummer"]).to_string();
        if number.is_empty() {
            skipped += 1;
            continue;
        }
        let mut v = c.fleet.iter().find(|x| x.number.eq_ignore_ascii_case(&number)).cloned().unwrap_or_default();
        if v.id == 0 {
            added += 1;
        } else {
            changed += 1;
        }
        v.number = number;
        let cell = |names: &[&str]| t.get(r, names).to_string();
        let set = |field: &mut String, names: &[&str]| {
            let x = cell(names);
            if t.col(names).is_some() {
                *field = x;
            }
        };
        set(&mut v.plate, &["plate", "kenteken", "registration"]);
        set(&mut v.name, &["model", "type", "make"]);
        set(&mut v.depot, &["depot", "stalling"]);
        set(&mut v.notes, &["notes", "opmerkingen"]);
        if let Some(sz) = pick(&BusSize::ALL, &cell(&["size", "kind", "soort"]), BusSize::label) {
            v.kind.size = sz;
        }
        if let Some(d) = pick(&Drive::ALL, &cell(&["drive", "fuel", "aandrijving", "brandstof"]), Drive::label) {
            v.kind.drive = d;
        }
        if let Some(n) = csv::number(&cell(&["seats", "zitplaatsen"])) {
            v.seats = n as u32;
        }
        if let Some(n) = csv::number(&cell(&["standing", "staanplaatsen"])) {
            v.standing = n as u32;
        }
        for (field, names) in
            [(&mut v.built, &["built", "bouwjaar", "first registered"][..]), (&mut v.from, &["from", "in service", "vanaf"][..]), (&mut v.until, &["until", "out of service", "tot"][..])]
        {
            if t.col(names).is_some() {
                let x = cell(names);
                *field = if x.len() == 4 && x.chars().all(|c| c.is_ascii_digit()) { x } else { csv::date(&x) };
            }
        }
        if t.col(&["workshop until", "werkplaats tot"]).is_some() {
            v.workshop_until = Some(csv::date(&cell(&["workshop until", "werkplaats tot"]))).filter(|x| !x.is_empty());
        }
        put_vehicle(c, v)?;
    }
    Ok((added, changed, skipped))
}

fn import_staff(c: &mut Company, t: &Table) -> Result<(usize, usize, usize), String> {
    let (mut added, mut changed, mut skipped) = (0, 0, 0);
    for r in &t.rows {
        let name = t.get(r, &["name", "naam"]).to_string();
        let number = t.get(r, &["number", "personnel number", "personeelsnummer", "nummer"]).to_string();
        if name.is_empty() {
            skipped += 1;
            continue;
        }
        let mut e = c.staff.iter().find(|x| (!number.is_empty() && x.number.trim() == number) || (number.is_empty() && x.name.eq_ignore_ascii_case(&name))).cloned().unwrap_or_default();
        if e.id == 0 {
            added += 1;
        } else {
            changed += 1;
        }
        e.name = name;
        e.number = number;
        let cell = |names: &[&str]| t.get(r, names).to_string();
        let has = |names: &[&str]| t.col(names).is_some();
        if has(&["phone", "telefoon"]) {
            e.phone = cell(&["phone", "telefoon"]);
        }
        if has(&["email", "e-mail"]) {
            e.email = cell(&["email", "e-mail"]);
        }
        if has(&["notes", "opmerkingen"]) {
            e.notes = cell(&["notes", "opmerkingen"]);
        }
        if let Some(role) = pick(&Role::ALL, &cell(&["role", "functie"]), Role::label) {
            e.role = role;
        }
        match cell(&["licence", "license", "rijbewijs"]).trim().to_uppercase().as_str() {
            "D1" => e.licence = Licence::D1,
            "D" => e.licence = Licence::D,
            _ => {}
        }
        if has(&["licence valid until", "code 95", "rijbewijs geldig tot"]) {
            e.licence_until = csv::date(&cell(&["licence valid until", "code 95", "rijbewijs geldig tot"]));
        }
        if has(&["endorsements", "bevoegdheden"]) {
            e.endorsements = cell(&["endorsements", "bevoegdheden"]).split([',', '/', '|']).filter_map(|x| pick(&licences::Endorsement::ALL, x, licences::Endorsement::label)).collect();
        }
        if has(&["type trainings", "typeopleidingen"]) {
            e.types = cell(&["type trainings", "typeopleidingen"]).split([',', '|']).map(licences::type_key).filter(|x| !x.is_empty()).collect();
        }
        if let Some(h) = csv::number(&cell(&["hours a week", "contract hours", "uren per week"])) {
            e.contract_hours = h;
        }
        if let Some(d) = csv::number(&cell(&["days a week", "dagen per week"])) {
            e.week_days_max = d as u32;
        }
        if let Some(w) = csv::number(&cell(&["hourly wage", "uurloon"])) {
            e.wage = (w * 100.0).round() as i64;
        }
        if has(&["hired", "in dienst"]) {
            e.hired = csv::date(&cell(&["hired", "in dienst"]));
        }
        if has(&["leaves", "uit dienst"]) {
            e.notice_until = Some(csv::date(&cell(&["leaves", "uit dienst"]))).filter(|x| !x.is_empty());
        }
        if let Some(d) = csv::number(&cell(&["holiday days", "vakantiedagen"])) {
            e.holiday_days = d as u32;
        }
        if has(&["days off", "vrije dagen"]) {
            let days = crate::company::words::WEEKDAYS;
            e.days_off = cell(&["days off", "vrije dagen"])
                .split([',', ' ', '/'])
                .filter_map(|x| {
                    let x = x.trim().to_lowercase();
                    days.iter().position(|d| d.to_lowercase() == x || tr(d).to_lowercase() == x).map(|k| k as u8)
                })
                .collect();
        }
        match cell(&["shift", "dienst"]).trim().to_lowercase().as_str() {
            "early" | "vroeg" => e.shift = ShiftPref::Early,
            "late" | "laat" => e.shift = ShiftPref::Late,
            "any" | "" => {}
            _ => {}
        }
        put_employee(c, e, false)?;
    }
    Ok((added, changed, skipped))
}

/// The popup that leads to a page of master data (the planning's refusals use it).
pub fn lead(text: &str, go: Go) -> Value {
    Popup::new("info", "Master data", tr(text), "", Some(go)).json()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::company::Session;
    use rosterflow_core::company::BusKind;

    fn session() -> Session {
        crate::store::set_dry(true);
        Session::of(co::found(&co::Founding { name: "Test".into(), ..Default::default() }, ""))
    }

    #[test]
    fn master_data_is_entered_listed_and_taken_out() {
        let mut s = session();
        s.company.as_mut().unwrap().date = "2026-03-02".into();
        let a = stop_save(&mut s, &json!({ "stop": { "code": "1001", "name": "Station" } })).unwrap()["id"].as_i64().unwrap();
        let b = stop_save(&mut s, &json!({ "stop": { "code": "1002", "name": "Markt" } })).unwrap()["id"].as_i64().unwrap();
        assert!(stop_save(&mut s, &json!({ "stop": { "code": "1001", "name": "Dubbel" } })).is_err());
        let id = line_new(&mut s, &json!({ "number": "5" })).unwrap()["id"].as_u64().unwrap();
        let mut l = line_view(&mut s, id).unwrap()["line"].clone();
        l["directions"] = json!([
            { "stops": [{ "id": a }, { "id": b }], "times": [0, 12], "manual_times": true },
            { "stops": [{ "id": b }, { "id": a }], "times": [0, 12], "manual_times": true },
        ]);
        line_save(&mut s, &json!({ "line": l })).unwrap();
        let rows = lines_view(&mut s).unwrap();
        assert_eq!(rows["lines"][0]["problems"], json!([]), "{rows}");
        assert_eq!(rows["lines"][0]["dirs"][0]["to"], "Markt");
        assert!(s.company.as_ref().unwrap().lines.iter().any(|x| x.name == "L5"));
        // a stop a line uses stays
        assert!(stop_delete(&mut s, a).is_err());
        // buses and people
        let v = vehicle_save(&mut s, &json!({ "vehicle": { "number": "201", "kind": { "size": "articulated", "drive": "electric" } } })).unwrap()["id"].as_u64().unwrap();
        assert!(vehicle_save(&mut s, &json!({ "vehicle": { "number": "201" } })).is_err());
        let e = employee_save(&mut s, &json!({ "employee": { "name": "An de Vries", "number": "17", "wage_text": "21,50" } })).unwrap()["id"].as_u64().unwrap();
        let c = s.company.as_ref().unwrap();
        assert_eq!(c.employee(e as u32).unwrap().hired, "2026-03-02");
        absence(&mut s, &json!({ "id": e, "kind": "sick" })).unwrap();
        assert!(s.company.as_ref().unwrap().employee(e as u32).unwrap().sick_on("2026-03-09"));
        absence(&mut s, &json!({ "id": e, "kind": "fit" })).unwrap();
        leave_add(&mut s, &json!({ "id": e, "from": "2026-04-06", "until": "2026-04-10", "reason": "break" })).unwrap();
        let no_ = leave_view(&mut s).unwrap()["requests"][0]["no"].as_u64().unwrap();
        leave_answer(&mut s, &json!({ "no": no_, "approve": true })).unwrap();
        assert!(s.company.as_ref().unwrap().employee(e as u32).unwrap().on_holiday("2026-04-07").is_some());
        vehicle_delete(&mut s, v as u32).unwrap();
        employee_delete(&mut s, e as u32).unwrap();
        line_delete(&mut s, id).unwrap();
        assert!(s.company.as_ref().unwrap().lines.is_empty());
        stop_delete(&mut s, a).unwrap();
    }

    #[test]
    fn spreadsheets_are_read() {
        let mut s = session();
        let c = s.company.as_mut().unwrap();
        let t = Table::of("Code;Naam;Lat;Lon\n1001;Station;52,37;4,89\n;;;\n1002;Markt;;\n");
        assert_eq!(import_stops(c, &t), (2, 0, 0));
        assert_eq!(import_stops(c, &Table::of("code,name\n1001,Centraal Station\n")), (0, 1, 0));
        assert_eq!(c.network.stops[0].name, "Centraal Station");
        let t = Table::of("Number;Model;Size;Drive;Seats\n301;VDL Citea;articulated;Electric;44\n302;;solo;diesel;\n");
        assert_eq!(import_fleet(c, &t).unwrap(), (2, 0, 0));
        assert_eq!(c.fleet[0].kind, BusKind { size: BusSize::Articulated, drive: Drive::Electric });
        let t = Table::of("Personeelsnummer;Naam;Rijbewijs;Uurloon;Vrije dagen;Bevoegdheden\n12;Piet;D;19,75;Sat, Sun;articulated\n");
        assert_eq!(import_staff(c, &t).unwrap(), (1, 0, 0));
        let e = &c.staff[0];
        assert_eq!((e.wage, e.days_off.clone(), e.endorsements.len()), (19_75, vec![5, 6], 1));
        let _ = &mut s;
    }
}
