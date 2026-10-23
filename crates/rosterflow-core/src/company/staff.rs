//! The company's people as the planning sees them: the working-time rules a duty must keep - a
//! duty of at most 9½ hours, a working day of at most 10 (overtime above 8), 20 minutes between
//! two pieces of work, 11 hours' rest overnight, a transfer to another stop needs 45 minutes -
//! and who may drive what. RosterFlow's preferences (`rosterflow::Prefs`) set the limits its
//! duties are checked against; these are the roster's own.

use super::licences::Endorsement;
use super::model::{BusSize, Cents, Employee, Licence, Role};
use serde::{Deserialize, Serialize};

// --- working time ----------------------------------------------------------------------------

/// A duty lasts at most this long (minutes); a longer tour is split.
pub const DUTY_MAX: i32 = 570;
/// A tour is split only where the bus stands at least this long.
pub const SPLIT_PAUSE: i32 = 3;
/// Rest between two pieces of work, overnight rest, time for a transfer between stops.
pub const REST: i32 = 20;
pub const NIGHT_REST: i32 = 660;
pub const TRANSFER: i32 = 45;
/// A working day's target (overtime above it) and its hard limit.
pub const DAY_TARGET: i32 = 480;
pub const DAY_MAX: i32 = 600;
/// Between two tours of the same bus.
pub const BUS_MARGIN: i32 = 10;
/// Days a week someone works at most.
pub const WEEK_DAYS: u32 = 5;
/// Holiday days a year.
pub const HOLIDAYS: u32 = 25;
/// The night (20:00 to 06:00), paid half as much again in the figures.
pub const NIGHT_FROM: i32 = 20 * 60;
pub const NIGHT_TO: i32 = 6 * 60;
pub const NIGHT_SUPPLEMENT: f64 = 0.5;
/// A day worked pays at least four hours.
pub const MIN_PAID: i32 = 240;
/// A full-time week, hours.
pub const FULL_WEEK: f64 = 38.0;

/// A piece of work of one driver or bus: minutes of the day, and the stops it begins and
/// ends at.
#[derive(Clone, Debug, PartialEq, Default)]
pub struct Block {
    pub key: String,
    pub from: i32,
    pub to: i32,
    pub from_stop: String,
    pub to_stop: String,
}

pub fn overlaps(a: &Block, b: &Block, margin: i32) -> bool {
    a.from < b.to + margin && b.from < a.to + margin
}

pub fn work_minutes(blocks: &[Block]) -> i32 {
    blocks.iter().map(|b| b.to - b.from).sum()
}

/// How a new piece of work fits what a driver has already.
#[derive(Clone, Debug, PartialEq, Default)]
pub struct Check {
    /// The key of a block it overlaps.
    pub double: Option<String>,
    /// The working day goes over `DAY_MAX`.
    pub too_long: bool,
    /// The shortest pause to a neighbouring block.
    pub rest: Option<i32>,
    /// That pause, when it is shorter than `TRANSFER` and the stops differ.
    pub transfer: Option<i32>,
    /// Minutes over `DAY_TARGET`.
    pub overtime: i32,
    /// The shortest rest against the day before (its last end, minutes of that day) or the
    /// day after (its first start).
    pub night_rest: Option<i32>,
}

impl Check {
    /// Allowed at all (the hard limits; the rest are warnings).
    pub fn allowed(&self) -> bool {
        self.double.is_none() && !self.too_long && self.rest.is_none_or(|r| r >= REST) && self.night_rest.is_none_or(|r| r >= NIGHT_REST)
    }
}

pub fn check(existing: &[Block], new: &Block, prev_end: Option<i32>, next_start: Option<i32>) -> Check {
    let others: Vec<&Block> = existing.iter().filter(|b| b.key != new.key).collect();
    let double = others.iter().find(|b| overlaps(b, new, 0)).map(|b| b.key.clone());
    let mut all: Vec<&Block> = others.clone();
    all.push(new);
    all.sort_by_key(|b| b.from);
    let work: i32 = all.iter().map(|b| b.to - b.from).sum();
    let at = all.iter().position(|b| std::ptr::eq(*b, new)).unwrap_or(0);
    let mut pairs = Vec::new();
    if at > 0 {
        pairs.push((all[at - 1], new));
    }
    if at + 1 < all.len() {
        pairs.push((new, all[at + 1]));
    }
    let (mut rest, mut transfer) = (None::<i32>, None::<i32>);
    for (a, b) in pairs {
        let pause = b.from - a.to;
        if pause < 0 {
            continue;
        }
        rest = Some(rest.map_or(pause, |r| r.min(pause)));
        if pause < TRANSFER && !a.to_stop.is_empty() && !b.from_stop.is_empty() && a.to_stop != b.from_stop {
            transfer = Some(transfer.map_or(pause, |t| t.min(pause)));
        }
    }
    let mut nights = Vec::new();
    if let Some(p) = prev_end {
        nights.push(1440 + all[0].from - p);
    }
    if let Some(n) = next_start {
        nights.push(1440 + n - all[all.len() - 1].to);
    }
    Check { double, too_long: work > DAY_MAX, rest, transfer, overtime: (work - DAY_TARGET).max(0), night_rest: nights.into_iter().min() }
}

