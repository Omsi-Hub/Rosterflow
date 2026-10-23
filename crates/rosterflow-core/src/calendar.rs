//! What runs when: the kind of service a line is (`ServiceKind`) and what day a date is
//! (`DayCalendar`: its weekday, a public holiday, the school holidays).
//!
//! The kinds, as operators know them:
//! * *Regular*: every day as the timetable has it.
//! * *School transport* (leerlingenvervoer, Schülerverkehr): Monday to Friday on school days
//!   only - not in the school holidays, not on public holidays.
//! * *Weekend and leisure*: Saturdays, Sundays and public holidays.
//! * *On demand* (belbus, buurtbus, Rufbus): a timetable whose trips run when booked; planned
//!   as a regular line.
//!
//! A date is a working day, a Saturday, or a Sunday or public holiday (`DayType::group`, an index
//! into `lines::DAY_GROUPS`): the timetable a line runs that day. The public holidays are the
//! country's (`public_holiday`) and those the company adds; the school holidays are the periods
//! the company enters (a yearly `MM-DD` range, or a dated one `YYYY-MM-DD`).

use crate::company::dates;
use serde::{Deserialize, Serialize};

// --- the kind of service -------------------------------------------------------------------------

#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Debug, Default, Hash)]
#[serde(rename_all = "snake_case")]
pub enum ServiceKind {
    #[default]
    Regular,
    School,
    Leisure,
    OnDemand,
}

impl ServiceKind {
    pub const ALL: [ServiceKind; 4] = [ServiceKind::Regular, ServiceKind::School, ServiceKind::Leisure, ServiceKind::OnDemand];

    pub fn label(self) -> &'static str {
        match self {
            ServiceKind::Regular => "Regular service",
            ServiceKind::School => "School transport",
            ServiceKind::Leisure => "Weekend and leisure",
            ServiceKind::OnDemand => "On demand",
        }
    }

    /// What it means, in a sentence.
    pub fn note(self) -> &'static str {
        match self {
            ServiceKind::Regular => "Every day as the timetable has it.",
            ServiceKind::School => "Monday to Friday on school days only: not in the school holidays, not on public holidays.",
            ServiceKind::Leisure => "Saturdays, Sundays and public holidays.",
            ServiceKind::OnDemand => "Trips that run when booked, planned as a regular line.",
        }
    }

    /// The line runs the day group `group` (`lines::DAY_GROUPS`) at all.
    pub fn allows(self, group: usize) -> bool {
        match self {
            ServiceKind::School => group == 0,
            ServiceKind::Leisure => group == 1 || group == 2,
            ServiceKind::Regular | ServiceKind::OnDemand => true,
        }
    }
}

// --- the day -------------------------------------------------------------------------------------

/// A holiday period: from one day to another, both counted. `MM-DD` repeats every year (one that
/// ends before it begins goes over the new year); `YYYY-MM-DD` is that once.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Default)]
#[serde(default)]
pub struct HolidayPeriod {
    pub name: String,
    pub from: String,
    pub to: String,
}

/// `MM-DD` as month × 100 + day (None: not one).
pub fn month_day(s: &str) -> Option<u32> {
    let (m, d) = s.trim().split_once('-')?;
    let (m, d): (u32, u32) = (m.trim().parse().ok()?, d.trim().parse().ok()?);
    ((1..=12).contains(&m) && (1..=dates::days_in_month(2024, m)).contains(&d)).then_some(m * 100 + d)
}

/// `YYYY-MM-DD` as a date code (YYYYMMDD; None: not one).
pub fn date_code(s: &str) -> Option<i32> {
    let d = dates::parse(s.trim())?;
    Some(code_of_day(d))
}

impl HolidayPeriod {
    pub fn new(name: &str, from: &str, to: &str) -> HolidayPeriod {
        HolidayPeriod { name: name.into(), from: from.into(), to: to.into() }
    }

    fn dated(&self) -> Option<(i32, i32)> {
        Some((date_code(&self.from)?, date_code(&self.to)?))
    }

    /// Both ends can be read.
    pub fn valid(&self) -> bool {
        self.dated().is_some() || (month_day(&self.from).is_some() && month_day(&self.to).is_some())
    }

