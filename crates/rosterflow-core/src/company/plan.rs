//! The planning: who drives what, day by day.
//!
//! The company's lines give it tours every day (`network::tours_of_day`), each cut into
//! duties (`network::duties_of`). The roster is fixed per weekday and repeats every week: a bus
//! for a tour, a driver for each duty. "Fill the roster" asks the dispatcher to fill what the
//! roster leaves free (the free buses, the free drivers by the working-time rules,
//! `staff::check`) and fixes it in the roster for the planner to see. What falls out on the day
//! is open: someone ill, on holiday or late, a bus in the workshop or broken down (reported by
//! the dispatcher, `Planning::reported`). The dispatcher's choice for it counts (a colleague, an
//! agency driver, another bus, a hired bus, or dropping its trips); for what came suddenly the
//! plan proposes a free colleague by itself (`Source::Central`).

use super::dates;
use super::model::{BusKind, BusSize, Cents, Company, Drive, Employee, Licence};
use super::network::{self, TourOfDay};
use super::staff::{self, Block, BUS_MARGIN};
use serde::{Deserialize, Serialize};

/// The driver id an agency driver stands under in a day's plan.
pub const AGENCY: u32 = u32::MAX;
/// The driver id a spare line's driver stands under as a late driver's cover (`LatePiece::cover`):
/// this and the spare duty's id (`rosterflow::Spare`, `spare_lines`) - under the agency's booked
/// drivers, over any employee.
pub const SPARE_COVER: u32 = 0x9000_0000;

/// The spare duty (its id) a late driver's cover `id` is, if it is a spare line's driver.
pub fn spare_of_cover(id: u32) -> Option<u32> {
    (SPARE_COVER..super::agency::BOOKED_BASE).contains(&id).then(|| id - SPARE_COVER)
}

/// The agency drivers a day's plan takes for what fell out or stayed free (not those fixed in the
/// roster, nor those booked ahead: `agency`).
pub fn agency_used(dp: &DayPlan) -> usize {
    dp.tours.iter().flat_map(|t| t.duties.iter()).filter(|d| d.who == Some(Who::Agency) && matches!(d.from_, Source::Dispatcher | Source::Central)).count()
}

/// Whether duty `k` of tour `ti` has an agency driver sent for the day (not the roster's).
fn agency_sent_for(dp: &DayPlan, ti: usize, k: usize) -> bool {
    dp.tours.get(ti).and_then(|t| t.duties.get(k)).is_some_and(|d| d.who == Some(Who::Agency) && matches!(d.from_, Source::Dispatcher | Source::Central))
}

/// `agency_used` by the duty: the pieces joined into one driver's duty in the studio take one
/// agency driver together, not one for each piece.
pub fn agency_sent(c: &Company, dp: &DayPlan) -> usize {
    let slot = super::plandays::slot_of(c, &dp.date);
    let mut seen: Vec<(usize, usize)> = Vec::new();
    let mut n = 0;
    for (i, t) in dp.tours.iter().enumerate() {
        for k in 0..t.duties.len() {
            if !agency_sent_for(dp, i, k) || seen.contains(&(i, k)) {
                continue;
            }
            n += 1;
            seen.extend(joined_pieces(c, slot, &dp.tours, i, k));
        }
    }
    n
}

/// Whether duty `k` of tour `ti` can have an agency driver for the day: always - the agency sends
/// as many as are asked for. Only what they cost limits them.
pub fn agency_room(_c: &Company, _dp: &DayPlan, _ti: usize, _k: usize) -> bool {
    true
}

/// What an agency driver costs for duty `k` of tour `ti`: its hours and those of the pieces joined
/// with it, at the company's agency rate (`agency::rate`).
pub fn agency_cost(c: &Company, dp: &DayPlan, ti: usize, k: usize) -> Cents {
    let slot = super::plandays::slot_of(c, &dp.date);
    let hour = super::agency::rate(c) as f64;
    std::iter::once((ti, k))
        .chain(joined_pieces(c, slot, &dp.tours, ti, k))
        .filter_map(|(i, m)| dp.tours.get(i).and_then(|t| t.duties.get(m)))
        .map(|d| ((d.to - d.from).max(0) as f64 / 60.0 * hour).round() as Cents)
        .sum()
}

/// The pieces joined with duty `k` of tour `i` into one driver's duty in the studio, on the
/// plan of `slot` (`plandays::slot_of`): (tour, duty) in `tours`, the day plan's.
pub fn joined_pieces(c: &Company, slot: u8, tours: &[DayTour], i: usize, k: usize) -> Vec<(usize, usize)> {
    use super::studio::{PieceRef, TripRef};
    if c.planning.joined.is_empty() {
        return Vec::new();
    }
    let Some((t, d)) = tours.get(i).and_then(|t| t.duties.get(k).map(|d| (t, d))) else {
        return Vec::new();
    };
    let Some(first) = t.tour.trips.get(d.start) else {
        return Vec::new();
    };
    let me = PieceRef { line: t.tour.line.clone(), tour: t.tour.tour.clone(), first: TripRef::of(first) };
    let Some(group) = c.planning.joined.iter().find(|j| j.weekday == slot && j.pieces.iter().any(|p| p.is(&me))) else {
        return Vec::new();
    };
    let mut out = Vec::new();
    for p in group.pieces.iter().filter(|p| !p.is(&me)) {
        for (j, x) in tours.iter().enumerate().filter(|(_, x)| same(&x.tour.line, &p.line) && same(&x.tour.tour, &p.tour)) {
            if let Some(m) = x.duties.iter().position(|y| x.tour.trips.get(y.start).is_some_and(|f| p.first.is(f))) {
                out.push((j, m));
            }
        }
    }
    out
}

// --- the roster ------------------------------------------------------------------------------

/// Who drives a duty.
#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Debug, Hash)]
#[serde(rename_all = "snake_case")]
pub enum Who {
    Staff(u32),
    /// A driver of an agency, for the day.
    Agency,
}

/// A tour of the weekly roster: on a weekday, its bus and who drives each of its duties.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Default)]
pub struct RosterTour {
    /// 0 Monday .. 6 Sunday.
    pub weekday: u8,
    /// The company line (its timetable name) and the tour.
    pub line: String,
    pub tour: String,
    #[serde(default)]
    pub bus: Option<u32>,
    #[serde(default)]
    pub duties: Vec<Option<Who>>,
}

impl RosterTour {
    fn is_empty(&self) -> bool {
        self.bus.is_none() && self.duties.iter().all(Option::is_none)
    }
}

/// The dispatcher's choice for something open today.
#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Debug)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Fill {
    Colleague {
        id: u32,
    },
    Agency,
    Bus {
        id: u32,
    },
    Rental,
    Drop,
    /// The driver of spare line `id` (`rosterflow::spare_lines`) drives a late driver's first
    /// trips, within its hours.
    Spare {
        id: u32,
    },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct FillChoice {
    /// `tour_key`, `duty_key` or `late_key`.
    pub key: String,
    pub fill: Fill,
}

/// The company's planning as it is saved.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct Planning {
    #[serde(default)]
    pub week: Vec<RosterTour>,
    /// The dispatcher fills what the roster leaves free - only for "Fill the roster"
    /// (`fill_day`), which shows the planner what it would take and fixes it in the roster: the
    /// day itself runs what is planned, nothing behind the planner's back. (Not kept: an older
    /// file's "auto" is read as off.)
    #[serde(skip)]
    pub auto: bool,
    /// The dispatcher's own only for this line's tours (a changed line planned anew,
    /// `fill_line`: the other lines' tours run as they are planned). Not kept either.
    #[serde(skip)]
    pub auto_only: Option<String>,
    /// The dispatcher's own only for these places (`tour_key`s for a bus, `duty_key`s for a
    /// driver: the roster's repair, `repair_day`). Not kept either.
    #[serde(skip)]
    pub auto_keys: Option<Vec<String>>,
    /// The day the dispatcher's choices are for (they hold for that day only).
    #[serde(default)]
    pub date: String,
    #[serde(default)]
    pub fills: Vec<FillChoice>,
    /// The lines (their timetable names) the planning's chart shows folded to their heading
    /// (each line can be collapsed and expanded); kept with the company.
    #[serde(default)]
    pub collapsed: Vec<String>,
    /// The scheduling studio's blocks, a line and a weekday at a time, as the planner laid
    /// them out (`studio`): none, the timetable's tours cut automatically.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub layouts: Vec<super::studio::LineLayout>,
    /// The duties the planner joined from pieces of work (a split duty, or one that changes
    /// buses), a weekday at a time.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub joined: Vec<super::studio::JoinedDuty>,
    /// RosterFlow's own (`rosterflow`): its preferences, the locks, links and duty notes of
    /// each weekday, the issues dismissed, and the draft not saved yet.
    #[serde(default, skip_serializing_if = "super::rosterflow::Flow::is_empty")]
    pub flow: super::rosterflow::Flow,
    /// When the drivers who come late on the company's day arrive (`Arrival`, `pin_arrivals`).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub arrivals: Vec<Arrival>,
    /// The agency drivers booked ahead for a day (`agency`).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub agency: Vec<super::agency::Booking>,
    /// What the dispatcher reported for a day: a driver coming late, a bus that broke down.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub reported: Vec<Reported>,
    /// What a trip dropped costs on the contract (0: nothing).
    #[serde(default)]
    pub trip_penalty: Cents,
    /// The last agency booking's number given (`agency::book`): never given twice.
    #[serde(default)]
    pub agency_last: u32,
}

impl Default for Planning {
    fn default() -> Self {
        Planning {
            week: Vec::new(),
            auto: false,
            auto_only: None,
            auto_keys: None,
            date: String::new(),
            fills: Vec::new(),
            collapsed: Vec::new(),
            layouts: Vec::new(),
            joined: Vec::new(),
            flow: Default::default(),
            arrivals: Vec::new(),
            agency: Vec::new(),
            agency_last: 0,
            reported: Vec::new(),
            trip_penalty: 0,
        }
    }
}

/// When a driver who comes late (`Disruption::Late`) arrives on a day: the sign-on of the first
/// duty he had when it was first planned, and his minutes late. Kept, so that moved to another
/// duty he is late only for one that begins before he is there: swapped onto a later duty, he is
/// no longer late.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct Arrival {
    pub date: String,
    pub employee: u32,
    /// Minutes of the day.
    pub at: i32,
}

