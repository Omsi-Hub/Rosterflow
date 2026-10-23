//! Tables as spreadsheets save them: read with the separator they use (`;`, `,` or a tab, the
//! one the header has most of), quotes and all; written with `;` (what a spreadsheet in most of
//! Europe opens as columns) and a byte-order mark (so that it reads the accents right).

/// The rows of `text` (empty rows left out), each its cells trimmed.
pub fn read(text: &str) -> Vec<Vec<String>> {
    let text = text.trim_start_matches('\u{feff}');
    let head = text.lines().next().unwrap_or("");
    let sep = [';', ',', '\t'].into_iter().max_by_key(|c| head.matches(*c).count()).unwrap_or(';');
    let mut rows = Vec::new();
    let mut row: Vec<String> = Vec::new();
    let mut cell = String::new();
    let mut quoted = false;
    let mut chars = text.chars().peekable();
    while let Some(c) = chars.next() {
        if quoted {
            match c {
                '"' if chars.peek() == Some(&'"') => {
                    cell.push('"');
                    chars.next();
                }
                '"' => quoted = false,
                c => cell.push(c),
            }
            continue;
        }
        match c {
            '"' => quoted = true,
            '\r' => {}
            '\n' => {
                row.push(std::mem::take(&mut cell).trim().to_string());
                if row.iter().any(|c| !c.is_empty()) {
                    rows.push(std::mem::take(&mut row));
                } else {
                    row.clear();
                }
            }
            c if c == sep => row.push(std::mem::take(&mut cell).trim().to_string()),
            c => cell.push(c),
        }
    }
    row.push(cell.trim().to_string());
    if row.iter().any(|c| !c.is_empty()) {
        rows.push(row);
    }
    rows
}

/// `rows` as a file's text.
pub fn write(rows: &[Vec<String>]) -> String {
    let mut out = String::from("\u{feff}");
    for r in rows {
        let cells: Vec<String> = r
            .iter()
            .map(|c| if c.contains([';', '"', '\n', ',']) { format!("\"{}\"", c.replace('"', "\"\"")) } else { c.clone() })
            .collect();
        out.push_str(&cells.join(";"));
        out.push_str("\r\n");
    }
    out
}

/// A table read with its header: each row's cell under a column (its name in lower case, without
/// spaces), "" where it has none.
pub struct Table {
    pub head: Vec<String>,
    pub rows: Vec<Vec<String>>,
}

impl Table {
    pub fn of(text: &str) -> Table {
        let mut rows = read(text);
        let head = if rows.is_empty() { Vec::new() } else { rows.remove(0).into_iter().map(|h| key(&h)).collect() };
        Table { head, rows }
    }

    /// The column of the first of `names` the header has.
    pub fn col(&self, names: &[&str]) -> Option<usize> {
        names.iter().find_map(|n| self.head.iter().position(|h| *h == key(n)))
    }

    /// Row `r`'s cell of the first of `names`.
    pub fn get<'a>(&self, r: &'a [String], names: &[&str]) -> &'a str {
        self.col(names).and_then(|k| r.get(k)).map(String::as_str).unwrap_or("")
    }
}

fn key(s: &str) -> String {
    s.trim().to_lowercase().chars().filter(|c| c.is_alphanumeric()).collect()
}

/// A number as typed in a cell ("1.234,5", "1234.5"); None when empty or not one.
pub fn number(s: &str) -> Option<f64> {
    let t = s.trim().replace(' ', "");
    if t.is_empty() {
        return None;
    }
    // (both marks: the last is the decimal one)
    let t = match (t.rfind(','), t.rfind('.')) {
        (Some(c), Some(d)) if c > d => t.replace('.', "").replace(',', "."),
        (Some(_), Some(_)) => t.replace(',', ""),
        (Some(_), None) => t.replace(',', "."),
        _ => t,
    };
    t.parse().ok()
}

/// A date as typed in a cell (`2026-03-05`, `5-3-2026`, `05/03/2026`) as `YYYY-MM-DD`; "" when
/// empty or not one.
pub fn date(s: &str) -> String {
    let t = s.trim();
    if rosterflow_core::company::dates::parse(t).is_some() {
        return t.to_string();
    }
    let parts: Vec<&str> = t.split(['-', '/', '.']).collect();
    if let [d, m, y] = parts.as_slice() {
        if let (Ok(d), Ok(m), Ok(y)) = (d.parse::<u32>(), m.parse::<u32>(), y.parse::<i32>()) {
            let s = format!("{y:04}-{m:02}-{d:02}");
            if rosterflow_core::company::dates::parse(&s).is_some() {
                return s;
            }
        }
    }
    String::new()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_table_goes_both_ways() {
        let t = Table::of("\u{feff}Code;Name;Lat\r\n1001;\"Station; Noord\";52,1\r\n\r\n1002;Markt;\r\n");
        assert_eq!(t.rows.len(), 2);
        assert_eq!(t.get(&t.rows[0], &["name"]), "Station; Noord");
        assert_eq!(number(t.get(&t.rows[0], &["lat"])), Some(52.1));
        assert_eq!(t.get(&t.rows[1], &["lon"]), "");
        let back = read(&write(&[vec!["a\"b".into(), "c;d".into()]]));
        assert_eq!(back, vec![vec!["a\"b".to_string(), "c;d".to_string()]]);
        assert_eq!(read("a,b\n1,2").len(), 2);
        assert_eq!(number("1.234,5"), Some(1234.5));
        assert_eq!(date("5-3-2026"), "2026-03-05");
        assert_eq!(date("x"), "");
    }
}
