//! ADR-97 / MILE-129: `embodiment.claim-expired` expires a claim after a period
//! declared per record type, status and evidence tier; `embodiment.claim-renewal-unattested`
//! catches a date typed in by hand. Real git fixtures, real binary, JSON only.

mod common;

use common::{commit_all, date_days_ago, git};

use std::path::Path;
use std::process::Command;

fn day_days_ago(n: u32) -> String {
    date_days_ago(n)[..10].to_string()
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

const SRC: &str = "fn a() {}\n";

fn config(rule: &str, extra_rules: &str) -> String {
    format!(
        "schema_version: 2\nrules:\n  embodiment.claim-expired:\n{rule}\n{extra_rules}\
         record_types:\n  \
         milestone:\n    dir: docs/milestones\n    prefix: MILE\n    header_shape: yaml-frontmatter\n    \
         required_fields: [Status]\n    known_fields: [Embodiment, Realized-by, Last-audited, Stable-Id]\n    \
         statuses:\n      values: [Planned, Done, Superseded]\n      success: [Done]\n      claim_retained: [Superseded]\n  \
         adr:\n    dir: docs/adr\n    prefix: ADR\n    header_shape: yaml-frontmatter\n    \
         required_fields: [Status]\n    known_fields: [Embodiment, Realized-by, Last-audited, Stable-Id]\n    \
         statuses:\n      values: [Proposed, Accepted, Superseded]\n      success: [Accepted]\n      claim_retained: [Superseded]\n"
    )
}

const RULE: &str = "    level: warn\n    anchor_field: Last-audited\n    require_pin: [code, test]\n    max_batch: 3\n    periods:\n      - {type: milestone, status: [Done], days: 30}\n      - {type: adr, status: [Accepted], days: 90}\n";

fn record(prefix: &str, status: &str, stable_id: &str, extra: &str, locators: &str) -> String {
    format!(
        "---\nStable-Id: {stable_id}\nStatus: {status}\n{extra}Realized-by: {locators}\n---\n# {prefix}: T\n\n> **Revision log**\n>\n> | Date | Change | Class |\n> |---|---|---|\n> | 2026-01-01 | Initial. | **structural** |\n"
    )
}

fn fixture(name: &str, cfg: &str) -> common::TempDir {
    let dir = common::TempDir::new(&format!("urzua-claimexp-{name}"));
    for d in ["docs/milestones", "docs/adr", "src", ".urzua"] {
        std::fs::create_dir_all(dir.join(d)).unwrap();
    }
    git(&dir, &["init", "-q"]);
    git(&dir, &["config", "user.email", "test@example.com"]);
    git(&dir, &["config", "user.name", "test"]);
    std::fs::write(dir.join(".urzua/config.yaml"), cfg).unwrap();
    std::fs::write(dir.join("src/a.rs"), SRC).unwrap();
    dir
}

fn findings<'a>(parsed: &'a serde_json::Value, rule: &str) -> Vec<&'a serde_json::Value> {
    parsed["findings"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|f| f["rule"] == rule)
        .collect()
}

const EXPIRED: &str = "embodiment.claim-expired";
const M1: &str = "docs/milestones/MILE-1-a.md";
const A1: &str = "docs/adr/ADR-1-a.md";

#[test]
fn a_repository_that_does_not_declare_the_rule_runs_nothing_new_observed_failing() {
    let dir = fixture("off", "schema_version: 2\nrecord_types:\n  milestone:\n    dir: docs/milestones\n    prefix: MILE\n    header_shape: yaml-frontmatter\n    required_fields: [Status]\n");
    std::fs::write(
        dir.join(M1),
        record(
            "MILE-1",
            "Done",
            "01AAAAAAAAAAAAAAAAAAAAAAAA",
            "",
            "code:src/a.rs",
        ),
    )
    .unwrap();
    commit_all(&dir, 400);
    let parsed = json(&run_urzua(&dir, &["check"]));
    assert!(findings(&parsed, EXPIRED).is_empty(), "{parsed}");
    let executed = parsed["rules_executed"]
        .as_array()
        .map(|r| {
            r.iter()
                .any(|e| e["rule"] == EXPIRED && e["status"] == "ran")
        })
        .unwrap_or(false);
    assert!(!executed, "an undeclared rule does not run: {parsed}");
}

#[test]
fn a_done_milestone_whose_claim_is_older_than_its_period_expires_with_the_action_observed_failing()
{
    let dir = fixture("expired", &config(RULE, ""));
    std::fs::write(
        dir.join(M1),
        record(
            "MILE-1",
            "Done",
            "01AAAAAAAAAAAAAAAAAAAAAAAA",
            "",
            "code:src/a.rs",
        ),
    )
    .unwrap();
    std::fs::write(
        dir.join(A1),
        record(
            "ADR-1",
            "Accepted",
            "01BBBBBBBBBBBBBBBBBBBBBBBB",
            "",
            "code:src/a.rs",
        ),
    )
    .unwrap();
    commit_all(&dir, 40);
    let parsed = json(&run_urzua(&dir, &["check"]));
    let found = findings(&parsed, EXPIRED);
    assert_eq!(
        found.len(),
        1,
        "milestone at 40 days of 30, ADR at 40 of 90: {parsed}"
    );
    let f = found[0];
    assert!(f["file"].as_str().unwrap().ends_with("MILE-1-a.md"), "{f}");
    assert_eq!(f["severity"], "warning", "{f}");
    let message = f["message"].as_str().unwrap();
    for needle in ["30-day", "40 days", "line last changed", "Done"] {
        assert!(message.contains(needle), "{needle}: {message}");
    }
    // ADR-73: the action is the review report, not a write.
    assert_eq!(f["suggested_action"]["kind"], "run-command", "{f}");
    assert_eq!(
        f["suggested_action"]["target"]["argv"],
        serde_json::json!(["urzua", "reverify", "--ids", M1]),
        "{f}"
    );
    // The population counts both records, examined.
    let exec = parsed["rules_executed"]
        .as_array()
        .unwrap()
        .iter()
        .find(|e| e["rule"] == EXPIRED)
        .unwrap();
    assert_eq!(exec["population"]["examined"], 2, "{exec}");
}

