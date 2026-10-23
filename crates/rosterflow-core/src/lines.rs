//! The company's network as it designs it: its stops (`Stop`) and its lines (`LineDesign`) -
//! each line's directions with their stops and the minutes between them, the days it runs and
//! its timetable - kept with the company (`Company::network`, a `Registry`).
//!
//! A line's timetable is either made from its day patterns (first and last departure and how
//! often, or time bands with a headway each) or - `LineDesign::table_on` - a table of trips with
//! a time at every stop (`TableTrip`), which RosterFlow's Timeplan edits. Either way the trips
//! are chained into tours - one bus's day - by `tour_plan`: each departure goes to a bus ready
//! at that end of the line after its layover, else to a new bus. The tours are what the
//! planning starts from (`timetable::lines_on`); RosterFlow's Vehicles tab lays them out anew.

use crate::calendar::{HolidayPeriod, ServiceKind};
use serde::{Deserialize, Serialize};

/// The registry's format; a newer one is read as far as it is understood.
pub const REGISTRY_VERSION: u32 = 1;

/// The days of a pattern (bits 0 - 6 Monday to Sunday, 7 public holidays): working days,
/// Saturday, Sunday and public holidays.
pub const DAY_GROUPS: [(&str, u16); 3] = [("Mon - Fri", 0b0001_1111), ("Saturday", 0b0010_0000), ("Sunday", 0b1100_0000)];

/// The speed model for running times made from distances: an average on the road between two
/// stops and the time a stop takes (braking, the doors, pulling out).
pub const CRUISE_MS: f32 = 8.3;
pub const STOP_S: f32 = 20.0;

/// The company's network.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(default)]
pub struct Registry {
    pub version: u32,
    pub next_id: u64,
    pub lines: Vec<LineDesign>,
    /// The stops (halteplaatsen) the lines are made of.
    pub stops: Vec<Stop>,
    pub next_stop: i64,
    /// The school holidays (a school line does not run then).
    pub school_holidays: Vec<HolidayPeriod>,
    /// Public holidays besides the country's (they run Sunday's timetable).
    pub public_holidays: Vec<HolidayPeriod>,
}

impl Default for Registry {
    fn default() -> Self {
        Registry { version: REGISTRY_VERSION, next_id: 1, lines: Vec::new(), stops: Vec::new(), next_stop: 1, school_holidays: Vec::new(), public_holidays: Vec::new() }
    }
}

/// A stop of the network.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Default)]
#[serde(default)]
pub struct Stop {
    /// Stable for the life of the stop.
    pub id: i64,
    /// The stop's code (haltecode), as the timetables show it.
    pub code: String,
    pub name: String,
    /// Where it is (WGS84 degrees; 0, 0: not entered).
    pub lat: f64,
    pub lon: f64,
    /// Drivers may be relieved here (a terminus and the depot always may).
    pub relief: bool,
    pub notes: String,
}

/// A line of the company.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(default)]
pub struct LineDesign {
    /// Stable for the life of the line.
    pub id: u64,
    /// What the plate and the destination displays show.
    pub number: String,
    pub name: String,
    /// `#rrggbb`.
    pub colour: String,
    /// The size of bus its tours ask for (empty: any; `network::wants_size` reads it).
    pub ai_group: String,
    /// Outbound, then the way back (a line of one direction runs it round).
    pub directions: Vec<Direction>,
    /// One per `DAY_GROUPS` entry: when the outbound buses leave (and, without `back_days`, the
    /// return buses too).
    pub days: Vec<DayPattern>,
    /// The return direction's own day patterns, one per `DAY_GROUPS` entry. Empty: the return
    /// leaves as the outbound does (`pattern`).
    pub back_days: Vec<DayPattern>,
    /// Seconds since 1970.
    pub created: u64,
    pub modified: u64,
    /// What kind of service it is: on which days it runs.
    pub service: ServiceKind,
    /// The timetable as a table of trips (`TableTrip`), and whether it is the line's (else the
    /// day patterns make its trips).
    pub table: Vec<TableTrip>,
    pub table_on: bool,
    /// What the line is called in public; empty: its name.
    pub title: String,
    /// The depot its buses come from (empty: the company's main depot), and the empty run from
    /// there to the line's first stop and back from its last: minutes and kilometres.
    pub depot: String,
    pub depot_minutes: f32,
    pub depot_km: f32,
    /// The sizes of bus that may run it (none: any).
    pub sizes: Vec<crate::company::BusSize>,
    /// RosterFlow's timeplan of the line (`company::timeplan`): running times per time band,
    /// day group and direction, and the timepoints. None: the directions' own times.
    pub timeplan: Option<crate::company::timeplan::RunPlan>,
}

impl Default for LineDesign {
    fn default() -> Self {
        LineDesign {
            id: 0,
            number: String::new(),
            name: String::new(),
            colour: "#2a75f7".into(),
            ai_group: String::new(),
            directions: vec![Direction::default(), Direction::default()],
            days: default_days(),
            back_days: Vec::new(),
            created: 0,
            modified: 0,
            service: ServiceKind::Regular,
            table: Vec::new(),
            table_on: false,
            title: String::new(),
            depot: String::new(),
            depot_minutes: 8.0,
            depot_km: 3.0,
            sizes: Vec::new(),
            timeplan: None,
        }
    }
}

