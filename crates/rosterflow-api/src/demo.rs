//! An example company to try everything on: a small town's network of three lines with their
//! stops and timetables, a fleet that runs it and the drivers to plan on it. Every name is made
//! up.

use rosterflow_core::company::{licences::Endorsement, BusKind, BusSize, Company, Drive, Employee, Licence, Role, ShiftPref, Vehicle};
use rosterflow_core::lines::{default_bands, DayPattern, Direction, StopRef, DAY_GROUPS};

/// The stops: code, name, latitude, longitude, a relief point.
const STOPS: [(&str, &str, f64, f64, bool); 14] = [
    ("1001", "Station", 52.0907, 5.1214, true),
    ("1002", "Stationsplein", 52.0915, 5.1180, false),
    ("1003", "Markt", 52.0925, 5.1150, true),
    ("1004", "Stadhuis", 52.0938, 5.1120, false),
    ("1005", "Ziekenhuis", 52.0990, 5.1010, true),
    ("1006", "Parkweg", 52.0960, 5.1080, false),
    ("1007", "Noorderplein", 52.1030, 5.1150, false),
    ("1008", "Bedrijvenpark Noord", 52.1080, 5.1210, true),
    ("1009", "Zwembad", 52.0870, 5.1290, false),
    ("1010", "Oosterpoort", 52.0850, 5.1350, false),
    ("1011", "Winkelcentrum Oost", 52.0830, 5.1420, true),
    ("1012", "Hogeschool", 52.0800, 5.1500, false),
    ("1013", "Sportpark", 52.0950, 5.0950, false),
    ("1014", "Westwijk", 52.0970, 5.0890, true),
];

/// The lines: number, colour, the outbound stops (codes) and the minutes from the first to each,
/// how often on the working days (none: bands), the first and last departures.
const LINES: [(&str, &str, &[&str], &[f32], f32, f32, f32); 3] = [
    ("1", "#e53935", &["1001", "1002", "1003", "1004", "1006", "1005"], &[0.0, 2.0, 5.0, 7.0, 10.0, 14.0], 0.0, 6.0 * 60.0, 23.0 * 60.0),
    ("2", "#1e88e5", &["1001", "1009", "1010", "1011", "1012"], &[0.0, 4.0, 7.0, 11.0, 15.0], 15.0, 6.0 * 60.0 + 15.0, 22.0 * 60.0),
    ("3", "#43a047", &["1014", "1013", "1003", "1007", "1008"], &[0.0, 4.0, 11.0, 16.0, 21.0], 30.0, 6.0 * 60.0 + 30.0, 20.0 * 60.0),
];

const DRIVERS: [&str; 30] = [
    "Anouk de Vries", "Bram Jansen", "Chantal Bakker", "Dennis Visser", "Esra Yilmaz", "Frank Smit", "Gerda Mulder", "Hamza El Amrani", "Iris de Boer", "Jeroen Peters",
    "Kim Hendriks", "Lars van Dijk", "Mila Bos", "Noah Vermeulen", "Olga Kowalska", "Pieter de Wit", "Rachid Bouzid", "Sanne Dekker", "Thijs Brouwer", "Ufuk Demir",
    "Vera Kok", "Wim Jacobs", "Xander de Graaf", "Yara Mertens", "Zoë Willems", "Ahmed Benali", "Bas Koster", "Carmen Ruiz", "Daan Prins", "Eva Huisman",
];