#[test]
fn the_command_a_finding_offers_produces_a_report_for_that_record_observed_failing() {
    let dir = fixture("offer", &config(RULE, ""));
    std::fs::write(
        dir.join(M1),
        record(
            "MILE-1",
            "Done",
            "01AAAAAAAAAAAAAAAAAAAAAAAA",
            "",
            "code:src/a.rs",
        ),
    )
    .unwrap();
    commit_all(&dir, 40);
    let parsed = json(&run_urzua(&dir, &["check"]));
    let f = findings(&parsed, EXPIRED)[0].clone();
    let argv: Vec<String> = f["suggested_action"]["target"]["argv"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_str().unwrap().to_string())
        .collect();
    assert_eq!(argv[0], "urzua");
    let out = run_urzua(
        &dir,
        &argv[1..].iter().map(String::as_str).collect::<Vec<_>>(),
    );
    let report = json(&out);
    assert_eq!(out.status.code(), Some(0), "{report}");
    assert_eq!(report["records"][0]["record"], M1, "{report}");
}

#[test]
fn a_claim_inside_its_period_is_silent() {
    let dir = fixture("fresh", &config(RULE, ""));
    std::fs::write(
        dir.join(M1),
        record(
            "MILE-1",
            "Done",
            "01AAAAAAAAAAAAAAAAAAAAAAAA",
            "",
            "code:src/a.rs",
        ),
    )
    .unwrap();
    commit_all(&dir, 10);
    let parsed = json(&run_urzua(&dir, &["check"]));
    assert!(findings(&parsed, EXPIRED).is_empty(), "{parsed}");
}

#[test]
fn a_milestone_that_states_no_embodiment_is_still_judged_observed_failing() {
    // The record states no `Embodiment`; the old option could never see it.
    let dir = fixture("noemb", &config(RULE, ""));
    std::fs::write(
        dir.join(M1),
        record(
            "MILE-1",
            "Done",
            "01AAAAAAAAAAAAAAAAAAAAAAAA",
            "",
            "code:src/a.rs",
        ),
    )
    .unwrap();
    commit_all(&dir, 40);
    let parsed = json(&run_urzua(&dir, &["check"]));
    assert_eq!(findings(&parsed, EXPIRED).len(), 1, "{parsed}");
}

#[test]
fn a_status_no_entry_names_and_a_retained_claim_are_not_examined() {
    let dir = fixture("status", &config(RULE, ""));
    std::fs::write(
        dir.join(M1),
        record(
            "MILE-1",
            "Planned",
            "01AAAAAAAAAAAAAAAAAAAAAAAA",
            "",
            "code:src/a.rs",
        ),
    )
    .unwrap();
    std::fs::write(
        dir.join("docs/milestones/MILE-2-b.md"),
        record(
            "MILE-2",
            "Superseded",
            "01CCCCCCCCCCCCCCCCCCCCCCCC",
            "",
            "code:src/a.rs",
        ),
    )
    .unwrap();
    commit_all(&dir, 400);
    let parsed = json(&run_urzua(&dir, &["check"]));
    assert!(findings(&parsed, EXPIRED).is_empty(), "{parsed}");
}

#[test]
fn the_most_specific_entry_wins_and_a_lower_entry_level_is_honoured_observed_failing() {
    let rule = "    level: error\n    anchor_field: Last-audited\n    periods:\n      - {type: adr, status: [Accepted], days: 30}\n      - {type: adr, status: [Accepted], tier: Specified, days: 365, level: info}\n";
    let dir = fixture("tier", &config(rule, ""));
    // A spec: locator computes to Specified; the narrower entry gives it 365 days.
    std::fs::write(dir.join("docs/s.md"), "s\n").unwrap();
    std::fs::write(
        dir.join(A1),
        record(
            "ADR-1",
            "Accepted",
            "01BBBBBBBBBBBBBBBBBBBBBBBB",
            "",
            "spec:docs/s.md",
        ),
    )
    .unwrap();
    std::fs::write(
        dir.join("docs/adr/ADR-2-b.md"),
        record(
            "ADR-2",
            "Accepted",
            "01DDDDDDDDDDDDDDDDDDDDDDDD",
            "",
            "code:src/a.rs",
        ),
    )
    .unwrap();
    commit_all(&dir, 100);
    let parsed = json(&run_urzua(&dir, &["check"]));
    let found = findings(&parsed, EXPIRED);
    // ADR-1 (Specified, 365 days) is silent; ADR-2 (Implemented, 30 days) expires.
    assert_eq!(found.len(), 1, "{parsed}");
    assert!(found[0]["file"].as_str().unwrap().ends_with("ADR-2-b.md"));
    assert_eq!(found[0]["severity"], "error");

    // The same corpus with ADR-1 old enough for the narrower entry: advisory.
    let dir = fixture("tier2", &config(rule, ""));
    std::fs::write(dir.join("docs/s.md"), "s\n").unwrap();
    std::fs::write(
        dir.join(A1),
        record(
            "ADR-1",
            "Accepted",
            "01BBBBBBBBBBBBBBBBBBBBBBBB",
            "",
            "spec:docs/s.md",
        ),
    )
    .unwrap();
    commit_all(&dir, 400);
    let parsed = json(&run_urzua(&dir, &["check"]));
    let found = findings(&parsed, EXPIRED);
    assert_eq!(found.len(), 1, "{parsed}");
    assert_eq!(
        found[0]["severity"], "info",
        "the entry level lowers it: {parsed}"
    );
}

#[test]
fn an_entry_level_cannot_raise_a_finding_above_the_rules_level() {
    let rule = "    level: warn\n    periods:\n      - {type: milestone, status: [Done], days: 30, level: error}\n";
    let dir = fixture("ceiling", &config(rule, ""));
    std::fs::write(
        dir.join(M1),
        record(
            "MILE-1",
            "Done",
            "01AAAAAAAAAAAAAAAAAAAAAAAA",
            "",
            "code:src/a.rs",
        ),
    )
    .unwrap();
    commit_all(&dir, 100);
    let parsed = json(&run_urzua(&dir, &["check"]));
    let found = findings(&parsed, EXPIRED);
    assert_eq!(found.len(), 1, "{parsed}");
    assert_eq!(found[0]["severity"], "warning", "{parsed}");
}