    /// The date code (YYYYMMDD) lies in it.
    pub fn covers(&self, code: i32) -> bool {
        if let Some((a, b)) = self.dated() {
            return a <= code && code <= b;
        }
        let (Some(a), Some(b)) = (month_day(&self.from), month_day(&self.to)) else {
            return false;
        };
        let x = (code % 10000) as u32;
        if a <= b {
            a <= x && x <= b
        } else {
            x >= a || x <= b
        }
    }
}

/// Easter Sunday of a year (the Gregorian computus): (month, day).
pub fn easter(y: i32) -> (u32, u32) {
    let a = y % 19;
    let b = y / 100;
    let c = y % 100;
    let d = b / 4;
    let e = b % 4;
    let f = (b + 8) / 25;
    let g = (b - f + 1) / 3;
    let h = (19 * a + b - d - g + 15) % 30;
    let i = c / 4;
    let k = c % 4;
    let l = (32 + 2 * e + 2 * i - h - k) % 7;
    let m = (a + 11 * h + 22 * l) / 451;
    let month = (h + l - 7 * m + 114) / 31;
    let day = (h + l - 7 * m + 114) % 31 + 1;
    (month as u32, day as u32)
}

/// The countries whose public holidays are known (ISO code, name).
pub const COUNTRIES: [(&str, &str); 5] = [("NL", "Netherlands"), ("BE", "Belgium"), ("DE", "Germany"), ("GB", "United Kingdom"), ("", "Other (enter them yourself)")];

/// A date code (YYYYMMDD) is a public holiday in `country` (ISO code; another: none but those
/// the company enters). Bus timetables run Sunday's timetable on them.
pub fn public_holiday(country: &str, code: i32) -> bool {
    let (y, md) = (code / 10000, code % 10000);
    let Some(day) = day_of_code(code) else {
        return false;
    };
    let (em, ed) = easter(y);
    let e = dates::days_from_civil(y, em, ed);
    let after_easter = |ks: &[i64]| ks.iter().any(|k| e + k == day);
    match country.trim().to_ascii_uppercase().as_str() {
        // New Year, Good Friday is no general day off, Easter Monday, King's Day (27 April; the
        // 26th when the 27th is a Sunday), Liberation Day (5 May), Ascension, Whit Monday,
        // Christmas
        "NL" => {
            let kings = if dates::weekday(dates::days_from_civil(y, 4, 27)) == 6 { 426 } else { 427 };
            matches!(md, 101 | 505 | 1225 | 1226) || md == kings || after_easter(&[1, 39, 50])
        }
        "BE" => matches!(md, 101 | 501 | 721 | 815 | 1101 | 1111 | 1225) || after_easter(&[1, 39, 50]),
        "DE" => matches!(md, 101 | 501 | 1003 | 1225 | 1226) || after_easter(&[-2, 1, 39, 50]),
        // (England: the bank holidays on Mondays - the first and last of May, the last of August)
        "GB" => {
            let wd = dates::weekday(day);
            let (m, d) = (md / 100, (md % 100) as u32);
            let first_monday = wd == 0 && d <= 7;
            let last_monday = wd == 0 && d + 7 > dates::days_in_month(y, m as u32);
            matches!(md, 101 | 1225 | 1226) || after_easter(&[-2, 1]) || (m == 5 && (first_monday || last_monday)) || (m == 8 && last_monday)
        }
        _ => false,
    }
}

/// The day number of a date code (YYYYMMDD).
pub fn day_of_code(code: i32) -> Option<i64> {
    let (y, m, d) = (code / 10000, (code / 100 % 100) as u32, (code % 100) as u32);
    ((1..=12).contains(&m) && d >= 1 && d <= dates::days_in_month(y, m)).then(|| dates::days_from_civil(y, m, d))
}

/// The date code (YYYYMMDD) of a day number.
pub fn code_of_day(day: i64) -> i32 {
    let (y, m, d) = dates::civil_from_days(day);
    y * 10000 + m as i32 * 100 + d as i32
}

/// What day a date is: its weekday (0 Monday), and whether it is a public holiday or in the
/// school holidays.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub struct DayType {
    pub weekday: u32,
    pub public_holiday: bool,
    pub school_holiday: bool,
}

impl DayType {
    /// Its day group (`lines::DAY_GROUPS`): 0 a working day, 1 Saturday, 2 Sunday or a public
    /// holiday.
    pub fn group(&self) -> usize {
        if self.public_holiday || self.weekday == 6 {
            2
        } else if self.weekday == 5 {
            1
        } else {
            0
        }
    }

