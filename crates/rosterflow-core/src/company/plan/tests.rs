//! The roster over a company's own network: its lines made from stops and a design, the day's
//! tours from them, filled with its buses and drivers by the rules.

use super::super::model::{CompanyLine, Employee, Vehicle};
use super::super::network::tours_of_day;
use super::super::{found, Founding};
use super::*;
use crate::lines::{Direction, StopRef};

/// Line 1 from the Station to the Hospital and back, 25 minutes a way, every 30 minutes from
/// 06:00 to 20:00 on working days; a company with `buses` buses and `drivers` drivers.
fn company(buses: u32, drivers: u32) -> Company {
    let mut c = found(&Founding { name: "Stadsbus".into(), date: "2026-06-01".into(), ..Default::default() }, "planner");
    let station = c.network.add_stop("1001", "Station");
    let market = c.network.add_stop("1002", "Markt");
    let hospital = c.network.add_stop("1003", "Ziekenhuis");
    let s = |id: i64, n: &str| StopRef { id, name: n.into() };
    let out = Direction { stops: vec![s(station, "Station"), s(market, "Markt"), s(hospital, "Ziekenhuis")], times: vec![0.0, 12.0, 25.0], manual_times: true, ..Default::default() };
    let back = Direction { stops: vec![s(hospital, "Ziekenhuis"), s(market, "Markt"), s(station, "Station")], times: vec![0.0, 13.0, 25.0], manual_times: true, ..Default::default() };
    let l = c.network.add_line("1");
    l.directions = vec![out, back];
    l.days = crate::lines::default_days();
    l.days[0].first = 360.0;
    l.days[0].last = 1200.0;
    l.days[0].headway = 30.0;
    l.days[1].on = false;
    l.days[2].on = false;
    let today = c.date.clone();
    crate::timetable::sync_lines(&mut c, &today);
    for k in 0..buses {
        c.fleet.push(Vehicle { id: k + 1, number: (101 + k).to_string(), name: "VDL Citea".into(), ..Default::default() });
    }
    for k in 0..drivers {
        c.staff.push(Employee { id: k + 1, name: format!("Driver {}", k + 1), ..Default::default() });
    }
    c
}

fn day(c: &Company, date: &str) -> Vec<TourOfDay> {
    let lines = crate::timetable::lines_on(c, date);
    tours_of_day(c, &lines, date)
}

#[test]
fn a_line_of_the_network_runs_its_tours_with_depot_runs() {
    let c = company(3, 8);
    assert_eq!(c.lines.len(), 1);
    assert_eq!(
        c.lines[0],
        CompanyLine {
            name: "L1".into(),
            number: "1".into(),
            numbers: vec!["1".into()],
            colour: c.network.lines[0].colour.clone(),
            caption: "Station - Ziekenhuis".into(),
            added: c.date.clone(),
            ..Default::default()
        }
    );
    let tours = day(&c, "2026-06-01");
    // 25 minutes a way and 5 standing: a round of an hour, a bus every 30 minutes - two buses
    assert_eq!(tours.len(), 2);
    for t in &tours {
        assert!(t.trips[0].empty && t.trips[0].from == "Depot" && t.trips.last().unwrap().to == "Depot");
        assert!(t.trips[1..t.trips.len() - 1].iter().all(|x| x.counts()));
    }
    // Saturday the line does not run
    assert!(day(&c, "2026-06-06").is_empty());
    // a public holiday (Whit Monday 2026) runs Sunday's timetable: nothing
    assert!(day(&c, "2026-05-25").is_empty());
}