#[test]
fn a_recorded_audit_date_is_the_anchor_not_the_line_commit_observed_failing() {
    let dir = fixture("anchor", &config(RULE, ""));
    // Line committed 200 days ago, re-verified 5 days ago: not expired.
    let recent = day_days_ago(5);
    std::fs::write(
        dir.join(M1),
        record(
            "MILE-1",
            "Done",
            "01AAAAAAAAAAAAAAAAAAAAAAAA",
            &format!("Last-audited: {recent}\n"),
            "code:src/a.rs",
        ),
    )
    .unwrap();
    // Line committed recently but audited 60 days ago: expired, by the date.
    let old = day_days_ago(60);
    std::fs::write(
        dir.join("docs/milestones/MILE-2-b.md"),
        record(
            "MILE-2",
            "Done",
            "01CCCCCCCCCCCCCCCCCCCCCCCC",
            &format!("Last-audited: {old}\n"),
            "code:src/a.rs",
        ),
    )
    .unwrap();
    commit_all(&dir, 2);
    let parsed = json(&run_urzua(&dir, &["check"]));
    let found = findings(&parsed, EXPIRED);
    assert_eq!(found.len(), 1, "{parsed}");
    assert!(found[0]["file"].as_str().unwrap().ends_with("MILE-2-b.md"));
    assert!(
        found[0]["message"]
            .as_str()
            .unwrap()
            .contains(&format!("Last-audited {old}")),
        "{found:?}"
    );
}

#[test]
fn a_future_audit_date_is_malformed_reported_and_falls_back_observed_failing() {
    let dir = fixture("future", &config(RULE, ""));
    std::fs::write(
        dir.join(M1),
        record(
            "MILE-1",
            "Done",
            "01AAAAAAAAAAAAAAAAAAAAAAAA",
            "Last-audited: 2999-01-01\n",
            "code:src/a.rs",
        ),
    )
    .unwrap();
    commit_all(&dir, 100);
    let parsed = json(&run_urzua(&dir, &["check"]));
    let found = findings(&parsed, EXPIRED);
    // One finding that the date is invalid, one that the claim expired by the line.
    assert_eq!(found.len(), 2, "{parsed}");
    assert!(
        found
            .iter()
            .any(|f| f["message"].as_str().unwrap().contains("future")),
        "{found:?}"
    );
    assert!(
        found
            .iter()
            .any(|f| f["message"].as_str().unwrap().contains("line last changed")),
        "{found:?}"
    );
}

/// The first 8 bytes of SHA-256 of the Stable-Id, big-endian, mod `spread + 1`.
fn offset(stable_id: &str, spread: u64) -> u64 {
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
        .write_all(stable_id.as_bytes())
        .unwrap();
    let out = String::from_utf8(child.wait_with_output().unwrap().stdout).unwrap();
    let head = u64::from_str_radix(&out[..16], 16).unwrap();
    head % (spread + 1)
}

#[test]
fn the_first_expiry_is_lengthened_by_a_stable_id_offset_observed_failing() {
    let rule = "    level: warn\n    anchor_field: Last-audited\n    periods:\n      - {type: milestone, status: [Done], days: 10, spread_days: 20}\n";
    let dir = fixture("spread", &config(rule, ""));
    // Pick two Stable-Ids with different offsets, and an age between them.
    let ids = [
        "01EEEEEEEEEEEEEEEEEEEEEEE1",
        "01EEEEEEEEEEEEEEEEEEEEEEE2",
        "01EEEEEEEEEEEEEEEEEEEEEEE3",
        "01EEEEEEEEEEEEEEEEEEEEEEE4",
        "01EEEEEEEEEEEEEEEEEEEEEEE5",
        "01EEEEEEEEEEEEEEEEEEEEEEE6",
    ];
    let offsets: Vec<u64> = ids.iter().map(|i| offset(i, 20)).collect();
    let age = 10 + offsets.iter().copied().min().unwrap() + 1;
    let wants: Vec<bool> = offsets
        .iter()
        .map(|o| u64::from(age as u32) > 10 + o)
        .collect();
    assert!(
        wants.iter().any(|w| *w) && wants.iter().any(|w| !*w),
        "{offsets:?}"
    );
    for (n, id) in ids.iter().enumerate() {
        std::fs::write(
            dir.join(format!("docs/milestones/MILE-{}-m.md", n + 1)),
            record(&format!("MILE-{}", n + 1), "Done", id, "", "code:src/a.rs"),
        )
        .unwrap();
    }
    commit_all(&dir, age as u32);
    let parsed = json(&run_urzua(&dir, &["check"]));
    let found = findings(&parsed, EXPIRED);
    for (n, want) in wants.iter().enumerate() {
        let name = format!("MILE-{}-m.md", n + 1);
        let got = found
            .iter()
            .any(|f| f["file"].as_str().unwrap().ends_with(&name));
        assert_eq!(
            got, *want,
            "{name} offset {} age {age}: {parsed}",
            offsets[n]
        );
    }
    // The finding names the offset so a reviewer sees why it expires that day.
    let sample = found[0]["message"].as_str().unwrap();
    assert!(sample.contains("offset"), "{sample}");
}

#[test]
fn a_valid_audit_date_ends_the_offset() {
    let rule = "    level: warn\n    anchor_field: Last-audited\n    periods:\n      - {type: milestone, status: [Done], days: 10, spread_days: 20}\n";
    let dir = fixture("spread-end", &config(rule, ""));
    // Whatever the offset, 11 days after an audit date a record is expired.
    let audited = day_days_ago(11);
    std::fs::write(
        dir.join(M1),
        record(
            "MILE-1",
            "Done",
            "01EEEEEEEEEEEEEEEEEEEEEEE9",
            &format!("Last-audited: {audited}\n"),
            "code:src/a.rs",
        ),
    )
    .unwrap();
    commit_all(&dir, 1);
    let parsed = json(&run_urzua(&dir, &["check"]));
    assert_eq!(findings(&parsed, EXPIRED).len(), 1, "{parsed}");
}

