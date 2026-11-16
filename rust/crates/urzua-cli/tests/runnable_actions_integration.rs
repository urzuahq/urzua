//! `MILE-266` and `MILE-267`: every `next_action` and every `run-command`
//! action the engine prints runs as printed, under a `gh` that verifies a
//! login (the identity that makes `fix --apply` demand `--actor`); a new
//! record is staged before it is examined; a changed pinned file leads to the
//! re-pin, never to a written drift state; and a record from each built-in
//! profile takes a tool-written revision row.
//!
//! Verifies: SPEC-2, SPEC-8

mod common;

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use common::{commit_all, git};

/// A directory holding a `gh` that verifies the login `maintainer` and an
/// `urzua` link to the binary under test, so a printed command runs through a
/// shell exactly as an agent would run it.
fn bin_dir() -> PathBuf {
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR"))
        .join(format!("runnable-actions-bin-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let gh = dir.join("gh");
    if !gh.exists() {
        use std::os::unix::fs::PermissionsExt;
        std::fs::write(&gh, "#!/bin/sh\necho maintainer\n").unwrap();
        std::fs::set_permissions(&gh, std::fs::Permissions::from_mode(0o755)).unwrap();
    }
    let link = dir.join("urzua");
    if !link.exists() {
        let _ = std::os::unix::fs::symlink(env!("CARGO_BIN_EXE_urzua"), &link);
    }
    dir
}

fn path_env() -> std::ffi::OsString {
    let mut path = bin_dir().into_os_string();
    path.push(":");
    path.push(std::env::var_os("PATH").unwrap_or_default());
    path
}

/// Runs `command` through `sh` in `dir`.
fn sh(dir: &Path, command: &str) -> Output {
    Command::new("sh")
        .arg("-c")
        .arg(command)
        .env("PATH", path_env())
        .current_dir(dir)
        .output()
        .expect("sh should run")
}

fn json(out: &Output) -> serde_json::Value {
    serde_json::from_slice(&out.stdout).unwrap_or_else(|e| {
        panic!(
            "stdout is not JSON ({e}): {}{}",
            String::from_utf8_lossy(&out.stdout),
            String::from_utf8_lossy(&out.stderr)
        )
    })
}

fn urzua(dir: &Path, args: &str) -> serde_json::Value {
    json(&sh(dir, &format!("urzua {args}")))
}

/// The command a report's `next_action` names with each declared input
/// replaced by a test value; `None` when it has none. An undeclared `<...>`
/// word fails the test: a placeholder must be listed in `next_action_inputs`.
fn filled_next_action(report: &serde_json::Value) -> Option<String> {
    let mut command = report["next_action"].as_str()?.to_string();
    let inputs = report["next_action_inputs"]
        .as_array()
        .unwrap_or_else(|| panic!("next_action_inputs is always an array: {report}"));
    for input in inputs {
        let placeholder = input["placeholder"].as_str().unwrap();
        assert!(input["flag"].as_str().is_some_and(|f| f.starts_with("--")));
        assert!(input["means"].as_str().is_some_and(|m| !m.is_empty()));
        assert!(command.contains(placeholder), "{report}");
        command = command.replace(
            &format!("'{placeholder}'"),
            "'the test changed the cited file'",
        );
    }
    assert!(
        !command.contains('<'),
        "a placeholder not declared in next_action_inputs: {command}"
    );
    Some(command)
}

/// Runs a printed command and asserts it was not refused as a usage error
/// (`not-run`, exit 2); returns its report.
fn runs(dir: &Path, command: &str) -> serde_json::Value {
    let out = sh(dir, command);
    if command.starts_with("git ") {
        assert!(out.status.success(), "{command}: {out:?}");
        return serde_json::Value::Null;
    }
    assert_ne!(
        out.status.code(),
        Some(2),
        "`{command}` did not run: {}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    let report = json(&out);
    assert_ne!(report["status"], "not-run", "`{command}`: {report}");
    report
}

/// Follows `next_action` from `start` until none is left, running each one;
/// returns the commands run. A chain longer than `limit` is a loop.
fn follow(dir: &Path, start: &str, limit: usize) -> Vec<String> {
    let mut ran = Vec::new();
    let mut report = runs(dir, start);
    while let Some(next) = filled_next_action(&report) {
        assert!(ran.len() < limit, "next_action loops: {ran:?}");
        ran.push(next.clone());
        report = runs(dir, &next);
        if next.starts_with("git ") {
            report = runs(dir, "urzua check");
        }
    }
    ran
}

const MAIN_CONFIG: &str = "schema_version: 2
rules:
  status.delivery-agreement: error
  embodiment.consistency: error
  index.freshness:
    level: error
    indexes:
      - {path: docs/milestones/README.md, type: milestone, format: \"- [{id}]({file}): {title}\", begin: \"<!-- begin -->\", end: \"<!-- end -->\"}
record_types:
  milestone:
    dir: docs/milestones
    prefix: MILE
    header_shape: yaml-frontmatter
    required_fields: [Status]
    known_fields: [Realized-by, Embodiment]
    statuses:
      values: [Planned, InProgress, Done, WontDo]
      success: [Done]
      failure: [WontDo]
      delivery:
        delivered_tiers: [Implemented, Verified]
        unevidenced_tiers: [\"Not started\"]
        not_done: [Planned, InProgress]
        set_status: Done
";

fn milestone(n: u32, status: &str, extra: &str) -> String {
    format!(
        "---\nStatus: {status}\n{extra}---\n# {n} - M\n\n> **Revision log**\n>\n> | Date | Change | Class |\n> |---|---|---|\n> | 2026-11-04 | Filed. | **substantive** |\n"
    )
}

fn sha256(content: &str) -> String {
    use std::io::Write;
    let mut child = Command::new("sha256sum")
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .spawn()
        .unwrap();
    child
        .stdin
        .take()
        .unwrap()
        .write_all(content.as_bytes())
        .unwrap();
    String::from_utf8(child.wait_with_output().unwrap().stdout)
        .unwrap()
        .split_whitespace()
        .next()
        .unwrap()
        .to_string()
}

fn repo(name: &str) -> common::TempDir {
    let dir = common::TempDir::new(&format!("urzua-runnable-{name}"));
    for d in ["docs/milestones", "src", ".urzua"] {
        std::fs::create_dir_all(dir.join(d)).unwrap();
    }
    git(&dir, &["init", "-q"]);
    dir
}

/// One repository with a finding of each `run-command` variant of
/// `embodiment.consistency`, `embodiment.pin-stale`,
/// `status.delivery-agreement` and `index.freshness`.
fn main_fixture(name: &str) -> common::TempDir {
    let dir = repo(name);
    std::fs::write(dir.join(".urzua/config.yaml"), MAIN_CONFIG).unwrap();
    for f in ["a", "b", "c"] {
        std::fs::write(dir.join(format!("src/{f}.rs")), "fn a() {}\n").unwrap();
    }
    let pin = sha256("fn a() {}\n");
    let records = [
        (1, milestone(1, "Planned", "Realized-by: code:src/a.rs\n")),
        (
            2,
            milestone(
                2,
                "InProgress",
                "Embodiment: Specified\nRealized-by: code:src/a.rs\n",
            ),
        ),
        (
            3,
            milestone(
                3,
                "Done",
                &format!("Embodiment: Implemented\nRealized-by: code:src/b.rs@sha256:{pin}\n"),
            ),
        ),
        (
            4,
            milestone(
                4,
                "InProgress",
                "Embodiment: Implemented\nRealized-by: code:src/c.rs\n",
            ),
        ),
    ];
    for (n, text) in &records {
        std::fs::write(dir.join(format!("docs/milestones/MILE-{n}-m.md")), text).unwrap();
    }
    std::fs::write(
        dir.join("docs/milestones/README.md"),
        "# Milestones\n\n<!-- begin -->\n- [MILE-1](MILE-1-m.md): M\n<!-- end -->\n",
    )
    .unwrap();
    commit_all(&dir, 3);
    for f in ["b", "c"] {
        std::fs::write(dir.join(format!("src/{f}.rs")), "fn a() { }\n").unwrap();
    }
    commit_all(&dir, 1);
    dir
}

const CLAIM_CONFIG: &str = "schema_version: 2
rules:
  embodiment.claim-expired:
    level: warn
    anchor_field: Last-audited
    require_pin: [code]
    periods:
      - {type: milestone, status: [Done], days: 30}
record_types:
  milestone:
    dir: docs/milestones
    prefix: MILE
    header_shape: yaml-frontmatter
    required_fields: [Status]
    known_fields: [Embodiment, Realized-by, Last-audited]
    statuses:
      values: [Planned, Done]
      success: [Done]
";

fn claim_fixture(name: &str) -> common::TempDir {
    let dir = repo(name);
    std::fs::write(dir.join(".urzua/config.yaml"), CLAIM_CONFIG).unwrap();
    std::fs::write(dir.join("src/a.rs"), "fn a() {}\n").unwrap();
    std::fs::write(
        dir.join("docs/milestones/MILE-1-m.md"),
        milestone(1, "Done", "Realized-by: code:src/a.rs\n"),
    )
    .unwrap();
    commit_all(&dir, 40);
    dir
}

fn outdated_schema_fixture(name: &str) -> common::TempDir {
    let dir = repo(name);
    std::fs::write(
        dir.join(".urzua/config.yaml"),
        "schema_version: 1\nrecord_types:\n  milestone:\n    dir: docs/milestones\n    header_shape: yaml-frontmatter\n    required_fields: [Status]\n",
    )
    .unwrap();
    std::fs::write(
        dir.join("docs/milestones/MILE-1-m.md"),
        milestone(1, "Planned", ""),
    )
    .unwrap();
    commit_all(&dir, 1);
    dir
}

/// The `run-command` variants the fixtures produce: the variant, its rule and
/// the argv prefix its action carries.
const COVERED: &[(&str, &str, &[&str])] = &[
    (
        "embodiment-drift",
        "embodiment.consistency",
        &["urzua", "fix", "--apply"],
    ),
    (
        "embodiment-drift-pinned",
        "embodiment.consistency",
        &["urzua", "fix", "--rebaseline"],
    ),
    (
        "pin-stale",
        "embodiment.pin-stale",
        &["urzua", "fix", "--rebaseline"],
    ),
    (
        "delivery-not-done",
        "status.delivery-agreement",
        &["urzua", "fix", "--apply"],
    ),
    ("index-stale", "index.freshness", &["urzua", "index"]),
    ("claim-expired", "embodiment.claim-expired", &["urzua", "reverify"]),
    (
        "schema-version-outdated",
        "config.schema-version-outdated",
        &["urzua", "migrate"],
    ),
];

/// `run-command` variants no fixture here produces, each with why.
const EXEMPT: &[(&str, &str)] = &[(
    "delivery-partial-not-started",
    "needs a Done-means items declaration; its action is built by the same code as delivery-not-done (`urzua fix --apply --ids <record> --actor agent`), and done_items_integration runs it",
)];

/// Every finding of `report` that offers a `run-command` action, as
/// (rule, argv).
fn offered(report: &serde_json::Value) -> Vec<(String, Vec<String>)> {
    report["findings"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|f| f["suggested_action"]["kind"] == "run-command")
        .map(|f| {
            let argv = f["suggested_action"]["target"]["argv"]
                .as_array()
                .unwrap()
                .iter()
                .map(|a| a.as_str().unwrap().to_string())
                .collect();
            (f["rule"].as_str().unwrap().to_string(), argv)
        })
        .collect()
}

fn shell_line(argv: &[String]) -> String {
    argv.iter()
        .map(|w| format!("'{}'", w.replace('\'', "'\\''")))
        .collect::<Vec<_>>()
        .join(" ")
}

#[test]
fn every_run_command_variant_is_covered_by_a_fixture_or_exempt_with_a_reason() {
    let dir = repo("coverage");
    let rules = urzua(&dir, "rules");
    let declared: BTreeSet<String> = rules["rules"]
        .as_array()
        .unwrap()
        .iter()
        .flat_map(|r| r["actions"].as_array().cloned().unwrap_or_default())
        .filter(|a| a["kind"] == "run-command")
        .map(|a| a["variant"].as_str().unwrap().to_string())
        .collect();
    let known: BTreeSet<String> = COVERED
        .iter()
        .map(|(v, _, _)| v.to_string())
        .chain(EXEMPT.iter().map(|(v, _)| v.to_string()))
        .collect();
    assert_eq!(
        declared, known,
        "a run-command variant needs a fixture in COVERED or a reason in EXEMPT"
    );
    assert!(EXEMPT.iter().all(|(_, why)| !why.is_empty()));
}

#[test]
fn every_suggested_action_and_every_next_action_runs_as_printed() {
    let fixtures: [(&str, fn(&str) -> common::TempDir); 3] = [
        ("main", main_fixture),
        ("claim", claim_fixture),
        ("schema", outdated_schema_fixture),
    ];
    let mut seen: BTreeSet<&str> = BTreeSet::new();
    for (name, build) in fixtures {
        let dir = build(name);
        let report = urzua(&dir, "check");
        let actions = offered(&report);
        for (variant, rule, prefix) in COVERED {
            let hit = actions.iter().any(|(r, argv)| {
                r == rule
                    && argv.len() >= prefix.len()
                    && argv.iter().zip(prefix.iter()).all(|(a, p)| a == p)
            });
            if hit {
                seen.insert(variant);
            }
        }
        // Each action, run on a fresh copy of the fixture, then the chain of
        // next actions its report leads to.
        for (i, (rule, argv)) in actions.iter().enumerate() {
            assert!(
                argv.iter().all(|w| !w.starts_with('<')),
                "{rule} offers a placeholder in its argv: {argv:?}"
            );
            let fresh = build(&format!("{name}-{i}"));
            follow(&fresh, &shell_line(argv), 8);
        }
        // The check's own next actions, followed to the end.
        follow(&dir, "urzua check", 12);
        let after = urzua(&dir, "check");
        assert!(
            after["next_action"].is_null(),
            "the chain ends with nothing left to run: {after}"
        );
    }
    let covered: BTreeSet<&str> = COVERED.iter().map(|(v, _, _)| *v).collect();
    assert_eq!(seen, covered, "every covered variant appears in a fixture");
}

#[test]
fn a_fix_apply_as_printed_runs_under_a_verified_gh_login() {
    let dir = main_fixture("verified");
    let report = urzua(&dir, "check");
    let next = report["next_action"].as_str().unwrap();
    assert!(next.starts_with("urzua fix --apply --ids "), "{report}");
    assert!(next.ends_with(" --actor agent"), "{next}");
    assert_eq!(report["next_action_inputs"], serde_json::json!([]));
    let applied = runs(&dir, next);
    assert_eq!(applied["status"], "applied", "{applied}");
    assert_eq!(applied["identity"]["source"], "gh-verified", "{applied}");
    assert_eq!(applied["identity"]["actor"], "agent", "{applied}");
}

#[test]
fn an_error_with_a_known_remedy_names_it_and_the_remedy_runs() {
    let dir = main_fixture("errors");
    let out = sh(&dir, "urzua check --rules embodiment.consistancy");
    assert_eq!(out.status.code(), Some(2));
    let report = json(&out);
    assert_eq!(
        report["next_action"], "urzua check --rules embodiment.consistency",
        "{report}"
    );
    runs(&dir, report["next_action"].as_str().unwrap());

    let report = urzua(&dir, "explain --rule embodiment.consistancy");
    assert_eq!(
        report["next_action"], "urzua explain --rule embodiment.consistency",
        "{report}"
    );
    runs(&dir, report["next_action"].as_str().unwrap());

    let report = urzua(&dir, "check --base origin/nosuch");
    assert!(
        report["error"]
            .as_str()
            .unwrap()
            .contains("\"origin/nosuch\" names no commit"),
        "{report}"
    );
    assert_eq!(report["next_action"], "git fetch origin nosuch", "{report}");
}

#[test]
fn a_new_record_is_reported_untracked_until_staged_then_examined() {
    let dir = repo("untracked");
    git(&dir, &["commit", "-q", "--allow-empty", "-m", "init"]);
    runs(&dir, "urzua init --types adr");
    commit_all(&dir, 0);
    let made = urzua(&dir, "new adr \"x\"");
    let path = made["path"].as_str().unwrap().to_string();
    assert_eq!(made["next_action"], format!("git add -- {path}"), "{made}");

    let report = urzua(&dir, "check");
    assert_eq!(report["files_examined"], 0, "{report}");
    assert!(
        report["notices"]
            .as_array()
            .unwrap()
            .iter()
            .any(|n| n["subject"] == "discovery.untracked-skipped"
                && n["message"].as_str().unwrap().contains(&path)),
        "the report says the record is untracked: {report}"
    );
    let next = report["next_action"].as_str().unwrap();
    assert_eq!(next, format!("git add -- {path}"), "{report}");
    runs(&dir, next);
    let report = urzua(&dir, "check");
    assert_eq!(report["files_examined"], 1, "{report}");
}

#[test]
fn a_changed_pinned_file_leads_to_the_repin_and_cannot_go_green_without_one() {
    let dir = main_fixture("drift");
    let record = "docs/milestones/MILE-3-m.md";
    let before = std::fs::read_to_string(dir.join(record)).unwrap();

    // Every other repair applied first, so only the drift is left.
    runs(
        &dir,
        "urzua fix --apply --ids docs/milestones/MILE-1-m.md,docs/milestones/MILE-2-m.md,docs/milestones/MILE-4-m.md --actor agent",
    );
    runs(&dir, "urzua index --apply");
    let report = urzua(&dir, "check");
    assert_eq!(
        report["next_action"],
        format!("urzua fix --rebaseline --ids {record}"),
        "{report}"
    );
    for f in report["findings"].as_array().unwrap() {
        if f["file"] == record {
            assert_eq!(
                f["suggested_action"]["target"]["argv"],
                serde_json::json!(["urzua", "fix", "--rebaseline", "--ids", record]),
                "{f}"
            );
        }
    }

    // `fix --apply`, named or forced, writes no drift state over the stale pin.
    let refused = json(&sh(
        &dir,
        &format!("urzua fix --apply --ids {record} --actor agent"),
    ));
    assert!(
        refused["failed"][0]["error"]
            .as_str()
            .unwrap()
            .contains("fix --rebaseline"),
        "{refused}"
    );
    runs(&dir, "urzua fix --apply --force --actor agent");
    assert_eq!(std::fs::read_to_string(dir.join(record)).unwrap(), before);
    let report = urzua(&dir, "check");
    assert_ne!(report["status"], "ok", "{report}");

    // The listing's next action is the attested re-pin, with its reason to fill.
    let listing = urzua(&dir, &format!("fix --rebaseline --ids {record}"));
    let inputs = listing["next_action_inputs"].as_array().unwrap();
    assert_eq!(inputs.len(), 1, "{listing}");
    assert_eq!(inputs[0]["flag"], "--reason");
    let repin = filled_next_action(&listing).unwrap();
    assert!(repin.contains("--actor agent"), "{repin}");
    runs(&dir, &repin);
    let report = urzua(&dir, "check");
    assert_eq!(report["status"], "ok", "{report}");
    assert!(std::fs::read_to_string(dir.join(record))
        .unwrap()
        .contains("Re-baselined"));
}

/// A greenfield repository with one built-in profile, a record made by
/// `urzua new`, and the evidence fields declared: the inputs an adopter adds,
/// never a hand edit of the revision log.
fn profile_repo(profile: &str) -> (common::TempDir, String) {
    let dir = repo(&format!("profile-{profile}"));
    git(&dir, &["commit", "-q", "--allow-empty", "-m", "init"]);
    runs(&dir, &format!("urzua init --types {profile}"));
    let made = urzua(&dir, &format!("new {profile} \"x\""));
    let path = made["path"].as_str().unwrap().to_string();
    let config_path = dir.join(".urzua/config.yaml");
    let config = std::fs::read_to_string(&config_path)
        .unwrap()
        .replace("    known_fields: []", "    known_fields: [Embodiment, Realized-by]")
        .replace("rules:\n", "rules:\n  embodiment.consistency: warn\n");
    std::fs::write(&config_path, config).unwrap();
    std::fs::write(dir.join("src/a.rs"), "fn a() {}\n").unwrap();
    let record = std::fs::read_to_string(dir.join(&path)).unwrap().replacen(
        "\n---\n",
        "\nEmbodiment: Specified\nRealized-by: code:src/a.rs\n---\n",
        1,
    );
    std::fs::write(dir.join(&path), record).unwrap();
    commit_all(&dir, 0);
    (dir, path)
}

fn log_rows(text: &str) -> usize {
    let at = text.find("**Revision log**").expect("a revision log");
    text[at..]
        .lines()
        .filter(|l| l.trim_start_matches(['>', ' ']).starts_with("| 20"))
        .count()
}

#[test]
fn a_record_from_each_built_in_profile_takes_fix_apply_with_no_hand_edit() {
    for profile in ["adr", "rfc", "spec", "prd"] {
        let (dir, path) = profile_repo(profile);
        let text = std::fs::read_to_string(dir.join(&path)).unwrap();
        assert_eq!(log_rows(&text), 1, "{profile} starts with one dated row: {text}");
        assert!(!text.contains("YYYY-MM-DD"), "{text}");

        let applied = runs(&dir, &format!("urzua fix --apply --ids {path} --actor agent"));
        assert_eq!(applied["status"], "applied", "{profile}: {applied}");
        let text = std::fs::read_to_string(dir.join(&path)).unwrap();
        assert_eq!(log_rows(&text), 2, "{profile}: {text}");
        assert!(text.contains("Embodiment: Implemented"), "{text}");
    }
}

#[test]
fn an_empty_revision_log_table_receives_its_first_row() {
    let (dir, path) = profile_repo("adr");
    let text = std::fs::read_to_string(dir.join(&path)).unwrap();
    let emptied: String = text
        .lines()
        .filter(|l| !l.contains("Initial record."))
        .map(|l| format!("{l}\n"))
        .collect();
    std::fs::write(dir.join(&path), &emptied).unwrap();
    commit_all(&dir, 0);
    assert_eq!(log_rows(&emptied), 0);

    let applied = runs(&dir, &format!("urzua fix --apply --ids {path} --actor agent"));
    assert_eq!(applied["status"], "applied", "{applied}");
    let text = std::fs::read_to_string(dir.join(&path)).unwrap();
    assert_eq!(log_rows(&text), 1, "{text}");
    let row = text
        .lines()
        .find(|l| l.contains("Tool-authored"))
        .expect("the tool row");
    assert!(row.starts_with("> | "), "inside the blockquote table: {row}");
}