/// The line is shown folded to its heading on the planning's chart.
pub fn is_collapsed(c: &Company, line: &str) -> bool {
    c.planning.collapsed.iter().any(|x| same(x, line))
}

/// Fold a line of the planning's chart to its heading, or unfold it.
pub fn set_collapsed(c: &mut Company, line: &str, folded: bool) {
    c.planning.collapsed.retain(|x| !same(x, line));
    if folded {
        c.planning.collapsed.push(line.trim().to_string());
    }
}

/// Fold every one of `lines`, or unfold them all (the lines the company gave up stay out).
pub fn set_all_collapsed(c: &mut Company, lines: &[String], folded: bool) {
    c.planning.collapsed.clear();
    if folded {
        c.planning.collapsed.extend(lines.iter().map(|l| l.trim().to_string()));
    }
}

pub fn weekday_of(date: &str) -> u8 {
    dates::parse(date).map(|d| dates::weekday(d) as u8).unwrap_or(0)
}

pub fn tour_key(line: &str, tour: &str) -> String {
    format!("{line}|{tour}")
}

pub fn duty_key(line: &str, tour: &str, duty: usize) -> String {
    format!("{line}|{tour}|{duty}")
}

/// The first piece of a duty whose driver comes late.
pub fn late_key(line: &str, tour: &str, duty: usize) -> String {
    format!("{line}|{tour}|{duty}|late")
}

fn same(a: &str, b: &str) -> bool {
    a.trim().eq_ignore_ascii_case(b.trim())
}

/// The roster's tour on a weekday.
pub fn roster<'a>(c: &'a Company, weekday: u8, line: &str, tour: &str) -> Option<&'a RosterTour> {
    c.planning.week.iter().find(|r| r.weekday == weekday && same(&r.line, line) && same(&r.tour, tour))
}

fn roster_mut<'a>(c: &'a mut Company, weekday: u8, line: &str, tour: &str) -> &'a mut RosterTour {
    let at = match c.planning.week.iter().position(|r| r.weekday == weekday && same(&r.line, line) && same(&r.tour, tour)) {
        Some(k) => k,
        None => {
            c.planning.week.push(RosterTour { weekday, line: line.to_string(), tour: tour.to_string(), bus: None, duties: Vec::new() });
            c.planning.week.len() - 1
        }
    };
    &mut c.planning.week[at]
}

fn prune(c: &mut Company) {
    c.planning.week.retain(|r| !r.is_empty());
}

/// Give a tour of the roster its bus (None: none fixed).
pub fn set_bus(c: &mut Company, weekday: u8, line: &str, tour: &str, bus: Option<u32>) {
    roster_mut(c, weekday, line, tour).bus = bus;
    prune(c);
}

/// Give a duty of the roster its driver (None: none fixed).
pub fn set_driver(c: &mut Company, weekday: u8, line: &str, tour: &str, duty: usize, who: Option<Who>) {
    let r = roster_mut(c, weekday, line, tour);
    if r.duties.len() <= duty {
        r.duties.resize(duty + 1, None);
    }
    r.duties[duty] = who;
    while r.duties.last() == Some(&None) {
        r.duties.pop();
    }
    prune(c);
}

/// Clear a weekday of the roster.
pub fn clear_day(c: &mut Company, weekday: u8) -> Rostered {
    let gone = rostered(c, weekday);
    c.planning.week.retain(|r| r.weekday != weekday);
    gone
}

/// A line renamed (an own line's new number: its timetable name follows): its roster too.
pub fn rename_line(c: &mut Company, old: &str, new: &str) {
    if old.eq_ignore_ascii_case(new) {
        return;
    }
    for r in c.planning.week.iter_mut().filter(|r| r.line.eq_ignore_ascii_case(old)) {
        r.line = new.to_string();
    }
    // (folded on the chart under its old name: folded under its new one)
    for x in c.planning.collapsed.iter_mut().filter(|x| x.eq_ignore_ascii_case(old)) {
        *x = new.to_string();
    }
    // (the studio's blocks and joined duties of the line follow it too)
    super::studio::rename_line(c, old, new);
}

/// The roster's entries of a line's `tours` taken out, every weekday (a changed timetable:
/// their buses and drivers are given anew). Returns how many went.
pub fn forget_tours(c: &mut Company, line: &str, tours: &[String]) -> usize {
    let before = c.planning.week.len();
    c.planning.week.retain(|r| !(r.line.eq_ignore_ascii_case(line) && tours.iter().any(|t| t.trim() == r.tour.trim())));
    before - c.planning.week.len()
}

/// What the roster kept when the day's blocks and duties changed (`follow`).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Followed {
    /// The duties that have a driver, and those of lines in service without one (their trips
    /// are dropped until the roster gives them one).
    pub kept: usize,
    pub open: usize,
}

/// The roster of `weekday` kept with its buses and duties when the day's blocks and duties
/// change (#96: RosterFlow's Save, a layout the timetable no longer fits). The roster gives
/// them by their place - the block's name, the duty's index in it - so a duty cut or joined
/// anew moved the drivers onto other duties and left others without one, and the day dropped
/// their trips. `before` and `after` are the day's tours (`network::tours_of_day`) as they
/// were and are now. A block keeps its bus by its name; a duty keeps its driver by the trip it
/// begins with (on its bus, else on another of its line's); a duty that begins with another
/// trip now takes the driver of a duty it is joined with (one driver works a joined duty), else
/// the driver who had most of its trips (a duty cut in two stays with its driver, both halves,
/// until the roster gives one to another), else it has none until the roster gives it one.
pub fn follow(c: &mut Company, weekday: u8, before: &[TourOfDay], after: &[TourOfDay]) -> Followed {
    use super::studio::TripRef;
    // the drivers as the roster gave them: by line, block and the trip their duty begins with;
    // and every trip a driver had
    let mut had: Vec<(String, String, TripRef, Who)> = Vec::new();
    let mut driven: Vec<(String, TripRef, Who)> = Vec::new();
    for t in before {
        let Some(r) = roster(c, weekday, &t.line, &t.tour) else {
            continue;
        };
        for (k, p) in network::duties_of(t).iter().enumerate() {
            let (Some(Some(w)), Some(x)) = (r.duties.get(k), t.trips.get(p.start)) else {
                continue;
            };
            had.push((t.line.clone(), t.tour.clone(), TripRef::of(x), *w));
            driven.extend(t.trips[p.clone()].iter().filter(|x| x.counts()).map(|x| (t.line.clone(), TripRef::of(x), *w)));
        }
    }
    let touched = |r: &RosterTour| r.weekday == weekday && before.iter().chain(after).any(|t| same(&t.line, &r.line) && same(&t.tour, &r.tour));
    let old: Vec<RosterTour> = c.planning.week.iter().filter(|r| touched(r)).cloned().collect();
    c.planning.week.retain(|r| !touched(r));
    let pieces: Vec<Vec<std::ops::Range<usize>>> = after.iter().map(network::duties_of).collect();
    let mut given: Vec<Vec<Option<Who>>> = after
        .iter()
        .zip(&pieces)
        .map(|(t, ps)| {
            ps.iter()
                .map(|p| {
                    let first = TripRef::of(t.trips.get(p.start)?);
                    let mine = |h: &&(String, String, TripRef, Who)| same(&h.0, &t.line) && h.2 == first;
                    had.iter().filter(mine).find(|h| same(&h.1, &t.tour)).or_else(|| had.iter().find(mine)).map(|h| h.3)
                })
                .collect()
        })
        .collect();
    // (a duty's other pieces: its driver)
    for i in 0..after.len() {
        for k in 0..given[i].len() {
            if given[i][k].is_none() {
                given[i][k] = super::studio::partners(c, weekday, after, i, k).into_iter().find_map(|(j, m)| given.get(j).and_then(|g| g.get(m).copied().flatten()));
            }
        }
    }
    // (the driver who had most of its trips with passengers)
    for (i, t) in after.iter().enumerate() {
        for (k, p) in pieces[i].iter().enumerate() {
            if given[i][k].is_some() {
                continue;
            }
            let mut count: Vec<(Who, usize)> = Vec::new();
            for x in t.trips[p.clone()].iter().filter(|x| x.counts()) {
                let Some(w) = driven.iter().find(|(l, r, _)| same(l, &t.line) && r.is(x)).map(|d| d.2) else {
                    continue;
                };
                match count.iter_mut().find(|n| n.0 == w) {
                    Some(n) => n.1 += 1,
                    None => count.push((w, 1)),
                }
            }
            given[i][k] = count.iter().fold(None, |best: Option<&(Who, usize)>, n| if best.is_none_or(|b| n.1 > b.1) { Some(n) } else { best }).map(|n| n.0);
        }
    }
    let mut out = Followed::default();
    for (i, t) in after.iter().enumerate() {
        let bus = old.iter().find(|r| same(&r.line, &t.line) && same(&r.tour, &t.tour)).and_then(|r| r.bus);
        let mut duties = given[i].clone();
        for d in &duties {
            match d {
                Some(_) => out.kept += 1,
                None if !t.unplanned => out.open += 1,
                None => {}
            }
        }
        while duties.last() == Some(&None) {
            duties.pop();
        }
        if bus.is_some() || !duties.is_empty() {
            c.planning.week.push(RosterTour { weekday, line: t.line.clone(), tour: t.tour.clone(), bus, duties });
        }
    }
    // (the day's choices for its duties - an agency driver, a colleague - follow them too: they
    // are keyed by the duty's place in its block as the roster is)
    if !c.planning.date.is_empty() && weekday_of(&c.planning.date) == weekday && !c.planning.fills.is_empty() {
        c.planning.fills = follow_fills(&c.planning.fills, before, after);
    }
    out
}

/// A duty's key taken apart (`duty_key`, `late_key`): its line, its tour, its index, and whether
/// it is its late start. None: a tour's key (`tour_key`).
fn parse_duty_key(key: &str) -> Option<(String, String, usize, bool)> {
    let mut parts: Vec<&str> = key.split('|').collect();
    let late = parts.len() > 3 && parts.last() == Some(&"late");
    if late {
        parts.pop();
    }
    if parts.len() < 3 {
        return None;
    }
    let k = parts.pop()?.parse().ok()?;
    let tour = parts.pop()?.to_string();
    Some((parts.join("|"), tour, k, late))
}

