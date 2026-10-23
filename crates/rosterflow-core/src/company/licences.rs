//! Licences, endorsements and type training: a driver is planned only on a bus they are
//! qualified for.
//!
//! - *The licence*: D drives every bus, D1 only midibuses (`staff::may_drive`).
//! - *Endorsements* (what an operator asks besides the licence): an articulated bus, a
//!   double-decker, and the high-voltage instruction for an electric bus.
//! - *Type training* per bus model (`type_key`: the model's name): checked only where the
//!   company keeps it (`staff::StaffRules::type_training`).

use super::model::{BusKind, BusSize, Company, Drive, Employee, Licence};
use serde::{Deserialize, Serialize};

/// What an operator asks of a driver besides the licence for some buses.
#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Debug, Hash, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub enum Endorsement {
    Articulated,
    DoubleDecker,
    Electric,
}

impl Endorsement {
    pub const ALL: [Endorsement; 3] = [Endorsement::Articulated, Endorsement::DoubleDecker, Endorsement::Electric];

    pub fn label(self) -> &'static str {
        match self {
            Endorsement::Articulated => "Articulated buses",
            Endorsement::DoubleDecker => "Double-deckers",
            Endorsement::Electric => "High voltage (electric buses)",
        }
    }

    /// A short mark for lists ("G", "DD", "E").
    pub fn short(self) -> &'static str {
        match self {
            Endorsement::Articulated => "G",
            Endorsement::DoubleDecker => "DD",
            Endorsement::Electric => "E",
        }
    }

    /// Why someone without it may not drive such a bus.
    pub fn lacking(self) -> &'static str {
        match self {
            Endorsement::Articulated => "No licence for articulated buses",
            Endorsement::DoubleDecker => "No licence for double-deckers",
            Endorsement::Electric => "No high-voltage instruction for electric buses",
        }
    }
}

/// The endorsements a bus of `kind` asks for.
pub fn needed(kind: BusKind) -> Vec<Endorsement> {
    let mut v = Vec::new();
    match kind.size {
        BusSize::Articulated => v.push(Endorsement::Articulated),
        BusSize::Double => v.push(Endorsement::DoubleDecker),
        _ => {}
    }
    if kind.drive == Drive::Electric {
        v.push(Endorsement::Electric);
    }
    v
}

/// The type a bus model is trained as: its name, lower case, its words single-spaced.
pub fn type_key(model: &str) -> String {
    model.split_whitespace().collect::<Vec<_>>().join(" ").to_lowercase()
}

/// A model's name as the fleet calls it (the name of one of its buses, else its key).
pub fn type_name(c: &Company, key: &str) -> String {
    c.fleet.iter().find(|v| type_key(&v.name) == key).map(|v| v.name.trim().to_string()).unwrap_or_else(|| key.to_string())
}

/// The models of the fleet (held now), in the order the buses came.
pub fn fleet_types(c: &Company) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for v in c.fleet.iter().filter(|v| v.held_on(&c.date)) {
        let k = type_key(&v.name);
        if !k.is_empty() && !out.contains(&k) {
            out.push(k);
        }
    }
    out
}

/// What a driver lacks for a bus.
#[derive(Clone, Debug, PartialEq)]
pub enum Lack {
    /// A D1 licence and a bus bigger than a midibus.
    Licence,
    Endorsement(Endorsement),
    /// No type training for the model (its key).
    Type(String),
}

/// What `e` lacks to drive a bus of `kind` (`model`: its model, for the type training; None:
/// not checked) - None: qualified.
pub fn lack(e: &Employee, kind: BusKind, model: Option<&str>) -> Option<Lack> {
    if !super::staff::drives(e) || (e.licence == Licence::D1 && kind.size != BusSize::Midi) {
        return Some(Lack::Licence);
    }
    if let Some(x) = needed(kind).into_iter().find(|x| !e.endorsements.contains(x)) {
        return Some(Lack::Endorsement(x));
    }
    let key = model.map(type_key).filter(|k| !k.is_empty())?;
    // (an agency driver booked ahead drives any type: `agency::ANY_TYPE`)
    (!e.types.iter().any(|t| type_key(t) == key || t == super::agency::ANY_TYPE)).then_some(Lack::Type(key))
}

/// The model the type training is checked for on bus `v` (None: the company keeps no type
/// training).
pub fn model_checked(c: &Company, name: &str) -> Option<String> {
    c.staff_rules.type_training.then(|| name.trim().to_string()).filter(|n| !n.is_empty())
}

/// The drivers (employed, not leaving) who may drive a bus of `kind` and model `model`, and of
/// how many.
pub fn qualified_drivers(c: &Company, kind: BusKind, model: &str) -> (usize, usize) {
    let staff: Vec<&Employee> = c.staff.iter().filter(|e| super::staff::drives(e) && e.employed_on(&c.date) && e.notice_until.is_none()).collect();
    let checked = model_checked(c, model);
    (staff.iter().filter(|e| lack(e, kind, checked.as_deref()).is_none()).count(), staff.len())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_driver_lacks_what_a_bus_asks() {
        let artic = BusKind { size: BusSize::Articulated, drive: Drive::Diesel };
        let e = Employee::default();
        assert_eq!(lack(&e, artic, None), Some(Lack::Endorsement(Endorsement::Articulated)));
        let e = Employee { endorsements: vec![Endorsement::Articulated], types: vec!["Mercedes-Benz  Citaro G".into()], ..Default::default() };
        assert_eq!(lack(&e, artic, None), None);
        assert_eq!(lack(&e, artic, Some("Mercedes-Benz Citaro G")), None);
        assert_eq!(lack(&e, artic, Some("MAN Lion's City G")), Some(Lack::Type("man lion's city g".into())));
        let d1 = Employee { licence: Licence::D1, ..Default::default() };
        assert_eq!(lack(&d1, BusKind::default(), None), Some(Lack::Licence));
    }
}