/// `c` given the example network, fleet and staff.
pub fn fill(c: &mut Company) {
    let reg = &mut c.network;
    for (code, name, lat, lon, relief) in STOPS {
        let id = reg.add_stop(code, name);
        let s = reg.stops.iter_mut().find(|s| s.id == id).unwrap();
        (s.lat, s.lon, s.relief) = (lat, lon, relief);
    }
    let by_code = |reg: &rosterflow_core::lines::Registry, code: &str| reg.stops.iter().find(|s| s.code == code).map(|s| StopRef { id: s.id, name: s.name.clone() }).unwrap();
    for (number, colour, codes, times, headway, first, last) in LINES {
        let stops: Vec<StopRef> = codes.iter().map(|k| by_code(reg, k)).collect();
        let total = *times.last().unwrap();
        let back_times: Vec<f32> = times.iter().rev().map(|t| total - t).collect();
        let mut back = stops.clone();
        back.reverse();
        let out = Direction { stops, times: times.to_vec(), manual_times: true, ..Default::default() };
        let back = Direction { stops: back, times: back_times, manual_times: true, ..Default::default() };
        let l = reg.add_line(number);
        l.colour = colour.to_string();
        l.directions = vec![out, back];
        for d in &mut l.directions {
            d.fit_legs();
        }
        l.days = DAY_GROUPS
            .iter()
            .enumerate()
            .map(|(k, (_, days))| {
                let bands = if headway == 0.0 { default_bands(k) } else { Vec::new() };
                let (first, last) = if k == 0 { (first, last) } else { (first + 60.0, last - 60.0) };
                DayPattern { on: true, days: *days, first, last, headway: if k == 0 { headway.max(10.0) } else { headway.max(10.0) * 2.0 }, layover: 5.0, bands }
            })
            .collect();
        if number == "3" {
            // (the business park: no service on Sunday)
            l.days[2].on = false;
        }
    }
    c.depots = vec![rosterflow_core::company::Depot { name: c.depot.clone(), address: "Remiseweg 1".into() }];
    let models = [("VDL Citea SLF-120", BusSize::Solo, Drive::Diesel), ("VDL Citea SLF-120 Electric", BusSize::Solo, Drive::Electric), ("Mercedes-Benz Citaro G", BusSize::Articulated, Drive::Hybrid)];
    for k in 0..12usize {
        let (name, size, drive) = models[if k < 6 { 0 } else if k < 10 { 1 } else { 2 }];
        let number = c.next_fleet_number();
        c.counters.vehicle += 1;
        c.fleet.push(Vehicle {
            id: c.counters.vehicle,
            number,
            plate: format!("{}-BX-{}", 30 + k, 10 + k * 7),
            name: name.into(),
            kind: BusKind { size, drive },
            seats: if size == BusSize::Articulated { 48 } else { 34 },
            standing: if size == BusSize::Articulated { 90 } else { 55 },
            built: format!("{}", 2016 + k % 7),
            ..Default::default()
        });
    }
    for (k, name) in DRIVERS.iter().enumerate() {
        c.counters.employee += 1;
        let mut endorsements = vec![Endorsement::Electric];
        if k % 3 == 0 {
            endorsements.push(Endorsement::Articulated);
        }
        c.staff.push(Employee {
            id: c.counters.employee,
            name: name.to_string(),
            number: format!("{}", 1000 + k + 1),
            licence: Licence::D,
            wage: 1900 + (k as i64 % 5) * 75,
            contract_hours: if k % 4 == 3 { 24.0 } else { 0.0 },
            hired: "2020-01-01".into(),
            endorsements,
            role: Role::Driver,
            days_off: match k % 7 {
                0 => vec![6],
                3 => vec![5, 6],
                _ => Vec::new(),
            },
            shift: match k % 5 {
                1 => ShiftPref::Early,
                4 => ShiftPref::Late,
                _ => ShiftPref::Any,
            },
            standby: k == DRIVERS.len() - 1,
            ..Default::default()
        });
    }
    c.counters.employee += 1;
    c.staff.push(Employee { id: c.counters.employee, name: "Tom Kuipers".into(), number: "2001".into(), role: Role::Mechanic, hired: "2019-05-01".into(), ..Default::default() });
    let today = c.date.clone();
    rosterflow_core::timetable::sync_lines(c, &today);
}

#[cfg(test)]
mod tests {
    #[test]
    fn the_example_company_can_be_planned() {
        let mut c = rosterflow_core::company::found(&rosterflow_core::company::Founding { name: "Voorbeeld".into(), ..Default::default() }, "");
        c.date = "2026-03-02".into();
        super::fill(&mut c);
        assert_eq!(c.lines.len(), 3);
        assert!(c.network.lines.iter().all(rosterflow_core::lines::written), "{:?}", c.network.lines.iter().map(rosterflow_core::lines::problems).collect::<Vec<_>>());
        let lines = rosterflow_core::timetable::lines_on(&c, "2026-03-02");
        let tours: usize = lines.iter().map(|l| l.tours.len()).sum();
        assert!((5..=40).contains(&tours), "{tours} tours");
    }
}