/// May drive a bus of this size at all (a driver, and the licence: D1 only midibuses; a size
/// that wants an endorsement only with it).
pub fn may_drive(e: &Employee, size: BusSize) -> bool {
    drives(e) && (e.licence == Licence::D || size == BusSize::Midi)
}

/// Drives it without a warning: may drive it, and has the endorsement a large bus wants
/// (`licences::Endorsement`) where the company keeps them.
pub fn qualified(e: &Employee, size: BusSize) -> bool {
    may_drive(e, size)
        && match size {
            BusSize::Articulated => e.endorsements.is_empty() || e.endorsements.contains(&Endorsement::Articulated),
            BusSize::Double => e.endorsements.is_empty() || e.endorsements.contains(&Endorsement::DoubleDecker),
            _ => true,
        }
}

/// Whether someone drives buses. Only the drivers are planned on duties.
pub fn drives(e: &Employee) -> bool {
    e.role == Role::Driver
}

/// An hour of someone's wage (cents, gross): their own, else `rate`.
pub fn hourly_of(e: &Employee, rate: f64) -> f64 {
    if e.wage > 0 {
        e.wage as f64
    } else {
        rate
    }
}

/// A wage per hour (cents) as cents.
pub fn hourly(wage: Cents) -> f64 {
    wage as f64
}

/// The minutes of `from..to` (minutes of a day; `to` may pass midnight) that fall in the
/// night, 20:00 to 06:00.
pub fn night_minutes(from: i32, to: i32) -> i32 {
    in_window(from, to, NIGHT_FROM, NIGHT_TO)
}

/// The minutes of `from..to` (minutes of a day, `to` up to a day later) that fall in the daily
/// window `a..b` (minutes of the day; `b` before `a` passes midnight).
pub fn in_window(from: i32, to: i32, a: i32, b: i32) -> i32 {
    let len = (b - a).rem_euclid(1440);
    (-1..=2).map(|k| (to.min(a + 1440 * k + len) - from.max(a + 1440 * k)).max(0)).sum()
}

/// The company's rules for its staff besides RosterFlow's preferences.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(default)]
pub struct StaffRules {
    /// Days a week a driver works at most (their own `week_days_max` first).
    pub week_days: u32,
    /// A full-time week (hours): what `Employee::contract_hours` 0 means.
    pub full_week: f64,
    /// How many may be off on holiday a day, and the requests' numbers (`holidays`).
    pub holidays: super::holidays::HolidayRules,
    /// The company keeps type training per bus model: a driver is planned only on the models
    /// they were trained for (`licences::lack`).
    pub type_training: bool,
}

impl Default for StaffRules {
    fn default() -> Self {
        StaffRules { week_days: WEEK_DAYS, full_week: FULL_WEEK, holidays: Default::default(), type_training: false }
    }
}

impl StaffRules {
    /// Days a week `e` works at most.
    pub fn week_days_of(&self, e: &Employee) -> u32 {
        if e.week_days_max > 0 {
            e.week_days_max
        } else if self.week_days > 0 {
            self.week_days
        } else {
            WEEK_DAYS
        }
    }

    /// Hours a week `e` is contracted for.
    pub fn hours_of(&self, e: &Employee) -> f64 {
        if e.contract_hours > 0.0 {
            e.contract_hours
        } else {
            self.full_week
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn b(key: &str, from: i32, to: i32, a: &str, z: &str) -> Block {
        Block { key: key.into(), from, to, from_stop: a.into(), to_stop: z.into() }
    }

    #[test]
    fn the_working_time_rules_hold() {
        let day = vec![b("1", 360, 600, "A", "B")];
        // a second piece after a 30 minutes' break elsewhere: allowed, a transfer warned
        let c = check(&day, &b("2", 630, 840, "C", "A"), None, None);
        assert!(c.allowed() && c.transfer == Some(30) && c.overtime == 0);
        // overlapping: refused
        assert_eq!(check(&day, &b("2", 590, 700, "B", "A"), None, None).double.as_deref(), Some("1"));
        // too long a day: refused
        assert!(check(&day, &b("2", 620, 1000, "B", "A"), None, None).too_long);
        // too little rest after yesterday's late duty
        assert!(!check(&[], &b("1", 360, 600, "A", "B"), Some(1400), None).allowed());
        assert_eq!(night_minutes(19 * 60, 22 * 60), 120);
        assert_eq!(night_minutes(23 * 60, 26 * 60), 180);
    }

    #[test]
    fn a_licence_says_what_one_drives() {
        let e = Employee { licence: Licence::D1, ..Default::default() };
        assert!(may_drive(&e, BusSize::Midi) && !may_drive(&e, BusSize::Solo));
        let e = Employee { endorsements: vec![Endorsement::Articulated], ..Default::default() };
        assert!(qualified(&e, BusSize::Articulated) && !qualified(&e, BusSize::Double));
        assert!(qualified(&Employee::default(), BusSize::Double));
        let rules = StaffRules::default();
        assert_eq!(rules.week_days_of(&Employee { week_days_max: 4, ..Default::default() }), 4);
        assert_eq!(rules.hours_of(&Employee::default()), FULL_WEEK);
    }
}
