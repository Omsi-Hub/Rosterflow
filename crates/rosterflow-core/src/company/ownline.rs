//! What a line's design asks of the planning besides its trips: the empty runs from the depot to
//! the line and back (`add_depot_runs`), the size of bus its tours want (`wanted`) and the buses
//! that may run it (`line_allows`).

use super::model::{BusSize, Company, CompanyLine, Vehicle};
use super::network::{PlannedTrip, TourOfDay};
use crate::lines::{self, LineDesign};

/// A depot run's length where nothing else says (kilometres).
pub const DEPOT_RUN_KM: f64 = 3.0;
/// The name a depot run's trips carry.
pub const DEPOT_RUN: &str = "Depot run";

/// The design of company line `line` (its planning name).
pub fn design_of<'a>(c: &'a Company, line: &str) -> Option<&'a LineDesign> {
    let stems = lines::stems(&c.network);
    c.network.lines.iter().find(|l| stems.get(&l.id).is_some_and(|s| s.eq_ignore_ascii_case(line.trim())))
}

/// The depot a line's buses come from: its own, else the company's.
pub fn depot_of(c: &Company, line: &str) -> String {
    design_of(c, line).map(|l| l.depot.trim().to_string()).filter(|d| !d.is_empty()).unwrap_or_else(|| c.depot.trim().to_string())
}

/// The tours of a day with their depot runs: an empty run from the depot to the first stop
/// before the first trip, and back after the last. They carry no passengers
/// (`PlannedTrip::counts`), so the day costs their kilometres and the driver's time.
pub fn add_depot_runs(c: &Company, tours: &mut [TourOfDay]) {
    for t in tours.iter_mut() {
        let (Some(first), Some(last)) = (t.trips.first().cloned(), t.trips.last().cloned()) else {
            continue;
        };
        if !first.counts() {
            continue;
        }
        // (a line of the network: its own depot runs; a timetable without one has its own)
        let Some(l) = design_of(c, &t.line) else {
            continue;
        };
        let (min, km) = (l.depot_minutes.round().max(0.0) as i32, l.depot_km.max(0.0) as f64);
        if min <= 0 && km <= 0.0 {
            continue;
        }
        let depot = depot_of(c, &t.line);
        let run = |from: &str, to: &str, dep: i32| PlannedTrip {
            name: DEPOT_RUN.into(),
            line: String::new(),
            from: from.into(),
            to: to.into(),
            dep,
            arr: dep + min,
            km,
            stops: 0,
            empty: true,
            origin: String::new(),
            last: String::new(),
            part: None,
            of_line: String::new(),
        };
        t.trips.insert(0, run(&depot, &first.from, first.dep - min));
        t.trips.push(run(last.end(), &depot, last.arr));
    }
}

/// The size of bus a tour wants (None: any).
pub fn wanted(_c: &Company, t: &TourOfDay) -> Option<BusSize> {
    t.wants()
}

/// What a size carries, about (seats and standing).
pub fn capacity(size: BusSize) -> f64 {
    match size {
        BusSize::Midi => 45.0,
        BusSize::Solo => 70.0,
        BusSize::Double => 95.0,
        BusSize::Articulated => 105.0,
    }
}

/// Bus `v` may run line `line`: the line allows its size (a line that names none allows any).
pub fn line_allows(c: &Company, line: &str, v: &Vehicle) -> bool {
    design_of(c, line).is_none_or(|l| l.sizes.is_empty() || l.sizes.contains(&v.kind.size))
}

/// The fleet has a bus the line may run with (whether it is free or not).
pub fn fleet_has_bus_for(c: &Company, line: &str) -> bool {
    c.fleet.iter().any(|v| line_allows(c, line, v))
}

/// A line with a route but no departure yet: its timetable is made in the Timeplan.
pub fn no_timetable(c: &Company, cl: &CompanyLine) -> bool {
    design_of(c, &cl.name).is_some_and(|l| !lines::has_departures(l))
}

/// The sizes by what they carry, smallest first.
pub const SIZES: [BusSize; 4] = [BusSize::Midi, BusSize::Solo, BusSize::Double, BusSize::Articulated];

/// A size of bus the fleet has too few of for the day's tours: the most tours asking for that
/// size or a bigger one out at once, and the buses of that size or bigger the company has (a
/// bigger bus can drive a tour asking for a smaller one).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Shortfall {
    pub size: BusSize,
    pub needed: usize,
    pub have: usize,
}

/// What the fleet lacks for `tours`, the biggest size first (a smaller size only when it lacks
/// more than the bigger ones).
pub fn shortfall(c: &Company, tours: &[TourOfDay]) -> Vec<Shortfall> {
    let mut out = Vec::new();
    let mut worst = 0;
    for size in SIZES.into_iter().rev() {
        let mine: Vec<(i32, i32)> = tours.iter().filter(|t| wanted(c, t).is_some_and(|w| capacity(w) >= capacity(size))).map(|t| (t.from(), t.to())).collect();
        let needed = mine.iter().map(|&(a, _)| mine.iter().filter(|&&(x, z)| x <= a && a < z).count()).max().unwrap_or(0);
        let have = c.fleet.iter().filter(|v| capacity(v.kind.size) >= capacity(size) && v.held_on(&c.date)).count();
        if needed > have && needed - have > worst {
            worst = needed - have;
            out.push(Shortfall { size, needed, have });
        }
    }
    out
}
