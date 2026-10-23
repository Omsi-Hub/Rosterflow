//! The company as it is saved: who it is, its depot, its lines, its buses and its people, and
//! its planning. Amounts are whole cents everywhere, dates `YYYY-MM-DD`, times minutes of the
//! day (past 1440 the next day).

use super::dates;
use serde::{Deserialize, Serialize};

pub type Cents = i64;

/// The version of the saved file; raised when a field changes meaning.
pub const VERSION: u32 = 1;

/// A bus's length class: what it carries and who may drive it.
#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Debug, Default, Hash, PartialOrd, Ord)]
#[serde(rename_all = "lowercase")]
pub enum BusSize {
    /// Up to about 10.5 m.
    Midi,
    /// The 12 m standard bus.
    #[default]
    Solo,
    /// 18 m, with a trailer section.
    Articulated,
    Double,
}

impl BusSize {
    pub const ALL: [BusSize; 4] = [BusSize::Midi, BusSize::Solo, BusSize::Articulated, BusSize::Double];

    pub fn label(self) -> &'static str {
        match self {
            BusSize::Midi => "Midibus",
            BusSize::Solo => "Solo bus",
            BusSize::Articulated => "Articulated bus",
            BusSize::Double => "Double-decker",
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Debug, Default, Hash, PartialOrd, Ord)]
#[serde(rename_all = "lowercase")]
pub enum Drive {
    #[default]
    Diesel,
    Electric,
    Hybrid,
    Hydrogen,
    Gas,
}

impl Drive {
    pub const ALL: [Drive; 5] = [Drive::Diesel, Drive::Electric, Drive::Hybrid, Drive::Hydrogen, Drive::Gas];

    pub fn label(self) -> &'static str {
        match self {
            Drive::Diesel => "Diesel",
            Drive::Electric => "Electric",
            Drive::Hybrid => "Hybrid",
            Drive::Hydrogen => "Hydrogen",
            Drive::Gas => "Gas",
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Debug, Default, Hash)]
pub struct BusKind {
    pub size: BusSize,
    pub drive: Drive,
}

impl BusKind {
    pub fn label(self) -> String {
        match self.drive {
            Drive::Diesel => self.size.label().to_string(),
            d => format!("{} ({})", self.size.label(), d.label().to_lowercase()),
        }
    }
}

/// A bus of the fleet.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(default)]
pub struct Vehicle {
    /// Unique in the company, never given again.
    pub id: u32,
    /// The fleet number on its sides.
    pub number: String,
    pub plate: String,
    /// Its make and model ("Mercedes-Benz Citaro"): the type training a driver needs
    /// (`licences::type_key`).
    pub name: String,
    pub kind: BusKind,
    /// Seats and standing places.
    pub seats: u32,
    pub standing: u32,
    /// First registered.
    pub built: String,
    /// In the company's fleet from and up to and including (empty: always).
    pub from: String,
    pub until: String,
    /// In the workshop up to and including this day: not planned (`plan`).
    pub workshop_until: Option<String>,
    /// The depot it is stationed at (empty: the company's).
    pub depot: String,
    pub notes: String,
}

impl Default for Vehicle {
    fn default() -> Self {
        Vehicle {
            id: 0,
            number: String::new(),
            plate: String::new(),
            name: String::new(),
            kind: BusKind::default(),
            seats: 0,
            standing: 0,
            built: String::new(),
            from: String::new(),
            until: String::new(),
            workshop_until: None,
            depot: String::new(),
            notes: String::new(),
        }
    }
}

impl Vehicle {
    pub fn in_workshop(&self, today: &str) -> bool {
        self.workshop_until.as_deref().is_some_and(|u| !u.is_empty() && dates::between(today, u) >= 0)
    }

    /// Still the company's on `today`.
    pub fn held_on(&self, today: &str) -> bool {
        (self.from.trim().is_empty() || dates::between(&self.from, today) >= 0) && (self.until.trim().is_empty() || dates::between(today, &self.until) >= 0)
    }

    /// Its fleet number and model, as a list names it.
    pub fn label(&self) -> String {
        if self.name.trim().is_empty() {
            self.number.clone()
        } else {
            format!("{} · {}", self.number, self.name.trim())
        }
    }
}

/// A bus driving licence: D every bus, D1 only small ones.
#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum Licence {
    #[default]
    D,
    D1,
}

