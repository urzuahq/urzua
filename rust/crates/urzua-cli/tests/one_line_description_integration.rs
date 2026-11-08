//! The one-line description of urzua is the same sentence in `urzua --help`, in the
//! README's first paragraph and in `RFC-27`, the record that states what the engine is
//! for. A change to one without the others fails here.
//!
//! Verifies: RFC-27

mod common;

use std::path::{Path, PathBuf};

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(3)
        .unwrap()
        .to_path_buf()
}

fn help_description() -> String {
    let out = common::urzua().arg("--help").output().unwrap();
    assert!(out.status.success());
    String::from_utf8(out.stdout)
        .unwrap()
        .lines()
        .next()
        .expect("help has a first line")
        .trim()
        .to_string()
}

#[test]
fn help_readme_and_rfc_27_carry_the_same_one_line_description() {
    let description = help_description();
    assert!(description.len() > 40, "{description}");

    let readme = std::fs::read_to_string(repo_root().join("README.md")).unwrap();
    assert!(
        readme.lines().any(|l| l.trim() == description),
        "README.md has no line equal to the help description:\n{description}"
    );

    let rfc_dir = repo_root().join("docs/rfc");
    let rfc = std::fs::read_dir(&rfc_dir)
        .unwrap()
        .map(|e| e.unwrap().path())
        .find(|p| {
            p.file_name()
                .and_then(|n| n.to_str())
                .is_some_and(|n| n.starts_with("RFC-27-"))
        })
        .expect("RFC-27 exists");
    let rfc = std::fs::read_to_string(rfc).unwrap();
    assert!(
        rfc.lines()
            .any(|l| l.trim_start_matches('>').trim() == description),
        "RFC-27 has no line equal to the help description:\n{description}"
    );
}