/// The day's choices (`fills`) kept with their duties when the blocks and duties change from
/// `before` to `after` (`follow`): a duty's choice goes to every duty now that has most of its
/// trips from it (a duty cut in two keeps it in both halves), a late start's to the duty that
/// begins with the same trip; a tour's (its bus) stays with the tour's name. A choice whose duty
/// is gone goes too.
fn follow_fills(fills: &[FillChoice], before: &[TourOfDay], after: &[TourOfDay]) -> Vec<FillChoice> {
    use super::studio::TripRef;
    // (each trip with passengers of the day before, and the duty it was in)
    let mut was: Vec<(&str, TripRef, (usize, usize))> = Vec::new();
    for (i, t) in before.iter().enumerate() {
        for (k, p) in network::duties_of(t).iter().enumerate() {
            was.extend(t.trips[p.clone()].iter().filter(|x| x.counts()).map(|x| (t.line.as_str(), TripRef::of(x), (i, k))));
        }
    }
    // (each duty now: the duty before that had most of its trips)
    let mut owner: Vec<((usize, usize), Option<(usize, usize)>)> = Vec::new();
    for (i, t) in after.iter().enumerate() {
        for (k, p) in network::duties_of(t).iter().enumerate() {
            let mut count: Vec<((usize, usize), usize)> = Vec::new();
            for x in t.trips[p.clone()].iter().filter(|x| x.counts()) {
                let Some(o) = was.iter().find(|w| same(w.0, &t.line) && w.1.is(x)).map(|w| w.2) else {
                    continue;
                };
                match count.iter_mut().find(|n| n.0 == o) {
                    Some(n) => n.1 += 1,
                    None => count.push((o, 1)),
                }
            }
            let best = count.iter().fold(None, |best: Option<&((usize, usize), usize)>, n| if best.is_none_or(|b| n.1 > b.1) { Some(n) } else { best });
            owner.push(((i, k), best.map(|n| n.0)));
        }
    }
    let mut out: Vec<FillChoice> = Vec::new();
    let mut put = |key: String, fill: Fill| {
        if !out.iter().any(|f| f.key == key) {
            out.push(FillChoice { key, fill });
        }
    };
    for f in fills {
        let Some((line, tour, k, late)) = parse_duty_key(&f.key) else {
            put(f.key.clone(), f.fill);
            continue;
        };
        let Some(bi) = before.iter().position(|t| same(&t.line, &line) && same(&t.tour, &tour)) else {
            put(f.key.clone(), f.fill);
            continue;
        };
        let Some(p) = network::duties_of(&before[bi]).get(k).cloned() else {
            continue;
        };
        if late {
            // (the duty that begins with the same trip, on its block first)
            let first = TripRef::of(&before[bi].trips[p.start]);
            let begins = |t: &TourOfDay| network::duties_of(t).iter().position(|q| first.is(&t.trips[q.start]));
            let mine = after.iter().filter(|t| same(&t.line, &line));
            let to = mine.clone().filter(|t| same(&t.tour, &tour)).chain(mine).find_map(|t| begins(t).map(|q| (t, q)));
            if let Some((t, q)) = to {
                put(late_key(&t.line, &t.tour, q), f.fill);
            }
            continue;
        }
        for ((i, q), _) in owner.iter().filter(|o| o.1 == Some((bi, k))) {
            put(duty_key(&after[*i].line, &after[*i].tour, *q), f.fill);
        }
    }
    out
}

/// What a weekday's roster holds: the duties given to someone, the buses given.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Rostered {
    pub duties: usize,
    pub buses: usize,
}

impl Rostered {
    pub fn is_empty(&self) -> bool {
        self.duties + self.buses == 0
    }
}

pub fn rostered(c: &Company, weekday: u8) -> Rostered {
    let days = c.planning.week.iter().filter(|r| r.weekday == weekday);
    days.fold(Rostered::default(), |n, r| Rostered { duties: n.duties + r.duties.iter().flatten().count(), buses: n.buses + usize::from(r.bus.is_some()) })
}

/// The roster of one weekday on others too (Monday's on the other working days): returns
/// the days it went to (none when the weekday's roster is empty - the others are kept then).
pub fn copy_day(c: &mut Company, from: u8, to: &[u8]) -> Vec<u8> {
    if rostered(c, from).is_empty() {
        return Vec::new();
    }
    let src: Vec<RosterTour> = c.planning.week.iter().filter(|r| r.weekday == from).cloned().collect();
    let days: Vec<u8> = to.iter().copied().filter(|d| *d != from).collect();
    for &d in &days {
        clear_day(c, d);
        c.planning.week.extend(src.iter().cloned().map(|mut r| {
            r.weekday = d;
            r
        }));
    }
    days
}

/// The dispatcher's choice for something open on the company's day (None: the central's).
pub fn set_fill(c: &mut Company, key: &str, fill: Option<Fill>) {
    if c.planning.date != c.date {
        c.planning.date = c.date.clone();
        c.planning.fills.clear();
    }
    c.planning.fills.retain(|f| f.key != key);
    if let Some(fill) = fill {
        c.planning.fills.push(FillChoice { key: key.to_string(), fill });
    }
}

fn fill_of(c: &Company, date: &str, key: &str) -> Option<Fill> {
    (c.planning.date == date).then(|| c.planning.fills.iter().find(|f| f.key == key).map(|f| f.fill)).flatten()
}

/// Fill the roster of `date`'s weekday with what the dispatcher would take (the free buses
/// and drivers by the rules, those fixed already kept): returns how many were added.
pub fn fill_day(c: &mut Company, date: &str, tours: Vec<TourOfDay>) -> usize {
    let f = fill_day_told(c, date, tours);
    f.duties + f.buses
}

/// What "Fill the roster" did, and what it left open and why.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Filled {
    /// Duties given to drivers, buses given to tours.
    pub duties: usize,
    pub buses: usize,
    /// Duties still without a driver: for which nobody employed is qualified (the licence,
    /// an endorsement, the type training), and for which the qualified are not free.
    pub unqualified: usize,
    pub busy: usize,
    /// Tours still without a bus.
    pub no_bus: usize,
}

/// `fill_day`, told. What the roster has that cannot run that day (a driver ill, on holiday
/// or clashing, a bus gone, broken down or busy) is given anew first - without "Clear the day"
/// (`repair_day`) - and what is good is kept.
pub fn fill_day_told(c: &mut Company, date: &str, tours: Vec<TourOfDay>) -> Filled {
    let wd = weekday_of(date);
    let (buses, duties) = broken(c, date, tours.clone());
    for (line, tour) in &buses {
        set_bus(c, wd, line, tour, None);
    }
    for (line, tour, k) in &duties {
        set_driver(c, wd, line, tour, *k, None);
    }
    fill(c, date, tours, None)
}

/// What the morning's repair of the roster did (`repair_day`).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Repaired {
    /// The roster's places that could not run (duties and buses), and of them those given
    /// anew: the rest is open.
    pub broken: usize,
    pub duties: usize,
    pub buses: usize,
}

impl Repaired {
    pub fn open(&self) -> usize {
        self.broken.saturating_sub(self.duties + self.buses)
    }
}

/// The roster's places of `date`'s weekday that cannot run that day: the tours whose bus is
/// gone, broken down, in the workshop, not one of the line's or busy, and the duties whose
/// driver is ill, away, not qualified or clashes. (The agency's duties are
/// left as they are, and so are those of a driver on holiday: known ahead, the day covers them
/// and the place stays theirs for when they are back - `day_plan`.)
pub fn broken(c: &Company, date: &str, tours: Vec<TourOfDay>) -> (Vec<(String, String)>, Vec<(String, String, usize)>) {
    let wd = weekday_of(date);
    let mut probe = c.clone();
    probe.planning.auto = false;
    let p = day_plan(&probe, date, tours);
    let mut buses = Vec::new();
    let mut duties = Vec::new();
    for t in p.tours.iter() {
        let Some(r) = roster(c, wd, &t.tour.line, &t.tour.tour) else {
            continue;
        };
        if r.bus.is_some() && t.bus_from != Source::Roster {
            buses.push((t.tour.line.clone(), t.tour.tour.clone()));
        }
        for (k, d) in t.duties.iter().enumerate() {
            // (a driver on holiday keeps their place: the day covers it, `day_plan`)
            if d.problem == Some(Problem::Holiday) {
                continue;
            }
            if let Some(Some(w @ Who::Staff(_))) = r.duties.get(k) {
                if d.who != Some(*w) || d.from_ != Source::Roster {
                    duties.push((t.tour.line.clone(), t.tour.tour.clone(), k));
                }
            }
        }
    }
    (buses, duties)
}

/// The roster repaired for `date`, every morning by itself: each place of its weekday that cannot
/// run that day (`broken`) is given to a driver or a bus that fits, as the dispatcher would; the
/// good ones are kept, and nothing the roster left free is filled. What found nobody stays open.
pub fn repair_day(c: &mut Company, date: &str, tours: Vec<TourOfDay>) -> Repaired {
    let wd = weekday_of(date);
    let (buses, duties) = broken(c, date, tours.clone());
    let mut out = Repaired { broken: buses.len() + duties.len(), ..Default::default() };
    if out.broken == 0 {
        return out;
    }
    let mut keys = Vec::new();
    for (line, tour) in &buses {
        set_bus(c, wd, line, tour, None);
        keys.push(tour_key(line, tour));
    }
    for (line, tour, k) in &duties {
        set_driver(c, wd, line, tour, *k, None);
        keys.push(duty_key(line, tour, *k));
    }
    let mut probe = c.clone();
    probe.planning.auto = true;
    probe.planning.auto_keys = Some(keys.clone());
    let p = day_plan(&probe, date, tours);
    for t in p.tours.iter() {
        if let (Some(BusOf::Own(id)), Source::Auto) = (t.bus, t.bus_from) {
            if keys.contains(&tour_key(&t.tour.line, &t.tour.tour)) {
                set_bus(c, wd, &t.tour.line, &t.tour.tour, Some(id));
                out.buses += 1;
            }
        }
        for (k, d) in t.duties.iter().enumerate() {
            if let (Some(w @ Who::Staff(_)), Source::Auto) = (d.who, d.from_) {
                if keys.contains(&duty_key(&t.tour.line, &t.tour.tour, k)) {
                    keep_driver(c, wd, t, k, w);
                    out.duties += 1;
                }
            }
        }
    }
    out
}