#[test]
fn fill_the_roster_gives_every_tour_a_bus_and_every_duty_a_driver() {
    let mut c = company(3, 8);
    let date = c.date.clone();
    let tours = day(&c, &date);
    let f = fill_day_told(&mut c, &date, tours.clone());
    assert_eq!((f.buses, f.no_bus, f.unqualified, f.busy), (2, 0, 0, 0));
    assert!(f.duties >= 2);
    let p = day_plan(&c, &date, tours.clone());
    assert!(p.tours.iter().all(DayTour::covered), "{:?}", p.open_rows());
    // the roster repeats every Monday
    let next = dates::add(&date, 7);
    let p = day_plan(&c, &next, day(&c, &next));
    assert!(p.tours.iter().all(DayTour::covered));
    // a driver on holiday that Monday: a free colleague covers the duty, the roster stays theirs
    let first = p.tours[0].duties[0].who;
    let Some(Who::Staff(id)) = first else { panic!("no driver") };
    c.staff.iter_mut().find(|e| e.id == id).unwrap().holiday_until = Some(next.clone());
    c.staff.iter_mut().find(|e| e.id == id).unwrap().hired = "2020-01-01".into();
    let p = day_plan(&c, &next, day(&c, &next));
    let d = &p.tours[0].duties[0];
    assert!(d.who.is_some() && d.who != first && d.from_ == Source::Central, "{d:?}");
    assert_eq!(roster(&c, weekday_of(&next), &p.tours[0].tour.line, &p.tours[0].tour.tour).unwrap().duties[0], first);
}

#[test]
fn what_the_dispatcher_reports_opens_the_day() {
    let mut c = company(2, 4);
    let date = c.date.clone();
    let tours = day(&c, &date);
    fill_day_told(&mut c, &date, tours.clone());
    // the first tour's bus broke down: no spare bus left for it
    let bus = match day_plan(&c, &date, tours.clone()).tours[0].bus {
        Some(BusOf::Own(id)) => id,
        b => panic!("{b:?}"),
    };
    report(&mut c, &date, Disruption::Breakdown { vehicle: bus });
    let p = day_plan(&c, &date, tours.clone());
    assert_eq!(p.tours[0].bus_problem, Some(Problem::Breakdown));
    assert!(p.tours[0].bus.is_none());
    // a hired bus chosen for it
    set_fill(&mut c, &tour_key(&p.tours[0].tour.line, &p.tours[0].tour.tour), Some(Fill::Rental));
    assert_eq!(day_plan(&c, &date, tours.clone()).tours[0].bus, Some(BusOf::Rental));
    unreport(&mut c, &date, None, Some(bus));
    set_fill(&mut c, &tour_key(&p.tours[0].tour.line, &p.tours[0].tour.tour), None);
    assert_eq!(day_plan(&c, &date, tours.clone()).tours[0].bus, Some(BusOf::Own(bus)));
    // a driver 40 minutes late: the first trips of their first duty wait for a cover
    let Some(Who::Staff(late)) = day_plan(&c, &date, tours.clone()).tours[0].duties[0].who else { panic!() };
    report(&mut c, &date, Disruption::Late { employee: late, minutes: 40 });
    let p = day_plan(&c, &date, tours.clone());
    let l = p.tours[0].duties[0].late.expect("late");
    assert_eq!(l.until, p.tours[0].duties[0].from + 40);
    // reports of another day leave this one alone
    assert!(disruptions(&c, &dates::add(&date, 1)).is_empty());
}

#[test]
fn a_driver_works_no_more_days_a_week_than_the_rule() {
    let mut c = company(2, 3);
    c.staff_rules.week_days = 1;
    let date = c.date.clone();
    // Monday rostered for drivers 1 to 3
    let tours = day(&c, &date);
    fill_day_told(&mut c, &date, tours);
    let monday: Vec<u32> = c.planning.week.iter().flat_map(|r| r.duties.iter().flatten()).filter_map(|w| if let Who::Staff(id) = w { Some(*id) } else { None }).collect();
    assert!(!monday.is_empty());
    // Tuesday: who worked Monday has had their one day
    let tue = dates::add(&date, 1);
    let p = prepared(&c, &tue, &std::collections::HashMap::new());
    for id in &monday {
        let e = p.staff.iter().find(|e| e.id == *id).unwrap();
        assert_eq!(day_unavailable(e, &tue), Some(Cannot::WeekDone { days: 1 }));
    }
    // and a day off by contract
    c.staff[0].days_off = vec![2];
    assert_eq!(day_unavailable(&c.staff[0], &dates::add(&date, 2)), Some(Cannot::DayOff));
}
