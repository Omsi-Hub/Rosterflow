//! Taking a driver off a duty, or a bus off a tour, on a day's roster: an easy way to deallocate
//! a driver or a bus. What is taken off is what the day shows
//! (`plan::day_plan`): the dispatcher's choice for the day is taken back (`plan::set_fill`), the
//! central's cover of what fell out is given up - its trips are dropped today, with the
//! contract's penalty - and else the roster's own entry goes (`plan::set_driver`,
//! `plan::set_bus`; the weekday's, so every week). A rental bus taken off is not rented: only
//! the rental buses the day still has are hired.
//!
//! What has begun on the company's day, what has been driven already and what is running now
//! stays, with the reason why. The pieces of a duty joined in the studio lose their driver
//! together, as "Give to" gave them together.

use super::model::{Cents, Company};
use super::plan::{self, BusOf, DayPlan, Fill, Source, Who};

/// Why something is not taken off (English, translated when shown).
pub const NOT_FOUND: &str = "The roster changed meanwhile: try again.";
pub const BEGUN: &str = "This duty has begun already: its driver can no longer be taken off today.";
pub const UNDER_WAY: &str = "This tour is under way: its bus can no longer be taken off today.";
pub const NO_DRIVER: &str = "Nobody drives it: there is nothing to take off.";
pub const NO_BUS: &str = "It has no bus: there is nothing to take off.";
pub const COVER_LATER: &str = "The office covers this duty on its day: what it chooses can be taken off on that day.";

/// Every English text of taking off (for the translations' check).
pub fn texts() -> [&'static str; 6] {
    [NOT_FOUND, BEGUN, UNDER_WAY, NO_DRIVER, NO_BUS, COVER_LATER]
}

/// What taking off did.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Off {
    /// Out of the weekday's roster.
    Roster,
    /// The dispatcher's choice for the day taken back.
    Choice,
    /// The day's rental bus cancelled: its rent for the day is not paid.
    Rental { rent: Cents },
    /// The central's cover given up: the trips are dropped today (and its rental bus, if it
    /// was one, not rented).
    Dropped { penalty: Cents, rent: Cents },
}

/// Whether what begins at `from` (minutes of the plan's day) has begun: on the company's day,
/// the clock past it.
pub fn begun(_c: &Company, p: &DayPlan, from: i32) -> bool {
    p.today && super::dates::now_minutes() >= from
}

fn rostered_driver(c: &Company, p: &DayPlan, ti: usize, k: usize) -> Option<Who> {
    let t = &p.tours[ti].tour;
    plan::roster(c, p.weekday, &t.line, &t.tour).and_then(|r| r.duties.get(k).copied().flatten())
}

/// Take the driver off duty `k` of tour `ti` of the day `p` (see the module).
pub fn driver_off(c: &mut Company, p: &DayPlan, ti: usize, k: usize) -> Result<Off, &'static str> {
    let t = p.tours.get(ti).ok_or(NOT_FOUND)?;
    let d = t.duties.get(k).ok_or(NOT_FOUND)?;
    // (a driver who is not there can be taken out of the roster whenever)
    if d.who.is_some() && begun(c, p, d.from) {
        return Err(BEGUN);
    }
    let (line, tour) = (t.tour.line.clone(), t.tour.tour.clone());
    let key = plan::duty_key(&line, &tour, k);
    // (an agency driver booked for the day, the duty given him for it: given back - `agency`)
    if let (Some(Who::Staff(id)), Source::Dispatcher) = (d.who, d.from_) {
        let piece = t.tour.trips.get(d.start).map(|x| super::studio::PieceRef { line: line.clone(), tour: tour.clone(), first: super::studio::TripRef::of(x) });
        if super::agency::is_booked(id) && piece.is_some_and(|piece| super::agency::release(c, &p.date, &piece)) {
            return Ok(Off::Choice);
        }
    }
    // (the dispatcher's choices and the office's cover are the company's day's own: on another
    // day's plan they took back - or dropped - today's duty of the same tour, and the cover stayed)
    if !p.today && d.who.is_some() && matches!(d.from_, Source::Dispatcher | Source::Central) {
        return Err(COVER_LATER);
    }
    match (d.who, d.from_) {
        (Some(_), Source::Dispatcher) => {
            plan::set_fill(c, &key, None);
            // (the pieces joined with it have the same choice - one agency driver drives them
            // all: taken back with it; one that has begun keeps it, and this one is dropped)
            let slot = super::plandays::slot_of(c, &p.date);
            let mut kept = false;
            for (i, j) in plan::joined_pieces(c, slot, &p.tours, ti, k) {
                let x = &p.tours[i];
                let y = &x.duties[j];
                if y.from_ != Source::Dispatcher {
                    continue;
                }
                if y.who.is_some() && begun(c, p, y.from) {
                    kept = true;
                } else {
                    plan::set_fill(c, &plan::duty_key(&x.tour.line, &x.tour.tour, j), None);
                }
            }
            if kept {
                let penalty = plan::drop_penalty(c, &t.tour, d.start, d.end);
                plan::set_fill(c, &key, Some(Fill::Drop));
                return Ok(Off::Dropped { penalty, rent: 0 });
            }
            return Ok(Off::Choice);
        }
        (Some(_), Source::Central) => {
            let penalty = plan::drop_penalty(c, &t.tour, d.start, d.end);
            plan::set_fill(c, &key, Some(Fill::Drop));
            return Ok(Off::Dropped { penalty, rent: 0 });
        }
        _ => {}
    }
    let Some(who) = rostered_driver(c, p, ti, k) else {
        return Err(NO_DRIVER);
    };
    // (the pieces joined with it: the same driver off them too, where not begun)
    let blocks: Vec<_> = p.tours.iter().map(|x| x.tour.clone()).collect();
    let partners = super::studio::partners(c, super::plandays::slot_of(c, &p.date), &blocks, ti, k);
    plan::set_driver(c, p.weekday, &line, &tour, k, None);
    for (i, j) in partners {
        let x = &p.tours[i];
        let busy = x.duties.get(j).is_some_and(|y| y.who.is_some() && begun(c, p, y.from));
        if rostered_driver(c, p, i, j) == Some(who) && !busy {
            plan::set_driver(c, p.weekday, &x.tour.line, &x.tour.tour, j, None);
        }
    }
    Ok(Off::Roster)
}

