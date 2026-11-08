//! ADR-97 amendment of 2026-11-08: a global `--as-of YYYY-MM-DD` fixes the
//! evaluation date every comparison reads, and the report echoes it as `as_of`.

mod common;

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

fn git(dir: &Path, args: &[&str]) {
    assert!(Command::new("git")
        .args(["-c", "user.email=t@t", "-c", "user.name=t"])
        .args(args)
        .current_dir(dir)
        .status()
        .unwrap()
        .success());
}

fn urzua(dir: &Path, args: &[&str]) -> Output {
    common::urzua()
        .args(args)
        .current_dir(dir)
        .output()
        .unwrap()
}

fn json(out: &Output) -> serde_json::Value {
    serde_json::from_slice(&out.stdout).unwrap_or_else(|e| panic!("not JSON ({e}): {out:?}"))
}

/// A UTC calendar date `offset` days from today, via `date`.
fn day(offset: i64) -> String {
    let out = Command::new("date")
        .args(["-u", "-d", &format!("{offset} days"), "+%Y-%m-%d"])
        .output()
        .unwrap();
    String::from_utf8(out.stdout).unwrap().trim().to_string()
}

const WAIVER_CONFIG: &str = "schema_version: 2
rules:
  header.required-fields: error
record_types:
  adr:
    dir: docs/adr
    required_fields: [Status, Author]
    header_shape: yaml-frontmatter
  waiver:
    dir: docs/waiver
    required_fields: [Rule, Scope]
    known_fields: [Expires]
    header_shape: yaml-frontmatter
";

/// ADR-1 lacks Author; a waiver for it expires on `expires`.
fn waiver_corpus(name: &str, expires: &str) -> common::TempDir {
    let dir = common::TempDir::new(&format!("urzua-asof-{name}"));
    for d in ["docs/adr", "docs/waiver", ".urzua"] {
        std::fs::create_dir_all(dir.join(d)).unwrap();
    }
    std::fs::write(dir.join(".urzua/config.yaml"), WAIVER_CONFIG).unwrap();
    std::fs::write(
        dir.join("docs/adr/ADR-1-a.md"),
        "---\nStatus: Accepted\n---\n# 1 - A\n",
    )
    .unwrap();
    std::fs::write(
        dir.join("docs/waiver/WAIVER-1-a.md"),
        format!(
            "---\nRule: header.required-fields\nScope: docs/adr/ADR-1-a.md\nExpires: {expires}\n---\n# Waiver\n"
        ),
    )
    .unwrap();
    git(&dir, &["init", "-q"]);
    git(&dir, &["add", "-A"]);
    git(&dir, &["commit", "-q", "-m", "base"]);
    dir
}

fn unwaived_required_fields(report: &serde_json::Value) -> usize {
    report["findings"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|f| f["rule"] == "header.required-fields" && f["waived"].is_null())
        .count()
}

#[test]
fn check_as_of_past_a_waivers_expiry_reports_what_plain_check_does_not_observed_failing() {
    let expires = day(30);
    let dir = waiver_corpus("expiry", &expires);

    let plain = json(&urzua(&dir, &["check"]));
    assert_eq!(unwaived_required_fields(&plain), 0, "waived today: {plain}");

    let later = day(60);
    let out = urzua(&dir, &["check", "--as-of", &later]);
    let preview = json(&out);
    assert_eq!(
        unwaived_required_fields(&preview),
        1,
        "lapsed by {later}: {preview}"
    );
    assert_eq!(out.status.code(), Some(1), "{out:?}");
    assert_eq!(
        preview["as_of"],
        serde_json::json!({"date": later, "source": "flag"})
    );

    // The last day it is still active.
    let edge = json(&urzua(&dir, &["check", "--as-of", &expires]));
    assert_eq!(unwaived_required_fields(&edge), 0, "{edge}");
}

#[test]
fn every_dated_report_echoes_as_of_with_its_source() {
    let waivers = waiver_corpus("echo", &day(30));
    let claims = claim_corpus("echo");
    let today = day(0);
    for args in [vec!["check"], vec!["audit"], vec!["fix"], vec!["reverify"]] {
        let dir = if args[0] == "check" || args[0] == "audit" {
            &waivers
        } else {
            &claims
        };
        let report = json(&urzua(dir, &args));
        assert_eq!(
            report["as_of"],
            serde_json::json!({"date": today, "source": "clock"}),
            "{args:?}: {report}"
        );
        let mut with = args.clone();
        with.extend(["--as-of", "2030-01-02"]);
        let report = json(&urzua(dir, &with));
        assert_eq!(
            report["as_of"],
            serde_json::json!({"date": "2030-01-02", "source": "flag"}),
            "{with:?}: {report}"
        );
    }
    // `--as-of` may sit before the subcommand: it is global.
    let report = json(&urzua(&waivers, &["--as-of", "2030-01-02", "check"]));
    assert_eq!(report["as_of"]["date"], "2030-01-02");
}

