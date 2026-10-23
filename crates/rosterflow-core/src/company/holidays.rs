//! Holidays asked for and planned: the planner enters each driver's request (`add_request`) and
//! answers it (`answer`) - approved, the days are theirs and the planning knows them ahead
//! (`Employee::on_holiday`): the dispatcher covers their duties with a free colleague without
//! touching the roster (`plan::day_plan`). The company sets how many drivers may be off on one
//! day (`HolidayRules::max_off`; by default a quarter of the fleet, `default_limit`), and RosterFlow
//! advises on each request (`advice`): within the limit, a date that cannot move, a driver turned
//! down twice already this year.

use super::dates;
use super::model::{Company, Employee, Role};
use serde::{Deserialize, Serialize};

/// The most the limit can be set to.
pub const MOST: u32 = 99;

/// Why someone asks for days off.
#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Debug, Default, Hash)]
#[serde(rename_all = "snake_case")]
pub enum Reason {
    /// A week away, a long weekend.
    #[default]
    Break,
    /// The summer holiday (late June to August).
    Summer,
    /// Christmas and the New Year.
    Christmas,
    /// A wedding, a christening, a big birthday: its day is fixed.
    Family,
    /// Moving house, an appointment: a day, often at short notice.
    Personal,
}

/// How much the days matter to them.
#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Debug, PartialOrd, Ord, Hash)]
#[serde(rename_all = "snake_case")]
pub enum Priority {
    Low,
    Normal,
    High,
}

impl Priority {
    pub fn label(self) -> &'static str {
        match self {
            Priority::Low => "Low priority",
            Priority::Normal => "Normal priority",
            Priority::High => "High priority",
        }
    }
}

impl Reason {
    pub const ALL: [Reason; 5] = [Reason::Break, Reason::Summer, Reason::Christmas, Reason::Family, Reason::Personal];

    /// Its name (English, translated when shown).
    pub fn label(self) -> &'static str {
        match self {
            Reason::Break => "A short break",
            Reason::Summer => "The summer holiday",
            Reason::Christmas => "Christmas with the family",
            Reason::Family => "A family occasion",
            Reason::Personal => "Personal matters",
        }
    }

    /// A date that cannot move is high, a holiday planned with the family normal, a break low.
    pub fn priority(self) -> Priority {
        match self {
            Reason::Family | Reason::Personal => Priority::High,
            Reason::Summer | Reason::Christmas => Priority::Normal,
            Reason::Break => Priority::Low,
        }
    }
}

/// Where a request stands.
#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Debug, Default, Hash)]
#[serde(rename_all = "snake_case")]
pub enum State {
    #[default]
    Asked,
    Approved,
    Refused,
}

/// Who gave the answer.
#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Debug, Default, Hash)]
#[serde(rename_all = "snake_case")]
pub enum By {
    /// The planner, by hand.
    #[default]
    Planner,
    /// RosterFlow's advice taken as it was (`advice`).
    Advice,
}

/// A request for days off: its days (both counted), why, and its answer.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Default)]
pub struct Request {
    /// Unique in the company (`HolidayRules::last`).
    pub no: u32,
    pub from: String,
    pub until: String,
    #[serde(default)]
    pub reason: Reason,
    pub asked: String,
    /// The answer is due by the end of this day.
    pub due: String,
    #[serde(default)]
    pub state: State,
    #[serde(default)]
    pub answered: Option<String>,
    #[serde(default)]
    pub by: By,
    /// It began: its days were taken from what is left of the year's.
    #[serde(default)]
    pub begun: bool,
    /// The planner's note.
    #[serde(default)]
    pub note: String,
}

impl Request {
    /// Its days, both counted.
    pub fn days(&self) -> i64 {
        dates::between(&self.from, &self.until) + 1
    }

    /// The working days it takes of the year's.
    pub fn working_days(&self) -> u32 {
        working_days(self.days())
    }

    pub fn covers(&self, date: &str) -> bool {
        self.from.as_str() <= date && date <= self.until.as_str()
    }
}

/// The working days of a holiday of `days` days (five of every seven, as the week has them).
pub fn working_days(days: i64) -> u32 {
    ((days.max(0) * 5 + 6) / 7) as u32
}

/// Someone's holidays asked for, and their answers (`Employee::leave`).
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Default)]
#[serde(default)]
pub struct Leave {
    pub requests: Vec<Request>,
}

