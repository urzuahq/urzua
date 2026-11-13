//! Fixtures shared by the `urzua waive` integration tests (MILE-205).

#![allow(dead_code)]

use std::path::Path;
use std::process::Command;

use super::{urzua, TempDir};

pub fn git(dir: &Path, args: &[&str]) {
    let status = Command::new("git")
        .args(args)
        .current_dir(dir)
        .status()
        .expect("git should run");
    assert!(status.success(), "git {args:?} failed");
}

/// What a fixture varies: the waiver type's own declarations, extra rules and fail_on.
pub struct Setup<'a> {
    /// Lines spliced into the waiver type (4-space indent each).
    pub waiver: &'a str,
    /// Rule levels, each a `name: level` pair.
    pub rules: &'a str,
    pub fail_on: &'a str,
    /// `(file name, content)` records under docs/adr and docs/waiver.
    pub adrs: Vec<(&'a str, String)>,
    pub waivers: Vec<(&'a str, String)>,
    /// The waiver template lists `Owner` and the type declares it.
    pub owner: bool,
}

pub const PERMANENT_OR_EXPIRES: &str = "    effective_requires: [Reason, Expires]\n";

impl Default for Setup<'_> {
    fn default() -> Self {
        Setup {
            waiver: PERMANENT_OR_EXPIRES,
            rules: "header.required-fields: error, field.quality: error, field.pending: warn, waiver.matches-nothing: warn",
            fail_on: "warning",
            adrs: Vec::new(),
            waivers: Vec::new(),
            owner: false,
        }
    }
}

pub fn adr(status: &str, date: &str, author: &str) -> String {
    format!("---\nStatus: {status}\nDate: {date}\nAuthor: {author}\nOwner: o\n---\n# 1 - X\n")
}

/// Three rules fire in a record with `Date: TBD` (field.quality), `Author: pending`
/// (field.pending) and no `Owner` (header.required-fields).
pub fn noisy() -> String {
    "---\nStatus: Proposed\nDate: TBD\nAuthor: pending\n---\n# 1 - X\n".to_string()
}

pub fn waiver_file(rule: &str, scope: &str, reason: &str, expires: &str) -> String {
    format!(
        "---\nStable-Id: 01M7WAIVERAAAAAAAAAAAAAAAA\nRule: {rule}\nScope: {scope}\nReason: {reason}\nExpires: {expires}\n---\n# WAIVER-0007 -- X\n\n## What this covers\n\nx\n\n> **Revision log**\n>\n> | Date | Change | Class |\n> |---|---|---|\n> | 2026-11-01 | Filed. | **substantive** |\n"
    )
}

pub fn fixture(name: &str, setup: Setup) -> TempDir {
    let dir = TempDir::new(&format!("urzua-waive-{name}"));
    std::fs::create_dir_all(dir.join(".urzua/templates")).unwrap();
    std::fs::create_dir_all(dir.join("docs/adr")).unwrap();
    std::fs::create_dir_all(dir.join("docs/waiver")).unwrap();
    git(&dir, &["init", "-q"]);
    git(&dir, &["config", "user.email", "test@example.com"]);
    git(&dir, &["config", "user.name", "test"]);
    std::fs::write(
        dir.join(".urzua/templates/waiver.md"),
        format!("# NNNN -- Title\n\n> Rule: the rule\n> Scope: a path\n> Reason: why\n> Expires: YYYY-MM-DD\n> Category: x\n> Justified-by: y\n{}\n## What this covers\n\nThe finding.\n\n> **Revision log**\n>\n> | Date | Change | Class |\n> |---|---|---|\n", if setup.owner { "> Owner: who\n" } else { "" }),
    )
    .unwrap();
    std::fs::write(
        dir.join(".urzua/config.yaml"),
        format!(
            "schema_version: 2\nfail_on: {}\nrules: {{{}}}\nrecord_types:\n  adr:\n    dir: docs/adr\n    prefix: ADR\n    header_shape: yaml-frontmatter\n    required_fields: [Status, Date, Author, Owner]\n    known_fields: [Note]\n  waiver:\n    dir: docs/waiver\n    prefix: WAIVER\n    role: suppression\n    header_shape: yaml-frontmatter\n    identity: {{from: filename, pattern: '^(?P<prefix>WAIVER)-(?P<number>\\d{{4}})-(?P<slug>.+)$'}}\n    required_fields: [Rule, Scope, Reason]\n    known_fields: [Expires, Category, Justified-by{}]\n    revision_log:\n      marker: \"**Revision log**\"\n      required: true\n      classes: [substantive, structural]\n      class_column: Class\n      date_column: Date\n      tool_class: structural\n{}",
            setup.fail_on, setup.rules, if setup.owner { ", Owner" } else { "" }, setup.waiver
        ),
    )
    .unwrap();
    for (name, content) in &setup.adrs {
        std::fs::write(dir.join("docs/adr").join(name), content).unwrap();
    }
    for (name, content) in &setup.waivers {
        std::fs::write(dir.join("docs/waiver").join(name), content).unwrap();
    }
    git(&dir, &["add", "-A"]);
    git(&dir, &["commit", "-q", "-m", "fixture"]);
    dir
}

pub fn run(dir: &Path, args: &[&str]) -> (i32, String, String) {
    let out = urzua()
        .args(args)
        .current_dir(dir)
        .output()
        .expect("urzua binary should run");
    (
        out.status.code().unwrap_or(-1),
        String::from_utf8_lossy(&out.stdout).into_owned(),
        String::from_utf8_lossy(&out.stderr).into_owned(),
    )
}

pub fn waive(dir: &Path, extra: &[&str]) -> (i32, serde_json::Value) {
    let mut args = vec!["waive", "--from-report", "--by", "tester"];
    args.extend_from_slice(extra);
    let (code, stdout, stderr) = run(dir, &args);
    let json = serde_json::from_str(&stdout)
        .unwrap_or_else(|e| panic!("stdout is not one JSON document ({e}): {stdout} / {stderr}"));
    (code, json)
}

pub fn untouched(dir: &Path) {
    let (_, status, _) = run_git_status(dir);
    assert_eq!(status.trim(), "", "a dry run wrote something: {status}");
}

pub fn run_git_status(dir: &Path) -> (i32, String, String) {
    let out = Command::new("git")
        .args(["status", "--porcelain", "--untracked-files=all"])
        .current_dir(dir)
        .output()
        .unwrap();
    (
        out.status.code().unwrap_or(-1),
        String::from_utf8_lossy(&out.stdout).into_owned(),
        String::new(),
    )
}

pub fn str_of<'a>(v: &'a serde_json::Value, key: &str) -> &'a str {
    v[key].as_str().unwrap_or_else(|| panic!("{key} in {v}"))
}

pub fn two_noisy() -> Setup<'static> {
    Setup {
        adrs: vec![("ADR-1-a.md", noisy()), ("ADR-2-b.md", noisy())],
        ..Setup::default()
    }
}
