//! RosterFlow in a browser: `cargo run -p rosterflow-dev [port]` serves the page (`ui/`) at
//! http://127.0.0.1:1420 and answers its commands (`POST /api/call`, `/api/bytes` with
//! `{cmd, args}`) as the desktop app does. Files the planner saves go to the data folder's
//! `exports` (no save dialog); `ROSTERFLOW_DATA` puts the data folder elsewhere.

use std::io::{BufRead, BufReader, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::{Path, PathBuf};

fn main() {
    let port: u16 = std::env::args().nth(1).and_then(|p| p.parse().ok()).unwrap_or(1420);
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../ui").canonicalize().expect("ui folder");
    let listener = TcpListener::bind(("127.0.0.1", port)).expect("port");
    println!("RosterFlow: http://127.0.0.1:{port}  (data: {})", rosterflow_api::store::data_dir().display());
    for stream in listener.incoming().flatten() {
        let root = root.clone();
        // (one thread a connection: the commands themselves take the company's lock)
        std::thread::spawn(move || {
            if let Err(e) = serve(stream, &root) {
                eprintln!("{e}");
            }
        });
    }
}

fn serve(mut stream: TcpStream, root: &Path) -> std::io::Result<()> {
    let mut reader = BufReader::new(stream.try_clone()?);
    loop {
        let mut first = String::new();
        if reader.read_line(&mut first)? == 0 {
            return Ok(());
        }
        let mut parts = first.split_whitespace();
        let (method, target) = (parts.next().unwrap_or("").to_string(), parts.next().unwrap_or("/").to_string());
        let mut length = 0usize;
        let mut close = false;
        loop {
            let mut line = String::new();
            reader.read_line(&mut line)?;
            let line = line.trim_end();
            if line.is_empty() {
                break;
            }
            let lower = line.to_ascii_lowercase();
            if let Some(v) = lower.strip_prefix("content-length:") {
                length = v.trim().parse().unwrap_or(0);
            }
            if lower.starts_with("connection:") && lower.contains("close") {
                close = true;
            }
        }
        let mut body = vec![0u8; length];
        reader.read_exact(&mut body)?;
        let (status, kind, out) = answer(&method, &target, &body, root);
        write!(stream, "HTTP/1.1 {status}\r\nContent-Type: {kind}\r\nContent-Length: {}\r\nCache-Control: no-store\r\n\r\n", out.len())?;
        stream.write_all(&out)?;
        stream.flush()?;
        if close {
            return Ok(());
        }
    }
}

fn answer(method: &str, target: &str, body: &[u8], root: &Path) -> (&'static str, &'static str, Vec<u8>) {
    let path = target.split('?').next().unwrap_or("/");
    if method == "POST" && (path == "/api/call" || path == "/api/bytes") {
        let v: serde_json::Value = serde_json::from_slice(body).unwrap_or_default();
        let cmd = v.get("cmd").and_then(|c| c.as_str()).unwrap_or("");
        let args = v.get("args").cloned().unwrap_or(serde_json::json!({}));
        let quiet = rosterflow_api::Quiet;
        if path == "/api/bytes" {
            return match rosterflow_api::call_bytes(cmd, &args, &quiet) {
                Ok(b) => ("200 OK", "application/octet-stream", b),
                Err(e) => ("500 Internal Server Error", "text/plain; charset=utf-8", e.into_bytes()),
            };
        }
        return match rosterflow_api::call(cmd, &args, &quiet) {
            Ok(v) => ("200 OK", "application/json", serde_json::to_vec(&v).unwrap_or_default()),
            Err(e) => ("500 Internal Server Error", "text/plain; charset=utf-8", e.into_bytes()),
        };
    }
    let rel = if path == "/" { "index.html" } else { path.trim_start_matches('/') };
    let file = root.join(rel);
    match file.canonicalize() {
        Ok(f) if f.starts_with(root) && f.is_file() => {
            let kind = match f.extension().and_then(|e| e.to_str()).unwrap_or("") {
                "html" => "text/html; charset=utf-8",
                "js" => "text/javascript; charset=utf-8",
                "css" => "text/css; charset=utf-8",
                "svg" => "image/svg+xml",
                "png" => "image/png",
                "woff2" => "font/woff2",
                "json" => "application/json",
                _ => "application/octet-stream",
            };
            ("200 OK", kind, std::fs::read(f).unwrap_or_default())
        }
        _ => ("404 Not Found", "text/plain", b"not found".to_vec()),
    }
}
