//! SPEC-21 success criterion 1: the layout table matches `ls -A` at the
//! repository root.
//!
//! Verifies: SPEC-21

use std::collections::BTreeSet;
use std::path::Path;

/// Backticked names in the first cell of each table row, and on the
/// `Root files:` lines, reduced to top-level entry names.
fn listed_entries(spec: &str) -> BTreeSet<String> {
    let mut out = BTreeSet::new();
    let mut in_root_files = false;
    for line in spec.lines() {
        if let Some(rest) = line.strip_prefix("| `") {
            if let Some((cell, _)) = rest.split_once('`') {
                out.insert(cell.trim_end_matches('/').to_string());
            }
        }
        if line.starts_with("Root files:") {
            in_root_files = true;
        } else if line.trim().is_empty() {
            in_root_files = false;
        }
        if in_root_files {
            let text = line.trim_start_matches("Root files:");
            for (i, part) in text.split('`').enumerate() {
                if i % 2 == 1 {
                    out.insert(part.to_string());
                }
            }
        }
    }
    out
}

#[test]
fn the_layout_table_matches_ls_a_of_the_repository_root() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../..");
    let spec = std::fs::read_to_string(root.join("docs/specs/SPEC-21-repository-layout.md"))
        .expect("SPEC-21 readable");
    let listed = listed_entries(&spec);

    // Git's view, not the raw directory: a gitignored entry such as `.DS_Store`
    // or `.idea/` on a developer machine is not part of the layout.
    let out = std::process::Command::new("git")
        .args([
            "ls-files",
            "-z",
            "--cached",
            "--others",
            "--exclude-standard",
        ])
        .current_dir(&root)
        .output()
        .expect("git should run");
    assert!(out.status.success(), "git ls-files failed");
    let on_disk: BTreeSet<String> = String::from_utf8_lossy(&out.stdout)
        .split('\0')
        .filter(|p| !p.is_empty() && root.join(p).exists())
        .map(|p| p.split('/').next().unwrap().to_string())
        .collect();

    let unlisted: Vec<_> = on_disk.difference(&listed).collect();
    let absent: Vec<_> = listed.difference(&on_disk).collect();
    assert!(
        unlisted.is_empty() && absent.is_empty(),
        "present and unlisted: {unlisted:?}; listed and absent: {absent:?}"
    );
}
