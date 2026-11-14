//! A repository that declares none of the opt-in options gets byte-for-byte the
//! output it got before each option existed.
//!
//! Every option below is opt-in: absent, the engine runs the code it ran before.
//! A per-option test asserts the behaviour it can see (a field is missing, a
//! counter is absent); this test asserts the whole report. It builds one
//! corpus whose config declares none of
//!
//! - `level: info` / `field_levels: info` (MILE-194, ADR-87),
//! - `roadmap.work_types` (MILE-196, ADR-82),
//! - `scope: changed` (MILE-203, ADR-90),
//! - `statuses.delivery.items` (MILE-214, ADR-96),
//!
//! runs `check`, `check --base`, `roadmap`, `export --format json`, `fix` and
//! `explain` over it, and compares each stdout with a golden file in
//! `tests/golden/undeclared-options/`.
//!
//! The goldens were not written by the current build. Each is the output of a
//! build of the commit before the first of these options landed (`8d676e86`, the
//! parent of the MILE-194 change `121a532c`) over this same corpus. The current engine
//! prints the same bytes for `roadmap`, `export`, `fix` and `explain`. For
//! `check` it prints the same bytes once two things that later work added for
//! every config, declared or not, are set aside by [`normalise`]: the
//! `not-enabled` entries of `rules_executed` (a rule the build ships and the
//! config does not turn on is listed, ADR-7, so each new rule adds one) and the
//! two `scope` fields of `discovery.include_untracked` (MILE-206, off and zero).
//! Every rule the config enables, and every other byte, is compared as is.
//! One golden value was changed on purpose: `check-base.json`'s `blocking` is `true`,
//! because the run exits 1 under `fail_on: warning` and `blocking` now says so
//! (SPEC-2, version 0.162).
//!
//! A golden changes only when the output of an undeclared config changes on
//! purpose, with the spec row that says so. To see what the current build says,
//! run with `URZUA_UNDECLARED_DUMP=<dir>`: the raw outputs are written there and
//! the corpus is left in `<dir>/corpus`, so any other build can be run over it.
//!
//! Verifies: MILE-194, MILE-196, MILE-203, MILE-214

mod common;

use std::path::{Path, PathBuf};
use std::process::Command;

const CONFIG: &str = "schema_version: 2
fail_on: warning
embodiment:
  drift: Drift detected
  tiers:
    - {state: Verified, evidence: test, claims_built: true}
    - {state: Implemented, evidence: code, claims_built: true}
    - {state: Specified, evidence: spec}
  none: Not started
roadmap:
  pairs: [sequencing]
relation_pairs:
  - {name: sequencing, forward: Depends-on, inverse: Blocks, reciprocity: none, cross_type: true, acyclic: true}
rules:
  header.required-fields: error
  pointer.resolution: warn
  sequence.cycle: error
  sequence.status-agreement: error
  status.delivery-agreement: warn
  embodiment.locator-exists: error
  embodiment.locator-on-back-pointer-path:
    level: warn
    back_pointer_paths: [src]