#[test]
fn a_record_without_a_stable_id_gets_offset_zero_and_is_counted_not_skipped() {
    let rule = "    level: warn\n    periods:\n      - {type: milestone, status: [Done], days: 10, spread_days: 20}\n";
    let dir = fixture("noid", &config(rule, ""));
    std::fs::write(
        dir.join(M1),
        "---\nStatus: Done\nRealized-by: code:src/a.rs\n---\n# MILE-1: T\n",
    )
    .unwrap();
    commit_all(&dir, 11);
    let parsed = json(&run_urzua(&dir, &["check"]));
    assert_eq!(
        findings(&parsed, EXPIRED).len(),
        1,
        "offset 0 expires at 11 of 10: {parsed}"
    );
}

#[test]
fn drift_and_expiry_both_report_neither_suppresses_the_other_observed_failing() {
    let extra = "  embodiment.consistency: warn\n";
    let dir = fixture("both", &config(RULE, extra));
    std::fs::write(
        dir.join(M1),
        record(
            "MILE-1",
            "Done",
            "01AAAAAAAAAAAAAAAAAAAAAAAA",
            "Embodiment: Implemented\n",
            "code:src/a.rs",
        ),
    )
    .unwrap();
    commit_all(&dir, 100);
    std::fs::write(dir.join("src/a.rs"), "fn a() { let _ = 1; }\n").unwrap();
    commit_all(&dir, 50);
    let parsed = json(&run_urzua(&dir, &["check"]));
    assert_eq!(findings(&parsed, EXPIRED).len(), 1, "{parsed}");
    assert_eq!(
        findings(&parsed, "embodiment.consistency").len(),
        1,
        "{parsed}"
    );
}

/// Review finding 8.2: the last valid day of a claim is its anchor plus the period, and the
/// finding names that day. A comparison off by one would expire it a day early.
#[test]
fn a_claim_is_valid_through_its_anchor_plus_the_period_and_expired_the_day_after() {
    let dir = fixture("boundary", &config(RULE, ""));
    std::fs::write(
        dir.join(M1),
        record(
            "MILE-1",
            "Done",
            "01AAAAAAAAAAAAAAAAAAAAAAAA",
            "Last-audited: 2026-01-01\n",
            "code:src/a.rs",
        ),
    )
    .unwrap();
    commit_all(&dir, 400);
    // A 30-day period from 2026-01-01 is valid through 2026-01-31.
    for (as_of, expired) in [
        ("2026-01-30", false),
        ("2026-01-31", false),
        ("2026-02-01", true),
    ] {
        let parsed = json(&run_urzua(&dir, &["--as-of", as_of, "check"]));
        let found = findings(&parsed, EXPIRED);
        assert_eq!(found.len(), usize::from(expired), "as of {as_of}: {parsed}");
        if expired {
            let message = found[0]["message"].as_str().unwrap();
            assert!(message.contains("is 31 days old"), "{message}");
            assert!(message.contains("valid through 2026-01-31"), "{message}");
        }
    }
}

#[test]
fn the_grace_window_does_not_delay_expiry() {
    let extra = "  embodiment.consistency: {level: warn, drift_grace_days: 365}\n";
    let dir = fixture("grace", &config(RULE, extra));
    std::fs::write(
        dir.join(M1),
        record(
            "MILE-1",
            "Done",
            "01AAAAAAAAAAAAAAAAAAAAAAAA",
            "Embodiment: Implemented\n",
            "code:src/a.rs",
        ),
    )
    .unwrap();
    commit_all(&dir, 100);
    std::fs::write(dir.join("src/a.rs"), "fn a() { let _ = 1; }\n").unwrap();
    commit_all(&dir, 1);
    let parsed = json(&run_urzua(&dir, &["check"]));
    assert!(
        findings(&parsed, "embodiment.consistency").is_empty(),
        "{parsed}"
    );
    assert_eq!(findings(&parsed, EXPIRED).len(), 1, "{parsed}");
}

fn load_error(name: &str, cfg: &str) -> String {
    let dir = fixture(name, cfg);
    let out = run_urzua(&dir, &["check"]);
    assert_eq!(out.status.code(), Some(2), "{out:?}");
    String::from_utf8_lossy(&out.stdout).to_string()
}

#[test]
fn every_misdeclaration_is_a_load_error_that_names_it_observed_failing() {
    let cases: &[(&str, &str, &str)] = &[
        ("unknown-type", "    level: warn\n    periods:\n      - {type: nope, days: 5}\n", "nope"),
        ("unknown-status", "    level: warn\n    periods:\n      - {type: milestone, status: [Gone], days: 5}\n", "Gone"),
        ("unknown-tier", "    level: warn\n    periods:\n      - {type: milestone, tier: Platinum, days: 5}\n", "Platinum"),
        ("zero-days", "    level: warn\n    periods:\n      - {type: milestone, days: 0}\n", "days"),
        ("level-off", "    level: warn\n    periods:\n      - {type: milestone, days: 5, level: off}\n", "off"),
        ("unknown-key", "    level: warn\n    periods:\n      - {type: milestone, days: 5, hours: 3}\n", "hours"),
        ("same-specificity", "    level: warn\n    periods:\n      - {type: milestone, status: [Done], days: 5}\n      - {type: milestone, status: [Done], days: 9}\n", "milestone"),
        ("anchor-undeclared", "    level: warn\n    anchor_field: Checked-on\n    periods:\n      - {type: milestone, days: 5}\n", "Checked-on"),
        ("pin-category", "    level: warn\n    require_pin: [docs]\n    periods:\n      - {type: milestone, days: 5}\n", "docs"),
        ("batch-zero", "    level: warn\n    max_batch: 0\n    periods:\n      - {type: milestone, days: 5}\n", "max_batch"),
        ("no-periods", "    level: warn\n", "periods"),
    ];
    for (name, rule, needle) in cases {
        let out = load_error(name, &config(rule, ""));
        assert!(out.contains(needle), "{name}: {out}");
    }
}

#[test]
fn declaring_the_old_rule_wide_age_as_well_is_a_load_error_naming_both() {
    let extra = "  embodiment.consistency: {level: warn, claim_max_age_days: 30}\n";
    let dir = fixture("both-opts", &config(RULE, extra));
    let out = run_urzua(&dir, &["check"]);
    assert_eq!(out.status.code(), Some(2), "{out:?}");
    let text = String::from_utf8_lossy(&out.stdout);
    assert!(
        text.contains("claim_max_age_days") && text.contains("embodiment.claim-expired"),
        "{text}"
    );
}