impl LineDesign {
    /// The day pattern direction `dir` leaves by on the group of days `day`: the return's own
    /// (`back_days`) when it has them, else the outbound's.
    pub fn pattern(&self, dir: usize, day: usize) -> Option<&DayPattern> {
        if dir == 1 && !self.back_days.is_empty() {
            self.back_days.get(day).or_else(|| self.days.get(day))
        } else {
            self.days.get(day)
        }
    }

    /// The return has day patterns of its own (`back_days`).
    pub fn back_own(&self) -> bool {
        !self.back_days.is_empty()
    }

    /// The return given day patterns of its own - the outbound's as they are - or made to leave
    /// as the outbound does again.
    pub fn set_back_own(&mut self, own: bool) {
        if own && self.back_days.is_empty() {
            self.back_days = self.days.clone();
        } else if !own {
            self.back_days.clear();
        }
    }

    /// The day patterns direction `dir` leaves by, to change (one per `DAY_GROUPS` entry).
    pub fn patterns_mut(&mut self, dir: usize) -> &mut Vec<DayPattern> {
        if self.days.len() < DAY_GROUPS.len() {
            self.days = default_days();
        }
        if dir == 1 && !self.back_days.is_empty() {
            if self.back_days.len() < DAY_GROUPS.len() {
                let more: Vec<DayPattern> = self.days[self.back_days.len()..].to_vec();
                self.back_days.extend(more);
            }
            &mut self.back_days
        } else {
            &mut self.days
        }
    }

    /// The day patterns of each direction with stops on the group of days `day` (two at the
    /// most), as `blocks_each` takes them.
    pub fn day_patterns(&self, day: usize) -> Vec<DayPattern> {
        let dirs = self.directions.iter().filter(|d| d.stops.len() >= 2).count().clamp(1, 2);
        (0..dirs).filter_map(|d| self.pattern(d, day).cloned()).collect()
    }

    /// The table's trips of the group of days `day` in the directions `dirs` made anew from the
    /// patterns (`table_from_patterns`); the other directions' and days' stay as they are.
    pub fn refill_table(&mut self, day: usize, dirs: &[usize]) {
        let fresh = table_from_patterns(self);
        self.table.retain(|t| !(t.day == day && dirs.contains(&t.dir)));
        self.table.extend(fresh.into_iter().filter(|t| t.day == day && dirs.contains(&t.dir)));
        self.table.sort_by(|a, b| a.day.cmp(&b.day).then(a.departure().total_cmp(&b.departure())).then(a.dir.cmp(&b.dir)));
    }

    /// Its buses go round: a line of one direction, or one whose directions are all loops - a
    /// bus at the end of a trip stands where every trip starts.
    pub fn goes_round(&self) -> bool {
        let with: Vec<&Direction> = self.directions.iter().filter(|d| d.stops.len() >= 2).collect();
        with.len() <= 1 || with.iter().all(|d| d.is_loop())
    }

    /// The planning's name of the line (`CompanyLine::name`): its stem (`stems`).
    pub fn timetable_name(&self, stem: &str) -> String {
        stem.to_string()
    }

    /// The line's name for people: its title, else its name, else where it goes.
    pub fn caption(&self) -> String {
        if !self.title.trim().is_empty() {
            return self.title.trim().to_string();
        }
        if !self.name.trim().is_empty() {
            return self.name.trim().to_string();
        }
        let ends: Vec<String> = self.directions.iter().filter(|d| d.stops.len() >= 2).map(|d| d.destination()).collect();
        match ends.as_slice() {
            [a, b, ..] => format!("{b} - {a}"),
            [a] => a.clone(),
            [] => String::new(),
        }
    }
}

/// A stop of a direction: the network's stop (`Stop::id`) and its name.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Default)]
#[serde(default)]
pub struct StopRef {
    pub id: i64,
    pub name: String,
}

/// The way from one stop to the next.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Default)]
#[serde(default)]
pub struct Leg {
    /// Metres from stop to stop.
    pub length: f32,
}

/// One direction of a line.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Default)]
#[serde(default)]
pub struct Direction {
    /// The destination its displays show: the last stop's name when empty.
    pub terminus: String,
    pub stops: Vec<StopRef>,
    /// `stops.len() - 1` of them.
    pub legs: Vec<Leg>,
    /// Minutes from the first stop to every stop (the first 0).
    pub times: Vec<f32>,
    /// The times were set by hand (else they follow the legs: `auto_times`).
    pub manual_times: bool,
    /// The number its trips show when it is not the line's; empty: the line's.
    pub number: String,
}

/// When a line runs on the days of one `DAY_GROUPS` entry: buses from the first to the last
/// departure every `headway` minutes, each standing at least `layover` at the end - or, with
/// `bands`, a headway of its own for each part of the day.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(default)]
pub struct DayPattern {
    pub on: bool,
    pub days: u16,
    /// Minutes after midnight.
    pub first: f32,
    pub last: f32,
    pub headway: f32,
    pub layover: f32,
    /// The time bands (none: `first` to `last` every `headway`).
    pub bands: Vec<TimeBand>,
}

