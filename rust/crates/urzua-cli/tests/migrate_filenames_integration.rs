//! MILE-134 (SPEC-14): `urzua migrate filenames` renames a corpus's record files to the
//! filename convention the type's `identity.pattern` declares, rewrites citations of the
//! renamed paths, and previews unless `--apply` is given. Each test plants a corpus in a
//! temporary git repository and runs the compiled binary.

mod common;

use std::path::Path;

const RULES: &str = "{header.required-fields: error, field.quality: error, filename.identity-pattern: error, filename.title-consistency: error, pointer.resolution: error}";
const PATTERN: &str = r"^(?P<prefix>ADR)-(?P<number>\d{4})-(?P<slug>.+)$";

fn record(number: u32, title: &str, body: &str) -> String {
    format!(
        "---\nStable-Id: 01ARZ3NDEKTSV4RRFFQ69G5F{number:02}\nStatus: Accepted\nDate: 2026-01-01\nAuthor: x\n---\n# ADR-{number:04} - {title}\n\n{body}\n"
    )
}

fn fixture(name: &str, pattern: Option<&str>) -> common::TempDir {
    let dir = common::TempDir::new(&format!("urzua-migrate-filenames-{name}"));
    std::fs::create_dir_all(dir.join("docs/adr")).unwrap();
    std::fs::create_dir_all(dir.join(".urzua")).unwrap();
    let identity = pattern
        .map(|p| format!("    identity: {{from: filename, pattern: '{p}'}}\n"))
        .unwrap_or_default();
    std::fs::write(
        dir.join(".urzua/config.yaml"),
        format!(
            "schema_version: 2\nrules: {RULES}\n\nrecord_types:\n  adr:\n    dir: \"docs/adr\"\n    prefix: \"ADR\"\n    required_fields: [\"Status\", \"Date\", \"Author\"]\n    header_shape: \"yaml-frontmatter\"\n{identity}    statuses:\n      values: [Proposed, Accepted]\n      success: [Accepted]\n"
        ),
    )
    .unwrap();
    common::git(&dir, &["init", "-q"]);
    common::git(&dir, &["config", "user.email", "test@example.com"]);
    common::git(&dir, &["config", "user.name", "test"]);
    dir
}

fn write(dir: &Path, rel: &str, text: &str) {
    let path = dir.join(rel);
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(path, text).unwrap();
}

fn commit(dir: &Path) {
    common::commit_all(dir, 1);
}

fn run(dir: &Path, args: &[&str]) -> (Option<i32>, serde_json::Value) {
    let out = common::urzua()
        .args(args)
        .current_dir(dir)
        .output()
        .unwrap();
    let json = serde_json::from_slice(&out.stdout).unwrap_or_else(|e| {
        panic!(
            "not JSON ({e}): {}{}",
            String::from_utf8_lossy(&out.stdout),
            String::from_utf8_lossy(&out.stderr)
        )
    });
    (out.status.code(), json)
}

/// Two bare zero-padded records and a record that cites the first three ways.
fn bare_corpus(dir: &Path) {
    write(
        dir,
        "docs/adr/0001-first-thing.md",
        &record(1, "First thing", "Self mention 0001-first-thing.md."),
    );
    write(dir, "docs/adr/0002-second.md", &record(2, "Second", "See [one](./0001-first-thing.md#why), [again](0001-first-thing.md) and docs/adr/0001-first-thing.md, but not other/0001-first-thing.md."));
    write(
        dir,
        "README.md",
        "Start at [the first](docs/adr/0001-first-thing.md).\n",
    );
    write(dir, "notes.txt", "see docs/adr/0002-second.md for more\n");
    commit(dir);
}