#[test]
fn the_options_belong_to_the_expiry_rule_only() {
    let extra = "  header.required-fields: {level: error, periods: [{type: milestone, days: 3}]}\n";
    let out = load_error("owner", &config(RULE, extra));
    assert!(out.contains("periods"), "{out}");
}

#[test]
fn an_entry_that_matches_no_record_is_reported() {
    let rule = "    level: warn\n    periods:\n      - {type: milestone, status: [Done], days: 30}\n      - {type: adr, status: [Superseded], days: 30}\n";
    let extra = "  config.scope-matches-nothing: warn\n";
    let dir = fixture("matches-nothing", &config(rule, extra));
    std::fs::write(
        dir.join(M1),
        record(
            "MILE-1",
            "Done",
            "01AAAAAAAAAAAAAAAAAAAAAAAA",
            "",
            "code:src/a.rs",
        ),
    )
    .unwrap();
    // An ADR that holds a claim, none of it covered by the Superseded entry.
    std::fs::write(
        dir.join(A1),
        record(
            "ADR-1",
            "Accepted",
            "01BBBBBBBBBBBBBBBBBBBBBBBB",
            "",
            "code:src/a.rs",
        ),
    )
    .unwrap();
    commit_all(&dir, 1);
    let parsed = json(&run_urzua(&dir, &["check"]));
    let found = findings(&parsed, "config.scope-matches-nothing");
    assert!(
        found.iter().any(|f| f["message"]
            .as_str()
            .unwrap()
            .contains("embodiment.claim-expired")),
        "{parsed}"
    );
}

#[test]
fn an_entry_for_a_type_that_holds_no_claim_yet_is_not_reported() {
    let rule = "    level: warn\n    periods:\n      - {type: milestone, status: [Done], days: 30}\n      - {type: adr, status: [Accepted], days: 30}\n";
    let extra = "  config.scope-matches-nothing: warn\n";
    let dir = fixture("no-claims", &config(rule, extra));
    std::fs::write(
        dir.join(M1),
        record(
            "MILE-1",
            "Done",
            "01AAAAAAAAAAAAAAAAAAAAAAAA",
            "",
            "code:src/a.rs",
        ),
    )
    .unwrap();
    commit_all(&dir, 1);
    let parsed = json(&run_urzua(&dir, &["check"]));
    assert!(
        findings(&parsed, "config.scope-matches-nothing").is_empty(),
        "{parsed}"
    );
}

#[test]
fn an_entry_covering_a_claim_that_cannot_be_dated_is_not_reported_as_matching_nothing() {
    // The records are staged and never committed, so no `Realized-by` line has a
    // commit to date it by: the claims are unreadable to the rule, yet each is
    // covered by its entry, so no entry reaches nothing.
    let extra = "  config.scope-matches-nothing: warn\n";
    let dir = fixture("undatable", &config(RULE, extra));
    std::fs::write(
        dir.join(M1),
        record(
            "MILE-1",
            "Done",
            "01AAAAAAAAAAAAAAAAAAAAAAAA",
            "",
            "code:src/a.rs",
        ),
    )
    .unwrap();
    std::fs::write(
        dir.join(A1),
        record(
            "ADR-1",
            "Accepted",
            "01BBBBBBBBBBBBBBBBBBBBBBBB",
            "",
            "code:src/a.rs",
        ),
    )
    .unwrap();
    git(&dir, &["add", "-A"]);
    let parsed = json(&run_urzua(&dir, &["check"]));
    assert!(
        findings(&parsed, "config.scope-matches-nothing").is_empty(),
        "{parsed}"
    );
    let exec = parsed["rules_executed"]
        .as_array()
        .unwrap()
        .iter()
        .find(|e| e["rule"] == EXPIRED)
        .unwrap();
    assert_eq!(exec["population"]["unreadable"], 2, "{exec}");
}

#[test]
fn a_waiver_without_an_expiry_cannot_exempt_the_rule_when_permanent_does_not_list_it_observed_failing(
) {
    let extra = "  waiver.matches-nothing: warn\n";
    let mut cfg = config(RULE, extra);
    cfg.push_str(
        "  waiver:\n    dir: docs/waiver\n    role: suppression\n    effective_requires: [Reason]\n    \
         permanent:\n      - rule: header.required-fields\n        reason_field: Category\n        categories:\n          - {name: perf}\n    \
         required_fields: [Rule, Scope, Reason]\n    known_fields: [Expires, Category]\n    header_shape: yaml-frontmatter\n",
    );
    let dir = fixture("waiver", &cfg);
    std::fs::create_dir_all(dir.join("docs/waiver")).unwrap();
    std::fs::write(
        dir.join(M1),
        record(
            "MILE-1",
            "Done",
            "01AAAAAAAAAAAAAAAAAAAAAAAA",
            "",
            "code:src/a.rs",
        ),
    )
    .unwrap();
    std::fs::write(
        dir.join("docs/waiver/WAIVER-1-x.md"),
        format!("---\nRule: {EXPIRED}\nScope: {M1}\nReason: r\n---\n# x\n"),
    )
    .unwrap();
    commit_all(&dir, 100);
    let parsed = json(&run_urzua(&dir, &["check"]));
    let found = findings(&parsed, EXPIRED);
    assert_eq!(found.len(), 1, "{parsed}");
    assert!(
        found[0]["waived"].is_null(),
        "a permanent waiver of an unlisted rule: {parsed}"
    );
    assert!(
        findings(&parsed, "waiver.matches-nothing")
            .iter()
            .any(|f| f["message"].as_str().unwrap().contains("not in effect")),
        "{parsed}"
    );

    // With an Expires it takes effect, and it does not renew the claim.
    std::fs::write(
        dir.join("docs/waiver/WAIVER-1-x.md"),
        format!("---\nRule: {EXPIRED}\nScope: {M1}\nReason: r\nExpires: 2999-01-01\n---\n# x\n"),
    )
    .unwrap();
    commit_all(&dir, 0);
    let parsed = json(&run_urzua(&dir, &["check"]));
    let found = findings(&parsed, EXPIRED);
    assert_eq!(found.len(), 1, "{parsed}");
    assert!(found[0]["waived"].is_string(), "{parsed}");
}