impl Default for DayPattern {
    fn default() -> Self {
        DayPattern { on: true, days: DAY_GROUPS[0].1, first: 6.0 * 60.0, last: 22.0 * 60.0, headway: 20.0, layover: 5.0, bands: Vec::new() }
    }
}

/// A part of the day with its own headway: departures from `from` every `headway` minutes, the
/// last before `to` (minutes after midnight; a band ends where the next begins).
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq)]
#[serde(default)]
pub struct TimeBand {
    pub from: f32,
    pub to: f32,
    pub headway: f32,
    /// The bus size the band wants (None: the line's).
    pub size: Option<crate::company::BusSize>,
}

impl Default for TimeBand {
    fn default() -> Self {
        TimeBand { from: 6.0 * 60.0, to: 9.0 * 60.0, headway: 15.0, size: None }
    }
}

/// What a loop shows as its destination when it is named nothing else.
pub const RING: &str = "Ring";

/// The latest a band may end.
pub const DAY_END: f32 = 26.0 * 60.0;

/// The bands a day group starts from: the working days with their two rush hours, Saturday's
/// busy middle of the day, Sunday's quiet one.
pub fn default_bands(group: usize) -> Vec<TimeBand> {
    let b = |from: f32, to: f32, headway: f32| TimeBand { from: from * 60.0, to: to * 60.0, headway, size: None };
    match group {
        0 => vec![b(5.0, 6.0, 30.0), b(6.0, 9.0, 10.0), b(9.0, 15.0, 15.0), b(15.0, 18.0, 10.0), b(18.0, 21.0, 20.0), b(21.0, 24.0, 30.0)],
        1 => vec![b(6.0, 9.0, 30.0), b(9.0, 18.0, 20.0), b(18.0, 24.0, 30.0)],
        _ => vec![b(8.0, 20.0, 30.0), b(20.0, 24.0, 60.0)],
    }
}

impl DayPattern {
    /// The bands in order of the day, each ending where the next begins.
    pub fn clean_bands(&self) -> Vec<TimeBand> {
        let mut v: Vec<TimeBand> = self.bands.iter().copied().filter(|b| b.headway >= 1.0).collect();
        v.sort_by(|a, b| a.from.total_cmp(&b.from));
        for i in 0..v.len() {
            let next = v.get(i + 1).map(|n| n.from).unwrap_or(DAY_END);
            v[i].to = v[i].to.min(next).min(DAY_END);
        }
        v.retain(|b| b.to > b.from);
        v
    }

    /// Every departure of a direction (minutes after midnight), with the band it is in.
    pub fn departures(&self) -> Vec<(f32, Option<usize>)> {
        let mut out = Vec::new();
        if !self.on {
            return out;
        }
        if self.bands.is_empty() {
            if self.headway < 1.0 || self.last < self.first {
                return out;
            }
            let mut t = self.first;
            while t <= self.last + 1e-3 && out.len() < 2000 {
                out.push((t, None));
                t += self.headway;
            }
            return out;
        }
        for (i, b) in self.clean_bands().iter().enumerate() {
            let mut t = b.from;
            while t < b.to - 1e-3 && out.len() < 2000 {
                out.push((t, Some(i)));
                t += b.headway;
            }
        }
        out
    }

    /// It runs (it has a departure).
    pub fn runs(&self) -> bool {
        !self.departures().is_empty()
    }
}

/// Working days every 20 minutes, Saturdays every 30, Sundays every 60.
pub fn default_days() -> Vec<DayPattern> {
    vec![
        DayPattern { days: DAY_GROUPS[0].1, ..Default::default() },
        DayPattern { days: DAY_GROUPS[1].1, first: 7.0 * 60.0, headway: 30.0, ..Default::default() },
        DayPattern { days: DAY_GROUPS[2].1, first: 8.0 * 60.0, last: 21.0 * 60.0, headway: 60.0, ..Default::default() },
    ]
}

/// The timetable a line of `kind` starts from: a school line's trips before school and after
/// it, a weekend line's every hour in the daytime, a regular line's with the rush hours.
pub fn days_for(kind: ServiceKind) -> Vec<DayPattern> {
    let b = |from: f32, to: f32, headway: f32| TimeBand { from: from * 60.0, to: to * 60.0, headway, size: None };
    let mut days = default_days();
    match kind {
        ServiceKind::Regular => {
            for (k, p) in days.iter_mut().enumerate() {
                p.bands = default_bands(k);
            }
        }
        ServiceKind::School => {
            days[0] = DayPattern { days: DAY_GROUPS[0].1, layover: 5.0, bands: vec![b(6.5, 8.25, 15.0), b(12.75, 16.5, 30.0)], ..Default::default() };
            days[1].on = false;
            days[2].on = false;
        }
        ServiceKind::Leisure => {
            days[0].on = false;
            days[1] = DayPattern { days: DAY_GROUPS[1].1, first: 9.0 * 60.0, last: 19.0 * 60.0, headway: 60.0, ..Default::default() };
            days[2] = DayPattern { days: DAY_GROUPS[2].1, first: 9.0 * 60.0, last: 19.0 * 60.0, headway: 60.0, ..Default::default() };
        }
        ServiceKind::OnDemand => {
            days[0] = DayPattern { days: DAY_GROUPS[0].1, first: 6.0 * 60.0, last: 22.0 * 60.0, headway: 60.0, ..Default::default() };
            days[1] = DayPattern { days: DAY_GROUPS[1].1, first: 7.0 * 60.0, last: 22.0 * 60.0, headway: 60.0, ..Default::default() };
            days[2] = DayPattern { days: DAY_GROUPS[2].1, first: 8.0 * 60.0, last: 21.0 * 60.0, headway: 60.0, ..Default::default() };
        }
    }
    days
}

