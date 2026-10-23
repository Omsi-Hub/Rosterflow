//! What the commands hand the page besides their views: a popup that says why something cannot
//! be done, and which page of the program solves it.

use super::words::tr;
use serde_json::{json, Value};

/// A page of the program a popup leads to.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Go {
    /// The drivers (their licences and type trainings).
    Licences,
    /// The drivers, to add one.
    Hire,
    /// The fleet, to add a bus.
    Dealer,
    /// The network's lines.
    Lines,
    /// The stops.
    Stops,
    /// The company's settings.
    Settings,
}

impl Go {
    pub fn json(self) -> Value {
        let (page, label) = match self {
            Go::Licences => ("staff", "Open the drivers"),
            Go::Hire => ("staff", "Add a driver"),
            Go::Dealer => ("fleet", "Add a bus"),
            Go::Lines => ("lines", "Open the lines"),
            Go::Stops => ("stops", "Open the stops"),
            Go::Settings => ("settings", "Open the settings"),
        };
        json!({ "page": page, "label": tr(label) })
    }
}

/// Something that cannot be done: what, why, and what solves it.
#[derive(Clone, Debug, PartialEq)]
pub struct Popup {
    pub icon: &'static str,
    pub title: String,
    pub text: String,
    pub unlock: String,
    pub go: Option<Go>,
}

impl Popup {
    pub fn new(icon: &'static str, title: &str, text: impl Into<String>, unlock: impl Into<String>, go: Option<Go>) -> Popup {
        Popup { icon, title: tr(title), text: text.into(), unlock: unlock.into(), go }
    }

    /// As the page draws it.
    pub fn json(&self) -> Value {
        json!({ "icon": self.icon, "title": self.title, "text": self.text, "unlock": self.unlock, "go": self.go.map(Go::json) })
    }
}