impl Leave {
    /// The approved holiday that covers `date`.
    pub fn approved_on(&self, date: &str) -> Option<&Request> {
        self.requests.iter().find(|r| r.state == State::Approved && r.covers(date))
    }

    /// The request waiting for an answer (one at a time).
    pub fn pending(&self) -> Option<&Request> {
        self.requests.iter().find(|r| r.state == State::Asked)
    }

    /// How often they were refused in the year up to `today`.
    pub fn refused(&self, today: &str) -> u32 {
        self.requests.iter().filter(|r| r.state == State::Refused && r.answered.as_deref().is_some_and(|a| dates::between(a, today) < 365)).count() as u32
    }

    /// The working days asked for or approved and not begun (besides request `but`).
    fn booked(&self, but: Option<u32>) -> u32 {
        self.requests.iter().filter(|r| Some(r.no) != but && !r.begun && r.state != State::Refused).map(Request::working_days).sum()
    }
}

/// The company's holiday rules: how many may be off a day, and the requests' numbers.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Default)]
#[serde(default)]
pub struct HolidayRules {
    /// At most this many drivers off on holiday on one day (None: `default_limit`).
    pub max_off: Option<u32>,
    /// The last request's number.
    pub last: u32,
}

/// The limit until the company sets its own: a quarter of the fleet, at least one.
pub fn default_limit(c: &Company) -> u32 {
    let buses = c.fleet.iter().filter(|v| v.held_on(&c.date)).count() as u32;
    buses.div_ceil(4).max(1)
}

/// How many drivers may be off on holiday on one day.
pub fn limit(c: &Company) -> u32 {
    c.staff_rules.holidays.max_off.unwrap_or_else(|| default_limit(c))
}

/// Set the limit (None: by the fleet again).
pub fn set_limit(c: &mut Company, n: Option<u32>) {
    c.staff_rules.holidays.max_off = n.map(|n| n.clamp(1, MOST));
}

/// How many of a role may be off on one day: the drivers by the company's limit, the others a
/// third of them (at least one).
pub fn limit_of(c: &Company, role: Role) -> u32 {
    if role == Role::Driver {
        limit(c)
    } else {
        (c.staff.iter().filter(|e| e.role == role && e.notice_until.is_none()).count() as u32 / 3).max(1)
    }
}

/// How many of `role` are off on holiday on `date` (`but` left out).
pub fn off_on(c: &Company, role: Role, date: &str, but: Option<u32>) -> usize {
    c.staff.iter().filter(|e| e.role == role && Some(e.id) != but && e.employed_on(date) && e.on_holiday(date).is_some()).count()
}

/// The days from `from` to `until`, both counted.
pub fn days_of(from: &str, until: &str) -> Vec<String> {
    let (Some(a), Some(b)) = (dates::parse(from), dates::parse(until)) else {
        return Vec::new();
    };
    (a..=b).map(dates::fmt).collect()
}

/// The days approving request `r` of `id` takes their role over its limit: each day, and how
/// many would be off then.
pub fn over_limit(c: &Company, id: u32, r: &Request) -> Vec<(String, u32)> {
    let Some(e) = c.employee(id) else {
        return Vec::new();
    };
    let max = limit_of(c, e.role);
    days_of(&r.from, &r.until)
        .into_iter()
        .filter_map(|d| {
            let n = off_on(c, e.role, &d, Some(id)) as u32 + 1;
            (n > max).then_some((d, n))
        })
        .collect()
}

/// The working days someone has left to ask for (`but`: a request not counted).
pub fn free_days(e: &Employee, but: Option<u32>) -> u32 {
    e.holiday_left.saturating_sub(e.leave.booked(but))
}

/// The planner's view of a request.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Advice {
    /// Within the limit.
    Fits,
    /// One over the limit, but a date that cannot move.
    Occasion,
    /// One over the limit, but refused twice this year already: fair.
    Fair,
    /// Too many would be off then.
    Over,
    /// Not enough holiday days left.
    NoDays,
}

impl Advice {
    pub fn approve(self) -> bool {
        matches!(self, Advice::Fits | Advice::Occasion | Advice::Fair)
    }

    /// Why, in a few words (English, translated when shown).
    pub fn why(self) -> &'static str {
        match self {
            Advice::Fits => "it stays within the limit",
            Advice::Occasion => "its date cannot move, and it is only one over the limit",
            Advice::Fair => "they were rejected twice this year already",
            Advice::Over => "too many would be off then",
            Advice::NoDays => "they have not enough holiday days left",
        }
    }
}