pub fn now_secs() -> u64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0)
}

impl Registry {
    /// A new line, numbered after the last, with the company's typical timetable.
    pub fn add_line(&mut self, number: &str) -> &mut LineDesign {
        let id = self.next_id.max(1);
        self.next_id = id + 1;
        let now = now_secs();
        let colour = crate::company::rosterflow::hex(crate::company::rosterflow::PALETTE[self.lines.len() % crate::company::rosterflow::PALETTE.len()]);
        self.lines.push(LineDesign { id, number: number.trim().to_string(), colour, created: now, modified: now, days: days_for(ServiceKind::Regular), ..Default::default() });
        self.lines.last_mut().unwrap()
    }

    pub fn line(&self, id: u64) -> Option<&LineDesign> {
        self.lines.iter().find(|l| l.id == id)
    }

    pub fn line_mut(&mut self, id: u64) -> Option<&mut LineDesign> {
        self.lines.iter_mut().find(|l| l.id == id)
    }

    pub fn stop(&self, id: i64) -> Option<&Stop> {
        self.stops.iter().find(|s| s.id == id)
    }

    /// A new stop; its id.
    pub fn add_stop(&mut self, code: &str, name: &str) -> i64 {
        let id = self.next_stop.max(self.stops.iter().map(|s| s.id).max().unwrap_or(0) + 1);
        self.next_stop = id + 1;
        self.stops.push(Stop { id, code: code.trim().to_string(), name: name.trim().to_string(), ..Default::default() });
        id
    }

    /// Every direction's stop names made the network's again (a stop renamed).
    pub fn rename_stops(&mut self) {
        let names: std::collections::HashMap<i64, String> = self.stops.iter().map(|s| (s.id, s.name.clone())).collect();
        for l in &mut self.lines {
            for d in &mut l.directions {
                for s in &mut d.stops {
                    if let Some(n) = names.get(&s.id) {
                        s.name = n.clone();
                    }
                }
            }
        }
    }

    /// Lines that use stop `id`.
    pub fn lines_with_stop(&self, id: i64) -> Vec<String> {
        self.lines.iter().filter(|l| l.directions.iter().any(|d| d.stops.iter().any(|s| s.id == id))).map(|l| l.number.clone()).collect()
    }
}

// --- times ------------------------------------------------------------------------------------

/// Seconds a leg of `length` metres takes (`CRUISE_MS`, plus `STOP_S` for the stop it ends at).
pub fn leg_seconds(length: f32) -> f32 {
    STOP_S + length.max(0.0) / CRUISE_MS
}

/// Minutes from the first stop to every stop, from the legs' lengths, in whole minutes.
pub fn auto_times(legs: &[Leg]) -> Vec<f32> {
    let mut out = vec![0.0f32];
    let mut acc = 0.0f32;
    for l in legs {
        acc += leg_seconds(l.length);
        let m = (acc / 60.0).round().max(out.last().copied().unwrap_or(0.0));
        out.push(m);
    }
    if out.len() > 1 && out.last().copied().unwrap_or(0.0) < 1.0 {
        *out.last_mut().unwrap() = 1.0;
    }
    out
}

impl Direction {
    /// Its times again from the legs, unless they were set by hand (and then only made to fit
    /// the stops there are).
    pub fn refresh_times(&mut self) {
        let n = self.stops.len();
        if self.manual_times && !self.times.is_empty() {
            // (a stop added at the end: two minutes after the one before)
            while self.times.len() < n {
                let last = self.times.last().copied().unwrap_or(0.0);
                self.times.push(last + 2.0);
            }
            self.times.truncate(n);
        } else {
            self.times = auto_times(&self.legs);
            self.times.resize(n, self.times.last().copied().unwrap_or(0.0));
            self.manual_times = false;
        }
    }

    /// Minutes from the first stop to the last.
    pub fn minutes(&self) -> f32 {
        self.times.last().copied().unwrap_or(0.0)
    }

    /// Kilometres from the first stop to the last.
    pub fn km(&self) -> f64 {
        self.legs.iter().map(|l| l.length as f64).sum::<f64>() / 1000.0
    }

    /// The whole trip made to take `total` minutes: every stop's time scaled with it.
    pub fn set_total(&mut self, total: f32) {
        let old = self.minutes();
        if total < 1.0 || old <= 0.0 {
            return;
        }
        for t in &mut self.times {
            *t = (*t * total / old).round();
        }
        if let Some(l) = self.times.last_mut() {
            *l = total.round();
        }
        self.manual_times = true;
    }

