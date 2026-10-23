//! Every text the commands answer, in the page's language (the English text is the key,
//! `locales`), and money, numbers and dates written as that language writes them. The page hands
//! its language with every command (`lang`); the views say their texts whole.

use rosterflow_core::company::{dates, Cents};
use std::cell::RefCell;
use std::collections::HashMap;
use std::sync::{Arc, Mutex, OnceLock};

type Table = Arc<HashMap<String, String>>;

thread_local! {
    /// The language of the command being answered on this thread, and its table.
    static LANG: RefCell<(String, Table)> = RefCell::new((String::new(), Arc::new(HashMap::new())));
    /// The currency of the company the command is answered on (`set_currency`).
    static CURRENCY: RefCell<String> = RefCell::new("EUR".to_string());
}

/// The tables read so far, by language.
fn tables() -> &'static Mutex<HashMap<String, Table>> {
    static T: OnceLock<Mutex<HashMap<String, Table>>> = OnceLock::new();
    T.get_or_init(|| Mutex::new(HashMap::new()))
}

/// Answer in `lang` on this thread (an ISO code, "" or "en" for English).
pub fn set_language(lang: &str) {
    let lang = if lang == "en" { "" } else { lang };
    if LANG.with(|l| l.borrow().0 == lang) {
        return;
    }
    let table = if lang.is_empty() {
        Arc::new(HashMap::new())
    } else {
        let mut all = tables().lock().unwrap_or_else(|e| e.into_inner());
        all.entry(lang.to_string()).or_insert_with(|| Arc::new(crate::locales::texts(lang))).clone()
    };
    LANG.with(|l| *l.borrow_mut() = (lang.to_string(), table));
}

/// The language answered in ("" for English).
pub fn language() -> String {
    LANG.with(|l| l.borrow().0.clone())
}

/// A text in the language answered in (as it is where the tables lack it).
pub fn tr(text: &str) -> String {
    LANG.with(|l| l.borrow().1.get(text).cloned()).unwrap_or_else(|| text.to_string())
}

/// A text with its `%{name}`s filled after the lookup.
pub fn trf(text: &str, args: &[(&str, &str)]) -> String {
    let mut t = tr(text);
    for (k, v) in args {
        t = t.replace(&format!("%{{{k}}}"), v);
    }
    t
}

/// A text with its `%{n}` filled.
pub fn n_of(key: &str, n: usize) -> String {
    tr(key).replace("%{n}", &n.to_string())
}

/// A count in words: the text for one, else the text for `n` with its `%{n}` filled.
pub fn count_text(n: usize, one: &str, many: &str) -> String {
    if n == 1 {
        tr(one)
    } else {
        n_of(many, n)
    }
}

// --- money and numbers ----------------------------------------------------------------------

/// Write money in currency `code` (ISO) on this thread (the open company's: `company::with`).
pub fn set_currency(code: &str) {
    CURRENCY.with(|x| *x.borrow_mut() = if code.trim().is_empty() { "EUR".into() } else { code.trim().to_uppercase() });
}

/// The currency money is written in (ISO code).
pub fn currency() -> String {
    CURRENCY.with(|x| x.borrow().clone())
}

/// The currencies a company may keep its money in: code, symbol, decimals.
pub const CURRENCIES: [(&str, &str, usize); 12] = [
    ("EUR", "€", 2),
    ("GBP", "£", 2),
    ("CHF", "CHF", 2),
    ("USD", "$", 2),
    ("DKK", "kr", 2),
    ("SEK", "kr", 2),
    ("NOK", "kr", 2),
    ("PLN", "zł", 2),
    ("CZK", "Kč", 2),
    ("HUF", "Ft", 0),
    ("RON", "lei", 2),
    ("CAD", "$", 2),
];

/// Where a currency's symbol goes.
#[derive(Clone, Copy, Debug, PartialEq)]
enum At {
    Before,
    BeforeSpaced,
    After,
}

