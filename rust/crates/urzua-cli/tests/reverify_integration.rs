//! ADR-97 / MILE-129: `urzua reverify` is the one attributed way to renew a
//! claim. Review first, then apply; a changed pin is refused; `code:` and
//! `test:` locators must be pinned; a batch is capped and shares one reason.
//! Real git fixtures, real binary, JSON only.

mod common;

use std::path::Path;
use std::process::Command;

fn git(dir: &Path, args: &[&str]) {
    let status = Command::new("git")
        .args(args)
        .current_dir(dir)
        .status()
        .expect("git should run");
    assert!(status.success(), "git {args:?} failed");
}

fn date_days_ago(n: u32) -> String {
    let out = Command::new("date")
        .args(["-u", "-d", &format!("{n} days ago"), "+%Y-%m-%dT12:00:00Z"])
        .output()
        .unwrap();
    String::from_utf8(out.stdout).unwrap().trim().to_string()
}

fn today() -> String {
    date_days_ago(0)[..10].to_string()
}

fn commit_all(dir: &Path, days_ago: u32) {
    git(dir, &["add", "-A"]);
    let date = date_days_ago(days_ago);
    let status = Command::new("git")
        .args(["commit", "-q", "-m", "fixture"])
        .env("GIT_AUTHOR_DATE", &date)
        .env("GIT_COMMITTER_DATE", &date)
        .current_dir(dir)
        .status()
        .unwrap();
    assert!(status.success());
}

fn run_urzua(dir: &Path, args: &[&str]) -> std::process::Output {
    common::urzua()
        .args(args)
        .current_dir(dir)
        .output()
        .expect("urzua binary should run")
}

