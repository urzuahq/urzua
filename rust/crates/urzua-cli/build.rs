//! Embeds the harness bundle (`bundle/`, `ADR-89`) into the binary.
//!
//! The table of adapters is generated from the directories under `bundle/`, so
//! it cannot lag the directory and the engine source never spells an adapter's
//! name: each directory is one adapter, named by its directory, holding
//! `adapter.json`, `settings.fragment.json` and `files/` (a tree installed at
//! the same relative paths).

use std::fmt::Write as _;
use std::path::{Path, PathBuf};

fn files_under(dir: &Path, base: &Path, out: &mut Vec<PathBuf>) {
    let mut entries: Vec<PathBuf> = std::fs::read_dir(dir)
        .unwrap_or_else(|e| panic!("cannot read {}: {e}", dir.display()))
        .map(|e| e.expect("a directory entry").path())
        .collect();
    entries.sort();
    for path in entries {
        if path.is_dir() {
            files_under(&path, base, out);
        } else {
            out.push(path.strip_prefix(base).expect("under base").to_path_buf());
        }
    }
}

fn main() {
    let root = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").expect("cargo sets it"));
    let bundle = root.join("bundle");
    println!("cargo:rerun-if-changed=bundle");
    let mut adapters: Vec<PathBuf> = std::fs::read_dir(&bundle)
        .expect("bundle/ exists")
        .map(|e| e.expect("a directory entry").path())
        .filter(|p| p.is_dir())
        .collect();
    adapters.sort();
    let mut table = String::from("pub const ADAPTERS: &[Adapter] = &[\n");
    for dir in adapters {
        let name = dir
            .file_name()
            .expect("a name")
            .to_string_lossy()
            .to_string();
        let mut files = Vec::new();
        files_under(&dir.join("files"), &dir.join("files"), &mut files);
        let abs = |p: &Path| format!("{:?}", p.to_string_lossy());
        writeln!(table, "    Adapter {{\n        name: {name:?},").unwrap();
        writeln!(
            table,
            "        manifest: include_str!({}),",
            abs(&dir.join("adapter.json"))
        )
        .unwrap();
        writeln!(
            table,
            "        settings_fragment: include_str!({}),",
            abs(&dir.join("settings.fragment.json"))
        )
        .unwrap();
        table.push_str("        commands: &[\n");
        let mut commands = Vec::new();
        if dir.join("commands").is_dir() {
            files_under(&dir.join("commands"), &dir.join("commands"), &mut commands);
        }
        for f in commands {
            let rel = f.to_string_lossy().replace('\\', "/");
            writeln!(
                table,
                "            ({rel:?}, include_str!({})),",
                abs(&dir.join("commands").join(&f))
            )
            .unwrap();
        }
        table.push_str("        ],\n");
        table.push_str("        files: &[\n");
        for f in files {
            let rel = f.to_string_lossy().replace('\\', "/");
            writeln!(
                table,
                "            ({rel:?}, include_str!({})),",
                abs(&dir.join("files").join(&f))
            )
            .unwrap();
        }
        table.push_str("        ],\n    },\n");
    }
    table.push_str("];\n");
    let out =
        PathBuf::from(std::env::var("OUT_DIR").expect("cargo sets it")).join("bundle_table.rs");
    std::fs::write(out, table).expect("write the generated table");
}