    /// The legs fit the stops (one fewer), new ones empty.
    pub fn fit_legs(&mut self) {
        let n = self.stops.len().saturating_sub(1);
        self.legs.resize(n, Leg::default());
    }

    /// It is a loop: at least three stops, the last the first again.
    pub fn is_loop(&self) -> bool {
        match (self.stops.first(), self.stops.last()) {
            (Some(a), Some(b)) => self.stops.len() >= 3 && a.id == b.id,
            _ => false,
        }
    }

    /// The destination it shows: the terminus chosen, else the last stop's name.
    pub fn destination(&self) -> String {
        if self.terminus.trim().is_empty() {
            if self.is_loop() {
                return RING.to_string();
            }
            self.stops.last().map(|s| s.name.trim().to_string()).unwrap_or_default()
        } else {
            self.terminus.trim().to_string()
        }
    }

    /// The number its trips show: its own, else the line's.
    pub fn number_or(&self, line: &str) -> String {
        if self.number.trim().is_empty() {
            line.trim().to_string()
        } else {
            self.number.trim().to_string()
        }
    }
}

// --- tours from a pattern ------------------------------------------------------------------

/// A trip of a bus's day: the direction, its departure (minutes after midnight), the time band
/// it is in; a table's trip its own minutes.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Planned {
    pub dir: usize,
    pub departure: f32,
    pub band: Option<usize>,
    /// Minutes the trip takes (0: its direction's, `run_minutes`).
    pub run: f32,
    /// Its place in the table (`table_trips`), for its own times; None: a pattern's.
    pub row: Option<usize>,
}

impl Planned {
    /// Minutes it takes: its own, else its direction's.
    pub fn minutes(&self, run: &[f32]) -> f32 {
        if self.run > 0.0 { self.run } else { run.get(self.dir).copied().unwrap_or(1.0) }.max(1.0)
    }
}

/// A bus standing longer than this at the end (minutes) goes back to the depot between its
/// trips: the day of a bus that is wanted only in the rush hours is two tours, not one.
pub const PARK: f32 = 90.0;

/// The buses a pattern needs and the trips of each, in order (`blocks_each` with one pattern).
pub fn blocks(run: &[f32], p: &DayPattern) -> Vec<Vec<Planned>> {
    blocks_each(run, std::slice::from_ref(p), false)
}

/// The buses the patterns need (`pats`, one per direction, the outbound's first): every
/// direction leaves at its departures, and each departure is given to a bus ready at that end
/// (its trip and the `layover` after it), else to a new bus. `round`: every trip starts where
/// the one before ended.
pub fn blocks_each(run: &[f32], pats: &[DayPattern], round: bool) -> Vec<Vec<Planned>> {
    if run.is_empty() || pats.is_empty() {
        return Vec::new();
    }
    let dirs = run.len().clamp(1, 2);
    let pat = |d: usize| pats.get(d).unwrap_or(&pats[0]);
    let banded = (0..dirs).any(|d| !pat(d).bands.is_empty());
    let mut deps: Vec<Planned> = Vec::new();
    for dir in 0..dirs {
        deps.extend(pat(dir).departures().into_iter().take(4000 / dirs).map(|(departure, band)| Planned { dir, departure, band, run: 0.0, row: None }));
    }
    let layovers: Vec<f32> = (0..dirs).map(|d| pat(d).layover).collect();
    chain(deps, run, dirs, &layovers, banded, round)
}

/// Each trip given to a bus ready at the end it leaves from, else to a new bus: the one that
/// has stood longest, or - `first_out` - the one that came out first.
fn chain(mut deps: Vec<Planned>, run: &[f32], dirs: usize, layover: &[f32], first_out: bool, round: bool) -> Vec<Vec<Planned>> {
    deps.sort_by(|a, b| a.departure.total_cmp(&b.departure).then(a.dir.cmp(&b.dir)));
    let one_end = dirs == 1 || round;
    let from = |d: usize| if one_end { 0 } else { d };
    let to = |d: usize| if one_end { 0 } else { 1 - d };
    let stands = |d: usize| layover.get(d).or(layover.first()).copied().unwrap_or(5.0).max(0.0);
    struct Bus {
        at: usize,
        free: f32,
        trips: Vec<Planned>,
    }
    let mut buses: Vec<Bus> = Vec::new();
    for d in deps {
        let mut ready_buses = buses.iter().enumerate().filter(|(_, b)| b.at == from(d.dir) && b.free <= d.departure + 1e-3);
        let ready = if first_out { ready_buses.next().map(|(i, _)| i) } else { ready_buses.min_by(|a, b| a.1.free.total_cmp(&b.1.free)).map(|(i, _)| i) };
        let i = ready.unwrap_or_else(|| {
            buses.push(Bus { at: from(d.dir), free: 0.0, trips: Vec::new() });
            buses.len() - 1
        });
        let b = &mut buses[i];
        b.trips.push(d);
        b.at = to(d.dir);
        b.free = d.departure + d.minutes(run) + stands(d.dir);
    }
    buses.into_iter().map(|b| b.trips).collect()
}

