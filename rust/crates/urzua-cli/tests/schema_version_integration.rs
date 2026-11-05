//! `ADR-77` end to end: the support window, the in-memory upgrade, the
//! structured out-of-window error and `urzua migrate config`. The fixtures are
//! the acceptance corpus's own (`acceptance/corpora/corpus-k`), one per
//! supported schema version.

use std::path::{Path, PathBuf};
use std::process::Command;

fn corpus() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/acceptance/corpora/corpus-k")
}

fn git(dir: &Path, args: &[&str]) {
    assert!(Command::new("git")
        .args(args)
        .current_dir(dir)
        .status()
        .expect("git should run")
        .success());
}

fn copy_dir(from: &Path, to: &Path) {
    std::fs::create_dir_all(to).unwrap();
    for entry in std::fs::read_dir(from).unwrap().flatten() {
        let (src, dst) = (entry.path(), to.join(entry.file_name()));
        if src.is_dir() {
            copy_dir(&src, &dst);
        } else {
            std::fs::copy(&src, &dst).unwrap();
        }
    }
}

/// A git repository holding corpus-k's records and configs.
fn repo(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("urzua-schema-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    copy_dir(&corpus().join("records"), &dir.join("records"));
    copy_dir(&corpus().join(".urzua"), &dir.join(".urzua"));
    git(&dir, &["init", "-q"]);
    git(&dir, &["config", "user.email", "test@example.com"]);
    git(&dir, &["config", "user.name", "test"]);
    git(&dir, &["add", "-A"]);
    git(&dir, &["commit", "-q", "-m", "fixture"]);
    dir
}

fn urzua(dir: &Path, args: &[&str]) -> (i32, serde_json::Value) {
    let out = Command::new(env!("CARGO_BIN_EXE_urzua"))
        .args(args)
        .current_dir(dir)
        .output()
        .expect("urzua binary should run");
    let stdout = String::from_utf8_lossy(&out.stdout);
    let json = serde_json::from_str(&stdout).unwrap_or_else(|e| panic!("not JSON ({e}): {stdout}"));
    (out.status.code().unwrap(), json)
}

fn load(file: &str) -> urzua_core::config::Config {
    let text = std::fs::read_to_string(corpus().join(".urzua").join(file)).unwrap();
    urzua_core::config::parse(&text).unwrap_or_else(|e| panic!("{file}: {e}"))
}

#[test]
fn the_fixture_at_each_supported_version_loads_to_the_same_effective_config() {
    let previous = load("config-v1.yaml");
    let current = load("config.yaml");
    assert_eq!(previous.upgraded_from.as_ref().map(|u| u.from), Some(1));
    assert!(current.upgraded_from.is_none());
    assert_eq!(
        serde_json::to_value(&previous).unwrap(),
        serde_json::to_value(&current).unwrap(),
        "the version-1 fixture, upgraded in memory, must be the version-2 fixture"
    );
}

#[test]
fn the_rewritten_file_loads_to_the_same_effective_config_as_the_upgrade() {
    let dir = repo("rewrite-equal");
    let (code, report) = urzua(
        &dir,
        &["--config", ".urzua/config-v1.yaml", "migrate", "config"],
    );
    assert_eq!(code, 0, "{report}");
    let rewritten = std::fs::read_to_string(dir.join(".urzua/config-v1.yaml")).unwrap();
    let from_file = urzua_core::config::parse(&rewritten).unwrap();
    assert!(from_file.upgraded_from.is_none());
    assert_eq!(
        serde_json::to_value(&from_file).unwrap(),
        serde_json::to_value(load("config-v1.yaml")).unwrap()
    );
    let _ = std::fs::remove_dir_all(&dir);
}

fn findings_without_the_outdated_rule(report: &serde_json::Value) -> Vec<serde_json::Value> {
    report["findings"]
        .as_array()
        .cloned()
        .unwrap_or_default()
        .into_iter()
        .filter(|f| f["rule"] != "config.schema-version-outdated")
        .collect()
}

#[test]
fn a_previous_version_config_gives_the_findings_the_current_one_does_plus_one_warning() {
    let dir = repo("same-findings");
    // Plant one violation both configs report, so the comparison is not of two
    // empty lists.
    std::fs::write(
        dir.join("records/ADR-0003-third.md"),
        "---\nStatus: Accepted\nDerives-from: ADR-0099\n---\n# ADR-0003 — Third\n",
    )
    .unwrap();
    git(&dir, &["add", "-A"]);
    git(&dir, &["commit", "-q", "-m", "plant"]);

    let (_, current) = urzua(&dir, &["--config", ".urzua/config.yaml", "check"]);
    let (_, previous) = urzua(&dir, &["--config", ".urzua/config-v1.yaml", "check"]);
    let planted = findings_without_the_outdated_rule(&current);
    assert!(
        !planted.is_empty(),
        "the planted violation must be reported"
    );
    assert_eq!(planted, findings_without_the_outdated_rule(&previous));

    let outdated: Vec<&serde_json::Value> = previous["findings"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|f| f["rule"] == "config.schema-version-outdated")
        .collect();
    assert_eq!(outdated.len(), 1, "{previous}");
    assert_eq!(outdated[0]["severity"], "warning");
    assert_eq!(
        outdated[0]["suggested_action"]["target"]["argv"],
        serde_json::json!(["urzua", "migrate", "config"])
    );
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn check_never_rewrites_the_config() {
    let dir = repo("check-readonly");
    let before = std::fs::read_to_string(dir.join(".urzua/config-v1.yaml")).unwrap();
    urzua(&dir, &["--config", ".urzua/config-v1.yaml", "check"]);
    assert_eq!(
        before,
        std::fs::read_to_string(dir.join(".urzua/config-v1.yaml")).unwrap()
    );
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn migrate_config_keeps_comments_and_a_second_run_changes_nothing() {
    let dir = repo("migrate-twice");
    let args = ["--config", ".urzua/config-v1.yaml", "migrate", "config"];
    let (code, first) = urzua(&dir, &args);
    assert_eq!(code, 0);
    assert_eq!(first["outcome"], "migrated");
    let keys: Vec<&str> = first["changes"]
        .as_array()
        .unwrap()
        .iter()
        .map(|c| c["key"].as_str().unwrap())
        .collect();
    assert!(keys.contains(&"record_types.adr.header_layout"), "{keys:?}");
    assert!(keys.contains(&"rules.narrative-field.stale"), "{keys:?}");
    let text = std::fs::read_to_string(dir.join(".urzua/config-v1.yaml")).unwrap();
    assert!(text.starts_with("# Schema version 1:"), "{text}");
    assert!(text.contains("schema_version: 2\nrules:\n"), "{text}");
    assert!(!text.contains("header_layout:"), "{text}");

    let (code, second) = urzua(&dir, &args);
    assert_eq!(code, 0);
    assert_eq!(second["outcome"], "nothing-to-do");
    assert_eq!(second["changes"], serde_json::json!([]));
    assert_eq!(
        text,
        std::fs::read_to_string(dir.join(".urzua/config-v1.yaml")).unwrap()
    );
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn migrate_config_never_reads_a_version_outside_the_window() {
    let dir = repo("migrate-outside");
    for (file, found) in [("config-v0.yaml", 0), ("config-v3.yaml", 3)] {
        let path = format!(".urzua/{file}");
        let before = std::fs::read_to_string(dir.join(&path)).unwrap();
        let (code, report) = urzua(&dir, &["--config", &path, "migrate", "config"]);
        assert_eq!(code, 2, "{report}");
        assert_eq!(report["status"], "not-run");
        assert_eq!(report["schema_version_error"]["found"], found);
        assert_eq!(
            report["schema_version_error"]["supported"],
            serde_json::json!([1, 2])
        );
        assert_eq!(report["schema_version_error"]["current"], 2);
        assert!(report["schema_version_error"]["migration"]["note"].is_string());
        assert_eq!(before, std::fs::read_to_string(dir.join(&path)).unwrap());
    }
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn migrate_config_leaves_a_file_it_cannot_make_valid_exactly_as_it_was() {
    let dir = repo("migrate-refused");
    // A removed header shape is a load error in version 2 as well: the upgrade
    // cannot make the file valid, so it must not write it.
    let text = "schema_version: 1\nrecord_types:\n  adr:\n    dir: records\n    header_shape: blockquote\n";
    std::fs::write(dir.join(".urzua/config-bad.yaml"), text).unwrap();
    let (code, report) = urzua(
        &dir,
        &["--config", ".urzua/config-bad.yaml", "migrate", "config"],
    );
    assert_eq!(code, 2, "{report}");
    assert!(
        report["error"].as_str().unwrap().contains("unchanged"),
        "{report}"
    );
    assert_eq!(
        text,
        std::fs::read_to_string(dir.join(".urzua/config-bad.yaml")).unwrap()
    );
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn the_supported_versions_table_in_spec_3_matches_the_releases_in_code() {
    let spec = std::fs::read_to_string(
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../docs/specs/SPEC-3-configuration.md"),
    )
    .unwrap();
    let section = spec
        .split("## Supported schema versions")
        .nth(1)
        .expect("SPEC-3 has a `Supported schema versions` section");
    let table: Vec<&str> = section
        .lines()
        .skip_while(|l| !l.starts_with("| Release"))
        .skip(2)
        .take_while(|l| l.starts_with('|'))
        .collect();
    assert!(!table.is_empty(), "the table has no rows");
    for release in urzua_core::schema_version::RELEASES {
        let reads: Vec<String> = release.reads.iter().map(u32::to_string).collect();
        let row = table
            .iter()
            .find(|r| r.contains(&format!("`{}`", release.release)))
            .unwrap_or_else(|| panic!("SPEC-3's table has no row for {}", release.release));
        let cells: Vec<&str> = row.split('|').map(str::trim).collect();
        assert_eq!(
            cells[2],
            reads.join(", "),
            "versions read by {}",
            release.release
        );
        assert_eq!(
            cells[3],
            release.writes.to_string(),
            "version written by {}",
            release.release
        );
    }
    // The current release reads exactly the window.
    let window = urzua_core::schema_version::supported_versions();
    let last = urzua_core::schema_version::RELEASES.last().unwrap();
    assert_eq!(last.reads, window.as_slice());
    assert_eq!(last.writes, urzua_core::config::CURRENT_SCHEMA_VERSION);
}