#[test]
fn a_hand_typed_audit_date_is_reported_by_the_companion_rule_observed_failing() {
    let extra = "  embodiment.claim-renewal-unattested: warn\n";
    let dir = fixture("unattested", &config(RULE, extra));
    let old = day_days_ago(20);
    std::fs::write(
        dir.join(M1),
        record(
            "MILE-1",
            "Done",
            "01AAAAAAAAAAAAAAAAAAAAAAAA",
            &format!("Last-audited: {old}\n"),
            "code:src/a.rs",
        ),
    )
    .unwrap();
    std::fs::write(
        dir.join("docs/milestones/MILE-2-b.md"),
        record(
            "MILE-2",
            "Done",
            "01CCCCCCCCCCCCCCCCCCCCCCCC",
            "",
            "code:src/a.rs",
        ),
    )
    .unwrap();
    commit_all(&dir, 20);
    git(&dir, &["branch", "-M", "main"]);
    git(&dir, &["checkout", "-q", "-b", "topic"]);
    // MILE-1: a newer date typed in with no row. MILE-2: field added with no row.
    let today = day_days_ago(0);
    let text = std::fs::read_to_string(dir.join(M1)).unwrap();
    std::fs::write(dir.join(M1), text.replace(&old, &today)).unwrap();
    let text = std::fs::read_to_string(dir.join("docs/milestones/MILE-2-b.md")).unwrap();
    std::fs::write(
        dir.join("docs/milestones/MILE-2-b.md"),
        text.replace(
            "Realized-by:",
            &format!("Last-audited: {today}\nRealized-by:"),
        ),
    )
    .unwrap();
    commit_all(&dir, 0);
    let parsed = json(&run_urzua(&dir, &["check", "--base", "main"]));
    let found = findings(&parsed, "embodiment.claim-renewal-unattested");
    assert_eq!(found.len(), 2, "{parsed}");

    // The same date with a Re-verified row for that day is attested.
    let text = std::fs::read_to_string(dir.join(M1)).unwrap();
    std::fs::write(
        dir.join(M1),
        format!(
            "{text}> | {today} | Re-verified (attested by ana, identity: asserted, actor: human): 1 locators re-resolved, 1 hash-checked, 0 spec locators checked by git history only. Evidence: src/a.rs Reason: read it. **Why:** the claim was 20 days old against a 30-day period. | **structural** |\n"
        ),
    )
    .unwrap();
    commit_all(&dir, 0);
    let parsed = json(&run_urzua(&dir, &["check", "--base", "main"]));
    let found = findings(&parsed, "embodiment.claim-renewal-unattested");
    assert_eq!(found.len(), 1, "only MILE-2 remains: {parsed}");
    assert!(found[0]["file"].as_str().unwrap().ends_with("MILE-2-b.md"));

    // Removing the field the base carried is reported too.
    let dir2 = fixture("unattested-removed", &config(RULE, extra));
    std::fs::write(
        dir2.join(M1),
        record(
            "MILE-1",
            "Done",
            "01AAAAAAAAAAAAAAAAAAAAAAAA",
            &format!("Last-audited: {old}\n"),
            "code:src/a.rs",
        ),
    )
    .unwrap();
    commit_all(&dir2, 20);
    git(&dir2, &["branch", "-M", "main"]);
    git(&dir2, &["checkout", "-q", "-b", "topic"]);
    let text = std::fs::read_to_string(dir2.join(M1)).unwrap();
    std::fs::write(
        dir2.join(M1),
        text.replace(&format!("Last-audited: {old}\n"), ""),
    )
    .unwrap();
    commit_all(&dir2, 0);
    let parsed = json(&run_urzua(&dir2, &["check", "--base", "main"]));
    let found = findings(&parsed, "embodiment.claim-renewal-unattested");
    assert_eq!(found.len(), 1, "{parsed}");
    assert!(
        found[0]["message"].as_str().unwrap().contains("removed"),
        "{found:?}"
    );
}

#[test]
fn a_row_that_is_not_the_tools_shape_does_not_attest_a_date_observed_failing() {
    let extra = "  embodiment.claim-renewal-unattested: warn\n";
    let dir = fixture("unattested-shape", &config(RULE, extra));
    let old = day_days_ago(20);
    std::fs::write(
        dir.join(M1),
        record(
            "MILE-1",
            "Done",
            "01AAAAAAAAAAAAAAAAAAAAAAAA",
            &format!("Last-audited: {old}\n"),
            "code:src/a.rs",
        ),
    )
    .unwrap();
    commit_all(&dir, 20);
    git(&dir, &["branch", "-M", "main"]);
    git(&dir, &["checkout", "-q", "-b", "topic"]);
    let today = day_days_ago(0);
    let text = std::fs::read_to_string(dir.join(M1)).unwrap();
    std::fs::write(
        dir.join(M1),
        format!(
            "{}> | {today} | Re-verified (attested by ana): I looked. | **structural** |\n",
            text.replace(&old, &today)
        ),
    )
    .unwrap();
    commit_all(&dir, 0);
    let parsed = json(&run_urzua(&dir, &["check", "--base", "main"]));
    let found = findings(&parsed, "embodiment.claim-renewal-unattested");
    assert_eq!(found.len(), 1, "{parsed}");
}

