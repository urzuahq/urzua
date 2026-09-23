//! Mechanical guard against `BUG-118`/`BUG-119`/`BUG-120`'s recurring shape:
//! a fix to a declared-vocabulary field name landed everywhere the reviewed
//! diff touched, while a sibling call site elsewhere in the tree kept
//! reading the old literal, three times in one review round. `RFC-42`/
//! `ADR-61` made `Status`/`Embodiment`/`Realized-by`/
//! `Supersedes / Superseded-by` adopter-declared; this walks every source
//! file in both crates and fails if one of those literals reappears in
//! production code outside `config.rs`, where `RelationRole`'s own
//! defaults are declared. Ships with planted-violation cases, since a check
//! never observed failing is unverified, not passing.

use std::path::{Path, PathBuf};

const GUARDED_LITERALS: &[&str] = &[
    "\"Status\"",
    "\"Embodiment\"",
    "\"Realized-by\"",
    "\"Supersedes / Superseded-by\"",
];

/// `mod tests` bodies are exempt: they legitimately write these as
/// record-fixture content and declared config vocabulary, not as a
/// production lookup key.
fn production_source(source: &str) -> &str {
    match source.find("#[cfg(test)]\nmod tests") {
        Some(idx) => &source[..idx],
        None => source,
    }
}

/// The actual assertion, extracted as a pure function over already-read file
/// contents so it can be exercised against a synthetic planted violation
/// without touching the filesystem.
fn find_violations<'a>(files: &[(&'a Path, &'a str)]) -> Vec<(&'a Path, &'static str)> {
    let mut violations = Vec::new();
    for (path, contents) in files {
        if path.file_name().and_then(|n| n.to_str()) == Some("config.rs") {
            continue;
        }
        let production = production_source(contents);
        for literal in GUARDED_LITERALS {
            if production.contains(literal) {
                violations.push((*path, *literal));
            }
        }
    }
    violations
}

fn rust_source_files(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            rust_source_files(&path, out);
        } else if path.extension().and_then(|e| e.to_str()) == Some("rs") {
            out.push(path);
        }
    }
}

#[test]
fn no_relation_field_literal_escapes_config_rs() {
    let crates_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("..");
    let mut paths = Vec::new();
    for crate_name in ["urzua-core", "urzua-cli"] {
        rust_source_files(&crates_dir.join(crate_name).join("src"), &mut paths);
    }
    assert!(
        paths.len() > 5,
        "sanity check: should find several source files, found {}",
        paths.len()
    );

    let contents: Vec<(PathBuf, String)> = paths
        .into_iter()
        .map(|p| {
            let c = std::fs::read_to_string(&p)
                .unwrap_or_else(|e| panic!("{} should be readable: {e}", p.display()));
            (p, c)
        })
        .collect();
    let refs: Vec<(&Path, &str)> = contents
        .iter()
        .map(|(p, c)| (p.as_path(), c.as_str()))
        .collect();

    let violations = find_violations(&refs);
    assert!(
        violations.is_empty(),
        "found a hardcoded relation-field literal outside config.rs -- read it through \
         RecordTypeConfig::relation_field / rules::relation_field_name instead \
         (BUG-118/BUG-119/BUG-120): {violations:?}"
    );
}

#[test]
fn the_check_is_observed_failing_on_a_planted_violation() {
    let planted = "fn f(r: &Record) { r.header.get(\"Status\") }\n";
    let path = Path::new("planted.rs");
    let violations = find_violations(&[(path, planted)]);
    assert_eq!(violations, vec![(path, "\"Status\"")]);
}

#[test]
fn a_literal_inside_mod_tests_is_not_a_violation() {
    let source = "fn f() {}\n\n#[cfg(test)]\nmod tests {\n    fn g() { let _ = \"Status\"; }\n}\n";
    let path = Path::new("with_tests.rs");
    assert!(find_violations(&[(path, source)]).is_empty());
}

#[test]
fn config_rs_itself_is_exempt() {
    let source = "fn default_field_name() -> &'static str { \"Status\" }\n";
    let path = Path::new("config.rs");
    assert!(find_violations(&[(path, source)]).is_empty());
}