/// The tours of a pattern: each bus's day (`blocks`), cut where - with time bands - it stands
/// longer than `PARK` at the end.
pub fn tours(run: &[f32], p: &DayPattern) -> Vec<Vec<Planned>> {
    tours_each(run, std::slice::from_ref(p), false)
}

/// `tours` with a day pattern for each direction.
pub fn tours_each(run: &[f32], pats: &[DayPattern], round: bool) -> Vec<Vec<Planned>> {
    let all = blocks_each(run, pats, round);
    if pats.iter().all(|p| p.bands.is_empty()) {
        return all;
    }
    cut_parked(all, run)
}

/// Each bus's day cut where it stands longer than `PARK` at the end, in order of the tours'
/// first departures.
fn cut_parked(all: Vec<Vec<Planned>>, run: &[f32]) -> Vec<Vec<Planned>> {
    let mut out = Vec::new();
    for bus in all {
        let mut cur: Vec<Planned> = Vec::new();
        for t in bus {
            if let Some(last) = cur.last() {
                let back = last.departure + last.minutes(run);
                if t.departure - back > PARK {
                    out.push(std::mem::take(&mut cur));
                }
            }
            cur.push(t);
        }
        if !cur.is_empty() {
            out.push(cur);
        }
    }
    out.sort_by(|a, b| a[0].departure.total_cmp(&b[0].departure));
    out
}

/// A tour of a line: its number, the day group (an index into `DAY_GROUPS`) and its trips.
#[derive(Clone, Debug, PartialEq)]
pub struct PlannedTour {
    pub number: String,
    pub day: usize,
    pub trips: Vec<Planned>,
}

impl PlannedTour {
    /// When its first trip leaves and its last arrives (minutes).
    pub fn span(&self, run: &[f32]) -> (f32, f32) {
        let a = self.trips.first().map(|t| t.departure).unwrap_or(0.0);
        let z = self.trips.iter().map(|t| t.departure + t.minutes(run)).fold(a, f32::max);
        (a, z)
    }
}

/// Minutes each direction of a line takes from its first stop to its last (one entry per
/// direction with stops).
pub fn run_minutes(l: &LineDesign) -> Vec<f32> {
    l.directions.iter().filter(|d| d.stops.len() >= 2).map(|d| fitted_times(d).last().copied().unwrap_or(1.0).max(1.0)).collect()
}

/// Every tour of a line, numbered on through the day groups (the groups its kind of service
/// does not run on left out) - of its table when it has one on (`table_tours`).
pub fn tour_plan(l: &LineDesign) -> Vec<PlannedTour> {
    if l.table_on {
        return table_tours(l);
    }
    let run = run_minutes(l);
    let mut out: Vec<PlannedTour> = Vec::new();
    for day in 0..l.days.len() {
        if !l.service.allows(day) {
            continue;
        }
        for trips in tours_each(&run, &l.day_patterns(day), l.goes_round()) {
            out.push(PlannedTour { number: (out.len() + 1).to_string(), day, trips });
        }
    }
    out
}

// --- the timetable as a table -------------------------------------------------------------------

/// A trip of a line's timetable table: its direction, its group of days (an index into
/// `DAY_GROUPS`) and its time at every stop (minutes after midnight).
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Default)]
#[serde(default)]
pub struct TableTrip {
    pub dir: usize,
    pub day: usize,
    pub times: Vec<f32>,
    /// A note of the planner's (kept with the trip; the Timeplan's "tour").
    pub tour: String,
}

impl TableTrip {
    pub fn departure(&self) -> f32 {
        self.times.first().copied().unwrap_or(0.0)
    }

    /// Minutes from its first stop to its last.
    pub fn minutes(&self) -> f32 {
        self.times.last().copied().unwrap_or(0.0) - self.departure()
    }

    /// The first stop it would reach before it left the stop before (None: its times run
    /// forward along the stops).
    pub fn first_fall(&self) -> Option<usize> {
        (1..self.times.len()).find(|&i| self.times[i] < self.times[i - 1] - 1e-3)
    }

    /// The whole trip `by` minutes later (earlier when negative; not before midnight).
    pub fn shift(&mut self, by: f32) {
        let by = by.max(-self.departure());
        for t in &mut self.times {
            *t += by;
        }
    }

    /// Its times from its departure on.
    pub fn relative(&self) -> Vec<f32> {
        let a = self.departure();
        self.times.iter().map(|t| t - a).collect()
    }
}

/// Minutes from the first stop to every stop of a direction: its own, else made from its legs.
pub fn fitted_times(d: &Direction) -> Vec<f32> {
    let mut times = d.times.clone();
    if times.len() != d.stops.len() {
        times = auto_times(&d.legs);
        times.resize(d.stops.len(), times.last().copied().unwrap_or(0.0));
    }
    times
}

/// A table trip's times as its direction's stops are now.
pub fn trip_times(l: &LineDesign, t: &TableTrip) -> Vec<f32> {
    match l.directions.get(t.dir) {
        Some(d) if t.times.len() != d.stops.len() => fitted_times(d).iter().map(|x| t.departure() + x).collect(),
        _ => t.times.clone(),
    }
}

