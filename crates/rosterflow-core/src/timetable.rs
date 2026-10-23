//! The timetable as the planning works with it: the company's lines, their tours, the tours'
//! trips and the trips' stops, on one date (`lines_on`), made from the lines' designs
//! (`lines::LineDesign`) and the company's calendar.
//!
//! Times are seconds of the day (`08:15` is 29 700.0); a trip past midnight goes on above
//! 86 400.

use crate::calendar::DayCalendar;
use crate::company::{Company, CompanyLine};
use crate::lines::{self, LineDesign, Planned};
use serde::{Deserialize, Serialize};

/// A stop of a trip.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct StopInfo {
    pub name: String,
    /// The network's stop (`lines::Stop::id`): where a trip ends and another begins, the two
    /// meet at it.
    pub id: i64,
    /// When the bus is due there, and leaves (seconds of the day).
    pub arr: f64,
    pub dep: f64,
}

/// A trip of a tour.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct TripInfo {
    /// The trip's name (its line's planning name and direction, `lines::trip_name`).
    pub name: String,
    /// Its place in the tour, 1 = the first.
    pub index: usize,
    /// The line its displays show.
    pub line: String,
    /// Its first and last stop.
    pub from: String,
    pub terminus: String,
    pub departure: f64,
    pub arrival: f64,
    pub stops: Vec<StopInfo>,
    /// Its length.
    pub km: f64,
}

/// A tour (one bus's work) of a line.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct TourInfo {
    pub number: String,
    /// The size of bus it asks for (`network::wants_size`; empty: any).
    pub ai_group: String,
    /// Its first departure and last arrival.
    pub first: f64,
    pub last: f64,
    /// The days it runs, in words (`Mon - Fri`, `Saturday`, `Sunday`).
    pub days: String,
    /// It runs on the date asked for.
    pub runs: bool,
    pub next_run: Option<String>,
    pub trips: Vec<TripInfo>,
}

/// A line of the company on a date.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct LineInfo {
    /// The line's planning name (`CompanyLine::name`).
    pub name: String,
    pub user_allowed: bool,
    /// Its ends: the first and last stop of its longest trip.
    pub termini: Vec<String>,
    pub tours: Vec<TourInfo>,
}

impl TourInfo {
    /// Its trips' stops in order, each once where one trip ends where the next begins.
    pub fn stops(&self) -> Vec<&StopInfo> {
        let mut out: Vec<&StopInfo> = Vec::new();
        for t in &self.trips {
            for (k, s) in t.stops.iter().enumerate() {
                if k == 0 && out.last().is_some_and(|l| l.id == s.id && s.id != 0) {
                    continue;
                }
                out.push(s);
            }
        }
        out
    }
}

/// The company's calendar (`calendar::DayCalendar`).
pub fn calendar_of(c: &Company) -> DayCalendar {
    DayCalendar { country: c.country.clone(), holidays: c.network.public_holidays.clone(), school: c.network.school_holidays.clone() }
}

/// The day group `date` runs (`lines::DAY_GROUPS`), and whether its school-only lines run.
pub fn day_of(c: &Company, date: &str) -> crate::calendar::DayType {
    calendar_of(c).day_of(date).unwrap_or_default()
}

/// The company's lines on `date` (`YYYY-MM-DD`): each line's tours of that day's group, the
/// tours of the other groups there too but not running (`TourInfo::runs`).
pub fn lines_on(c: &Company, date: &str) -> Vec<LineInfo> {
    let day = day_of(c, date);
    let stems = lines::stems(&c.network);
    c.network.lines.iter().filter(|l| lines::route_problems(l).is_empty()).map(|l| line_info(l, stems.get(&l.id).map(String::as_str).unwrap_or("L"), day)).collect()
}