#[test]
fn a_dry_run_lists_every_rename_and_citation_and_writes_nothing() {
    let dir = fixture("dry", Some(PATTERN));
    bare_corpus(&dir);
    let (code, report) = run(&dir, &["migrate", "filenames"]);
    assert_eq!(code, Some(0), "{report}");
    assert_eq!(report["apply"], false);
    let renames = report["renames"].as_array().unwrap();
    assert_eq!(renames.len(), 2, "{report}");
    assert_eq!(renames[0]["from"], "docs/adr/0001-first-thing.md");
    assert_eq!(renames[0]["to"], "docs/adr/ADR-0001-first-thing.md");
    assert!(
        report["rewrites"].as_array().unwrap().len() >= 4,
        "{report}"
    );
    let unrewritten = report["unrewritten"].as_array().unwrap();
    assert!(
        unrewritten.iter().any(|u| u["file"] == "notes.txt"),
        "{report}"
    );
    assert!(dir.join("docs/adr/0001-first-thing.md").exists());
    assert!(!dir.join("docs/adr/ADR-0001-first-thing.md").exists());
}

#[test]
fn apply_renames_keeps_identity_rewrites_citations_and_check_passes() {
    let dir = fixture("apply", Some(PATTERN));
    bare_corpus(&dir);
    let before = std::fs::read_to_string(dir.join("docs/adr/0001-first-thing.md")).unwrap();
    let (code, report) = run(&dir, &["migrate", "filenames", "--apply"]);
    assert_eq!(code, Some(0), "{report}");
    assert_eq!(report["status"], "ok", "{report}");

    let renamed = std::fs::read_to_string(dir.join("docs/adr/ADR-0001-first-thing.md")).unwrap();
    assert!(
        renamed.contains("Stable-Id: 01ARZ3NDEKTSV4RRFFQ69G5F01"),
        "{renamed}"
    );
    assert!(renamed.contains("# ADR-0001 - First thing"), "{renamed}");
    assert!(
        renamed.contains("Self mention ADR-0001-first-thing.md."),
        "{renamed}"
    );
    assert_eq!(
        before.replace("0001-first-thing.md", "ADR-0001-first-thing.md"),
        renamed
    );

    let second = std::fs::read_to_string(dir.join("docs/adr/ADR-0002-second.md")).unwrap();
    assert!(
        second.contains("[one](./ADR-0001-first-thing.md#why)"),
        "{second}"
    );
    assert!(
        second.contains("[again](ADR-0001-first-thing.md)"),
        "{second}"
    );
    assert!(
        second.contains("and docs/adr/ADR-0001-first-thing.md,"),
        "{second}"
    );
    assert!(
        second.contains("not other/0001-first-thing.md."),
        "{second}"
    );
    let readme = std::fs::read_to_string(dir.join("README.md")).unwrap();
    assert!(
        readme.contains("(docs/adr/ADR-0001-first-thing.md)"),
        "{readme}"
    );
    let notes = std::fs::read_to_string(dir.join("notes.txt")).unwrap();
    assert!(
        notes.contains("docs/adr/0002-second.md"),
        "a non-markdown file is reported, not edited"
    );

    commit(&dir);
    let (code, check) = run(&dir, &["check"]);
    assert_eq!(code, Some(0), "{check}");
    assert_eq!(check["findings"].as_array().unwrap().len(), 0, "{check}");
}

#[test]
fn a_second_applied_run_changes_nothing() {
    let dir = fixture("idempotent", Some(PATTERN));
    bare_corpus(&dir);
    let (code, _) = run(&dir, &["migrate", "filenames", "--apply"]);
    assert_eq!(code, Some(0));
    commit(&dir);
    let (code, report) = run(&dir, &["migrate", "filenames", "--apply"]);
    assert_eq!(code, Some(0), "{report}");
    assert_eq!(report["renames"].as_array().unwrap().len(), 0, "{report}");
    assert_eq!(report["rewrites"].as_array().unwrap().len(), 0, "{report}");
    assert!(report.get("results").is_none(), "{report}");
    let (_, status) = run_git_status(&dir);
    assert_eq!(status, "", "nothing changed on disk");
}

fn run_git_status(dir: &Path) -> (bool, String) {
    let out = std::process::Command::new("git")
        .args(["status", "--porcelain"])
        .current_dir(dir)
        .output()
        .unwrap();
    (
        out.status.success(),
        String::from_utf8_lossy(&out.stdout).trim().to_string(),
    )
}