/// The trip `t` of a line's table runs on the group of days `group`.
pub fn trip_in_group(l: &LineDesign, t: &TableTrip, group: usize) -> bool {
    t.day == group && l.service.allows(group)
}

/// The table the day patterns make: every departure of every group of days the line's kind of
/// service runs on, in each direction, with the direction's times to the stops.
pub fn table_from_patterns(l: &LineDesign) -> Vec<TableTrip> {
    let dirs: Vec<(usize, Vec<f32>)> = l.directions.iter().enumerate().filter(|(_, d)| d.stops.len() >= 2).map(|(k, d)| (k, fitted_times(d))).take(2).collect();
    let mut out = Vec::new();
    for day in (0..l.days.len()).filter(|k| l.service.allows(*k)) {
        for (dir, rel) in &dirs {
            let Some(p) = l.pattern(*dir, day) else {
                continue;
            };
            for (dep, _) in p.departures() {
                out.push(TableTrip { dir: *dir, day, times: rel.iter().map(|x| dep + x).collect(), tour: String::new() });
            }
        }
    }
    out.sort_by(|a, b| a.day.cmp(&b.day).then(a.departure().total_cmp(&b.departure())).then(a.dir.cmp(&b.dir)));
    out
}

/// The trips of the table the line drives: of the groups of days its kind runs on and of a
/// direction it has, fitted to the stops, in order of departure.
pub fn table_trips(l: &LineDesign) -> Vec<TableTrip> {
    let dirs = l.directions.iter().filter(|d| d.stops.len() >= 2).count().min(2);
    let known = |t: &TableTrip| t.dir < dirs && t.day < DAY_GROUPS.len() && l.service.allows(t.day);
    let mut out: Vec<TableTrip> = l.table.iter().filter(|t| known(t) && !t.times.is_empty()).map(|t| TableTrip { times: trip_times(l, t), ..t.clone() }).collect();
    out.sort_by(|a, b| a.day.cmp(&b.day).then(a.departure().total_cmp(&b.departure())).then(a.dir.cmp(&b.dir)));
    out
}

/// The tours of a line's table: per group of days its trips given to buses as the day
/// patterns' are, each bus standing at least the group's layover at the end, and a bus that
/// stands longer than `PARK` sent back to the depot in between.
pub fn table_tours(l: &LineDesign) -> Vec<PlannedTour> {
    let trips = table_trips(l);
    let dirs = l.directions.iter().filter(|d| d.stops.len() >= 2).count().clamp(1, 2);
    let mut out: Vec<PlannedTour> = Vec::new();
    for day in 0..DAY_GROUPS.len() {
        let deps: Vec<Planned> =
            trips.iter().enumerate().filter(|(_, t)| t.day == day).map(|(row, t)| Planned { dir: t.dir, departure: t.departure(), band: None, run: t.minutes().max(1.0), row: Some(row) }).collect();
        if deps.is_empty() {
            continue;
        }
        let layover = l.days.get(day).map(|p| p.layover).unwrap_or(5.0);
        let layovers: Vec<f32> = (0..dirs).map(|d| l.pattern(d, day).map(|p| p.layover).unwrap_or(layover)).collect();
        let chained = chain(deps, &[], dirs, &layovers, false, l.goes_round());
        let mut buses = cut_parked(chained, &[]);
        buses.sort_by(|a, b| a[0].departure.total_cmp(&b[0].departure));
        for trips in buses {
            out.push(PlannedTour { number: (out.len() + 1).to_string(), day, trips });
        }
    }
    out
}

/// Departures a group of days has in each hour, all directions together.
pub fn hourly_departures(l: &LineDesign, day: usize) -> [u32; 24] {
    let mut out = [0u32; 24];
    if !l.service.allows(day) {
        return out;
    }
    let hour = |t: f32| (t / 60.0).floor().max(0.0) as usize % 24;
    if l.table_on {
        for t in table_trips(l).iter().filter(|t| trip_in_group(l, t, day)) {
            out[hour(t.departure())] += 1;
        }
    } else if l.days.get(day).is_some() {
        let dirs = run_minutes(l).len().clamp(1, 2);
        for dir in 0..dirs {
            for (t, _) in l.pattern(dir, day).map(|p| p.departures()).unwrap_or_default() {
                out[hour(t)] += 1;
            }
        }
    }
    out
}

/// The line runs on the group of days at all.
pub fn runs_on_group(l: &LineDesign, day: usize) -> bool {
    hourly_departures(l, day).iter().any(|n| *n > 0)
}

/// The planning's name of each line (`CompanyLine::name`): `L<number>`, with its id after it
/// when a line before it has that name already - the oldest line first, so that a line keeps
/// its name when another takes its number.
pub fn stems(reg: &Registry) -> std::collections::HashMap<u64, String> {
    let mut order: Vec<&LineDesign> = reg.lines.iter().collect();
    order.sort_by_key(|l| l.id);
    let mut taken: std::collections::HashSet<String> = std::collections::HashSet::new();
    let mut out = std::collections::HashMap::new();
    for l in order {
        let plain = format!("L{}", safe_name(&l.number));
        let mut stem = plain.clone();
        if !taken.insert(stem.to_lowercase()) {
            stem = format!("{plain}_{}", l.id);
            taken.insert(stem.to_lowercase());
        }
        out.insert(l.id, stem);
    }
    out
}