/// Review finding 8.1: a `Re-verified` row attests the date it is dated, and only that date. A
/// genuine row of another day does not attest an anchor typed in for a later one.
#[test]
fn a_reverified_row_of_another_day_does_not_attest_a_moved_anchor() {
    let extra = "  embodiment.claim-renewal-unattested: warn\n";
    let dir = fixture("unattested-other-day", &config(RULE, extra));
    let old = day_days_ago(20);
    std::fs::write(
        dir.join(M1),
        record(
            "MILE-1",
            "Done",
            "01AAAAAAAAAAAAAAAAAAAAAAAA",
            &format!("Last-audited: {old}\n"),
            "code:src/a.rs",
        ),
    )
    .unwrap();
    commit_all(&dir, 20);
    git(&dir, &["branch", "-M", "main"]);
    git(&dir, &["checkout", "-q", "-b", "topic"]);
    let (yesterday, today) = (day_days_ago(1), day_days_ago(0));
    let row = |day: &str| {
        format!(
            "> | {day} | Re-verified (attested by ana, identity: asserted, actor: human): 1 locators re-resolved, 1 hash-checked, 0 spec locators checked by git history only. Evidence: src/a.rs Reason: read it. **Why:** the claim was 20 days old against a 30-day period. | **structural** |\n"
        )
    };
    let text = std::fs::read_to_string(dir.join(M1)).unwrap();
    // The row is real and dated yesterday; the anchor says today.
    std::fs::write(
        dir.join(M1),
        format!("{}{}", text.replace(&old, &today), row(&yesterday)),
    )
    .unwrap();
    commit_all(&dir, 0);
    let parsed = json(&run_urzua(&dir, &["check", "--base", "main"]));
    let found = findings(&parsed, "embodiment.claim-renewal-unattested");
    assert_eq!(
        found.len(),
        1,
        "a row of another day attests nothing: {parsed}"
    );
    assert!(
        found[0]["message"]
            .as_str()
            .unwrap()
            .contains(&format!("moved to {today}")),
        "{found:?}"
    );

    // The same row dated the day the anchor names does attest it.
    let text = std::fs::read_to_string(dir.join(M1)).unwrap();
    std::fs::write(dir.join(M1), format!("{text}{}", row(&today))).unwrap();
    commit_all(&dir, 0);
    let parsed = json(&run_urzua(&dir, &["check", "--base", "main"]));
    assert!(
        findings(&parsed, "embodiment.claim-renewal-unattested").is_empty(),
        "{parsed}"
    );
}

#[test]
fn a_malformed_anchor_is_reported_as_malformed_not_removed() {
    let extra = "  embodiment.claim-renewal-unattested: warn\n";
    let dir = fixture("unattested-malformed", &config(RULE, extra));
    let old = day_days_ago(20);
    std::fs::write(
        dir.join(M1),
        record(
            "MILE-1",
            "Done",
            "01AAAAAAAAAAAAAAAAAAAAAAAA",
            &format!("Last-audited: {old}\n"),
            "code:src/a.rs",
        ),
    )
    .unwrap();
    commit_all(&dir, 20);
    git(&dir, &["branch", "-M", "main"]);
    git(&dir, &["checkout", "-q", "-b", "topic"]);
    let text = std::fs::read_to_string(dir.join(M1)).unwrap();
    std::fs::write(
        dir.join(M1),
        text.replace(
            &format!("Last-audited: {old}\n"),
            "Last-audited: not-a-date\n",
        ),
    )
    .unwrap();
    commit_all(&dir, 0);
    let parsed = json(&run_urzua(&dir, &["check", "--base", "main"]));
    let found = findings(&parsed, "embodiment.claim-renewal-unattested");
    assert_eq!(found.len(), 1, "{parsed}");
    let message = found[0]["message"].as_str().unwrap();
    assert!(message.contains("is malformed"), "{message}");
    assert!(!message.contains("was removed"), "{message}");
}

#[test]
fn a_removed_and_a_blank_anchor_are_each_reported_as_what_they_are() {
    let extra = "  embodiment.claim-renewal-unattested: warn\n";
    for (name, replacement, says, not_says) in [
        ("unattested-removed", "", "was removed", "is empty"),
        (
            "unattested-blank",
            "Last-audited:\n",
            "is empty",
            "was removed",
        ),
    ] {
        let dir = fixture(name, &config(RULE, extra));
        let old = day_days_ago(20);
        std::fs::write(
            dir.join(M1),
            record(
                "MILE-1",
                "Done",
                "01AAAAAAAAAAAAAAAAAAAAAAAA",
                &format!("Last-audited: {old}\n"),
                "code:src/a.rs",
            ),
        )
        .unwrap();
        commit_all(&dir, 20);
        git(&dir, &["branch", "-M", "main"]);
        git(&dir, &["checkout", "-q", "-b", "topic"]);
        let text = std::fs::read_to_string(dir.join(M1)).unwrap();
        std::fs::write(
            dir.join(M1),
            text.replace(&format!("Last-audited: {old}\n"), replacement),
        )
        .unwrap();
        commit_all(&dir, 0);
        let parsed = json(&run_urzua(&dir, &["check", "--base", "main"]));
        let found = findings(&parsed, "embodiment.claim-renewal-unattested");
        assert_eq!(found.len(), 1, "{name}: {parsed}");
        let message = found[0]["message"].as_str().unwrap();
        assert!(message.contains(says), "{name}: {message}");
        assert!(!message.contains(not_says), "{name}: {message}");
        assert!(!message.contains("is malformed"), "{name}: {message}");
    }
}

#[test]
fn the_companion_rule_needs_the_expiry_rule() {
    let cfg = "schema_version: 2\nrules:\n  embodiment.claim-renewal-unattested: warn\nrecord_types:\n  milestone:\n    dir: docs/milestones\n    prefix: MILE\n    header_shape: yaml-frontmatter\n    required_fields: [Status]\n";
    let out = load_error("companion-alone", cfg);
    assert!(out.contains("embodiment.claim-expired"), "{out}");
}

const ITEMS_DELIVERY: &str = "      delivery:\n        delivered_tiers: [Implemented, Verified]\n        unevidenced_tiers: [\"Not started\"]\n        not_done: [Planned]\n        set_status: Done\n        items:\n          section: \"Done means\"\n          clause_separator: \" — \"\n          evidence_marker: \"evidence:\"\n          waiver_marker: \"waived:\"\n          partial_status: \"Planned\"\n          resolving_tiers: [Specified, Implemented, Verified]\n          waiver_categories:\n            - {name: not-applicable}\n";

/// `config`, with the milestone type reading Done means as items.
fn items_config() -> String {
    let base = config(
        RULE,
        "  status.delivery-agreement: warn\n  status.done-items-shape: warn\n",
    );
    let marker = "      claim_retained: [Superseded]\n  adr:";
    assert!(base.contains(marker));
    base.replace(
        marker,
        &format!("      claim_retained: [Superseded]\n{ITEMS_DELIVERY}    sections:\n      from: headings\n      required:\n        - {{name: Done means}}\n  adr:"),
    )
}