/// What the planner makes of request `r` of `id`: within the limit it is approved; one over it
/// still, for a date that cannot move or for someone refused twice this year; else refused.
pub fn advice(c: &Company, id: u32, r: &Request) -> Advice {
    let Some(e) = c.employee(id) else {
        return Advice::Over;
    };
    if r.working_days() > free_days(e, Some(r.no)) {
        return Advice::NoDays;
    }
    let over = over_limit(c, id, r);
    let max = limit_of(c, e.role);
    let excess = over.iter().map(|x| x.1.saturating_sub(max)).max().unwrap_or(0);
    if over.is_empty() {
        Advice::Fits
    } else if excess <= 1 && r.reason.priority() == Priority::High {
        Advice::Occasion
    } else if excess <= 1 && e.leave.refused(&c.date) >= 2 {
        Advice::Fair
    } else {
        Advice::Over
    }
}

/// Request `no` and whose it is.
pub fn find(c: &Company, no: u32) -> Option<(&Employee, &Request)> {
    c.staff.iter().find_map(|e| e.leave.requests.iter().find(|r| r.no == no).map(|r| (e, r)))
}

/// The requests waiting for an answer, the soonest due first.
pub fn waiting(c: &Company) -> Vec<(&Employee, &Request)> {
    let mut v: Vec<(&Employee, &Request)> = c.staff.iter().flat_map(|e| e.leave.requests.iter().filter(|r| r.state == State::Asked).map(move |r| (e, r))).collect();
    v.sort_by(|a, b| a.1.due.cmp(&b.1.due).then(a.1.from.cmp(&b.1.from)).then(a.1.no.cmp(&b.1.no)));
    v
}

/// Answer request `no`: approved, the days are theirs; refused, they are not.
pub fn answer(c: &mut Company, no: u32, approve: bool, by: By) -> Result<(), &'static str> {
    let today = c.date.clone();
    let Some((id, r)) = find(c, no).map(|(e, r)| (e.id, r.clone())) else {
        return Err("That request is no longer there.");
    };
    if approve && c.employee(id).is_some_and(|e| r.working_days() > free_days(e, Some(no))) {
        return Err("They have not enough holiday days left for it.");
    }
    let Some(e) = c.staff.iter_mut().find(|e| e.id == id) else {
        return Err("That request is no longer there.");
    };
    if let Some(q) = e.leave.requests.iter_mut().find(|x| x.no == no) {
        q.state = if approve { State::Approved } else { State::Refused };
        q.answered = Some(today);
        q.by = by;
    }
    Ok(())
}

/// A request entered for `id`: from `from` until `until` (both counted). Returns its number.
pub fn add_request(c: &mut Company, id: u32, from: &str, until: &str, reason: Reason, note: &str) -> Result<u32, &'static str> {
    let (Some(a), Some(b)) = (dates::parse(from), dates::parse(until)) else {
        return Err("Enter the first and the last day.");
    };
    if b < a {
        return Err("The last day comes before the first.");
    }
    if !c.staff.iter().any(|e| e.id == id) {
        return Err("This person does not work here.");
    }
    c.staff_rules.holidays.last += 1;
    let no = c.staff_rules.holidays.last;
    let today = c.date.clone();
    let due = if dates::between(&today, from) > 1 { dates::add(from, -1) } else { today.clone() };
    let e = c.staff.iter_mut().find(|e| e.id == id).unwrap();
    e.leave.requests.push(Request { no, from: dates::fmt(a), until: dates::fmt(b), reason, asked: today, due, note: note.trim().to_string(), ..Default::default() });
    Ok(no)
}

/// Request `no` taken out (withdrawn, or entered by mistake).
pub fn remove_request(c: &mut Company, no: u32) -> bool {
    for e in c.staff.iter_mut() {
        let before = e.leave.requests.len();
        e.leave.requests.retain(|r| r.no != no);
        if e.leave.requests.len() != before {
            return true;
        }
    }
    false
}

/// Every English text this module writes, for the translations.
pub fn texts() -> Vec<&'static str> {
    let mut v: Vec<&'static str> = Reason::ALL.iter().map(|r| r.label()).collect();
    v.extend([Priority::Low, Priority::Normal, Priority::High].iter().map(|p| p.label()));
    v.extend([Advice::Fits, Advice::Occasion, Advice::Fair, Advice::Over, Advice::NoDays].iter().map(|a| a.why()));
    v.extend(["That request is no longer there.", "They have not enough holiday days left for it.", "Enter the first and the last day.", "The last day comes before the first."]);
    v
}