/// A name with the characters a key cannot hold taken out.
pub fn safe_name(s: &str) -> String {
    let t: String = s.trim().chars().map(|c| if c.is_alphanumeric() || matches!(c, '-' | '_' | '+' | '.') { c } else { '_' }).collect();
    let t = t.trim_matches('.').to_string();
    if t.is_empty() {
        "line".into()
    } else {
        t
    }
}

/// The trip name of direction `dir` of a line with planning name `stem`: `_a` outbound, `_b`
/// back.
pub fn trip_name(stem: &str, dir: usize) -> String {
    match dir {
        0..=25 => format!("{stem}_{}", (b'a' + dir as u8) as char),
        _ => format!("{stem}_{dir}"),
    }
}

// --- checks -------------------------------------------------------------------------------------

/// Why a line cannot run yet: the text (an interface text, `%{n}` for the number) and the
/// number.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Problem {
    pub text: &'static str,
    pub n: usize,
}

impl Problem {
    pub fn english(&self) -> String {
        self.text.replace("%{n}", &self.n.to_string())
    }
}

/// The problem of a line without a single departure.
pub const NO_DAY: &str = "The line runs on no day";

/// The line has a departure on some day.
pub fn has_departures(l: &LineDesign) -> bool {
    if l.table_on {
        !table_trips(l).is_empty()
    } else {
        (0..l.days.len()).any(|k| l.service.allows(k) && l.day_patterns(k).iter().any(DayPattern::runs))
    }
}

/// Why the line's route is not complete: `problems` but its timetable's.
pub fn route_problems(l: &LineDesign) -> Vec<Problem> {
    problems(l).into_iter().filter(|p| p.text != NO_DAY).collect()
}

/// Why a line cannot run yet (none: it can).
pub fn problems(l: &LineDesign) -> Vec<Problem> {
    let mut out = Vec::new();
    let p = |text, n| Problem { text, n };
    if l.number.trim().is_empty() {
        out.push(p("The line needs a number", 0));
    }
    match l.directions.first() {
        Some(d) if d.stops.len() >= 2 => {}
        _ => out.push(p("The outbound direction needs at least two stops", 0)),
    }
    if let Some(d) = l.directions.get(1) {
        if d.stops.len() == 1 {
            out.push(p("The way back needs at least two stops", 0));
        }
    }
    if !has_departures(l) {
        out.push(p(NO_DAY, 0));
    }
    let falling = if l.table_on { table_trips(l).iter().filter(|t| t.first_fall().is_some()).count() } else { 0 };
    if falling > 0 {
        out.push(p("%{n} trip(s) of the table reach a stop before they left the one before", falling));
    }
    out
}

/// The line runs: it has a route and departures.
pub fn written(l: &LineDesign) -> bool {
    problems(l).is_empty()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn line() -> LineDesign {
        let s = |id: i64, n: &str| StopRef { id, name: n.into() };
        let out = Direction { stops: vec![s(1, "Station"), s(2, "Markt"), s(3, "Ziekenhuis")], times: vec![0.0, 6.0, 15.0], manual_times: true, ..Default::default() };
        let back = Direction { stops: vec![s(3, "Ziekenhuis"), s(2, "Markt"), s(1, "Station")], times: vec![0.0, 9.0, 15.0], manual_times: true, ..Default::default() };
        LineDesign { id: 1, number: "5".into(), directions: vec![out, back], days: default_days(), ..Default::default() }
    }

    #[test]
    fn a_line_makes_its_tours() {
        let l = line();
        assert!(problems(&l).is_empty());
        // every 20 minutes, 15 minutes a way and at least 5 standing: a bus every 40 minutes
        // a round - two buses on the working days
        let tours = tour_plan(&l);
        let monday: Vec<&PlannedTour> = tours.iter().filter(|t| t.day == 0).collect();
        assert_eq!(monday.len(), 2);
        assert!(monday[0].trips.windows(2).all(|w| w[0].dir != w[1].dir));
        assert_eq!(l.caption(), "Station - Ziekenhuis");
        // the table made from the patterns gives the same buses
        let mut t = l.clone();
        t.table = table_from_patterns(&t);
        t.table_on = true;
        assert_eq!(tour_plan(&t).iter().filter(|x| x.day == 0).count(), 2);
        assert_eq!(hourly_departures(&t, 0)[7], 6);
    }

    #[test]
    fn a_line_without_stops_cannot_run() {
        let l = LineDesign { number: "7".into(), ..Default::default() };
        assert!(!problems(&l).is_empty());
        let mut reg = Registry::default();
        let a = reg.add_stop("1001", "A");
        assert_eq!(reg.stop(a).unwrap().name, "A");
        reg.add_line("7");
        reg.add_line("7");
        let stems = stems(&reg);
        assert_eq!(stems[&1], "L7");
        assert_eq!(stems[&2], "L7_2");
    }
}