record_types:
  adr:
    dir: docs/adr
    prefix: ADR
    required_fields: [Status, Author]
    known_fields: [Embodiment, Realized-by, Depends-on, Blocks]
    pointer_fields: [Depends-on, Blocks]
    companion_fields:
      - [embodiment_state, embodiment_locator]
  milestone:
    dir: docs/milestones
    prefix: MILE
    required_fields: [Status]
    known_fields: [Realized-by, Depends-on, Blocks]
    pointer_fields: [Depends-on, Blocks]
    statuses:
      values: [Planned, InProgress, Done, WontDo]
      success: [Done]
      failure: [WontDo]
      progress: [InProgress]
      delivery:
        delivered_tiers: [Implemented, Verified]
        unevidenced_tiers: [\"Not started\"]
        not_done: [Planned, InProgress]
        set_status: Done
  task:
    dir: docs/tasks
    prefix: TASK
    required_fields: [Status]
    known_fields: [Depends-on, Blocks]
    pointer_fields: [Depends-on, Blocks]
    statuses:
      values: [Planned, Done, Dropped]
      success: [Done]
      failure: [Dropped]
      progress: [Done]
  fact:
    dir: docs/facts
    prefix: FACT
    required_fields: [Status]
    known_fields: [Depends-on, Blocks]
    pointer_fields: [Depends-on, Blocks]
    statuses:
      values: [Open, Met, Gone]
      success: [Met]
      failure: [Gone]
  waiver:
    dir: docs/waiver
    prefix: WAIVER
    role: suppression
    required_fields: [Rule, Scope, Reason]
";

/// `(path, content)`: ADR-1 is nudged by the locator rule, ADR-2 lacks `Author`
/// (waived), ADR-3 points at a record that is not there; MILE-1 is `Done` with
/// no evidence, MILE-2 is `Planned` with test evidence, MILE-3 waits on a fact;
/// TASK-2 is `Done` ahead of TASK-1; FACT-1 is open and gates TASK-3.
const RECORDS: &[(&str, &str)] = &[
    (
        "docs/adr/ADR-1-a.md",
        "---\nStatus: Accepted\nAuthor: a\nEmbodiment: Implemented\nRealized-by: code:src/a.rs\n---\n# ADR-1: A\n",
    ),
    (
        "docs/adr/ADR-2-b.md",
        "---\nStatus: Accepted\n---\n# ADR-2: B\n",
    ),
    (
        "docs/adr/ADR-3-c.md",
        "---\nStatus: Draft\nAuthor: c\nDepends-on: ADR-9\n---\n# ADR-3: C\n",
    ),
    (
        "docs/milestones/MILE-1-a.md",
        "---\nStatus: Done\n---\n# 1 \u{2014} A\n",
    ),
    (
        "docs/milestones/MILE-2-b.md",
        "---\nStatus: Planned\nRealized-by: test:tests/t.rs\n---\n# 2 \u{2014} B\n",
    ),
    (
        "docs/milestones/MILE-3-c.md",
        "---\nStatus: InProgress\nDepends-on: FACT-1, MILE-2\n---\n# 3 \u{2014} C\n",
    ),
    (
        "docs/tasks/TASK-1-a.md",
        "---\nStatus: Planned\n---\n# a\n",
    ),
    (
        "docs/tasks/TASK-2-b.md",
        "---\nStatus: Done\nDepends-on: TASK-1\n---\n# b\n",
    ),
    (
        "docs/tasks/TASK-3-c.md",
        "---\nStatus: Planned\nDepends-on: FACT-1\n---\n# c\n",
    ),
    (
        "docs/tasks/TASK-4-d.md",
        "---\nStatus: Planned\nDepends-on: FACT-2\n---\n# d\n",
    ),
    (
        "docs/facts/FACT-1-a.md",
        "---\nStatus: Open\n---\n# a\n",
    ),
    (
        "docs/facts/FACT-2-b.md",
        "---\nStatus: Met\n---\n# b\n",
    ),
    (
        "docs/waiver/WAIVER-1-x.md",
        "---\nRule: header.required-fields\nScope: docs/adr/ADR-2-b.md\nReason: the fixture needs one waived finding\n---\n# 1\n",
    ),
    ("src/a.rs", "fn a() {}\n"),
    ("tests/t.rs", "fn t() {}\n"),
];

fn git(dir: &Path, args: &[&str]) {
    let status = Command::new("git")
        .args(["-c", "user.email=t@example.com", "-c", "user.name=t"])
        .args(args)
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .current_dir(dir)
        .status()
        .expect("git should run");
    assert!(status.success(), "git {args:?} failed");
}

/// The corpus, committed twice: the second commit edits ADR-1, so
/// `--base HEAD~1` has one changed record.
fn corpus(dir: &Path) {
    std::fs::create_dir_all(dir.join(".urzua")).unwrap();
    std::fs::write(dir.join(".urzua/config.yaml"), CONFIG).unwrap();
    for (path, content) in RECORDS {
        let full = dir.join(path);
        std::fs::create_dir_all(full.parent().unwrap()).unwrap();
        std::fs::write(full, content).unwrap();
    }
    git(dir, &["init", "-q"]);
    git(dir, &["add", "-A"]);
    git(dir, &["commit", "-q", "-m", "fixture"]);
    let adr = dir.join("docs/adr/ADR-1-a.md");
    let mut text = std::fs::read_to_string(&adr).unwrap();
    text.push_str("\nA second commit touches this record.\n");
    std::fs::write(adr, text).unwrap();
    git(dir, &["add", "-A"]);
    git(dir, &["commit", "-q", "-m", "touch ADR-1"]);
}

/// `(golden file, arguments, exit code)`.
const RUNS: &[(&str, &[&str], i32)] = &[
    ("check.json", &["check"], 1),
    ("check-base.json", &["check", "--base", "HEAD~1"], 1),
    ("roadmap.json", &["roadmap"], 0),
    ("export.json", &["export", "--format", "json"], 0),
    ("fix.json", &["fix"], 0),
    ("explain.json", &["explain", "docs/adr/ADR-1-a.md"], 0),
];

/// Sets aside what later, unrelated work added to every report, declared or
/// not. Everything else, order and whitespace included, is untouched.
///
/// - `not-enabled` entries of `rules_executed`: a shipped rule the config does
///   not turn on is listed (ADR-7), so each new rule adds one. The ones the
///   golden's build lacked came from: `identity.open-pr-collision` (#388,
///   MILE-89), `status.done-items-shape` and `status.waived-item-stale` (#411,
///   MILE-214), `diff.decider-required` (#374, MILE-200). None concerns the
///   `info` level, `work_types`, `scope` or the delivery ADR-73 actions.
///   A rule the config enables (`ran`) is never set aside.
/// - `scope.include_untracked` and `scope.untracked_included`: added by #386
///   (MILE-206, `discovery.include_untracked`), `false` and `0` when undeclared.
///
/// - `as_of`: the report's evaluation date (`--as-of`, ADR-97's amendment of
///   2026-11-08), always present and the clock's date when no flag is given.
///
/// Nothing the `info` option adds (a `severity: "info"` finding, a changed
/// count or `status`) is touched: the fixture has no info finding, and one
/// would be a finding block, not a `not-enabled` entry.
fn normalise(text: &str) -> String {
    let lines: Vec<&str> = text.lines().collect();
    let mut kept: Vec<&str> = Vec::new();
    let mut i = 0;
    while i < lines.len() {
        let line = lines[i];
        let is_entry = line.trim() == "{"
            && i + 3 < lines.len()
            && lines[i + 1].trim_start().starts_with("\"rule\":")
            && lines[i + 2].trim() == "\"status\": \"not-enabled\""
            && lines[i + 3].trim_start().starts_with('}');
        if is_entry {
            i += 4;
        } else if line.trim() == "\"as_of\": {" {
            while !lines[i].trim_start().starts_with('}') {
                i += 1;
            }
            i += 1;
        } else if line.contains("\"include_untracked\":")
            || line.contains("\"untracked_included\":")
        {
            i += 1;
        } else {
            kept.push(line);
            i += 1;
        }
    }
    let mut out = String::new();
    for (n, line) in kept.iter().enumerate() {
        let closes = kept
            .get(n + 1)
            .is_some_and(|next| matches!(next.trim_start().chars().next(), Some('}' | ']')));
        let line = if closes {
            line.trim_end_matches(',')
        } else {
            line
        };
        out.push_str(line);
        out.push('\n');
    }
    out
}

fn golden_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/golden/undeclared-options")
}

