//! RosterFlow's domain, apart from the window: the bus company and its planning.
//!
//! * [`timetable`] - a day's lines, their tours (blocks), trips and stops ([`LineInfo`]), as
//!   the planning works with them, made from the company's own lines ([`lines`]);
//! * [`lines`] - the company's lines as it designs them: their directions and stops, the days
//!   they run, the timetable as a table of trips, the running times;
//! * [`company`] - the company itself (its fleet, its people, its lines) and the planning:
//!   the Timeplan, the vehicle blocks, the driver duties, the weekly roster and Optimise.

pub mod calendar;
pub mod company;
pub mod lines;
pub mod timetable;

pub use timetable::{lines_on, LineInfo, StopInfo, TourInfo, TripInfo};