/// "Fill the roster" for one line's tours only (a changed line planned anew), the day's
/// other tours weighed as they are.
pub fn fill_line(c: &mut Company, date: &str, tours: Vec<TourOfDay>, line: &str) -> Filled {
    fill(c, date, tours, Some(line))
}

/// The driver the dispatcher took for duty `k` of `t` kept: in the weekday's roster - or, an agency
/// driver booked for the day, given him for his day only (`agency::give`).
fn keep_driver(c: &mut Company, wd: u8, t: &DayTour, k: usize, w: Who) {
    if let Who::Staff(id) = w {
        if super::agency::is_booked(id) {
            let first = super::studio::TripRef::of(&t.tour.trips[t.duties[k].start]);
            super::agency::give(c, id, super::studio::PieceRef { line: t.tour.line.clone(), tour: t.tour.tour.clone(), first });
            return;
        }
    }
    set_driver(c, wd, &t.tour.line, &t.tour.tour, k, Some(w));
}

fn fill(c: &mut Company, date: &str, tours: Vec<TourOfDay>, only: Option<&str>) -> Filled {
    let ours = |t: &DayTour| only.is_none_or(|l| t.tour.line.eq_ignore_ascii_case(l));
    let mut probe = c.clone();
    probe.planning.auto = true;
    probe.planning.auto_only = only.map(str::to_string);
    let p = day_plan(&probe, date, tours);
    let wd = weekday_of(date);
    let mut f = Filled::default();
    let staff: Vec<&Employee> = c.staff.iter().filter(|e| staff::drives(e) && e.employed_on(date)).collect();
    for t in &p.tours {
        if !ours(t) {
            continue;
        }
        // (a tour not in service is not open)
        let open = !t.tour.unplanned;
        match (t.bus, t.bus_from) {
            (Some(BusOf::Own(_)), Source::Auto) => f.buses += 1,
            (None, _) if open => f.no_bus += 1,
            _ => {}
        }
        let bus = t.bus_of(&probe);
        let staff: Vec<&Employee> = staff.clone();
        for d in &t.duties {
            match (d.who, d.from_) {
                (Some(Who::Staff(_)), Source::Auto) => f.duties += 1,
                (None, _) if !open => {}
                (None, _) if staff.iter().all(|e| super::licences::lack(e, bus.0, bus.1.as_deref()).is_some()) => f.unqualified += 1,
                (None, _) => f.busy += 1,
                _ => {}
            }
        }
    }
    for t in &p.tours {
        if !ours(t) {
            continue;
        }
        if let (Some(BusOf::Own(id)), Source::Auto) = (t.bus, t.bus_from) {
            set_bus(c, wd, &t.tour.line, &t.tour.tour, Some(id));
        }
        for (k, d) in t.duties.iter().enumerate() {
            if let (Some(w @ Who::Staff(_)), Source::Auto) = (d.who, d.from_) {
                keep_driver(c, wd, t, k, w);
            }
        }
    }
    f
}

// --- the morning -----------------------------------------------------------------------------

/// What the dispatcher reported for a day.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Disruption {
    /// Someone comes `minutes` late to their first duty.
    Late { employee: u32, minutes: i32 },
    /// A bus broke down: in the workshop for the day.
    Breakdown { vehicle: u32 },
}

/// A disruption reported for a date.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct Reported {
    pub date: String,
    pub what: Disruption,
}

/// The disruptions reported for `date`.
pub fn disruptions(c: &Company, date: &str) -> Vec<Disruption> {
    c.planning.reported.iter().filter(|r| r.date == date).map(|r| r.what).collect()
}

/// Report a disruption for `date` (one of the same kind for the same driver or bus replaced),
/// or take it back (`None`, by its driver or bus).
pub fn report(c: &mut Company, date: &str, what: Disruption) {
    let same_one = |x: &Disruption| match (x, &what) {
        (Disruption::Late { employee: a, .. }, Disruption::Late { employee: b, .. }) => a == b,
        (Disruption::Breakdown { vehicle: a }, Disruption::Breakdown { vehicle: b }) => a == b,
        _ => false,
    };
    c.planning.reported.retain(|r| !(r.date == date && same_one(&r.what)));
    c.planning.reported.push(Reported { date: date.to_string(), what });
}

/// Take back what was reported for `date` of a driver (`employee`) or a bus (`vehicle`).
pub fn unreport(c: &mut Company, date: &str, employee: Option<u32>, vehicle: Option<u32>) {
    c.planning.reported.retain(|r| {
        !(r.date == date
            && match r.what {
                Disruption::Late { employee: e, .. } => Some(e) == employee,
                Disruption::Breakdown { vehicle: v } => Some(v) == vehicle,
            })
    });
}

/// The reports of days long gone dropped.
pub fn prune_reported(c: &mut Company, today: &str) {
    c.planning.reported.retain(|r| super::dates::between(&r.date, today) <= 14);
}

/// When `employee`, late on `date`, arrives (minutes of the day), if it was kept (`pin_arrivals`).
pub fn arrival(c: &Company, date: &str, employee: u32) -> Option<i32> {
    c.planning.arrivals.iter().find(|a| a.date == date && a.employee == employee).map(|a| a.at)
}

/// The arrivals of the late drivers of the company's day plan `dp` not kept yet: the sign-on of
/// the first duty each has in it, and his minutes late (a driver without a duty keeps nobody
/// waiting: none).
pub fn arrivals_due(c: &Company, dp: &DayPlan) -> Vec<Arrival> {
    if !dp.today {
        return Vec::new();
    }
    dp.disruptions
        .iter()
        .filter_map(|d| if let Disruption::Late { employee, minutes } = *d { Some((employee, minutes)) } else { None })
        .filter(|&(employee, _)| arrival(c, &dp.date, employee).is_none())
        .filter_map(|(employee, minutes)| {
            let &(i, k) = dp.duties_of(Who::Staff(employee)).first()?;
            Some(Arrival { date: dp.date.clone(), employee, at: dp.tours[i].duties[k].from + minutes })
        })
        .collect()
}

/// The late drivers' arrivals of the company's day kept, as the day is planned now (`dp`): in the
/// morning, and before the day's roster is first changed. Returns how many were new.
pub fn pin_arrivals(c: &mut Company, dp: &DayPlan) -> usize {
    let due = arrivals_due(c, dp);
    let date = c.date.clone();
    c.planning.arrivals.retain(|a| a.date == date);
    let n = due.len();
    c.planning.arrivals.extend(due);
    n
}

// --- the plan of a day -----------------------------------------------------------------------

/// How a bus or a driver came to a tour or a duty.
#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Debug, Default)]
#[serde(rename_all = "snake_case")]
pub enum Source {
    #[default]
    None,
    Roster,
    /// The dispatcher filled what the roster left free.
    Auto,
    /// The dispatcher's choice for the day.
    Dispatcher,
    /// The central filled what fell out.
    Central,
}

/// Why a tour has no bus, or a duty no driver.
#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Debug)]
#[serde(rename_all = "snake_case")]
pub enum Problem {
    Sick,
    Holiday,
    /// Not employed that day, gone, or five days worked this week.
    Away,
    /// The licence does not cover the bus.
    Licence,
    /// Drives something else then, or the working-time rules forbid it.
    Conflict,
    /// The bus did not start this morning.
    Breakdown,
    Workshop,
    /// No longer the company's bus.
    Gone,
    /// On another tour then.
    BusBusy,
    /// Nobody, or no bus, planned (only what is planned runs).
    Unassigned,
    /// The bus fixed for it is none of the buses its line asks for (`ownline::line_allows`).
    NotLineBus,
    /// The fleet has none of the buses its line asks for.
    NoLineBus,
    /// No endorsement for the bus (articulated, double-decker, electric: `licences`).
    NoEndorsement,
    /// No type training for the bus's model.
    NoTypeTraining,
}

impl Problem {
    /// Came suddenly: the central fills it by itself.
    pub fn sudden(self) -> bool {
        matches!(self, Problem::Sick | Problem::Breakdown)
    }

    pub fn label(self) -> &'static str {
        match self {
            Problem::Sick => "Ill",
            Problem::Holiday => "On holiday",
            Problem::Away => "Not available",
            Problem::Licence => "No licence for this bus",
            Problem::Conflict => "Clashes with other work",
            Problem::Breakdown => "Did not start",
            Problem::Workshop => "In the workshop",
            Problem::Gone => "No longer in the fleet",
            Problem::BusBusy => "On another tour then",
            Problem::Unassigned => "Not planned",
            Problem::NotLineBus => "Not one of the line's buses",
            Problem::NoLineBus => "The fleet has none of the line's buses",
            Problem::NoEndorsement => "No licence for this kind of bus",
            Problem::NoTypeTraining => "No type training for this bus",
        }
    }
}

/// What a fitting assignment warns of.
#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Debug)]
#[serde(rename_all = "snake_case")]
pub enum Warn {
    /// Minutes to change to another stop.
    Transfer(i32),
    /// Minutes over the day's eight hours.
    Overtime(i32),
    /// No endorsement recorded for a big bus (`staff::qualified`).
    Experience,
}

#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Debug)]
#[serde(rename_all = "snake_case")]
pub enum BusOf {
    Own(u32),
    /// A bus hired for the day.
    Rental,
}

/// A driver late for a duty's beginning: open until `until`, unless `cover` drives it (an
/// employee, a booked agency driver, or `AGENCY`: one the agency sends on the day).
#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Debug)]
pub struct LatePiece {
    pub until: i32,
    pub cover: Option<u32>,
    pub from_: Source,
    /// The cover was sent during the day (service control, `control::cover_from`): it drives
    /// the trips from this minute on, those before it are dropped. None: all of them.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cover_from: Option<i32>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Default)]
pub struct DayDuty {
    /// Its trips (indices into the tour's trips) and its time.
    pub start: usize,
    pub end: usize,
    pub from: i32,
    pub to: i32,
    pub who: Option<Who>,
    pub from_: Source,
    /// What made it open (it stays open while `who` is None).
    pub problem: Option<Problem>,
    pub late: Option<LatePiece>,
    pub warn: Vec<Warn>,
}

impl DayDuty {
    pub fn open(&self) -> bool {
        self.who.is_none()
    }