/// Someone on the payroll.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(default)]
pub struct Employee {
    pub id: u32,
    pub name: String,
    /// The personnel number the company knows them by.
    pub number: String,
    pub phone: String,
    pub email: String,
    pub licence: Licence,
    /// The licence's (code 95) validity: driving after it is refused (empty: not kept).
    pub licence_until: String,
    /// Hourly gross wage (empty: the company's rate, `rosterflow::Prefs::wage`).
    pub wage: Cents,
    /// Hours a week by contract (0: full time, `staff::StaffRules`).
    pub contract_hours: f64,
    /// Days a week they work at most (0: the company's rule).
    pub week_days_max: u32,
    pub hired: String,
    /// Leaves after this day.
    pub notice_until: Option<String>,
    /// Ill up to and including this day (open-ended: `sick_open`).
    pub sick_until: Option<String>,
    pub sick_open: bool,
    /// On holiday up to and including this day (one entered by the planner directly; the
    /// requests are `leave`).
    pub holiday_until: Option<String>,
    /// Away on a course up to and including this day.
    pub training_until: Option<String>,
    /// Holiday days a year, and those left this year.
    pub holiday_days: u32,
    pub holiday_left: u32,
    /// Besides the licence: the buses they may drive (`licences::Endorsement`), and the models
    /// they had the type training for (`licences::type_key`).
    pub endorsements: Vec<super::licences::Endorsement>,
    pub types: Vec<String>,
    /// What they do: drive, or another job (they are not planned on duties).
    pub role: Role,
    /// Their holidays and the answers (`holidays`).
    pub leave: super::holidays::Leave,
    /// Days of the week they never work (0 Monday .. 6 Sunday).
    pub days_off: Vec<u8>,
    /// Only early or only late duties, or either.
    pub shift: ShiftPref,
    /// A reserve driver: put on the open duties of the day first.
    pub standby: bool,
    /// A booked agency driver's number of his day (`agency::number_of`; 0: none).
    pub agency_no: u32,
    /// Runtime state of a week's roster (not entered): days worked so far, and when their
    /// last duty ended (minutes from the start of the day before the one planned).
    #[serde(skip)]
    pub week_days: u32,
    #[serde(skip)]
    pub last_end: Option<i32>,
    pub notes: String,
}

/// When a driver prefers to work.
#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Debug, Default)]
#[serde(rename_all = "snake_case")]
pub enum ShiftPref {
    #[default]
    Any,
    Early,
    Late,
}

impl Default for Employee {
    fn default() -> Self {
        Employee {
            id: 0,
            name: String::new(),
            number: String::new(),
            phone: String::new(),
            email: String::new(),
            licence: Licence::D,
            licence_until: String::new(),
            wage: 0,
            contract_hours: 0.0,
            week_days_max: 0,
            hired: String::new(),
            notice_until: None,
            sick_until: None,
            sick_open: false,
            holiday_until: None,
            training_until: None,
            holiday_days: 25,
            holiday_left: 25,
            endorsements: Vec::new(),
            types: Vec::new(),
            role: Role::Driver,
            leave: Default::default(),
            days_off: Vec::new(),
            shift: ShiftPref::Any,
            standby: false,
            agency_no: 0,
            week_days: 0,
            last_end: None,
            notes: String::new(),
        }
    }
}

/// What an employee does.
#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Debug, Default, Hash)]
#[serde(rename_all = "snake_case")]
pub enum Role {
    #[default]
    Driver,
    Mechanic,
    Planner,
    Dispatcher,
    Cleaner,
    Office,
}

impl Role {
    pub const ALL: [Role; 6] = [Role::Driver, Role::Mechanic, Role::Planner, Role::Dispatcher, Role::Cleaner, Role::Office];

    /// Its name (English, translated when shown).
    pub fn label(self) -> &'static str {
        match self {
            Role::Driver => "Bus driver",
            Role::Mechanic => "Mechanic",
            Role::Planner => "Planner",
            Role::Dispatcher => "Dispatcher",
            Role::Cleaner => "Cleaner",
            Role::Office => "Office",
        }
    }
}

impl Employee {
    fn until(u: &Option<String>, today: &str) -> bool {
        u.as_deref().is_some_and(|u| !u.is_empty() && dates::between(today, u) >= 0)
    }

    /// Ill on `today`: up to the day entered, or from it on until they are reported fit.
    pub fn sick_on(&self, today: &str) -> bool {
        if self.sick_open {
            return self.sick_until.as_deref().is_none_or(|from| from.is_empty() || dates::between(from, today) >= 0);
        }
        Self::until(&self.sick_until, today)
    }

    pub fn absent(&self, today: &str) -> bool {
        self.sick_on(today) || self.on_holiday(today).is_some() || Self::until(&self.training_until, today)
    }

    /// The last day of the holiday they are on at `date`: one entered directly
    /// (`holiday_until`), or one approved (`holidays`).
    pub fn on_holiday(&self, date: &str) -> Option<&str> {
        self.holiday_until.as_deref().filter(|u| !u.is_empty() && dates::between(date, u) >= 0).or_else(|| self.leave.approved_on(date).map(|r| r.until.as_str()))
    }

