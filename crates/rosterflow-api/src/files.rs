//! Files the planner saves and opens: where they go without a window (`export_path`), and
//! bytes as base64 for the page.

use std::path::PathBuf;

/// The folder files go to without a save dialog: the data folder's `exports`.
pub fn exports_dir() -> PathBuf {
    crate::store::data_dir().join("exports")
}

/// Where a file `name` goes in `exports` (kept apart from what is there: `name (2)` …).
pub fn export_path(name: &str) -> Option<PathBuf> {
    let dir = exports_dir();
    std::fs::create_dir_all(&dir).ok()?;
    let clean: String = name.chars().map(|c| if c.is_alphanumeric() || matches!(c, ' ' | '-' | '_' | '.' | '(' | ')') { c } else { '_' }).collect();
    let clean = if clean.trim().is_empty() { "file".to_string() } else { clean.trim().to_string() };
    let first = dir.join(&clean);
    if !first.exists() {
        return Some(first);
    }
    let (stem, ext) = match clean.rsplit_once('.') {
        Some((s, e)) => (s.to_string(), format!(".{e}")),
        None => (clean.clone(), String::new()),
    };
    (2..1000).map(|k| dir.join(format!("{stem} ({k}){ext}"))).find(|p| !p.exists())
}

/// Bytes as base64.
pub fn base64_encode(bytes: &[u8]) -> String {
    const ABC: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity(bytes.len().div_ceil(3) * 4);
    for chunk in bytes.chunks(3) {
        let b = [chunk[0], chunk.get(1).copied().unwrap_or(0), chunk.get(2).copied().unwrap_or(0)];
        let n = (b[0] as u32) << 16 | (b[1] as u32) << 8 | b[2] as u32;
        for k in 0..4 {
            if k <= chunk.len() {
                out.push(ABC[(n >> (18 - 6 * k) & 63) as usize] as char);
            } else {
                out.push('=');
            }
        }
    }
    out
}

/// Bytes from base64 (a `data:` prefix passed over).
pub fn base64_decode(text: &str) -> Result<Vec<u8>, String> {
    let text = text.split_once(',').filter(|(head, _)| head.starts_with("data:")).map_or(text, |(_, b)| b);
    let mut out = Vec::with_capacity(text.len() * 3 / 4);
    let (mut acc, mut bits) = (0u32, 0u32);
    for c in text.bytes() {
        let v = match c {
            b'A'..=b'Z' => c - b'A',
            b'a'..=b'z' => c - b'a' + 26,
            b'0'..=b'9' => c - b'0' + 52,
            b'+' | b'-' => 62,
            b'/' | b'_' => 63,
            b'=' | b'\r' | b'\n' | b' ' => continue,
            _ => return Err(format!("not base64: {}", c as char)),
        };
        acc = (acc << 6) | v as u32;
        bits += 6;
        if bits >= 8 {
            bits -= 8;
            out.push((acc >> bits) as u8);
        }
    }
    Ok(out)
}

/// A line in the program's log (`rosterflow.log` in the data folder).
pub fn log(text: &str) {
    use std::io::Write;
    let path = crate::store::data_dir().join("rosterflow.log");
    if let Some(d) = path.parent() {
        let _ = std::fs::create_dir_all(d);
    }
    if let Ok(mut f) = std::fs::OpenOptions::new().create(true).append(true).open(path) {
        let _ = writeln!(f, "{} {text}", chrono::Local::now().format("%Y-%m-%d %H:%M:%S"));
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn base64_goes_both_ways() {
        assert_eq!(super::base64_encode(b"hello"), "aGVsbG8=");
        assert_eq!(super::base64_decode("data:application/pdf;base64,aGVsbG8=").unwrap(), b"hello");
        assert_eq!(super::base64_decode(&super::base64_encode(&[0, 255, 7, 9])).unwrap(), [0, 255, 7, 9]);
    }
}