#[test]
fn the_target_follows_the_declared_pattern_never_a_hardcoded_shape() {
    // Padding to four digits is the pattern's, so `7-x.md` becomes ADR-0007.
    let dir = fixture("padding", Some(PATTERN));
    write(&dir, "docs/adr/7-lucky.md", &record(7, "Lucky", "x"));
    commit(&dir);
    let (_, report) = run(&dir, &["migrate", "filenames"]);
    assert_eq!(
        report["renames"][0]["to"], "docs/adr/ADR-0007-lucky.md",
        "{report}"
    );

    // With no declared pattern, the TYPE-NNNN-slug convention applies as written.
    let dir = fixture("convention", None);
    write(&dir, "docs/adr/0007-lucky.md", &record(7, "Lucky", "x"));
    commit(&dir);
    let (_, report) = run(&dir, &["migrate", "filenames"]);
    assert_eq!(
        report["renames"][0]["to"], "docs/adr/ADR-0007-lucky.md",
        "{report}"
    );
}

#[test]
fn a_rename_that_would_overwrite_a_file_is_skipped_and_reported() {
    let dir = fixture("conflict", Some(PATTERN));
    write(&dir, "docs/adr/0001-a.md", &record(1, "A", "x"));
    write(&dir, "docs/adr/ADR-0001-a.md", &record(1, "A again", "y"));
    write(&dir, "docs/adr/README.md", "# Index\n");
    commit(&dir);
    let (code, report) = run(&dir, &["migrate", "filenames", "--apply"]);
    assert_eq!(code, Some(0), "{report}");
    let skipped = report["skipped"].as_array().unwrap();
    assert!(
        skipped
            .iter()
            .any(|s| s["from"] == "docs/adr/0001-a.md" && s["reason"] == "target-exists"),
        "{report}"
    );
    assert!(
        skipped
            .iter()
            .any(|s| s["from"] == "docs/adr/README.md" && s["reason"] == "no-number"),
        "{report}"
    );
    assert!(dir.join("docs/adr/0001-a.md").exists());
    assert!(std::fs::read_to_string(dir.join("docs/adr/ADR-0001-a.md"))
        .unwrap()
        .contains("A again"));
}

#[test]
fn a_record_without_a_stable_id_gets_a_notice_to_backfill_first() {
    let dir = fixture("no-sid", Some(PATTERN));
    write(
        &dir,
        "docs/adr/0001-a.md",
        "---\nStatus: Accepted\nDate: 2026-01-01\nAuthor: x\n---\n# ADR-0001 - A\n",
    );
    commit(&dir);
    let (_, report) = run(&dir, &["migrate", "filenames"]);
    let notices = report["notices"].as_array().unwrap();
    assert!(
        notices.iter().any(|n| n["subject"] == "stable-id-missing"
            && n["message"].as_str().unwrap().contains("migrate ids")),
        "{report}"
    );
}

#[test]
fn two_files_that_would_share_a_record_id_are_both_skipped() {
    let dir = fixture("collision", Some(PATTERN));
    write(&dir, "docs/adr/5-x.md", &record(5, "X", "x"));
    write(&dir, "docs/adr/0005-y.md", &record(5, "Y", "y"));
    // A third file renaming onto an id a conforming record already holds.
    write(&dir, "docs/adr/ADR-0006-held.md", &record(6, "Held", "h"));
    write(&dir, "docs/adr/6-z.md", &record(6, "Z", "z"));
    commit(&dir);
    let (code, report) = run(&dir, &["migrate", "filenames", "--apply"]);
    assert_eq!(code, Some(0), "{report}");
    assert_eq!(report["renames"].as_array().unwrap().len(), 0, "{report}");
    let skipped = report["skipped"].as_array().unwrap();
    for from in ["docs/adr/5-x.md", "docs/adr/0005-y.md", "docs/adr/6-z.md"] {
        assert!(
            skipped
                .iter()
                .any(|s| s["from"] == from && s["reason"] == "number-collision"),
            "{from}: {report}"
        );
    }
    assert!(dir.join("docs/adr/5-x.md").exists() && dir.join("docs/adr/0005-y.md").exists());
}