    /// The trips leaving before this minute are dropped: its driver comes late, and nobody
    /// covers them (or the cover came later: `LatePiece::cover_from`).
    pub fn dropped_before(&self) -> Option<i32> {
        let l = self.late?;
        match l.cover {
            None => Some(l.until),
            Some(_) => l.cover_from.filter(|&f| f > self.from).map(|f| f.min(l.until)),
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Default)]
pub struct DayTour {
    pub tour: TourOfDay,
    pub bus: Option<BusOf>,
    pub bus_from: Source,
    pub bus_problem: Option<Problem>,
    pub duties: Vec<DayDuty>,
}

impl DayTour {
    pub fn covered(&self) -> bool {
        self.bus.is_some() && self.duties.iter().all(|d| d.who.is_some())
    }

    pub fn key(&self) -> String {
        tour_key(&self.tour.line, &self.tour.tour)
    }

    /// The bus it runs with as a driver's qualification sees it: its kind and its file (a
    /// rental or no bus yet: the size it asks for, no model).
    pub fn bus_of(&self, c: &Company) -> (BusKind, Option<String>) {
        match self.bus.and_then(|b| if let BusOf::Own(id) = b { c.vehicle(id) } else { None }) {
            Some(v) => (v.kind, super::licences::model_checked(c, &v.name)),
            None => (BusKind { size: self.size(c), drive: Drive::Diesel }, None),
        }
    }

    /// The size of bus it runs with (or asks for).
    pub fn size(&self, c: &Company) -> BusSize {
        match self.bus {
            Some(BusOf::Own(id)) => c.vehicle(id).map(|v| v.kind.size).unwrap_or_default(),
            _ => super::ownline::wanted(c, &self.tour).unwrap_or_default(),
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Default)]
pub struct DayPlan {
    pub date: String,
    pub weekday: u8,
    /// The company's own day (the morning is known, the dispatcher's choices count).
    pub today: bool,
    pub tours: Vec<DayTour>,
    pub disruptions: Vec<Disruption>,
}

/// Something open, or something that was open and got filled, for the list of open duties.
#[derive(Clone, Debug, PartialEq)]
pub struct OpenRow {
    pub tour: usize,
    /// None: the tour's bus.
    pub duty: Option<usize>,
    /// The late piece of the duty.
    pub late: bool,
    pub key: String,
    pub problem: Option<Problem>,
    /// Who or what fills it now (None: its trips are dropped).
    pub filled: bool,
    pub from_: Source,
}

impl DayPlan {
    /// Tours, tours covered, duties open (of tours with a bus), and tours without a bus.
    pub fn counts(&self) -> (usize, usize, usize, usize) {
        let mine: Vec<&DayTour> = self.tours.iter().collect();
        let covered = self.tours.iter().filter(|t| t.covered()).count();
        let open = mine.iter().filter(|t| t.bus.is_some()).flat_map(|t| t.duties.iter()).filter(|d| d.open()).count();
        let no_bus = mine.iter().filter(|t| t.bus.is_none()).count();
        (self.tours.len(), covered, open, no_bus)
    }

    /// What fell out or stayed free, filled or not (the dispatcher's list).
    pub fn open_rows(&self) -> Vec<OpenRow> {
        let mut out = Vec::new();
        for (ti, t) in self.tours.iter().enumerate() {
            if t.bus_problem.is_some() || t.bus.is_none() {
                out.push(OpenRow { tour: ti, duty: None, late: false, key: t.key(), problem: t.bus_problem.or(Some(Problem::Unassigned)), filled: t.bus.is_some(), from_: t.bus_from });
            }
            if t.bus.is_none() {
                continue;
            }
            for (k, d) in t.duties.iter().enumerate() {
                if d.problem.is_some() || d.who.is_none() {
                    out.push(OpenRow {
                        tour: ti,
                        duty: Some(k),
                        late: false,
                        key: duty_key(&t.tour.line, &t.tour.tour, k),
                        problem: d.problem.or(Some(Problem::Unassigned)),
                        filled: d.who.is_some(),
                        from_: d.from_,
                    });
                }
                if let Some(l) = d.late {
                    out.push(OpenRow { tour: ti, duty: Some(k), late: true, key: late_key(&t.tour.line, &t.tour.tour, k), problem: None, filled: l.cover.is_some(), from_: l.from_ });
                }
            }
        }
        out
    }

    /// The duties of a driver, (tour, duty), in the order they begin.
    pub fn duties_of(&self, who: Who) -> Vec<(usize, usize)> {
        let mut v: Vec<(usize, usize)> = self.tours.iter().enumerate().flat_map(|(ti, t)| t.duties.iter().enumerate().filter(|(_, d)| d.who == Some(who)).map(move |(k, _)| (ti, k))).collect();
        v.sort_by_key(|&(ti, k)| self.tours[ti].duties[k].from);
        v
    }
}

/// When each bus is out: (id, (from, to) of its tours).
#[derive(Default)]
struct Buses(Vec<(u32, Vec<(i32, i32)>)>);

impl Buses {
    fn free(&self, id: u32, from: i32, to: i32) -> bool {
        self.0.iter().find(|b| b.0 == id).is_none_or(|b| b.1.iter().all(|&(a, z)| to + BUS_MARGIN <= a || z + BUS_MARGIN <= from))
    }

    fn take(&mut self, id: u32, from: i32, to: i32) {
        match self.0.iter_mut().find(|b| b.0 == id) {
            Some(b) => b.1.push((from, to)),
            None => self.0.push((id, vec![(from, to)])),
        }
    }

    /// When it was last out before `from` (the latest end; MIN if never).
    fn last_end(&self, id: u32, from: i32) -> i32 {
        self.0.iter().find(|b| b.0 == id).and_then(|b| b.1.iter().filter(|x| x.1 <= from).map(|x| x.1).max()).unwrap_or(i32::MIN)
    }
}

/// What each driver drives: (id, blocks).
#[derive(Default)]
struct Work(Vec<(u32, Vec<Block>)>);

impl Work {
    fn of(&self, id: u32) -> &[Block] {
        self.0.iter().find(|w| w.0 == id).map(|w| w.1.as_slice()).unwrap_or(&[])
    }