/// A milestone with no `Realized-by`, whose evidence is its Done means items.
fn item_only_record(stable_id: &str, items: &str) -> String {
    format!(
        "---\nStable-Id: {stable_id}\nStatus: Done\n---\n# MILE-1: T\n\n## Done means\n\n{items}\n## Revision log\n\n> **Revision log**\n>\n> | Date | Change | Class |\n> |---|---|---|\n> | 2026-01-01 | Initial. | **structural** |\n"
    )
}

#[test]
fn a_milestone_whose_evidence_is_only_done_means_items_expires_after_its_period_observed_failing() {
    let dir = fixture("itemonly", &items_config());
    std::fs::write(
        dir.join(M1),
        item_only_record(
            "01AAAAAAAAAAAAAAAAAAAAAAAA",
            "- It is built. — evidence: code:src/a.rs\n",
        ),
    )
    .unwrap();
    commit_all(&dir, 40);
    let parsed = json(&run_urzua(&dir, &["check"]));
    let found = findings(&parsed, EXPIRED);
    assert_eq!(found.len(), 1, "{parsed}");
    assert!(
        found[0]["message"].as_str().unwrap().contains("40 days"),
        "{parsed}"
    );
}

#[test]
fn an_item_only_milestone_inside_its_period_is_silent() {
    let dir = fixture("itemfresh", &items_config());
    std::fs::write(
        dir.join(M1),
        item_only_record(
            "01AAAAAAAAAAAAAAAAAAAAAAAA",
            "- It is built. — evidence: code:src/a.rs\n",
        ),
    )
    .unwrap();
    commit_all(&dir, 5);
    let parsed = json(&run_urzua(&dir, &["check"]));
    assert!(findings(&parsed, EXPIRED).is_empty(), "{parsed}");
}

#[test]
fn a_record_with_neither_a_locator_field_nor_item_evidence_stays_not_examined() {
    let dir = fixture("noevidence", &items_config());
    std::fs::write(
        dir.join(M1),
        item_only_record("01AAAAAAAAAAAAAAAAAAAAAAAA", "- It is built.\n"),
    )
    .unwrap();
    commit_all(&dir, 40);
    let parsed = json(&run_urzua(&dir, &["check"]));
    assert!(findings(&parsed, EXPIRED).is_empty(), "{parsed}");
}

/// A prefix-lines type: bare `Key: value` lines, no front matter.
fn prefix_lines_config(extra_rules: &str) -> String {
    format!(
        "schema_version: 2\nrules:\n  embodiment.claim-expired:\n    level: warn\n    anchor_field: Last-audited\n    require_pin: []\n    periods:\n      - {{type: adr, status: [Accepted], days: 90}}\n{extra_rules}\
         record_types:\n  adr:\n    dir: docs/adr\n    prefix: ADR\n    \
         required_fields: [Status]\n    known_fields: [Embodiment, Realized-by, Last-audited]\n    \
         identity: {{from: filename, pattern: '^(?P<prefix>ADR)-(?P<number>\\d+)-(?P<slug>.+)$'}}\n    \
         fields: {{from: prefix-lines}}\n    \
         statuses:\n      values: [Proposed, Accepted]\n      success: [Accepted]\n"
    )
}

fn prefix_lines_record(audited: &str) -> String {
    format!(
        "# ADR-1. T\n\nStatus: Accepted\nLast-audited: {audited}\nRealized-by: code:src/a.rs\n\n> **Revision log**\n>\n> | Date | Change | Class |\n> |---|---|---|\n> | 2026-01-01 | Initial. | **structural** |\n"
    )
}

#[test]
fn a_hand_typed_audit_date_on_a_prefix_lines_type_is_reported_by_the_companion_rule() {
    // The companion rule read both copies with the front-matter header, which
    // finds no field in a prefix-lines record, so the date moved unseen.
    let extra = "  embodiment.claim-renewal-unattested: warn\n";
    let dir = fixture("unattested-prefix-lines", &prefix_lines_config(extra));
    let old = day_days_ago(20);
    std::fs::write(dir.join(A1), prefix_lines_record(&old)).unwrap();
    commit_all(&dir, 20);
    git(&dir, &["branch", "-M", "main"]);
    git(&dir, &["checkout", "-q", "-b", "topic"]);
    let today = day_days_ago(0);
    let text = std::fs::read_to_string(dir.join(A1)).unwrap();
    std::fs::write(dir.join(A1), text.replace(&old, &today)).unwrap();
    commit_all(&dir, 0);
    let parsed = json(&run_urzua(&dir, &["check", "--base", "main"]));
    let found = findings(&parsed, "embodiment.claim-renewal-unattested");
    assert_eq!(found.len(), 1, "{parsed}");
    assert_eq!(found[0]["file"], A1);
    assert!(
        found[0]["message"]
            .as_str()
            .unwrap()
            .contains("Re-verified"),
        "{found:?}"
    );
}

#[test]
fn removing_the_audit_date_from_a_prefix_lines_record_is_reported() {
    let extra = "  embodiment.claim-renewal-unattested: warn\n";
    let dir = fixture(
        "unattested-prefix-lines-removed",
        &prefix_lines_config(extra),
    );
    let old = day_days_ago(20);
    std::fs::write(dir.join(A1), prefix_lines_record(&old)).unwrap();
    commit_all(&dir, 20);
    git(&dir, &["branch", "-M", "main"]);
    git(&dir, &["checkout", "-q", "-b", "topic"]);
    let text = std::fs::read_to_string(dir.join(A1)).unwrap();
    std::fs::write(
        dir.join(A1),
        text.replace(&format!("Last-audited: {old}\n"), ""),
    )
    .unwrap();
    commit_all(&dir, 0);
    let parsed = json(&run_urzua(&dir, &["check", "--base", "main"]));
    let found = findings(&parsed, "embodiment.claim-renewal-unattested");
    assert_eq!(found.len(), 1, "{parsed}");
    assert!(
        found[0]["message"].as_str().unwrap().contains("removed"),
        "{found:?}"
    );
}