#[test]
fn no_flag_leaves_the_report_unchanged_apart_from_the_echoed_field() {
    let dir = waiver_corpus("same", &day(30));
    let plain = json(&urzua(&dir, &["check"]));
    let same_day = json(&urzua(&dir, &["check", "--as-of", &day(0)]));
    let mut a = plain.clone();
    let mut b = same_day.clone();
    assert_eq!(a["as_of"]["source"], "clock");
    assert_eq!(b["as_of"]["source"], "flag");
    a.as_object_mut().unwrap().remove("as_of");
    b.as_object_mut().unwrap().remove("as_of");
    assert_eq!(a, b);
}

#[test]
fn an_invalid_date_is_a_structured_could_not_run_with_exit_2() {
    let dir = waiver_corpus("invalid", &day(30));
    for bad in [
        "tomorrow",
        "2026-13-01",
        "2026-02-30",
        "2026-1-5",
        "20261105",
        "",
        "2026-11-08T00:00:00Z",
    ] {
        let out = urzua(&dir, &["check", "--as-of", bad]);
        assert_eq!(out.status.code(), Some(2), "{bad:?}: {out:?}");
        let v = json(&out);
        assert_eq!(v["status"], "not-run", "{bad:?}: {v}");
        assert!(v["error"].as_str().unwrap().contains("--as-of"), "{v}");
    }
}

#[test]
fn a_write_is_refused_with_as_of_and_nothing_is_written() {
    let dir = waiver_corpus("write", &day(30));
    let before = std::fs::read_to_string(dir.join("docs/adr/ADR-1-a.md")).unwrap();
    let cases: [&[&str]; 2] = [
        &[
            "fix",
            "--apply",
            "--force",
            "--by",
            "x",
            "--as-of",
            "2030-01-02",
        ],
        &[
            "reverify",
            "--apply",
            "--force",
            "--by",
            "x",
            "--reason",
            "r",
            "--as-of",
            "2030-01-02",
        ],
    ];
    for args in cases {
        let out = urzua(&dir, args);
        assert_eq!(out.status.code(), Some(2), "{args:?}: {out:?}");
        let v = json(&out);
        assert_eq!(v["status"], "not-run", "{v}");
        assert!(v["error"].as_str().unwrap().contains("--apply"), "{v}");
    }
    assert_eq!(
        std::fs::read_to_string(dir.join("docs/adr/ADR-1-a.md")).unwrap(),
        before
    );
}

#[test]
fn a_command_that_compares_no_dates_refuses_the_flag_instead_of_ignoring_it() {
    let dir = waiver_corpus("nodates", &day(30));
    for args in [
        vec!["rules"],
        vec!["graph"],
        vec!["doctor"],
        vec!["export", "--format", "json"],
        vec!["new", "adr", "T"],
    ] {
        let mut full = args.clone();
        full.extend(["--as-of", "2030-01-02"]);
        let out = urzua(&dir, &full);
        assert_eq!(out.status.code(), Some(2), "{full:?}: {out:?}");
        let v = json(&out);
        assert_eq!(v["status"], "not-run", "{v}");
        assert!(
            v["error"]
                .as_str()
                .unwrap()
                .contains("--as-of is not accepted"),
            "{v}"
        );
    }
}

// ---- claim ages -----------------------------------------------------------

fn date_days_ago(n: u32) -> String {
    let out = Command::new("date")
        .args(["-u", "-d", &format!("{n} days ago"), "+%Y-%m-%dT12:00:00Z"])
        .output()
        .unwrap();
    String::from_utf8(out.stdout).unwrap().trim().to_string()
}

