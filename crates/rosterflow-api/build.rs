//! Every table of `locales/` embedded (`locales::TABLES`): a table added there is served without
//! a change of the code.

use std::io::Write;

fn main() {
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("locales");
    println!("cargo:rerun-if-changed={}", dir.display());
    let mut names: Vec<String> = std::fs::read_dir(&dir).expect("locales").flatten().map(|e| e.file_name().to_string_lossy().into_owned()).filter(|n| n.ends_with(".yml")).collect();
    names.sort();
    let out = std::path::Path::new(&std::env::var("OUT_DIR").unwrap()).join("tables.rs");
    let mut f = std::fs::File::create(out).unwrap();
    writeln!(f, "pub const TABLES: &[(&str, &str)] = &[").unwrap();
    for n in names {
        println!("cargo:rerun-if-changed={}", dir.join(&n).display());
        writeln!(f, "    ({n:?}, include_str!(concat!(env!(\"CARGO_MANIFEST_DIR\"), \"/locales/{n}\"))),").unwrap();
    }
    writeln!(f, "];").unwrap();
}