    pub fn employed_on(&self, today: &str) -> bool {
        (self.hired.trim().is_empty() || dates::between(&self.hired, today) >= 0) && self.notice_until.as_deref().is_none_or(|u| u.is_empty() || dates::between(today, u) >= 0)
    }

    /// They never work on the weekday of `date`.
    pub fn day_off(&self, date: &str) -> bool {
        let wd = dates::parse(date).map(|d| dates::weekday(d) as u8).unwrap_or(0);
        self.days_off.contains(&wd)
    }
}

/// A line the company runs: the planning's name for it (`name`, the line's key) and what the
/// pages show of it. Its stops, days and timetable are its design (`Company::network`).
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Default)]
#[serde(default)]
pub struct CompanyLine {
    /// The line's key in the planning (its design's `timetable_name`).
    pub name: String,
    /// The number shown, and every number its trips show.
    pub number: String,
    pub numbers: Vec<String>,
    /// `#rrggbb`.
    pub colour: String,
    /// Where it goes.
    pub caption: String,
    pub added: String,
    /// What the timetable gave the last time it was read: tours and kilometres a day.
    pub tours: u32,
    pub km: f64,
    /// What the line is called in public; empty: its number and where it goes.
    pub title: String,
}

impl CompanyLine {
    pub fn public_name(&self) -> &str {
        if self.title.trim().is_empty() {
            self.caption.trim()
        } else {
            self.title.trim()
        }
    }
}

/// A depot (garage) of the company: where its buses sleep, and the stop name the timetable
/// knows it by.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Default)]
#[serde(default)]
pub struct Depot {
    pub name: String,
    pub address: String,
}

/// Numbers that only go up: a bus sold or a person who left never gives theirs away.
#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq)]
pub struct Counters {
    pub vehicle: u32,
    pub employee: u32,
    pub fleet_number: u32,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(default)]
pub struct Company {
    pub version: u32,
    /// The file's name (`<id>.rosterflow`).
    pub id: String,
    pub name: String,
    /// Two to four letters: the logo's monogram.
    pub short: String,
    /// `#rrggbb`: the main colour and the second.
    pub colours: [String; 2],
    /// The main depot: the name the timetable's depot runs start and end at.
    pub depot: String,
    /// Every depot (the first is `depot`).
    pub depots: Vec<Depot>,
    /// When the company was entered.
    pub founded: String,
    /// Today, as the planning sees it (set when the company is opened).
    pub date: String,
    /// The currency amounts are shown in (an ISO code).
    pub currency: String,
    /// The country of its public holidays (an ISO code: `holidays`).
    pub country: String,
    pub fleet: Vec<Vehicle>,
    pub staff: Vec<Employee>,
    pub lines: Vec<CompanyLine>,
    /// The stops and the lines' designs: routes, days, timetables, running times.
    pub network: crate::lines::Registry,
    pub counters: Counters,
    /// The weekly roster, the blocks and duties as laid out, and RosterFlow's own.
    pub planning: super::plan::Planning,
    /// The working-time rules of the staff.
    pub staff_rules: super::staff::StaffRules,
}

impl Default for Company {
    fn default() -> Self {
        Company {
            version: VERSION,
            id: String::new(),
            name: String::new(),
            short: String::new(),
            colours: ["#2f80ed".into(), "#0f1b2d".into()],
            depot: "Depot".into(),
            depots: Vec::new(),
            founded: String::new(),
            date: String::new(),
            currency: "EUR".into(),
            country: "NL".into(),
            fleet: Vec::new(),
            staff: Vec::new(),
            lines: Vec::new(),
            network: Default::default(),
            counters: Default::default(),
            planning: Default::default(),
            staff_rules: Default::default(),
        }
    }
}

impl Company {
    pub fn vehicle(&self, id: u32) -> Option<&Vehicle> {
        self.fleet.iter().find(|v| v.id == id)
    }

    pub fn employee(&self, id: u32) -> Option<&Employee> {
        // (an agency driver booked for a day is one of its drivers that day: `agency`)
        self.staff.iter().find(|e| e.id == id).or_else(|| super::agency::driver(self, id))
    }

    /// The next fleet number: from 101 on, never one given before.
    pub fn next_fleet_number(&mut self) -> String {
        loop {
            self.counters.fleet_number = self.counters.fleet_number.max(100) + 1;
            let n = self.counters.fleet_number.to_string();
            if !self.fleet.iter().any(|v| v.number == n) {
                return n;
            }
        }
    }

    /// Whether `place` is one of the company's depots.
    pub fn is_depot_name(&self, place: &str) -> bool {
        let p = place.trim();
        !p.is_empty() && (self.depot.trim().eq_ignore_ascii_case(p) || self.depots.iter().any(|d| d.name.trim().eq_ignore_ascii_case(p)))
    }
}