/// A line's tours on a day of type `day`.
pub fn line_info(l: &LineDesign, stem: &str, day: crate::calendar::DayType) -> LineInfo {
    let group = day.group();
    let runs_today = day.runs(l.service);
    let run = lines::run_minutes(l);
    let table = if l.table_on { lines::table_trips(l) } else { Vec::new() };
    let mut tours = Vec::new();
    for t in lines::tour_plan(l) {
        let trips: Vec<TripInfo> = t.trips.iter().enumerate().map(|(k, p)| trip_info(l, stem, p, k + 1, &run, &table)).collect();
        let (first, last) = (trips.first().map(|x| x.departure).unwrap_or(0.0), trips.iter().map(|x| x.arrival).fold(0.0, f64::max));
        let size = t.trips.iter().find_map(|p| p.band.and_then(|b| l.pattern(p.dir, t.day).and_then(|pat| pat.clean_bands().get(b).and_then(|x| x.size))));
        let ai_group = size.map(|s| format!("{s:?}").to_lowercase()).unwrap_or_else(|| l.ai_group.clone());
        tours.push(TourInfo {
            number: t.number,
            ai_group,
            first,
            last,
            days: lines::DAY_GROUPS.get(t.day).map(|g| g.0.to_string()).unwrap_or_default(),
            runs: t.day == group && runs_today,
            next_run: None,
            trips,
        });
    }
    let termini = l.directions.iter().filter(|d| d.stops.len() >= 2).flat_map(|d| [d.stops[0].name.clone(), d.stops[d.stops.len() - 1].name.clone()]).fold(Vec::new(), |mut v: Vec<String>, s| {
        if !v.contains(&s) {
            v.push(s);
        }
        v
    });
    LineInfo { name: stem.to_string(), user_allowed: true, termini, tours }
}

/// A planned trip as the planning knows it: its stops with their times.
fn trip_info(l: &LineDesign, stem: &str, p: &Planned, index: usize, run: &[f32], table: &[lines::TableTrip]) -> TripInfo {
    let d = l.directions.get(p.dir).cloned().unwrap_or_default();
    let times: Vec<f32> = match p.row.and_then(|r| table.get(r)) {
        Some(t) => t.times.clone(),
        None => {
            let rel = lines::fitted_times(&d);
            // (a trip of a time band of other running times: its minutes scaled to it)
            let own = rel.last().copied().unwrap_or(0.0).max(1.0);
            let want = p.minutes(run);
            rel.iter().map(|x| p.departure + x * want / own).collect()
        }
    };
    let secs = |m: f32| (m as f64 * 60.0).round();
    let stops: Vec<crate::StopInfo> = d.stops.iter().zip(times.iter()).map(|(s, &m)| StopInfo { name: s.name.clone(), id: s.id, arr: secs(m), dep: secs(m) }).collect();
    let departure = times.first().copied().map(secs).unwrap_or(0.0);
    let arrival = times.last().copied().map(secs).unwrap_or(departure);
    let km = if d.km() > 0.0 { d.km() } else { ((arrival - departure) / 60.0 * 0.35).max(0.5) };
    TripInfo {
        name: lines::trip_name(stem, p.dir),
        index,
        line: d.number_or(&l.number),
        from: d.stops.first().map(|s| s.name.clone()).unwrap_or_default(),
        terminus: d.destination(),
        departure,
        arrival,
        stops,
        km: (km * 10.0).round() / 10.0,
    }
}

/// The company's lines (`Company::lines`) made to match its network: one per line design that
/// can run, by its planning name - a line whose name changed renamed in the planning
/// (`rosterflow::rename_line`, `plan::rename_line`), a line gone dropped from the planning.
pub fn sync_lines(c: &mut Company, today: &str) {
    let stems = lines::stems(&c.network);
    let mut fresh: Vec<CompanyLine> = Vec::new();
    let mut renames: Vec<(String, String)> = Vec::new();
    for l in &c.network.lines {
        let Some(stem) = stems.get(&l.id) else {
            continue;
        };
        let name = l.timetable_name(stem);
        // (a line known by its old name: its number changed)
        let old = c
            .lines
            .iter()
            .find(|x| x.name.eq_ignore_ascii_case(&name))
            .or_else(|| c.lines.iter().find(|x| x.number.trim() == l.number.trim() && !stems.values().any(|s| s.eq_ignore_ascii_case(&x.name))));
        if let Some(o) = old {
            if !o.name.eq_ignore_ascii_case(&name) {
                renames.push((o.name.clone(), name.clone()));
            }
        }
        let mut numbers = vec![l.number.trim().to_string()];
        for d in &l.directions {
            let n = d.number_or(&l.number);
            if !numbers.contains(&n) {
                numbers.push(n);
            }
        }
        fresh.push(CompanyLine {
            name,
            number: l.number.trim().to_string(),
            numbers,
            colour: l.colour.clone(),
            caption: l.caption(),
            added: old.map(|o| o.added.clone()).unwrap_or_else(|| today.to_string()),
            tours: old.map(|o| o.tours).unwrap_or(0),
            km: old.map(|o| o.km).unwrap_or(0.0),
            title: l.title.clone(),
        });
    }
    for (old, new) in renames {
        crate::company::rosterflow::rename_line(c, &old, &new);
        crate::company::plan::rename_line(c, &old, &new);
    }
    c.lines = fresh;
}