    fn push(&mut self, id: u32, b: Block) {
        match self.0.iter_mut().find(|w| w.0 == id) {
            Some(w) => w.1.push(b),
            None => self.0.push((id, vec![b])),
        }
    }
}

fn block_of(t: &TourOfDay, d: &DayDuty, key: String) -> Block {
    Block { key, from: d.from, to: d.to, from_stop: t.trips[d.start].from.clone(), to_stop: t.trips[d.end - 1].to.clone() }
}

/// Duty `k` of tour `ti` of `p` as a driver's working time sees it (`fits_bus`).
pub fn duty_block(p: &DayPlan, ti: usize, k: usize) -> Block {
    let t = &p.tours[ti];
    block_of(&t.tour, &t.duties[k], duty_key(&t.tour.line, &t.tour.tour, k))
}

fn covers(u: &Option<String>, date: &str) -> bool {
    u.as_deref().is_some_and(|u| dates::between(date, u) >= 0)
}

/// Why someone cannot drive on `date` at all (None: they can). Only drivers drive: anyone else
/// is away from the duties.
fn unavailable(e: &Employee, date: &str) -> Option<Problem> {
    if !staff::drives(e) || !e.employed_on(date) {
        Some(Problem::Away)
    } else if e.sick_on(date) {
        Some(Problem::Sick)
    } else if e.on_holiday(date).is_some() {
        Some(Problem::Holiday)
    } else if covers(&e.training_until, date) {
        // (on a course, as `Employee::absent` has it: the roster gave them their duties all the same)
        Some(Problem::Away)
    } else if week_full(e) || e.day_off(date) {
        Some(Problem::Away)
    } else {
        None
    }
}

/// Worked the week's days already (`Employee::week_days`, filled in before a day is planned,
/// against `week_days_max`, the company's rule there).
fn week_full(e: &Employee) -> bool {
    e.week_days_max > 0 && e.week_days >= e.week_days_max
}

/// How well a bus of `size` fits a tour asking for `wants`: the size asked for (or none
/// asked), a bigger bus (it carries the load), a smaller one.
fn size_fit(wants: Option<BusSize>, size: BusSize) -> u8 {
    match wants {
        None => 2,
        Some(w) if w == size => 2,
        Some(w) if super::ownline::capacity(size) >= super::ownline::capacity(w) => 1,
        Some(_) => 0,
    }
}

/// How a duty (or a piece) on a bus (`DayTour::bus_of`) fits a driver: qualified for it - the
/// licence, the endorsements, the model's type training (`licences`) - and then as `fits`.
pub fn fits_bus(e: &Employee, bus: &(BusKind, Option<String>), work: &[Block], b: &Block, today: bool) -> Result<Vec<Warn>, Problem> {
    if !staff::drives(e) {
        return Err(Problem::Away);
    }
    match super::licences::lack(e, bus.0, bus.1.as_deref()) {
        Some(super::licences::Lack::Licence) => Err(Problem::Licence),
        Some(super::licences::Lack::Endorsement(_)) => Err(Problem::NoEndorsement),
        Some(super::licences::Lack::Type(_)) => Err(Problem::NoTypeTraining),
        None => fits(e, bus.0.size, work, b, today),
    }
}

/// How a duty (or a piece) fits a driver: allowed, and the warnings.
pub fn fits(e: &Employee, size: BusSize, work: &[Block], b: &Block, today: bool) -> Result<Vec<Warn>, Problem> {
    if !staff::drives(e) {
        return Err(Problem::Away);
    }
    if !staff::may_drive(e, size) {
        return Err(Problem::Licence);
    }
    let _ = today;
    let k = staff::check(work, b, e.last_end, None);
    if !k.allowed() {
        return Err(Problem::Conflict);
    }
    let mut w = Vec::new();
    if let Some(t) = k.transfer {
        w.push(Warn::Transfer(t));
    }
    if k.overtime > 0 {
        w.push(Warn::Overtime(k.overtime));
    }
    if !staff::qualified(e, size) {
        w.push(Warn::Experience);
    }
    Ok(w)
}

// --- who could take a duty, and why not ----------------------------------------------------------

/// Why a driver cannot take a duty, told exactly (a driver who is free but cannot cover a duty
/// is listed with the reason).
#[derive(Clone, Debug, PartialEq)]
pub enum Cannot {
    /// Not a driver (a mechanic, a cleaner...): only drivers drive.
    NotDriver,
    /// Not employed on the day (not yet, or gone).
    NotEmployed,
    Ill {
        until: String,
    },
    Holiday {
        until: String,
    },
    /// On a course (`Employee::training_until`): off the duties until its last day.
    Training {
        until: String,
    },
    /// Worked the week's days already (`StaffRules::week_days`).
    WeekDone {
        days: u32,
    },
    /// Never works on this weekday (`Employee::days_off`).
    DayOff,
    /// The licence, an endorsement or the type training for the bus.
    Lacks(super::licences::Lack),
    /// Drives another duty then (its `duty_key`, and its time).
    OnDuty {
        key: String,
        from: i32,
        to: i32,
    },
    /// The pause to the neighbouring duty is shorter than the rest (`staff::REST`).
    Rest {
        minutes: i32,
    },
    /// The night's rest since yesterday's last duty is too short: free from `from`.
    NightRest {
        from: i32,
    },
    /// The working day would come to `minutes`, over `staff::DAY_MAX`.
    TooLong {
        minutes: i32,
    },
}

/// Why someone cannot drive on `date` at all (the day itself, before any duty), exactly; on the
/// company's day the night's rest too (free only from `NightRest::from`). None: they can.
pub fn day_unavailable(e: &Employee, date: &str) -> Option<Cannot> {
    if !staff::drives(e) {
        Some(Cannot::NotDriver)
    } else if !e.employed_on(date) {
        Some(Cannot::NotEmployed)
    } else if e.sick_on(date) {
        Some(Cannot::Ill { until: if e.sick_open { String::new() } else { e.sick_until.clone().unwrap_or_default() } })
    } else if let Some(until) = e.on_holiday(date) {
        Some(Cannot::Holiday { until: until.to_string() })
    } else if covers(&e.training_until, date) {
        // (on a course, as `Employee::absent` has it: offered and given duties all the same)
        Some(Cannot::Training { until: e.training_until.clone().unwrap_or_default() })
    } else if e.day_off(date) {
        Some(Cannot::DayOff)
    } else if week_full(e) {
        Some(Cannot::WeekDone { days: e.week_days })
    } else {
        None
    }
}

/// When a driver may begin on the company's day after yesterday's last duty (None: any time).
pub fn free_from(e: &Employee, _today: bool) -> Option<i32> {
    e.last_end.map(|end| end + staff::NIGHT_REST - 1440).filter(|m| *m > 0)
}

/// Whether `e` can take a duty (`b`) on a bus (`DayTour::bus_of`) beside their other work of
/// the day (`work`): the warnings, or exactly why not (`day_plan`'s own rules: `unavailable`,
/// `fits_bus`).
pub fn can_take(e: &Employee, date: &str, today: bool, bus: &(BusKind, Option<String>), work: &[Block], b: &Block) -> Result<Vec<Warn>, Cannot> {
    if let Some(x) = day_unavailable(e, date) {
        return Err(x);
    }
    if let Some(x) = super::licences::lack(e, bus.0, bus.1.as_deref()) {
        return Err(Cannot::Lacks(x));
    }
    if !staff::may_drive(e, bus.0.size) {
        return Err(Cannot::Lacks(super::licences::Lack::Licence));
    }
    let k = staff::check(work, b, e.last_end, None);
    if let Some(key) = &k.double {
        let (from, to) = work.iter().find(|x| &x.key == key).map(|x| (x.from, x.to)).unwrap_or((b.from, b.to));
        return Err(Cannot::OnDuty { key: key.clone(), from, to });
    }
    if k.too_long {
        return Err(Cannot::TooLong { minutes: staff::work_minutes(work) + (b.to - b.from) });
    }
    if let Some(r) = k.rest.filter(|r| *r < staff::REST) {
        return Err(Cannot::Rest { minutes: r });
    }
    if k.night_rest.is_some_and(|r| r < staff::NIGHT_REST) {
        return Err(Cannot::NightRest { from: free_from(e, today).unwrap_or(b.from) });
    }
    fits_bus(e, bus, work, b, today).map_err(|_| Cannot::NotDriver)
}

/// A driver as a duty's lists show them: who, free all day (a standby), and whether they can
/// take it.
#[derive(Clone, Debug, PartialEq)]
pub struct Candidate {
    pub id: u32,
    /// No other duty that day: a standby driver.
    pub standby: bool,
    pub fit: Result<Vec<Warn>, Cannot>,
}

/// A driver's work on the plan's day without one duty (to see whether it fits).
pub fn work_without(dp: &DayPlan, id: u32, skip: (usize, usize)) -> Vec<Block> {
    dp.duties_of(Who::Staff(id))
        .into_iter()
        .filter(|x| *x != skip)
        .map(|(i, k)| {
            let t = &dp.tours[i];
            block_of(&t.tour, &t.duties[k], duty_key(&t.tour.line, &t.tour.tour, k))
        })
        .collect()
}

/// Every driver for duty `k` of tour `ti`: those who can take it first (the standby drivers,
/// then those with the fewest warnings), then those who cannot, each with the reason.
pub fn candidates(c: &Company, dp: &DayPlan, ti: usize, k: usize) -> Vec<Candidate> {
    let t = &dp.tours[ti];
    let bus = t.bus_of(c);
    let block = block_of(&t.tour, &t.duties[k], duty_key(&t.tour.line, &t.tour.tour, k));
    // (the agency drivers booked for the day too, first among those who can: `agency`)
    let mut out: Vec<(String, Candidate)> = c
        .staff
        .iter()
        .filter(|e| staff::drives(e))
        .chain(super::agency::drivers_on(c, &dp.date))
        .map(|e| {
            let work = work_without(dp, e.id, (ti, k));
            let fit = can_take(e, &dp.date, dp.today, &bus, &work, &block);
            let name = if super::agency::is_booked(e.id) { format!("{:06}", super::agency::number_of(e)) } else { e.name.clone() };
            (name, Candidate { id: e.id, standby: work.is_empty(), fit })
        })
        .collect();
    out.sort_by(|a, b| {
        let rank = |x: &Candidate| match &x.fit {
            Ok(w) => (0, !super::agency::is_booked(x.id), !x.standby, w.len()),
            Err(_) => (1, false, false, 0),
        };
        rank(&a.1).cmp(&rank(&b.1)).then(a.0.cmp(&b.0))
    });
    out.into_iter().map(|x| x.1).collect()
}

/// Who can drive the first trips of duty `k` of tour `ti` before its late driver comes
/// (`DayDuty::late`: service control's cover): the company's drivers and the agency drivers
/// booked for the day free then and fit for the bus, the booked ones first, then the standby
/// drivers (no other duty that day). (id, standby)
pub fn late_covers(c: &Company, dp: &DayPlan, ti: usize, k: usize) -> Vec<(u32, bool)> {
    let Some(t) = dp.tours.get(ti) else {
        return Vec::new();
    };
    let Some(d) = t.duties.get(k) else {
        return Vec::new();
    };
    let (Some(l), Some(Who::Staff(late))) = (d.late, d.who) else {
        return Vec::new();
    };
    let piece = Block {
        key: late_key(&t.tour.line, &t.tour.tour, k),
        from: d.from,
        to: l.until.min(d.to),
        from_stop: t.tour.trips.get(d.start).map(|x| x.from.clone()).unwrap_or_default(),
        to_stop: String::new(),
    };
    let bus = t.bus_of(c);
    let mut out: Vec<(bool, bool, u32)> = c
        .staff
        .iter()
        .chain(super::agency::drivers_on(c, &dp.date))
        .filter(|e| e.id != late && unavailable(e, &dp.date).is_none())
        .filter(|e| !super::agency::is_booked(e.id) || super::agency::within(c, e.id, &piece))
        .filter_map(|e| {
            let work = work_without(dp, e.id, (usize::MAX, 0));
            fits_bus(e, &bus, &work, &piece, dp.today).ok().map(|_| (!super::agency::is_booked(e.id), !work.is_empty(), e.id))
        })
        .collect();
    out.sort();
    out.into_iter().map(|(_, busy, id)| (id, !busy)).collect()
}

/// What the trips of a duty (or a whole tour: `0..trips`) dropped cost: each journey with
/// passengers the planning's penalty for a dropped trip (`Planning::trip_penalty`).
pub fn drop_penalty(c: &Company, t: &TourOfDay, start: usize, end: usize) -> Cents {
    let value = c.planning.trip_penalty;
    t.trips[start.min(t.trips.len())..end.min(t.trips.len())].iter().filter(|x| x.counts()).map(|_| value).sum()
}

/// A way to cover a duty that fell out or stayed free on the company's day (a driver off sick:
/// cover the duty rather than drop its trips).
#[derive(Clone, Debug, PartialEq)]
pub enum Cover {
    /// A colleague who may drive it (`standby`: free all day).
    Colleague { id: u32, standby: bool, warn: Vec<Warn> },
    /// An agency driver (`used` sent today already: as many as are asked for).
    Agency { used: usize },
    /// Its trips dropped, with the contract's penalty.
    Drop { penalty: Cents },
}

impl Cover {
    /// The dispatcher's choice it makes.
    pub fn fill(&self) -> Fill {
        match self {
            Cover::Colleague { id, .. } => Fill::Colleague { id: *id },
            Cover::Agency { .. } => Fill::Agency,
            Cover::Drop { .. } => Fill::Drop,
        }
    }
}

/// The ways to cover duty `k` of tour `ti` on the company's day, in the order to offer them:
/// each free colleague who may drive it (standby drivers first), an agency driver from the
/// agency (as many as are asked for), dropping its trips. Those who cannot
/// take it are `candidates` with their reasons.
pub fn covers_of_duty(c: &Company, dp: &DayPlan, ti: usize, k: usize) -> Vec<Cover> {
    let t = &dp.tours[ti];
    let d = &t.duties[k];
    let mut out: Vec<Cover> = candidates(c, dp, ti, k).into_iter().filter_map(|x| x.fit.ok().map(|warn| Cover::Colleague { id: x.id, standby: x.standby, warn })).collect();
    // (one agency driver for the whole duty, its joined pieces too)
    if agency_room(c, dp, ti, k) {
        out.push(Cover::Agency { used: agency_sent(c, dp) });
    }
    out.push(Cover::Drop { penalty: drop_penalty(c, &t.tour, d.start, d.end) });
    out
}

/// The company's buses free to run tour `ti` instead of its own today (a spare bus for one
/// that broke down, or none given): in the depot and fit to drive (not in the workshop, not
/// broken down this morning), one of the line's buses, not out on another tour then - the size
/// the tour asks for first, then the best kept.
pub fn spare_buses(c: &Company, dp: &DayPlan, ti: usize) -> Vec<u32> {
    let t = &dp.tours[ti];
    let (from, to, wants) = (t.tour.from(), t.tour.to(), super::ownline::wanted(c, &t.tour));
    let broken: Vec<u32> = dp.disruptions.iter().filter_map(|d| if let Disruption::Breakdown { vehicle, .. } = d { Some(*vehicle) } else { None }).collect();
    let mut v: Vec<&super::model::Vehicle> = c
        .fleet
        .iter()
        .filter(|v| v.held_on(&dp.date) && !v.in_workshop(&dp.date) && !broken.contains(&v.id))
        .filter(|v| t.bus != Some(BusOf::Own(v.id)) && t.tour.lines().iter().all(|l| super::ownline::line_allows(c, l, v)))
        .filter(|v| !dp.tours.iter().enumerate().any(|(i, x)| i != ti && x.bus == Some(BusOf::Own(v.id)) && from < x.tour.to() + BUS_MARGIN && x.tour.from() < to + BUS_MARGIN))
        .collect();
    v.sort_by(|a, b| size_fit(wants, b.kind.size).cmp(&size_fit(wants, a.kind.size)).then(a.id.cmp(&b.id)));
    v.into_iter().map(|v| v.id).collect()
}

/// Whether a hired bus can come for tour `ti` (always: the planner arranges it).
pub fn rental_for(_c: &Company, _dp: &DayPlan, _ti: usize) -> (Cents, bool) {
    (0, true)
}

/// The plan of `date`: its tours (`network::tours_of_day` of that date's timetable) given
/// buses and drivers - the roster's first, then the dispatcher's own when `auto` is on; on the
/// company's own day (`Company::date`) what was reported and what is open handled.
pub fn day_plan(c: &Company, date: &str, tours: Vec<TourOfDay>) -> DayPlan {
    let today = date == c.date;
    let wd = weekday_of(date);
    let dis = disruptions(c, date);
    let broken: Vec<u32> = dis.iter().filter_map(|d| if let Disruption::Breakdown { vehicle } = d { Some(*vehicle) } else { None }).collect();
    let usable = |id: u32| c.vehicle(id).is_some_and(|v| v.held_on(date) && !v.in_workshop(date)) && !broken.contains(&id);
    // (the dispatcher's own, for a line's tours)
    let auto = |line: &str| c.planning.auto && c.planning.auto_only.as_deref().is_none_or(|l| l.eq_ignore_ascii_case(line));
    // (and only the repair's places, `repair_day`)
    let auto_key = |key: &str| c.planning.auto_keys.as_ref().is_none_or(|k| k.iter().any(|x| x == key));
    let mut tours = tours;
    tours.sort_by_key(|t| t.from());
    let mut out: Vec<DayTour> = tours
        .into_iter()
        .map(|t| {
            let duties = network::duties_of(&t)
                .into_iter()
                .map(|r| DayDuty { from: t.trips[r.start].dep, to: t.trips[r.clone()].iter().map(|x| x.arr).max().unwrap_or(0), start: r.start, end: r.end, ..Default::default() })
                .collect();
            DayTour { tour: t, duties, ..Default::default() }
        })
        .collect();

    // 1. the buses: the roster's, then the dispatcher's own, then what fell out
    let mut buses = Buses::default();
    for t in out.iter_mut() {
        let Some(id) = roster(c, wd, &t.tour.line, &t.tour.tour).and_then(|r| r.bus) else {
            continue;
        };
        let (from, to) = (t.tour.from(), t.tour.to());
        let problem = match c.vehicle(id) {
            None => Some(Problem::Gone),
            Some(v) if !v.held_on(date) => Some(Problem::Gone),
            Some(_) if broken.contains(&id) => Some(Problem::Breakdown),
            Some(v) if v.in_workshop(date) => Some(Problem::Workshop),
            Some(v) if !t.tour.lines().iter().all(|l| super::ownline::line_allows(c, l, v)) => Some(Problem::NotLineBus),
            Some(_) if !buses.free(id, from, to) => Some(Problem::BusBusy),
            Some(_) => None,
        };
        match problem {
            None => {
                buses.take(id, from, to);
                t.bus = Some(BusOf::Own(id));
                t.bus_from = Source::Roster;
            }
            p => t.bus_problem = p,
        }
    }
    let rostered_bus: Vec<u32> = c.planning.week.iter().filter(|r| r.weekday == wd).filter_map(|r| r.bus).collect();
    for t in out.iter_mut().filter(|t| t.bus.is_none() && t.bus_problem.is_none()) {
        // (a line that asks for buses the fleet has none of says so)
        let none = if super::ownline::fleet_has_bus_for(c, &t.tour.line) { Problem::Unassigned } else { Problem::NoLineBus };
        if !auto(&t.tour.line) || !auto_key(&tour_key(&t.tour.line, &t.tour.tour)) {
            t.bus_problem = Some(none);
            continue;
        }
        let (from, to, wants) = (t.tour.from(), t.tour.to(), super::ownline::wanted(c, &t.tour));
        // (free in time and one of the line's buses; the size asked for first, a bus no roster
        // tour has, then the one free the latest: the others stay free for later tours)
        let best = c
            .fleet
            .iter()
            .filter(|v| usable(v.id) && buses.free(v.id, from, to) && t.tour.lines().iter().all(|l| super::ownline::line_allows(c, l, v)))
            .max_by_key(|v| (size_fit(wants, v.kind.size), !rostered_bus.contains(&v.id), buses.last_end(v.id, from), std::cmp::Reverse(v.id)));
        match best {
            Some(v) => {
                buses.take(v.id, from, to);
                t.bus = Some(BusOf::Own(v.id));
                t.bus_from = Source::Auto;
            }
            None => t.bus_problem = Some(none),
        }
    }
    if today {
        for t in out.iter_mut().filter(|t| t.bus.is_none()) {
            let (from, to, wants) = (t.tour.from(), t.tour.to(), super::ownline::wanted(c, &t.tour));
            match fill_of(c, date, &t.key()) {
                Some(Fill::Bus { id }) if usable(id) && buses.free(id, from, to) => {
                    buses.take(id, from, to);
                    t.bus = Some(BusOf::Own(id));
                    t.bus_from = Source::Dispatcher;
                }
                Some(Fill::Rental) => {
                    t.bus = Some(BusOf::Rental);
                    t.bus_from = Source::Dispatcher;
                }
                Some(Fill::Drop) => t.bus_from = Source::Dispatcher,
                // (what broke down: a spare bus of the line proposed)
                _ if t.bus_problem.is_some_and(Problem::sudden) => {
                    let best = c
                        .fleet
                        .iter()
                        .filter(|v| usable(v.id) && buses.free(v.id, from, to) && t.tour.lines().iter().all(|l| super::ownline::line_allows(c, l, v)))
                        .max_by_key(|v| (size_fit(wants, v.kind.size), std::cmp::Reverse(v.id)));
                    if let Some(v) = best {
                        buses.take(v.id, from, to);
                        t.bus = Some(BusOf::Own(v.id));
                        t.bus_from = Source::Central;
                    }
                }
                _ => {}
            }
        }
    }

    // 2. the drivers: the agency drivers booked for the day on the duties given them that day
    // (`agency`), then the roster's, in the order the duties begin
    let mut work = Work::default();
    let mut order: Vec<(usize, usize)> = out.iter().enumerate().flat_map(|(i, t)| (0..t.duties.len()).map(move |k| (i, k))).collect();
    order.sort_by_key(|&(i, k)| out[i].duties[k].from);
    let booked: Vec<&Employee> = super::agency::drivers_on(c, date);
    for e in &booked {
        for p in super::agency::duties_of(c, e.id) {
            let found = order.iter().copied().find(|&(i, k)| {
                let t = &out[i];
                same(&t.tour.line, &p.line) && same(&t.tour.tour, &p.tour) && t.tour.trips.get(t.duties[k].start).is_some_and(|x| p.first.is(x))
            });
            let Some((i, k)) = found.filter(|&(i, k)| out[i].duties[k].who.is_none()) else {
                continue;
            };
            let bus = out[i].bus_of(c);
            let t = &out[i];
            let block = block_of(&t.tour, &t.duties[k], duty_key(&t.tour.line, &t.tour.tour, k));
            if let Ok(w) = fits_bus(e, &bus, work.of(e.id), &block, today) {
                work.push(e.id, block);
                let d = &mut out[i].duties[k];
                d.who = Some(Who::Staff(e.id));
                d.from_ = Source::Dispatcher;
                d.warn = w;
            }
        }
    }
    for &(i, k) in &order {
        if out[i].duties[k].who.is_some() {
            continue;
        }
        let Some(who) = roster(c, wd, &out[i].tour.line, &out[i].tour.tour).and_then(|r| r.duties.get(k).copied().flatten()) else {
            continue;
        };
        let bus = out[i].bus_of(c);
        let t = &out[i];
        let block = block_of(&t.tour, &t.duties[k], duty_key(&t.tour.line, &t.tour.tour, k));
        let d = &mut out[i].duties[k];
        match who {
            Who::Agency => {
                d.who = Some(who);
                d.from_ = Source::Roster;
            }
            Who::Staff(id) => {
                let check = match c.employee(id) {
                    None => Err(Problem::Away),
                    Some(e) => match unavailable(e, date) {
                        Some(p) => Err(p),
                        None => fits_bus(e, &bus, work.of(id), &block, today),
                    },
                };
                match check {
                    Ok(w) => {
                        work.push(id, block);
                        d.who = Some(who);
                        d.from_ = Source::Roster;
                        d.warn = w;
                    }
                    Err(p) => d.problem = Some(p),
                }
            }
        }
    }
    // the dispatcher's own for what the roster left free (tours with a bus): the agency drivers
    // booked for the day first (they are paid for the day all the same), within the hours they
    // come for; then the standby drivers
    let people: Vec<&Employee> = c.staff.iter().filter(|e| unavailable(e, date).is_none()).chain(booked.iter().copied()).collect();
    let pick = |work: &Work, bus: &(BusKind, Option<String>), block: &Block| -> Option<(u32, Vec<Warn>)> {
        let mut best: Option<((bool, bool, bool, bool, i32, u32), u32, Vec<Warn>)> = None;
        for e in &people {
            let is_booked = super::agency::is_booked(e.id);
            if is_booked && !super::agency::within(c, e.id, block) {
                continue;
            }
            if !shift_fits(e, block) {
                continue;
            }
            let Ok(w) = fits_bus(e, bus, work.of(e.id), block, today) else {
                continue;
            };
            let worked = staff::work_minutes(work.of(e.id));
            let fills = worked > 0 && !w.iter().any(|x| matches!(x, Warn::Overtime(_)));
            let room = e.week_days_max.saturating_sub(e.week_days);
            let score = (is_booked, e.standby && worked == 0, staff::qualified(e, bus.0.size), fills, -worked, room);
            if best.as_ref().is_none_or(|b| score > b.0) {
                best = Some((score, e.id, w));
            }
        }
        best.map(|b| (b.1, b.2))
    };
    for &(i, k) in &order {
        let t = &out[i];
        if t.duties[k].who.is_some() || t.duties[k].problem.is_some() || t.bus.is_none() {
            continue;
        }
        if !auto(&t.tour.line) || !auto_key(&duty_key(&t.tour.line, &t.tour.tour, k)) {
            out[i].duties[k].problem = Some(Problem::Unassigned);
            continue;
        }
        let bus = t.bus_of(c);
        let block = block_of(&t.tour, &t.duties[k], duty_key(&t.tour.line, &t.tour.tour, k));
        // (a piece of a duty joined in the studio: its other pieces' driver first)
        let partner = super::studio::partner_driver(c, super::plandays::slot_of(c, date), &out, i, k).and_then(|id| {
            let e = c.employee(id).filter(|e| unavailable(e, date).is_none())?;
            fits_bus(e, &bus, work.of(id), &block, today).ok().map(|w| (id, w))
        });
        match partner.or_else(|| pick(&work, &bus, &block)) {
            Some((id, w)) => {
                work.push(id, block);
                let d = &mut out[i].duties[k];
                d.who = Some(Who::Staff(id));
                d.from_ = Source::Auto;
                d.warn = w;
            }
            None => out[i].duties[k].problem = Some(Problem::Unassigned),
        }
    }
    // a driver on holiday (known ahead): a free colleague covers their duty, on any day planned,
    // the roster kept theirs (today the dispatcher's own choice first)
    for &(i, k) in &order {
        let t = &out[i];
        let d = &t.duties[k];
        if d.who.is_some() || d.problem != Some(Problem::Holiday) || t.bus.is_none() {
            continue;
        }
        let key = duty_key(&t.tour.line, &t.tour.tour, k);
        if today && fill_of(c, date, &key).is_some() {
            continue;
        }
        let bus = t.bus_of(c);
        let block = block_of(&t.tour, d, key);
        if let Some((id, w)) = pick(&work, &bus, &block) {
            work.push(id, block);
            let d = &mut out[i].duties[k];
            d.who = Some(Who::Staff(id));
            d.from_ = Source::Central;
            d.warn = w;
        }
    }
    // what is open today: the dispatcher's choice, else a free colleague for what came suddenly
    if today {
        let slot = super::plandays::slot_of(c, date);
        for &(i, k) in &order {
            let t = &out[i];
            if t.duties[k].who.is_some() || t.bus.is_none() {
                continue;
            }
            let bus = t.bus_of(c);
            let key = duty_key(&t.tour.line, &t.tour.tour, k);
            let block = block_of(&t.tour, &t.duties[k], key.clone());
            let problem = t.duties[k].problem;
            // (a piece joined with others into one driver's duty: the choice for any of them is
            // the whole duty's, and one agency driver drives all of it)
            let joined = joined_pieces(c, slot, &out, i, k);
            let partner_fill = || joined.iter().find_map(|&(j, m)| fill_of(c, date, &duty_key(&out[j].tour.line, &out[j].tour.tour, m)).filter(|f| matches!(f, Fill::Agency | Fill::Colleague { .. })));
            let (who, from_) = match fill_of(c, date, &key).or_else(partner_fill) {
                Some(Fill::Colleague { id }) => match c.employee(id).filter(|e| unavailable(e, date).is_none()).map(|e| fits_bus(e, &bus, work.of(id), &block, today)) {
                    Some(Ok(_)) => (Some(Who::Staff(id)), Source::Dispatcher),
                    _ => (None, Source::None),
                },
                Some(Fill::Agency) => (Some(Who::Agency), Source::Dispatcher),
                Some(Fill::Drop) => (None, Source::Dispatcher),
                _ if problem.is_some_and(Problem::sudden) => match pick(&work, &bus, &block) {
                    Some((id, _)) => (Some(Who::Staff(id)), Source::Central),
                    None => (None, Source::None),
                },
                _ => (None, Source::None),
            };
            if let Some(Who::Staff(id)) = who {
                work.push(id, block);
            }
            let d = &mut out[i].duties[k];
            if who.is_some() || from_ != Source::None {
                d.who = who;
                d.from_ = from_;
            }
        }
        // late in the morning: the first duty of the day begins without them, until they arrive
        // (kept when first planned: moved to a duty that begins after it, they are not late)
        // (the day's spare duties, each driving one late driver's first trips at a time)
        let spares = super::rosterflow::spare_lines(c, date);
        let mut spare_busy: Vec<(u32, i32, i32)> = Vec::new();
        for dz in &dis {
            let Disruption::Late { employee, minutes } = *dz else {
                continue;
            };
            let mine: Vec<(usize, usize)> = order.iter().copied().filter(|&(i, k)| out[i].duties[k].who == Some(Who::Staff(employee))).collect();
            let Some(&(i, k)) = mine.first() else {
                continue;
            };
            let t = &out[i];
            let d = &t.duties[k];
            let until = arrival(c, date, employee).unwrap_or(d.from + minutes);
            if !t.tour.trips[d.start..d.end].iter().any(|x| x.dep < until) {
                continue;
            }
            let key = late_key(&t.tour.line, &t.tour.tour, k);
            let piece = Block { key: key.clone(), from: d.from, to: until.min(d.to), from_stop: t.tour.trips[d.start].from.clone(), to_stop: String::new() };
            let bus = t.bus_of(c);
            let (start, end) = (d.from, until.min(d.to));
            let spare_free = |want: Option<u32>, busy: &[(u32, i32, i32)]| {
                spares.iter().find(|x| want.is_none_or(|w| w == x.id) && x.covers(start, end) && !busy.iter().any(|b| b.0 == x.id && b.1 < end && start < b.2)).map(|x| x.id)
            };
            let (cover, from_) = match fill_of(c, date, &key) {
                Some(Fill::Spare { id }) => (spare_free(Some(id), &spare_busy).map(|id| SPARE_COVER + id), Source::Dispatcher),
                Some(Fill::Colleague { id }) if c.employee(id).is_some_and(|e| id != employee && unavailable(e, date).is_none() && fits_bus(e, &bus, work.of(id), &piece, today).is_ok()) => {
                    (Some(id), Source::Dispatcher)
                }
                Some(Fill::Agency) => (Some(AGENCY), Source::Dispatcher),
                Some(_) => (None, Source::Dispatcher),
                None => match (spare_free(None, &spare_busy), pick(&work, &bus, &piece)) {
                    (Some(id), _) => (Some(SPARE_COVER + id), Source::Central),
                    (None, Some((id, _))) if id != employee => (Some(id), Source::Central),
                    _ => (None, Source::None),
                },
            };
            match cover.and_then(spare_of_cover) {
                Some(id) => spare_busy.push((id, start, end)),
                None => {
                    if let Some(id) = cover {
                        work.push(id, piece);
                    }
                }
            }
            out[i].duties[k].late = Some(LatePiece { until, cover, from_, cover_from: None });
        }
    }
    DayPlan { date: date.to_string(), weekday: wd, today, tours: out, disruptions: dis }
}

/// A driver who works only early or only late duties takes a duty of that kind: early ends by
/// 14:30, late begins at 12:00 or later.
fn shift_fits(e: &Employee, b: &Block) -> bool {
    match e.shift {
        super::model::ShiftPref::Any => true,
        super::model::ShiftPref::Early => b.to <= 14 * 60 + 30,
        super::model::ShiftPref::Late => b.from >= 12 * 60,
    }
}

/// A stand-in for an agency driver in the day's model.
pub fn agency_driver() -> Employee {
    Employee {
        id: AGENCY,
        name: "Agency driver".into(),
        licence: Licence::D,
        // (the agency sends drivers for what they are sent to)
        endorsements: super::licences::Endorsement::ALL.to_vec(),
        types: vec![super::agency::ANY_TYPE.to_string()],
        role: super::model::Role::Driver,
        ..Default::default()
    }
}

/// The days of the weekly roster (other than `weekday`) on which each driver has a duty: what
/// `Employee::week_days` is when `weekday` is planned.
pub fn week_days_rostered(c: &Company, weekday: u8) -> std::collections::HashMap<u32, u32> {
    let mut days: std::collections::HashMap<u32, Vec<u8>> = std::collections::HashMap::new();
    for r in c.planning.week.iter().filter(|r| r.weekday != weekday && r.weekday < 7) {
        for w in r.duties.iter().flatten() {
            if let Who::Staff(id) = w {
                let v = days.entry(*id).or_default();
                if !v.contains(&r.weekday) {
                    v.push(r.weekday);
                }
            }
        }
    }
    days.into_iter().map(|(k, v)| (k, v.len() as u32)).collect()
}

/// The company as `date` is planned with it: each driver's days of the week rostered already
/// (`week_days`, against the company's rule in `week_days_max`) and when their last duty of the
/// day before ended (`prev`: driver id to the minute, of that day; `last_end`).
pub fn prepared(c: &Company, date: &str, prev: &std::collections::HashMap<u32, i32>) -> Company {
    let mut p = c.clone();
    let wd = weekday_of(date);
    let week = week_days_rostered(c, wd);
    let rules = c.staff_rules.clone();
    for e in p.staff.iter_mut() {
        e.week_days = week.get(&e.id).copied().unwrap_or(0);
        e.week_days_max = rules.week_days_of(e);
        e.last_end = prev.get(&e.id).copied();
    }
    p
}

/// When each driver's last duty of plan `dp` ends (minutes of its day).
pub fn last_ends(dp: &DayPlan) -> std::collections::HashMap<u32, i32> {
    let mut out = std::collections::HashMap::new();
    for t in &dp.tours {
        for d in &t.duties {
            if let Some(Who::Staff(id)) = d.who {
                let e = out.entry(id).or_insert(d.to);
                *e = (*e).max(d.to);
            }
        }
    }
    out
}

#[cfg(test)]
mod tests;