/// How a language writes a number: its thousands separator, its decimal mark, and where the euro
/// sign goes.
fn style(lang: &str) -> (char, char, At) {
    match lang {
        "" | "en" => (',', '.', At::Before),
        "nl" => ('.', ',', At::BeforeSpaced),
        "de" | "it" | "es" | "pt" | "da" => ('.', ',', At::After),
        _ => ('\u{202F}', ',', At::After),
    }
}

/// `x` with `places` decimals, grouped by `thousands`, with `decimal` as the mark.
fn number_with(x: f64, places: usize, thousands: char, decimal: char) -> String {
    let neg = x < 0.0 && (x.abs() * 10f64.powi(places as i32)).round() >= 1.0;
    let s = format!("{:.*}", places, x.abs());
    let (whole, frac) = s.split_once('.').map(|(a, b)| (a.to_string(), Some(b.to_string()))).unwrap_or((s.clone(), None));
    let mut grouped = String::new();
    for (i, ch) in whole.chars().enumerate() {
        if i > 0 && (whole.len() - i) % 3 == 0 {
            grouped.push(thousands);
        }
        grouped.push(ch);
    }
    let mut out = if neg { format!("-{grouped}") } else { grouped };
    if let Some(f) = frac {
        out.push(decimal);
        out.push_str(&f);
    }
    out
}

fn money_in(cents: f64, fine: bool, code: &str, lang: &str) -> String {
    let (thousands, decimal, at) = style(lang);
    let (symbol, places) = CURRENCIES.iter().find(|c| c.0 == code).map(|c| (c.1, c.2)).unwrap_or((code, 2));
    // (the euro as the language writes it; another currency after the number but the pound and
    // the dollar)
    let at = match code {
        "EUR" => at,
        "GBP" | "USD" | "CAD" => At::Before,
        _ => At::After,
    };
    let n = number_with(cents / 100.0, if fine { places } else { 0 }, thousands, decimal);
    let (sign, n) = match n.strip_prefix('-') {
        Some(rest) => ("-", rest.to_string()),
        None => ("", n),
    };
    match at {
        At::Before => format!("{sign}{symbol}{n}"),
        At::BeforeSpaced => format!("{sign}{symbol} {n}"),
        At::After => format!("{sign}{n} {symbol}"),
    }
}

/// Money in whole units of the company's currency, grouped as the language writes it ("€1,234",
/// "1.234 €").
pub fn eur(c: Cents) -> String {
    money_in(c as f64, false, &currency(), &language())
}

/// Money with its decimals (a rate, a price per kilometre).
pub fn eur_cents(c: f64) -> String {
    money_in(c, true, &currency(), &language())
}

/// An amount typed in the company's currency ("245000", "€ 245.000,50") as cents; None without a
/// digit.
pub fn typed_money(s: &str) -> Option<Cents> {
    let (_, decimal, _) = style(&language());
    let t: String = s.chars().filter(|c| c.is_ascii_digit() || *c == decimal || *c == '-').collect();
    if !t.chars().any(|c| c.is_ascii_digit()) {
        return None;
    }
    let t = t.replace(decimal, ".");
    t.parse::<f64>().ok().map(|x| (x * 100.0).round() as Cents)
}

/// How the pages write the company's money themselves (`money.js`).
pub fn money_json() -> serde_json::Value {
    let code = currency();
    let (thousands, decimal, at) = style(&language());
    let (symbol, places) = CURRENCIES.iter().find(|c| c.0 == code).map(|c| (c.1, c.2)).unwrap_or(("€", 2));
    let at = match (code.as_str(), at) {
        ("EUR", At::Before) | ("GBP" | "USD" | "CAD", _) => "before",
        ("EUR", At::BeforeSpaced) => "spaced",
        _ => "after",
    };
    serde_json::json!({ "code": code, "rate": 1.0, "symbol": symbol, "at": at, "thousands": thousands.to_string(), "decimal": decimal.to_string(), "places": places, "fare_step": 0.1 })
}