fn json(out: &std::process::Output) -> serde_json::Value {
    serde_json::from_slice(&out.stdout)
        .unwrap_or_else(|e| panic!("stdout is not JSON ({e}): {out:?}"))
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

const SRC: &str = "fn a() {}\n";
const TST: &str = "fn t() {}\n";
const V2: &str = "fn a() { let _ = 1; }\n";

fn config(rule: &str, extra: &str) -> String {
    format!(
        "schema_version: 2\nrules:\n  embodiment.claim-expired:\n{rule}{extra}\
         locator_resolvers:\n  - name: rust\n    glob: \"**/*.rs\"\n    start: '^\\s*fn\\s+{{symbol}}\\b'\n    end: brace_balanced\n\
         record_types:\n  \
         milestone:\n    dir: docs/milestones\n    prefix: MILE\n    header_shape: yaml-frontmatter\n    \
         required_fields: [Status]\n    known_fields: [Embodiment, Realized-by, Last-audited, Stable-Id]\n    \
         statuses:\n      values: [Planned, Done, Superseded]\n      success: [Done]\n      claim_retained: [Superseded]\n"
    )
}

const RULE: &str = "    level: warn\n    anchor_field: Last-audited\n    require_pin: [code, test]\n    max_batch: 3\n    periods:\n      - {type: milestone, status: [Done], days: 30}\n";

fn record(n: u32, locators: &str, extra: &str) -> String {
    format!(
        "---\nStable-Id: 01AAAAAAAAAAAAAAAAAAAAAA{n:02}\nStatus: Done\n{extra}Realized-by: {locators}\n---\n# MILE-{n}: T\n\n> **Revision log**\n>\n> | Date | Change | Class |\n> |---|---|---|\n> | 2026-01-01 | Initial. | **structural** |\n"
    )
}

fn path(n: u32) -> String {
    format!("docs/milestones/MILE-{n}-m.md")
}

const UNPINNED: &str = "code:src/a.rs, test:src/t.rs, spec:docs/s.md";

fn fixture(name: &str, cfg: &str, records: u32) -> common::TempDir {
    let dir = common::TempDir::new(&format!("urzua-reverify-{name}"));
    for d in ["docs/milestones", "src", ".urzua"] {
        std::fs::create_dir_all(dir.join(d)).unwrap();
    }
    git(&dir, &["init", "-q"]);
    git(&dir, &["config", "user.email", "test@example.com"]);
    git(&dir, &["config", "user.name", "test"]);
    std::fs::write(dir.join(".urzua/config.yaml"), cfg).unwrap();
    std::fs::write(dir.join("src/a.rs"), SRC).unwrap();
    std::fs::write(dir.join("src/t.rs"), TST).unwrap();
    std::fs::write(dir.join("docs/s.md"), "s\n").unwrap();
    for n in 1..=records {
        std::fs::write(dir.join(path(n)), record(n, UNPINNED, "")).unwrap();
    }
    commit_all(&dir, 40);
    dir
}

fn apply_args<'a>(ids: &'a str, reason: &'a str) -> Vec<&'a str> {
    vec![
        "reverify", "--apply", "--ids", ids, "--by", "ana", "--actor", "human", "--reason", reason,
    ]
}

fn read(dir: &Path, rel: &str) -> String {
    std::fs::read_to_string(dir.join(rel)).unwrap()
}

#[test]
fn detect_reviews_the_expired_claim_locator_by_locator_and_writes_nothing_observed_failing() {
    let dir = fixture("detect", &config(RULE, ""), 1);
    let before = read(&dir, &path(1));
    let out = run_urzua(&dir, &["reverify"]);
    let parsed = json(&out);
    assert_eq!(out.status.code(), Some(0), "{parsed}");
    assert_eq!(parsed["status"], "repairs-available", "{parsed}");
    let rec = &parsed["records"][0];
    assert_eq!(rec["record"], path(1), "{parsed}");
    assert!(rec["finding"].as_str().unwrap().contains("30-day"), "{rec}");
    let locs = rec["locators"].as_array().unwrap();
    assert_eq!(locs.len(), 3, "{rec}");
    let by = |l: &str| locs.iter().find(|x| x["locator"] == l).unwrap();
    assert_eq!(by("src/a.rs")["check"], "will-pin", "{rec}");
    assert_eq!(by("src/a.rs")["pinned"], false);
    assert_eq!(by("src/a.rs")["current_hash"], sha256(SRC));
    assert_eq!(by("src/t.rs")["check"], "will-pin");
    assert_eq!(by("docs/s.md")["check"], "history-only", "{rec}");
    assert_eq!(read(&dir, &path(1)), before);
}

#[test]
fn detect_lists_only_expired_claims_without_ids_and_any_listed_record_with_ids() {
    let dir = fixture("detect-ids", &config(RULE, ""), 2);
    // MILE-2 is renewed early: a recent audit date makes it not expired.
    let text = read(&dir, &path(2));
    std::fs::write(
        dir.join(path(2)),
        text.replace(
            "Status: Done\n",
            &format!("Status: Done\nLast-audited: {}\n", today()),
        ),
    )
    .unwrap();
    commit_all(&dir, 0);
    let parsed = json(&run_urzua(&dir, &["reverify"]));
    let listed: Vec<&str> = parsed["records"]
        .as_array()
        .unwrap()
        .iter()
        .map(|r| r["record"].as_str().unwrap())
        .collect();
    assert_eq!(listed, [path(1).as_str()], "{parsed}");
    let p2 = path(2);
    let parsed = json(&run_urzua(&dir, &["reverify", "--ids", &p2]));
    assert_eq!(parsed["records"][0]["record"], p2.as_str(), "{parsed}");
}

#[test]
fn apply_pins_code_and_test_then_renews_with_two_rows_and_the_anchor_observed_failing() {
    let dir = fixture("apply", &config(RULE, ""), 1);
    let p1 = path(1);
    let out = run_urzua(
        &dir,
        &apply_args(
            &p1,
            "Read src/a.rs and t.rs against the item; both still meet it.",
        ),
    );
    let parsed = json(&out);
    assert_eq!(parsed["status"], "applied", "{parsed} {out:?}");
    assert_eq!(parsed["identity"]["name"], "ana", "{parsed}");
    let text = read(&dir, &p1);
    // First pins, in the locator's own spelling; the spec locator is untouched.
    assert!(
        text.contains(&format!(
            "Realized-by: code:src/a.rs@sha256:{}, test:src/t.rs@sha256:{}, spec:docs/s.md\n",
            sha256(SRC),
            sha256(TST)
        )),
        "{text}"
    );
    // The anchor is set, and Status and Embodiment are not touched.
    assert!(
        text.contains(&format!("Last-audited: {}\n", today())),
        "{text}"
    );
    assert!(
        text.contains("Status: Done\n") && !text.contains("Embodiment"),
        "{text}"
    );
    let pinned: Vec<&str> = text.lines().filter(|l| l.contains("| Pinned (")).collect();
    let renewed: Vec<&str> = text
        .lines()
        .filter(|l| l.contains("| Re-verified ("))
        .collect();
    assert_eq!((pinned.len(), renewed.len()), (1, 1), "{text}");
    assert!(
        pinned[0].contains("Pinned (attested by ana, identity: asserted, actor: human): 2 locators pinned, 2 file-level, 0 scoped. Evidence: src/a.rs@sha256:"),
        "{}",
        pinned[0]
    );
    assert!(
        pinned[0].contains(&format!("src/a.rs@sha256:{}", &sha256(SRC)[..12]))
            && pinned[0].contains("Reason: Read src/a.rs and t.rs against the item; both still meet it. **Why:** a claim is renewed only over pinned code and test evidence."),
        "{}",
        pinned[0]
    );
    assert!(
        renewed[0].contains("Re-verified (attested by ana, identity: asserted, actor: human): 3 locators re-resolved, 2 hash-checked, 1 spec locators checked by git history only. Evidence: ")
            && renewed[0].contains("**Why:** the claim was 40 days old against a 30-day period."),
        "{}",
        renewed[0]
    );
    // Pinned row precedes the Re-verified row, both after the initial row.
    let at = |needle: &str| text.find(needle).unwrap();
    assert!(at("| Pinned (") < at("| Re-verified ("), "{text}");
    assert!(renewed[0].ends_with("| **structural** |"), "{}", renewed[0]);

    // The claim is renewed: no finding, and a second apply has nothing to pin.
    let parsed = json(&run_urzua(&dir, &["check"]));
    assert!(
        parsed["findings"]
            .as_array()
            .unwrap()
            .iter()
            .all(|f| f["rule"] != "embodiment.claim-expired"),
        "{parsed}"
    );
}

#[test]
fn a_changed_pin_is_refused_and_nothing_is_written_observed_failing() {
    let dir = fixture("changed", &config(RULE, ""), 1);
    let pinned = record(
        1,
        &format!(
            "code:src/a.rs@sha256:{}, test:src/t.rs@sha256:{}",
            sha256(SRC),
            sha256(TST)
        ),
        "",
    );
    std::fs::write(dir.join(path(1)), &pinned).unwrap();
    commit_all(&dir, 40);
    std::fs::write(dir.join("src/a.rs"), V2).unwrap();
    commit_all(&dir, 30);
    let before = read(&dir, &path(1));
    let out = run_urzua(&dir, &apply_args(&path(1), "checked"));
    let parsed = json(&out);
    assert_eq!(out.status.code(), Some(1), "{parsed}");
    assert_eq!(parsed["status"], "partial-failure", "{parsed}");
    let error = parsed["failed"][0]["error"].as_str().unwrap();
    assert!(error.contains("fix --rebaseline"), "{error}");
    assert_eq!(read(&dir, &path(1)), before);
}

#[test]
fn a_locator_that_does_not_resolve_is_refused() {
    let dir = fixture("unresolved", &config(RULE, ""), 1);
    std::fs::write(
        dir.join(path(1)),
        record(1, "code:src/gone.rs, test:src/t.rs", ""),
    )
    .unwrap();
    commit_all(&dir, 40);
    let before = read(&dir, &path(1));
    let parsed = json(&run_urzua(&dir, &apply_args(&path(1), "checked")));
    assert_eq!(parsed["status"], "partial-failure", "{parsed}");
    assert!(
        parsed["failed"][0]["error"]
            .as_str()
            .unwrap()
            .contains("src/gone.rs"),
        "{parsed}"
    );
    assert_eq!(read(&dir, &path(1)), before);
}

#[test]
fn drift_on_an_unpinned_locator_refuses_the_renewal_and_pin_only_clears_it_observed_failing() {
    let extra = "  embodiment.consistency: {level: warn, drift_grace_days: 365}\n";
    let dir = fixture("drift", &config(RULE, extra), 1);
    // The code changes after the Realized-by line was last touched (inside the grace window).
    std::fs::write(dir.join("src/a.rs"), V2).unwrap();
    commit_all(&dir, 1);
    let before = read(&dir, &path(1));
    let parsed = json(&run_urzua(&dir, &apply_args(&path(1), "checked")));
    assert_eq!(parsed["status"], "partial-failure", "{parsed}");
    let error = parsed["failed"][0]["error"].as_str().unwrap();
    assert!(
        error.contains("drift") && error.contains("--pin-only"),
        "{error}"
    );
    assert_eq!(read(&dir, &path(1)), before);

    // Detect with --pin-only lists the commits since the line was touched.
    let detect = json(&run_urzua(
        &dir,
        &["reverify", "--pin-only", "--ids", &path(1)],
    ));
    assert!(
        !detect["records"][0]["commits_since"]
            .as_array()
            .unwrap()
            .is_empty(),
        "{detect}"
    );

    // --pin-only pins against the current content, sets no anchor, writes one row.
    let p1 = path(1);
    let mut args = apply_args(&p1, "Pinned against the file as it is now: read the one-line change to a(), the item still holds.");
    args.push("--pin-only");
    let parsed = json(&run_urzua(&dir, &args));
    assert_eq!(parsed["status"], "applied", "{parsed}");
    let text = read(&dir, &path(1));
    assert!(
        text.contains(&format!("code:src/a.rs@sha256:{}", sha256(V2))),
        "{text}"
    );
    assert!(
        !text.contains("Last-audited"),
        "pin-only renews nothing: {text}"
    );
    assert_eq!(text.matches("| Pinned (").count(), 1, "{text}");
    assert_eq!(text.matches("| Re-verified (").count(), 0, "{text}");

    // Then the renewal, with its own reason, is accepted.
    let parsed = json(&run_urzua(
        &dir,
        &apply_args(&path(1), "Second act: the claim holds."),
    ));
    assert_eq!(parsed["status"], "applied", "{parsed}");
    let text = read(&dir, &path(1));
    assert_eq!(
        text.matches("| Pinned (").count(),
        1,
        "nothing left to pin: {text}"
    );
    assert_eq!(text.matches("| Re-verified (").count(), 1, "{text}");
}

#[test]
fn pin_only_refuses_a_pinned_locator_whose_hash_differs() {
    let dir = fixture("pin-only-changed", &config(RULE, ""), 1);
    std::fs::write(
        dir.join(path(1)),
        record(
            1,
            &format!("code:src/a.rs@sha256:{}, test:src/t.rs", sha256(SRC)),
            "",
        ),
    )
    .unwrap();
    commit_all(&dir, 40);
    std::fs::write(dir.join("src/a.rs"), V2).unwrap();
    commit_all(&dir, 30);
    let before = read(&dir, &path(1));
    let p1 = path(1);
    let mut args = apply_args(&p1, "checked");
    args.push("--pin-only");
    let parsed = json(&run_urzua(&dir, &args));
    assert_eq!(parsed["status"], "partial-failure", "{parsed}");
    assert!(
        parsed["failed"][0]["error"]
            .as_str()
            .unwrap()
            .contains("fix --rebaseline"),
        "{parsed}"
    );
    assert_eq!(read(&dir, &path(1)), before);
}

#[test]
fn pin_only_with_nothing_unpinned_says_so_instead_of_doing_nothing() {
    let dir = fixture("pin-only-none", &config(RULE, ""), 1);
    std::fs::write(
        dir.join(path(1)),
        record(
            1,
            &format!(
                "code:src/a.rs@sha256:{}, test:src/t.rs@sha256:{}",
                sha256(SRC),
                sha256(TST)
            ),
            "",
        ),
    )
    .unwrap();
    commit_all(&dir, 40);
    let p1 = path(1);
    let mut args = apply_args(&p1, "checked");
    args.push("--pin-only");
    let out = run_urzua(&dir, &args);
    let parsed = json(&out);
    assert_eq!(out.status.code(), Some(1), "{parsed}");
    assert!(
        parsed["failed"][0]["error"]
            .as_str()
            .unwrap()
            .contains("no unpinned"),
        "{parsed}"
    );
}

#[test]
fn a_scoped_locator_is_pinned_in_its_own_spelling_and_counted_as_scoped() {
    let dir = fixture("scoped", &config(RULE, ""), 1);
    std::fs::write(
        dir.join(path(1)),
        record(1, "code:src/a.rs#a, test:src/t.rs", ""),
    )
    .unwrap();
    commit_all(&dir, 40);
    let parsed = json(&run_urzua(
        &dir,
        &apply_args(&path(1), "checked the symbol"),
    ));
    assert_eq!(parsed["status"], "applied", "{parsed}");
    let text = read(&dir, &path(1));
    assert!(
        text.contains(&format!("code:src/a.rs#a@sha256:{}", sha256("fn a() {}"))),
        "the span, not the file: {text}"
    );
    assert!(
        text.contains("2 locators pinned, 1 file-level, 1 scoped."),
        "{text}"
    );
}

#[test]
fn the_gates_of_fix_apply_hold() {
    let dir = fixture("gates", &config(RULE, ""), 1);
    let p1 = path(1);
    // --ids or --force, never implied.
    let out = run_urzua(
        &dir,
        &["reverify", "--apply", "--by", "ana", "--reason", "r"],
    );
    assert_eq!(out.status.code(), Some(2), "{out:?}");
    // A reason is required, on one line, without a pipe.
    for reason in [None, Some(""), Some("a | b"), Some("a\nb")] {
        let mut args = vec!["reverify", "--apply", "--ids", &p1, "--by", "ana"];
        if let Some(r) = reason {
            args.extend(["--reason", r]);
        }
        let out = run_urzua(&dir, &args);
        assert_eq!(out.status.code(), Some(2), "{reason:?}: {out:?}");
    }
    // The write flags belong to --apply.
    for flag in [
        vec!["--by", "ana"],
        vec!["--reason", "r"],
        vec!["--force"],
        vec!["--actor", "agent"],
    ] {
        let mut args = vec!["reverify"];
        args.extend(flag.iter().copied());
        let out = run_urzua(&dir, &args);
        assert_eq!(out.status.code(), Some(2), "{flag:?}: {out:?}");
    }
    // An --ids entry that matches nothing fails.
    let out = run_urzua(&dir, &apply_args("docs/milestones/MILE-99-x.md", "r"));
    assert_eq!(out.status.code(), Some(1), "{out:?}");
    assert_eq!(json(&out)["status"], "partial-failure");
    assert_eq!(read(&dir, &p1), record(1, UNPINNED, ""));
}

#[test]
fn a_batch_is_capped_names_its_records_and_shares_one_reason_observed_failing() {
    let dir = fixture("batch", &config(RULE, ""), 4);
    let all = (1..=4).map(path).collect::<Vec<_>>().join(",");
    let before: Vec<String> = (1..=4).map(|n| read(&dir, &path(n))).collect();
    // Four records against a cap of three: refused whole, nothing truncated.
    let out = run_urzua(&dir, &apply_args(&all, "same words"));
    let parsed = json(&out);
    assert_eq!(out.status.code(), Some(2), "{parsed}");
    assert!(
        parsed["error"].as_str().unwrap().contains("max_batch"),
        "{parsed}"
    );
    for n in 1..=4u32 {
        assert_eq!(read(&dir, &path(n)), before[(n - 1) as usize]);
    }
    // --force cannot batch.
    let out = run_urzua(
        &dir,
        &[
            "reverify", "--apply", "--force", "--by", "ana", "--actor", "human", "--reason", "r",
        ],
    );
    assert_eq!(out.status.code(), Some(2), "{out:?}");
    assert!(
        json(&out)["error"].as_str().unwrap().contains("--force"),
        "{out:?}"
    );
    // Three records: the same reason in every row, reported once.
    let three = (1..=3).map(path).collect::<Vec<_>>().join(",");
    let parsed = json(&run_urzua(
        &dir,
        &apply_args(&three, "Batch reason: read all three against their items."),
    ));
    assert_eq!(parsed["status"], "applied", "{parsed}");
    assert_eq!(
        parsed["reason"],
        "Batch reason: read all three against their items."
    );
    for n in 1..=3 {
        let text = read(&dir, &path(n));
        assert_eq!(
            text.matches("Reason: Batch reason: read all three against their items.")
                .count(),
            2,
            "a Pinned and a Re-verified row: {text}"
        );
    }
    assert_eq!(read(&dir, &path(4)), before[3]);
}

#[test]
fn force_with_a_single_candidate_is_accepted() {
    let dir = fixture("force-one", &config(RULE, ""), 1);
    let out = run_urzua(
        &dir,
        &[
            "reverify", "--apply", "--force", "--by", "ana", "--actor", "human", "--reason", "r",
        ],
    );
    assert_eq!(json(&out)["status"], "applied", "{out:?}");
}

#[test]
fn an_agent_may_renew_but_not_where_the_entrys_level_is_error() {
    let strict = "    level: error\n    anchor_field: Last-audited\n    require_pin: [code, test]\n    periods:\n      - {type: milestone, status: [Done], days: 30}\n";
    let dir = fixture("agent-error", &config(strict, ""), 1);
    let before = read(&dir, &path(1));
    let args = [
        "reverify",
        "--apply",
        "--ids",
        &path(1),
        "--by",
        "ana",
        "--actor",
        "agent",
        "--reason",
        "r",
    ];
    let parsed = json(&run_urzua(&dir, &args));
    assert_eq!(parsed["status"], "partial-failure", "{parsed}");
    assert!(
        parsed["failed"][0]["error"]
            .as_str()
            .unwrap()
            .contains("human"),
        "{parsed}"
    );
    assert_eq!(read(&dir, &path(1)), before);

    let dir = fixture("agent-warn", &config(RULE, ""), 1);
    let parsed = json(&run_urzua(&dir, &args));
    assert_eq!(parsed["status"], "applied", "{parsed}");
    assert!(read(&dir, &path(1)).contains("actor: agent"));
}

#[test]
fn without_an_anchor_field_the_command_cannot_renew_and_says_so() {
    let rule = "    level: warn\n    require_pin: [code, test]\n    periods:\n      - {type: milestone, status: [Done], days: 30}\n";
    let dir = fixture("no-anchor", &config(rule, ""), 1);
    let out = run_urzua(&dir, &apply_args(&path(1), "r"));
    let parsed = json(&out);
    assert_eq!(parsed["status"], "partial-failure", "{parsed}");
    assert!(
        parsed["failed"][0]["error"]
            .as_str()
            .unwrap()
            .contains("anchor_field"),
        "{parsed}"
    );
}

#[test]
fn the_command_needs_the_rule_declared() {
    let dir = fixture("undeclared", "schema_version: 2\nrecord_types:\n  milestone:\n    dir: docs/milestones\n    prefix: MILE\n    header_shape: yaml-frontmatter\n    required_fields: [Status]\n", 1);
    let out = run_urzua(&dir, &["reverify"]);
    assert_eq!(out.status.code(), Some(2), "{out:?}");
    assert!(
        json(&out)["error"]
            .as_str()
            .unwrap()
            .contains("embodiment.claim-expired"),
        "{out:?}"
    );
}

#[test]
fn a_record_with_no_revision_log_is_refused_and_untouched() {
    let dir = fixture("no-log", &config(RULE, ""), 1);
    std::fs::write(
        dir.join(path(1)),
        "---\nStable-Id: 01AAAAAAAAAAAAAAAAAAAAAA01\nStatus: Done\nRealized-by: code:src/a.rs, test:src/t.rs\n---\n# MILE-1: T\n",
    )
    .unwrap();
    commit_all(&dir, 40);
    let before = read(&dir, &path(1));
    let parsed = json(&run_urzua(&dir, &apply_args(&path(1), "r")));
    assert_eq!(parsed["status"], "partial-failure", "{parsed}");
    assert_eq!(read(&dir, &path(1)), before);
}

#[test]
fn a_path_outside_the_repository_or_untracked_does_not_resolve_observed_failing() {
    let dir = fixture("outside", &config(RULE, ""), 1);
    std::fs::write(
        dir.join(path(1)),
        record(
            1,
            "code:src/a.rs, test:src/t.rs, code:/etc/passwd, code:untracked.txt, code:../outside.txt, code:untracked.txt#x",
            "",
        ),
    )
    .unwrap();
    commit_all(&dir, 40);
    std::fs::write(dir.join("untracked.txt"), "x\n").unwrap();
    let before = read(&dir, &path(1));
    let parsed = json(&run_urzua(&dir, &["reverify"]));
    let rec = &parsed["records"][0];
    for l in ["/etc/passwd", "untracked.txt", "../outside.txt"] {
        let loc = rec["locators"]
            .as_array()
            .unwrap()
            .iter()
            .find(|x| x["locator"] == l)
            .unwrap_or_else(|| panic!("{l}: {rec}"));
        assert_eq!(loc["resolved"], false, "{l}: {loc}");
        assert_eq!(loc["check"], "unresolved", "{l}: {loc}");
    }
    assert!(!rec["refused"].as_array().unwrap().is_empty(), "{rec}");
    let parsed = json(&run_urzua(&dir, &apply_args(&path(1), "r")));
    assert_eq!(parsed["status"], "partial-failure", "{parsed}");
    assert_eq!(read(&dir, &path(1)), before);
}

#[test]
fn an_unspecified_actor_is_refused_where_the_level_is_error_observed_failing() {
    let strict = "    level: error\n    anchor_field: Last-audited\n    require_pin: [code, test]\n    periods:\n      - {type: milestone, status: [Done], days: 30}\n";
    let dir = fixture("unspecified-error", &config(strict, ""), 1);
    let before = read(&dir, &path(1));
    let parsed = json(&run_urzua(
        &dir,
        &[
            "reverify",
            "--apply",
            "--ids",
            &path(1),
            "--by",
            "ana",
            "--reason",
            "r",
        ],
    ));
    assert_eq!(parsed["status"], "partial-failure", "{parsed}");
    assert!(
        parsed["failed"][0]["error"]
            .as_str()
            .unwrap()
            .contains("human"),
        "{parsed}"
    );
    assert_eq!(read(&dir, &path(1)), before);
}

#[test]
fn force_with_ids_renews_only_the_named_record_observed_failing() {
    let dir = fixture("force-ids", &config(RULE, ""), 2);
    let before1 = read(&dir, &path(1));
    let out = run_urzua(
        &dir,
        &[
            "reverify",
            "--apply",
            "--force",
            "--ids",
            &path(2),
            "--by",
            "ana",
            "--actor",
            "human",
            "--reason",
            "r",
        ],
    );
    let parsed = json(&out);
    assert_eq!(parsed["status"], "applied", "{parsed}");
    assert_eq!(parsed["records"].as_array().unwrap().len(), 1, "{parsed}");
    assert_eq!(parsed["records"][0]["record"], path(2), "{parsed}");
    assert!(read(&dir, &path(2)).contains(&format!("Last-audited: {}", today())));
    assert_eq!(read(&dir, &path(1)), before1);
}

#[test]
fn detect_reports_a_claim_with_no_revision_log_as_not_renewable_observed_failing() {
    let dir = fixture("detect-no-log", &config(RULE, ""), 1);
    std::fs::write(
        dir.join(path(1)),
        "---\nStable-Id: 01AAAAAAAAAAAAAAAAAAAAAA01\nStatus: Done\nRealized-by: code:src/a.rs, test:src/t.rs\n---\n# MILE-1: T\n",
    )
    .unwrap();
    commit_all(&dir, 40);
    let parsed = json(&run_urzua(&dir, &["reverify"]));
    assert_eq!(parsed["status"], "ok", "nothing renewable: {parsed}");
    let refused = parsed["records"][0]["refused"].as_array().unwrap();
    assert!(
        refused
            .iter()
            .any(|r| r.as_str().unwrap().contains("Revision log")),
        "{parsed}"
    );
}