    /// The children go to school: Monday to Friday, no public holiday, no school holidays.
    pub fn school_day(&self) -> bool {
        self.weekday < 5 && !self.public_holiday && !self.school_holiday
    }

    /// A line of `kind` runs that day at all.
    pub fn runs(&self, kind: ServiceKind) -> bool {
        match kind {
            ServiceKind::School => self.school_day(),
            ServiceKind::Leisure => self.group() > 0,
            ServiceKind::Regular | ServiceKind::OnDemand => true,
        }
    }
}

/// The company's calendar: its country's public holidays and those it adds, and its school
/// holidays.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct DayCalendar {
    pub country: String,
    /// Extra public holidays (`YYYY-MM-DD`, or `MM-DD` every year).
    pub holidays: Vec<HolidayPeriod>,
    pub school: Vec<HolidayPeriod>,
}

impl DayCalendar {
    pub fn public_holiday(&self, code: i32) -> bool {
        public_holiday(&self.country, code) || self.holidays.iter().any(|p| p.covers(code))
    }

    pub fn school_holiday(&self, code: i32) -> bool {
        self.school.iter().any(|p| p.covers(code))
    }

    /// The day a date code (YYYYMMDD) is.
    pub fn day(&self, code: i32) -> DayType {
        let weekday = day_of_code(code).map(dates::weekday).unwrap_or(0);
        DayType { weekday, public_holiday: self.public_holiday(code), school_holiday: self.school_holiday(code) }
    }

    /// The day `YYYY-MM-DD` is (None: not a date).
    pub fn day_of(&self, date: &str) -> Option<DayType> {
        date_code(date).map(|c| self.day(c))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_calendar_knows_the_holidays() {
        assert_eq!(easter(2024), (3, 31));
        assert_eq!(easter(2025), (4, 20));
        assert_eq!(easter(2026), (4, 5));
        // the Netherlands 2026: Easter Monday 6 April, King's Day 27 April, Liberation Day,
        // Ascension 14 May, Whit Monday 25 May, Christmas
        for c in [20260101, 20260406, 20260427, 20260505, 20260514, 20260525, 20261225, 20261226] {
            assert!(public_holiday("NL", c), "{c}");
        }
        assert!(!public_holiday("NL", 20260403) && public_holiday("DE", 20260403));
        // (King's Day 2025: the 26th, the 27th is a Sunday)
        assert!(public_holiday("NL", 20250426) && !public_holiday("NL", 20250427));
        // England: the early May and spring bank holidays, the summer one
        assert!(public_holiday("GB", 20260504) && public_holiday("GB", 20260525) && public_holiday("GB", 20260831));
        assert!(!public_holiday("", 20261225));
        let cal = DayCalendar {
            country: "NL".into(),
            holidays: vec![HolidayPeriod::new("Kermis", "2026-09-07", "2026-09-07")],
            school: vec![HolidayPeriod::new("Zomervakantie", "2026-07-18", "2026-08-30"), HolidayPeriod::new("Kerstvakantie", "12-19", "01-03")],
        };
        let d = cal.day_of("2026-06-02").unwrap();
        assert_eq!((d.weekday, d.group()), (1, 0));
        assert!(d.school_day());
        assert!(cal.day_of("2026-07-21").unwrap().school_holiday);
        assert!(cal.school_holiday(20261230) && cal.school_holiday(20270102) && !cal.school_holiday(20270105));
        assert_eq!(cal.day_of("2026-09-07").unwrap().group(), 2);
        assert_eq!(cal.day_of("2026-06-06").unwrap().group(), 1);
        assert!(!cal.day_of("2026-06-08").unwrap().runs(ServiceKind::Leisure));
    }

    #[test]
    fn periods_read_their_days() {
        assert_eq!(month_day("07-13"), Some(713));
        assert_eq!(month_day("02-30"), None);
        let p = HolidayPeriod::new("Christmas", "12-22", "01-05");
        assert!(p.covers(20241222) && p.covers(20250105) && !p.covers(20250106) && !p.covers(20241221));
        assert!(!HolidayPeriod::new("", "", "").covers(20240101));
        assert!(HolidayPeriod::new("x", "2026-01-01", "2026-01-31").valid());
    }
}
