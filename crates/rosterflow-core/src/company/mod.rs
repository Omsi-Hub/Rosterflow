//! The bus company and its planning.

pub mod agency;
pub mod dates;
pub mod holidays;
pub mod licences;
pub mod model;
pub mod network;
pub mod ownline;
pub mod plan;
pub mod plandays;
pub mod rng;
pub mod rosterflow;
pub mod rosterpdf;
pub mod staff;
pub mod studio;
pub mod timeplan;
pub mod unassign;

pub use model::*;

/// What a new company is made of.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Founding {
    pub name: String,
    pub short: String,
    pub colours: [String; 2],
    /// The main depot's name (empty: "Depot").
    pub depot: String,
    /// The first day planned (empty: today).
    pub date: String,
    /// The country of its public holidays (ISO code; empty: NL).
    pub country: String,
    /// The currency amounts are shown in (ISO code; empty: EUR).
    pub currency: String,
}

/// A name as a file name: lower case, letters and digits, dashes between.
pub fn slug(name: &str) -> String {
    let mut s = String::new();
    for c in name.trim().to_lowercase().chars() {
        if c.is_alphanumeric() {
            s.push(c);
        } else if !s.ends_with('-') && !s.is_empty() {
            s.push('-');
        }
    }
    let s = s.trim_end_matches('-').to_string();
    if s.is_empty() {
        "company".to_string()
    } else {
        s
    }
}

/// The short name a name suggests: its words' initials ("Stadsbus Noord" SN), or its first
/// letters.
pub fn short_of(name: &str) -> String {
    let words: Vec<&str> = name.split_whitespace().filter(|w| w.chars().next().is_some_and(char::is_alphanumeric)).collect();
    let s: String = if words.len() >= 2 { words.iter().filter_map(|w| w.chars().next()).take(4).collect() } else { name.chars().filter(|c| c.is_alphanumeric()).take(3).collect() };
    s.to_uppercase()
}

/// A new company: nothing in it yet but who it is. (`_by`: who entered it, kept for the log.)
pub fn found(f: &Founding, _by: &str) -> Company {
    let date = dates::parse(&f.date).map(dates::fmt).unwrap_or_else(dates::today);
    let short = if f.short.trim().is_empty() { short_of(&f.name) } else { f.short.trim().to_uppercase() };
    let colour = |c: &str, d: &str| {
        if c.trim().is_empty() {
            d.to_string()
        } else {
            c.trim().to_string()
        }
    };
    let depot = if f.depot.trim().is_empty() { "Depot".to_string() } else { f.depot.trim().to_string() };
    Company {
        id: slug(&f.name),
        name: f.name.trim().to_string(),
        short,
        colours: [colour(&f.colours[0], "#2f80ed"), colour(&f.colours[1], "#0f1b2d")],
        depots: vec![model::Depot { name: depot.clone(), address: String::new() }],
        depot,
        founded: date.clone(),
        date,
        currency: if f.currency.trim().is_empty() { "EUR".into() } else { f.currency.trim().to_uppercase() },
        country: if f.country.trim().is_empty() { "NL".into() } else { f.country.trim().to_uppercase() },
        ..Default::default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_company_is_founded() {
        let c = found(&Founding { name: "Stadsbus Noord".into(), date: "2026-05-04".into(), ..Default::default() }, "planner");
        assert_eq!((c.id.as_str(), c.short.as_str(), c.date.as_str(), c.depot.as_str()), ("stadsbus-noord", "SN", "2026-05-04", "Depot"));
        assert_eq!(slug("  Bus & Co BV "), "bus-co-bv");
        assert_eq!(slug("!!"), "company");
        assert_eq!(short_of("Arriva"), "ARR");
    }
}