#[test]
fn padding_may_narrow_and_a_prefixed_misspadded_name_is_fixed() {
    let dir = fixture("narrow", Some(PATTERN));
    write(&dir, "docs/adr/00012-z.md", &record(12, "Z", "z"));
    write(&dir, "docs/adr/ADR-5-x.md", &record(5, "X", "x"));
    commit(&dir);
    let (_, report) = run(&dir, &["migrate", "filenames", "--apply"]);
    let to: Vec<_> = report["renames"]
        .as_array()
        .unwrap()
        .iter()
        .map(|r| r["to"].as_str().unwrap().to_string())
        .collect();
    assert!(
        to.contains(&"docs/adr/ADR-0012-z.md".to_string()),
        "{report}"
    );
    assert!(
        to.contains(&"docs/adr/ADR-0005-x.md".to_string()),
        "{report}"
    );
    assert_eq!(report["skipped"].as_array().unwrap().len(), 0, "{report}");
    commit(&dir);
    let (code, check) = run(&dir, &["check"]);
    assert_eq!(code, Some(0), "{check}");
}

/// The state a failed rename leaves: citations already name the new path while the file
/// still has its old one. A rerun renames it and the corpus checks clean.
#[test]
fn a_rerun_heals_a_corpus_left_with_rewritten_citations_and_an_unrenamed_file() {
    let dir = fixture("heal", Some(PATTERN));
    write(&dir, "docs/adr/0001-first.md", &record(1, "First", "x"));
    write(
        &dir,
        "docs/adr/ADR-0002-second.md",
        &record(2, "Second", "See ADR-0001-first.md."),
    );
    commit(&dir);
    let (code, report) = run(&dir, &["migrate", "filenames", "--apply"]);
    assert_eq!(code, Some(0), "{report}");
    assert_eq!(report["rewrites"].as_array().unwrap().len(), 0, "{report}");
    assert_eq!(report["renames"].as_array().unwrap().len(), 1, "{report}");
    commit(&dir);
    let (code, check) = run(&dir, &["check"]);
    assert_eq!(code, Some(0), "{check}");
    assert_eq!(check["findings"].as_array().unwrap().len(), 0, "{check}");
}

#[test]
fn a_pin_on_a_rewritten_record_is_reported_stale_not_silently_repinned() {
    let dir = fixture("pins", Some(PATTERN));
    write(
        &dir,
        "docs/adr/0001-first.md",
        &record(1, "First", "Mentions 0001-first.md."),
    );
    write(
        &dir,
        "docs/adr/ADR-0003-pinner.md",
        &record(3, "Pinner", "Evidence: code:docs/adr/0001-first.md@sha256:0123456789ab and code:docs/adr/ADR-0003-pinner.md@sha256:0123456789ab"),
    );
    commit(&dir);
    let (_, report) = run(&dir, &["migrate", "filenames"]);
    let pins = report["stale_pins"].as_array().unwrap();
    // Both pins: the pinner's own text is rewritten too (it names the renamed file).
    assert_eq!(pins.len(), 2, "{report}");
    assert_eq!(pins[0]["file"], "docs/adr/ADR-0003-pinner.md");
    assert!(
        pins.iter()
            .any(|p| p["changed_file"] == "docs/adr/0001-first.md"),
        "{report}"
    );
    assert!(
        pins.iter()
            .any(|p| p["changed_file"] == "docs/adr/ADR-0003-pinner.md"),
        "{report}"
    );
    let notices = report["notices"].as_array().unwrap();
    assert!(
        notices.iter().any(|n| n["subject"] == "stale-pins"
            && n["message"].as_str().unwrap().contains("fix --rebaseline")),
        "{report}"
    );
    let (_, applied) = run(&dir, &["migrate", "filenames", "--apply"]);
    let text = std::fs::read_to_string(dir.join("docs/adr/ADR-0003-pinner.md")).unwrap();
    assert!(
        text.contains("code:docs/adr/ADR-0001-first.md@sha256:0123456789ab"),
        "{text}"
    );
    assert_eq!(applied["stale_pins"].as_array().unwrap().len(), 2);
}