/// A whole number grouped like money (kilometres, passengers).
pub fn grouped(n: f64) -> String {
    let (thousands, decimal, _) = style(&language());
    number_with(n.round(), 0, thousands, decimal)
}

/// A number with `places` decimals as the language writes it.
pub fn num(x: f64, places: usize) -> String {
    let (thousands, decimal, _) = style(&language());
    number_with(x, places, thousands, decimal)
}

/// An amount with its sign and its tone: what comes in green, what goes out red.
pub fn signed(c: Cents) -> (String, &'static str) {
    if c > 0 {
        (format!("+{}", eur(c)), "ok")
    } else if c < 0 {
        (format!("−{}", eur(-c)), "danger")
    } else {
        (eur(0), "dim")
    }
}

// --- dates --------------------------------------------------------------------------------------

pub const WEEKDAYS: [&str; 7] = ["Mon", "Tue", "Wed", "Thu", "Fri", "Sat", "Sun"];
pub const MONTHS: [&str; 12] = ["Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec"];

/// "Tue 5 Mar 2026".
pub fn day_label(date: &str) -> String {
    let Some(d) = dates::parse(date) else { return date.to_string() };
    let (y, m, day) = dates::civil_from_days(d);
    format!("{} {} {} {}", tr(WEEKDAYS[dates::weekday(d) as usize]), day, tr(MONTHS[(m as usize).clamp(1, 12) - 1]), y)
}

/// "12 Oct": a date short.
pub fn short_date(date: &str) -> String {
    let Some(d) = dates::parse(date) else { return date.to_string() };
    let (_, m, day) = dates::civil_from_days(d);
    format!("{} {}", day, tr(MONTHS[(m as usize).clamp(1, 12) - 1]))
}

/// A span of minutes in words, its two largest parts: "2 d 4 h", "4 h 10 min", "12 min".
pub fn span_text(minutes: i64) -> String {
    let m = minutes.max(0);
    let (d, h, m) = (m / 1440, m % 1440 / 60, m % 60);
    let t = match (d, h, m) {
        (0, 0, _) => tr("%{m} min"),
        (0, _, 0) => tr("%{h} h"),
        (0, _, _) => tr("%{h} h %{m} min"),
        (_, 0, _) => tr("%{d} d"),
        _ => tr("%{d} d %{h} h"),
    };
    t.replace("%{d}", &d.to_string()).replace("%{h}", &h.to_string()).replace("%{m}", &m.to_string())
}

/// Today's date as the computer has it, in its own time zone.
pub fn today() -> String {
    dates::today()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn money_is_grouped_as_the_language_groups_it() {
        assert_eq!(money_in(123_456_789.0, false, "EUR", ""), "€1,234,568");
        assert_eq!(money_in(-5_000_00.0, false, "EUR", ""), "-€5,000");
        assert_eq!(money_in(123_456_789.0, false, "EUR", "nl"), "€ 1.234.568");
        assert_eq!(money_in(123_456_789.0, false, "EUR", "de"), "1.234.568 €");
        assert_eq!(money_in(250.0, true, "GBP", "de"), "£2,50");
        assert_eq!(money_in(1_000_00.0, false, "HUF", "en"), "1,000 Ft");
        assert_eq!(number_with(-0.04, 1, '.', ','), "0,0");
    }

    #[test]
    fn a_text_is_said_in_the_language_asked() {
        set_language("nl");
        assert_eq!(language(), "nl");
        assert_eq!(tr("Start time"), "Begintijd");
        set_currency("EUR");
        assert_eq!(eur(1_234_00), "€ 1.234");
        assert_eq!(typed_money("€ 1.234,50"), Some(1_234_50));
        assert!(day_label("2026-05-30").ends_with("2026"));
        set_language("en");
        assert_eq!(tr("Start time"), "Start time");
        assert_eq!(day_label("2026-05-30"), "Sat 30 May 2026");
        assert_eq!(span_text(2 * 1440 + 4 * 60 + 7), "2 d 4 h");
    }
}