#[test]
fn a_config_declaring_none_of_the_opt_in_options_reports_byte_identically_to_before() {
    let dump = std::env::var_os("URZUA_UNDECLARED_DUMP").map(PathBuf::from);
    let temp = common::TempDir::new("urzua-undeclared");
    let dir: PathBuf = match &dump {
        Some(out) => {
            std::fs::create_dir_all(out).unwrap();
            out.join("corpus")
        }
        None => temp.to_path_buf(),
    };
    corpus(&dir);

    let mut mismatched = Vec::new();
    for (file, args, code) in RUNS {
        let out = common::urzua()
            .args(*args)
            .current_dir(&dir)
            .output()
            .expect("urzua binary should run");
        let stdout = String::from_utf8(out.stdout).expect("stdout is UTF-8");
        if let Some(out_dir) = &dump {
            std::fs::write(out_dir.join(file), &stdout).unwrap();
            continue;
        }
        assert_eq!(
            out.status.code(),
            Some(*code),
            "{args:?} exit code: {stdout}"
        );
        let golden = std::fs::read_to_string(golden_dir().join(file))
            .unwrap_or_else(|e| panic!("golden {file}: {e}"));
        if golden != normalise(&stdout) {
            mismatched.push(format!("{args:?} differs from {file}"));
        }
    }
    assert!(mismatched.is_empty(), "{}", mismatched.join("\n"));
}