/// A day of the holiday planner: the drivers off on holiday and those who asked for it, the
/// limit, and the cover - the drivers the weekday's roster has, those of them away (on holiday,
/// ill, on a course), and the free drivers who could stand in.
#[derive(Clone, Debug, PartialEq, Default)]
pub struct PlannerDay {
    pub date: String,
    pub off: Vec<u32>,
    pub asked: Vec<u32>,
    pub limit: u32,
    pub rostered: usize,
    pub away: usize,
    pub spare: usize,
}

impl PlannerDay {
    /// Rostered drivers away for whom no free driver is left.
    pub fn short(&self) -> usize {
        self.away.saturating_sub(self.spare)
    }
}

/// The planner's day `date` (the drivers).
pub fn planner_day(c: &Company, date: &str) -> PlannerDay {
    let wd = super::plan::weekday_of(date);
    let mut rostered: Vec<u32> =
        c.planning.week.iter().filter(|r| r.weekday == wd).flat_map(|r| r.duties.iter()).filter_map(|w| if let Some(super::plan::Who::Staff(id)) = w { Some(*id) } else { None }).collect();
    rostered.sort();
    rostered.dedup();
    let drivers: Vec<&Employee> = c.staff.iter().filter(|e| super::staff::drives(e) && e.employed_on(date)).collect();
    let off: Vec<u32> = drivers.iter().filter(|e| e.on_holiday(date).is_some()).map(|e| e.id).collect();
    let asked: Vec<u32> = drivers.iter().filter(|e| e.leave.pending().is_some_and(|r| r.covers(date))).map(|e| e.id).collect();
    let away = drivers.iter().filter(|e| rostered.contains(&e.id) && e.absent(date)).count();
    let spare = drivers.iter().filter(|e| !rostered.contains(&e.id) && !e.absent(date)).count();
    let rostered = drivers.iter().filter(|e| rostered.contains(&e.id)).count();
    PlannerDay { date: date.to_string(), off, asked, limit: limit(c), rostered, away, spare }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::company::model::Vehicle;

    fn company(drivers: u32, buses: u32) -> Company {
        let mut c = Company { date: "2026-03-02".into(), ..Default::default() };
        for id in 1..=drivers {
            c.staff.push(Employee { id, name: format!("Driver {id}"), holiday_left: 25, ..Default::default() });
        }
        for id in 1..=buses {
            c.fleet.push(Vehicle { id, number: (100 + id).to_string(), ..Default::default() });
        }
        c
    }

    #[test]
    fn the_limit_goes_by_the_fleet_and_an_approval_over_it_is_seen() {
        let mut c = company(6, 6);
        assert_eq!((default_limit(&c), limit(&c)), (2, 2));
        set_limit(&mut c, Some(1));
        assert_eq!(limit(&c), 1);
        let a = add_request(&mut c, 1, "2026-04-06", "2026-04-12", Reason::Break, "").unwrap();
        assert_eq!(advice(&c, 1, &find(&c, a).unwrap().1.clone()), Advice::Fits);
        answer(&mut c, a, true, By::Planner).unwrap();
        assert!(c.staff[0].on_holiday("2026-04-08").is_some());
        // a second driver the same week: over the limit, unless a date that cannot move
        let b = add_request(&mut c, 2, "2026-04-08", "2026-04-09", Reason::Break, "").unwrap();
        assert_eq!(advice(&c, 2, &find(&c, b).unwrap().1.clone()), Advice::Over);
        let f = add_request(&mut c, 3, "2026-04-08", "2026-04-08", Reason::Family, "wedding").unwrap();
        assert_eq!(advice(&c, 3, &find(&c, f).unwrap().1.clone()), Advice::Occasion);
        assert_eq!(over_limit(&c, 2, &find(&c, b).unwrap().1.clone()).len(), 2);
        assert!(remove_request(&mut c, b) && find(&c, b).is_none());
        assert!(add_request(&mut c, 1, "2026-05-02", "2026-05-01", Reason::Break, "").is_err());
        let day = planner_day(&c, "2026-04-08");
        assert_eq!(day.off, vec![1]);
    }
}
