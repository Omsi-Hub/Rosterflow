//! RosterFlow's translations: tables of texts (`locales/*.yml`, embedded at build time), each an
//! English text and its translation per language. The English text is the key: a text a table
//! lacks is shown in English.

include!(concat!(env!("OUT_DIR"), "/tables.rs"));

/// The languages the program is offered in: code and name.
pub const LANGUAGES: [(&str, &str); 7] = [("en", "English"), ("nl", "Nederlands"), ("de", "Deutsch"), ("fr", "Français"), ("pl", "Polski"), ("ru", "Русский"), ("uk", "Українська")];

fn unquote(s: &str) -> String {
    let s = s.trim();
    let Some(inner) = s.strip_prefix('"').and_then(|r| r.strip_suffix('"')) else { return s.to_string() };
    let mut out = String::with_capacity(inner.len());
    let mut chars = inner.chars();
    while let Some(c) = chars.next() {
        if c != '\\' {
            out.push(c);
            continue;
        }
        match chars.next() {
            Some('n') => out.push('\n'),
            Some('t') => out.push('\t'),
            Some(o) => out.push(o),
            None => {}
        }
    }
    out
}

/// One table read: each English text with its translations (`lang` → text). A line `"text":`
/// starts an entry, `  xx: "…"` gives its translation; comments and `_version` are skipped.
pub fn parse(table: &str) -> Vec<(String, Vec<(String, String)>)> {
    let mut out: Vec<(String, Vec<(String, String)>)> = Vec::new();
    for line in table.lines() {
        let line = line.trim_end_matches('\r');
        if line.trim_start().starts_with('#') || line.trim().is_empty() || line.starts_with("_version") {
            continue;
        }
        if let Some(rest) = line.strip_prefix("  ") {
            let Some((lang, text)) = rest.split_once(':') else { continue };
            if let Some(entry) = out.last_mut() {
                entry.1.push((lang.trim().to_string(), unquote(text)));
            }
        } else if let Some(key) = line.trim_end().strip_suffix(':') {
            out.push((unquote(key), Vec::new()));
        }
    }
    out
}

/// Every text of every table in `lang`: the English text → its translation (later tables win).
pub fn texts(lang: &str) -> std::collections::HashMap<String, String> {
    let lang = lang.trim().to_ascii_lowercase();
    let mut out = std::collections::HashMap::new();
    if lang.is_empty() || lang == "en" {
        return out;
    }
    for (_, table) in TABLES {
        for (key, langs) in parse(table) {
            if let Some((_, t)) = langs.iter().find(|(l, _)| l.eq_ignore_ascii_case(&lang)) {
                out.insert(key, t.clone());
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    #[test]
    fn the_tables_read() {
        assert!(super::TABLES.len() > 10);
        let nl = super::texts("nl");
        assert_eq!(nl.get("Start time").map(String::as_str), Some("Begintijd"));
        assert!(super::texts("en").is_empty());
        let t = super::parse("_version: 2\n# x\n\"A \\\"b\\\"\":\n  nl: \"C\\nD\"\n");
        assert_eq!(t, vec![("A \"b\"".to_string(), vec![("nl".to_string(), "C\nD".to_string())])]);
    }
}