/// Take the bus off tour `ti` of the day `p` (see the module).
pub fn bus_off(c: &mut Company, p: &DayPlan, ti: usize) -> Result<Off, &'static str> {
    let t = p.tours.get(ti).ok_or(NOT_FOUND)?;
    if t.bus.is_some() && begun(c, p, t.tour.from()) {
        return Err(UNDER_WAY);
    }
    let (line, tour) = (t.tour.line.clone(), t.tour.tour.clone());
    let key = plan::tour_key(&line, &tour);
    match (t.bus, t.bus_from) {
        (Some(BusOf::Rental), Source::Dispatcher) => {
            let rent = plan::rental_for(c, p, ti).0;
            plan::set_fill(c, &key, None);
            return Ok(Off::Rental { rent });
        }
        (Some(_), Source::Dispatcher) => {
            plan::set_fill(c, &key, None);
            return Ok(Off::Choice);
        }
        (Some(b), Source::Central) => {
            let rent = if b == BusOf::Rental { plan::rental_for(c, p, ti).0 } else { 0 };
            let penalty = plan::drop_penalty(c, &t.tour, 0, t.tour.trips.len());
            plan::set_fill(c, &key, Some(Fill::Drop));
            return Ok(Off::Dropped { penalty, rent });
        }
        _ => {}
    }
    if plan::roster(c, p.weekday, &line, &tour).and_then(|r| r.bus).is_none() {
        return Err(NO_BUS);
    }
    plan::set_bus(c, p.weekday, &line, &tour, None);
    Ok(Off::Roster)
}

/// What taking off several did: how many, and why the others stayed (each reason once).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Many {
    pub done: Vec<Off>,
    pub stayed: Vec<&'static str>,
}

impl Many {
    fn add(&mut self, r: Result<Off, &'static str>) {
        match r {
            Ok(o) => self.done.push(o),
            // (a piece taken off with its partner before: nothing more to say)
            Err(NO_DRIVER) | Err(NO_BUS) => {}
            Err(why) => {
                if !self.stayed.contains(&why) {
                    self.stayed.push(why);
                }
            }
        }
    }

    /// The penalties the trips dropped cost, and the rent of the rental buses not rented.
    pub fn money(&self) -> (Cents, Cents) {
        self.done.iter().fold((0, 0), |(p, r), o| match o {
            Off::Dropped { penalty, rent } => (p + penalty, r + rent),
            Off::Rental { rent } => (p, r + rent),
            _ => (p, r),
        })
    }
}

/// Take off what is chosen: duties (tour, duty) and tours' buses (tour, None), on the day as
/// it was when they were chosen (`p`).
pub fn many_off(c: &mut Company, p: &DayPlan, items: &[(usize, Option<usize>)]) -> Many {
    let mut out = Many::default();
    for &(ti, k) in items {
        out.add(match k {
            Some(k) => driver_off(c, p, ti, k),
            None => bus_off(c, p, ti),
        });
    }
    out
}

/// Take `who` off every duty the day `p` gives them.
pub fn driver_off_all(c: &mut Company, p: &DayPlan, who: Who) -> Many {
    let items: Vec<(usize, Option<usize>)> = p.duties_of(who).into_iter().map(|(ti, k)| (ti, Some(k))).collect();
    many_off(c, p, &items)
}

/// Take bus `bus` off every tour of the day `p` it runs or is fixed for in the roster.
pub fn bus_off_all(c: &mut Company, p: &DayPlan, bus: u32) -> Many {
    let items: Vec<(usize, Option<usize>)> = (0..p.tours.len())
        .filter(|&ti| {
            let t = &p.tours[ti];
            t.bus == Some(BusOf::Own(bus)) || plan::roster(c, p.weekday, &t.tour.line, &t.tour.tour).and_then(|r| r.bus) == Some(bus)
        })
        .map(|ti| (ti, None))
        .collect();
    many_off(c, p, &items)
}