fn claim_corpus(name: &str) -> common::TempDir {
    let cfg = "schema_version: 2
rules:
  embodiment.consistency: warn
  embodiment.claim-expired:
    level: warn
    anchor_field: Last-audited
    periods:
      - {type: milestone, status: [Done], days: 30}
record_types:
  milestone:
    dir: docs/milestones
    prefix: MILE
    header_shape: yaml-frontmatter
    required_fields: [Status]
    known_fields: [Embodiment, Realized-by, Last-audited, Stable-Id]
    statuses:
      values: [Planned, Done, Superseded]
      success: [Done]
      claim_retained: [Superseded]
";
    let dir = common::TempDir::new(&format!("urzua-asof-claim-{name}"));
    for d in ["docs/milestones", "src", ".urzua"] {
        std::fs::create_dir_all(dir.join(d)).unwrap();
    }
    std::fs::write(dir.join(".urzua/config.yaml"), cfg).unwrap();
    std::fs::write(dir.join("src/a.rs"), "fn a() {}\n").unwrap();
    std::fs::write(
        dir.join("docs/milestones/MILE-1-a.md"),
        "---\nStable-Id: 01AAAAAAAAAAAAAAAAAAAAAAAA\nStatus: Done\nRealized-by: code:src/a.rs\n---\n# MILE-1: T\n\n> **Revision log**\n>\n> | Date | Change | Class |\n> |---|---|---|\n> | 2026-01-01 | Initial. | **structural** |\n",
    )
    .unwrap();
    git(&dir, &["init", "-q"]);
    git(&dir, &["add", "-A"]);
    let date = date_days_ago(10);
    let status = Command::new("git")
        .args([
            "-c",
            "user.email=t@t",
            "-c",
            "user.name=t",
            "commit",
            "-q",
            "-m",
            "fixture",
        ])
        .env("GIT_AUTHOR_DATE", &date)
        .env("GIT_COMMITTER_DATE", &date)
        .current_dir(&dir)
        .status()
        .unwrap();
    assert!(status.success());

    dir
}

#[test]
fn claim_ages_are_measured_against_as_of() {
    let dir = claim_corpus("ages");

    let expired = |report: &serde_json::Value| {
        report["findings"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|f| f["rule"] == "embodiment.claim-expired")
            .cloned()
            .collect::<Vec<_>>()
    };
    let plain = json(&urzua(&dir, &["check"]));
    assert!(expired(&plain).is_empty(), "10 days of 30: {plain}");

    let later = day(25);
    let preview = json(&urzua(&dir, &["check", "--as-of", &later]));
    let found = expired(&preview);
    assert_eq!(found.len(), 1, "35 days of 30 as of {later}: {preview}");
    assert!(
        found[0]["message"].as_str().unwrap().contains("35 days"),
        "{}",
        found[0]
    );

    // The review report measures to the same date.
    let review = json(&urzua(&dir, &["reverify", "--as-of", &later]));
    assert_eq!(review["as_of"]["date"], later);
    assert_eq!(review["records"].as_array().unwrap().len(), 1, "{review}");
}

// ---- one clock ------------------------------------------------------------

fn rust_files(dir: &Path, out: &mut Vec<PathBuf>) {
    for entry in std::fs::read_dir(dir).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() {
            rust_files(&path, out);
        } else if path.extension().is_some_and(|e| e == "rs") {
            out.push(path);
        }
    }
}

/// Only one call site reads the wall clock, and it feeds the evaluation date.
/// A second reader would let a verdict depend on a date `--as-of` cannot move.
#[test]
fn the_wall_clock_is_read_in_one_place() {
    let crates = Path::new(env!("CARGO_MANIFEST_DIR")).join("..");
    let mut files = Vec::new();
    for krate in std::fs::read_dir(&crates).unwrap() {
        let src = krate.unwrap().path().join("src");
        if src.is_dir() {
            rust_files(&src, &mut files);
        }
    }
    let needles = [
        "Utc::now",
        "Local::now",
        "SystemTime::now",
        "OffsetDateTime::now",
    ];
    let mut hits = Vec::new();
    for file in files {
        let text = std::fs::read_to_string(&file).unwrap();
        // Production code only: a `#[cfg(test)]` module runs to the end of the
        // file in this codebase.
        let production = text.split("#[cfg(test)]").next().unwrap();
        for (n, line) in production.lines().enumerate() {
            let code = line.split("//").next().unwrap();
            if needles.iter().any(|needle| code.contains(needle)) {
                hits.push(format!("{}:{}", file.display(), n + 1));
            }
        }
    }
    assert_eq!(hits.len(), 1, "wall-clock reads: {hits:?}");
    assert!(hits[0].contains("urzua-io"), "{hits:?}");
}
