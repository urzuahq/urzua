//! A milestone declares its kind (`ADR-96`, amendment of 2026-11-07): the type
//! declares a field and a vocabulary under `statuses.delivery.items.kind`, and
//! each kind names the tiers that count as delivered and whether a record an item
//! cites must have reached a `success` status of its own type. A `decision`
//! milestone is done when every item cites a decided record; a `build` milestone,
//! or one that declares no kind, keeps the rule that needs a delivered tier.
//! Every test runs the real binary over a fixture corpus.
//!
//! Verifies: MILE-214

mod common;

use std::path::Path;
use std::process::Command;

const KIND: &str = "          kind:
            field: Kind
            kinds:
              - {name: build}
              - name: decision
                delivered_tiers: [Specified, Implemented, Verified]
                cited_records: success
";

const ITEMS: &str = "      delivery:
        delivered_tiers: [Implemented, Verified]
        unevidenced_tiers: [\"Not started\"]
        not_done: [Planned, InProgress]
        set_status: Done
        items:
          section: \"Done means\"
          clause_separator: \" — \"
          evidence_marker: \"evidence:\"
          waiver_marker: \"waived:\"
          partial_status: \"InProgress\"
          resolving_tiers: [Specified, Implemented, Verified]
";

fn items(kind: &str) -> String {
    format!("{ITEMS}{kind}")
}

fn config(delivery: &str, known: &str) -> String {
    format!(
        "schema_version: 2
rules:
  status.delivery-agreement: error
  status.done-items-shape: error
  embodiment.locator-exists: error
record_types:
  milestone:
    dir: docs/milestones
    prefix: MILE
    header_shape: yaml-frontmatter
    required_fields: [Status]
    known_fields: [{known}]
    sections:
      from: headings
      required:
        - {{name: Done means}}
    statuses:
      values: [Planned, InProgress, Done, WontDo]
      success: [Done]
      failure: [WontDo]
      progress: [InProgress, Done]
{delivery}  adr:
    dir: docs/adr
    prefix: ADR
    header_shape: yaml-frontmatter
    required_fields: [Status]
    statuses:
      values: [Proposed, Accepted, Rejected]
      success: [Accepted]
      failure: [Rejected]
  spec:
    dir: docs/specs
    prefix: SPEC
    header_shape: yaml-frontmatter
    required_fields: [Status]
    statuses:
      values: [Draft, Accepted, Shipped]
      success: [Accepted, Shipped]
"
    )
}

fn declared() -> String {
    config(&items(KIND), "Realized-by, Kind")
}

fn git(dir: &Path, args: &[&str]) {
    assert!(Command::new("git")
        .args(args)
        .current_dir(dir)
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .status()
        .unwrap()
        .success());
}

fn milestone(status: &str, kind: Option<&str>, done_means: &str) -> String {
    let kind = kind.map_or(String::new(), |k| format!("Kind: {k}\n"));
    format!(
        "---\nStatus: {status}\n{kind}---\n# X\n\n## Done means\n\n{done_means}\n## Revision log\n\n> **Revision log**\n>\n> | Date | Change | Class |\n> |---|---|---|\n> | 2026-11-07 | Filed. | **substantive** |\n"
    )
}

fn repo(name: &str, config: &str, milestones: &[(u32, String)]) -> common::TempDir {
    let dir = common::TempDir::new(&format!("urzua-kind-{name}"));
    for d in ["docs/milestones", "docs/adr", "docs/specs", "src", ".urzua"] {
        std::fs::create_dir_all(dir.join(d)).unwrap();
    }
    std::fs::write(dir.join(".urzua/config.yaml"), config).unwrap();
    for (n, content) in milestones {
        std::fs::write(dir.join(format!("docs/milestones/MILE-{n}-m.md")), content).unwrap();
    }
    for (file, status) in [
        ("docs/adr/ADR-1-accepted.md", "Accepted"),
        ("docs/adr/ADR-2-proposed.md", "Proposed"),
        ("docs/specs/SPEC-1-shipped.md", "Shipped"),
        ("docs/specs/SPEC-2-draft.md", "Draft"),
    ] {
        std::fs::write(dir.join(file), format!("---\nStatus: {status}\n---\n# R\n")).unwrap();
    }
    std::fs::write(dir.join("src/a.rs"), "fn a() {}\n").unwrap();
    git(&dir, &["init", "-q"]);
    git(&dir, &["add", "-A"]);
    dir
}

fn run(dir: &Path, args: &[&str]) -> std::process::Output {
    common::urzua()
        .args(args)
        .current_dir(dir)
        .output()
        .unwrap()
}

fn parsed(out: &std::process::Output) -> serde_json::Value {
    serde_json::from_slice(&out.stdout).unwrap_or_else(|e| {
        panic!(
            "json: {e}\n{}\n{}",
            String::from_utf8_lossy(&out.stdout),
            String::from_utf8_lossy(&out.stderr)
        )
    })
}

fn findings<'a>(report: &'a serde_json::Value, rule: &str) -> Vec<&'a serde_json::Value> {
    report["findings"]
        .as_array()
        .map(|a| a.iter().filter(|f| f["rule"] == rule).collect())
        .unwrap_or_default()
}

fn message(finding: &serde_json::Value) -> &str {
    finding["message"].as_str().unwrap()
}

const ACCEPTED_ADR: &str = "- It is decided. — evidence: spec:docs/adr/ADR-1-accepted.md\n";
const PROPOSED_ADR: &str = "- It is decided. — evidence: spec:docs/adr/ADR-2-proposed.md\n";
const SHIPPED_SPEC: &str = "- It is specified. — evidence: spec:docs/specs/SPEC-1-shipped.md\n";
const DRAFT_SPEC: &str = "- It is specified. — evidence: spec:docs/specs/SPEC-2-draft.md\n";

fn apply(dir: &Path) -> String {
    let out = run(
        dir,
        &[
            "fix",
            "--apply",
            "--ids",
            "docs/milestones/MILE-1-m.md",
            "--by",
            "tester",
        ],
    );
    assert!(
        out.status.success(),
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    std::fs::read_to_string(dir.join("docs/milestones/MILE-1-m.md")).unwrap()
}

#[test]
fn a_decision_milestone_citing_an_accepted_decision_computes_done() {
    let body = format!("{ACCEPTED_ADR}{SHIPPED_SPEC}");
    let dir = repo(
        "decision-done",
        &declared(),
        &[(1, milestone("Planned", Some("decision"), &body))],
    );
    let report = parsed(&run(&dir, &["check"]));
    let found = findings(&report, "status.delivery-agreement");
    assert_eq!(found.len(), 1, "{report}");
    assert!(
        message(found[0]).contains("set Status to 'Done'"),
        "{report}"
    );
    let text = apply(&dir);
    assert!(text.contains("Status: Done"), "{text}");
    assert!(text.contains("2 of 2 items resolved"), "{text}");
    let again = parsed(&run(&dir, &["check"]));
    assert!(
        findings(&again, "status.delivery-agreement").is_empty(),
        "{again}"
    );
}

#[test]
fn a_decision_milestone_evidenced_only_by_a_proposed_adr_is_not_done() {
    // Planned: nothing is computed delivered, so the status is left alone.
    let dir = repo(
        "decision-proposed-planned",
        &declared(),
        &[(1, milestone("Planned", Some("decision"), PROPOSED_ADR))],
    );
    let report = parsed(&run(&dir, &["check"]));
    assert!(
        findings(&report, "status.delivery-agreement").is_empty(),
        "{report}"
    );
    let fix = parsed(&run(&dir, &["fix"]));
    assert!(
        fix["repairs"].as_array().is_none_or(|r| r.is_empty()),
        "fix must not compute Done from a Proposed record: {fix}"
    );
    // Done: the item is unresolved and the record cannot claim Done. The cause
    // names the cited record and its status.
    let dir = repo(
        "decision-proposed-done",
        &declared(),
        &[(1, milestone("Done", Some("decision"), PROPOSED_ADR))],
    );
    let report = parsed(&run(&dir, &["check"]));
    let found = findings(&report, "status.delivery-agreement");
    assert_eq!(found.len(), 1, "{report}");
    assert!(message(found[0]).contains("ADR-2"), "{report}");
    assert!(message(found[0]).contains("Proposed"), "{report}");
}

#[test]
fn a_draft_spec_does_not_count_and_a_shipped_one_does() {
    let dir = repo(
        "draft-spec",
        &declared(),
        &[(1, milestone("Done", Some("decision"), DRAFT_SPEC))],
    );
    let report = parsed(&run(&dir, &["check"]));
    let found = findings(&report, "status.delivery-agreement");
    assert_eq!(found.len(), 1, "{report}");
    assert!(message(found[0]).contains("SPEC-2"), "{report}");
    assert!(message(found[0]).contains("Draft"), "{report}");
    let dir = repo(
        "shipped-spec",
        &declared(),
        &[(1, milestone("Done", Some("decision"), SHIPPED_SPEC))],
    );
    let report = parsed(&run(&dir, &["check"]));
    assert!(
        findings(&report, "status.delivery-agreement").is_empty(),
        "{report}"
    );
}

#[test]
fn one_undecided_item_leaves_a_decision_milestone_partial() {
    let body = format!("{ACCEPTED_ADR}- It is open.\n");
    let dir = repo(
        "decision-partial",
        &declared(),
        &[(1, milestone("Planned", Some("decision"), &body))],
    );
    let text = apply(&dir);
    assert!(text.contains("Status: InProgress"), "{text}");
    let body = format!("{ACCEPTED_ADR}{PROPOSED_ADR}");
    let dir = repo(
        "decision-mixed",
        &declared(),
        &[(1, milestone("Planned", Some("decision"), &body))],
    );
    let text = apply(&dir);
    assert!(
        text.contains("Status: InProgress"),
        "an accepted and a proposed record is partial: {text}"
    );
}

#[test]
fn a_build_milestone_and_one_without_a_kind_keep_the_delivered_tier_rule() {
    for (name, kind) in [("build", Some("build")), ("no-kind", None)] {
        // Specified-only evidence is not delivery for a build milestone, even
        // when the cited record is Accepted.
        let dir = repo(
            name,
            &declared(),
            &[(1, milestone("Planned", kind, ACCEPTED_ADR))],
        );
        let report = parsed(&run(&dir, &["check"]));
        assert!(
            findings(&report, "status.delivery-agreement").is_empty(),
            "{name}: {report}"
        );
        // A cited record's status is not read for a kind that does not ask: a
        // Proposed record still resolves at Specified, as before.
        let dir = repo(
            name,
            &declared(),
            &[(1, milestone("Done", kind, PROPOSED_ADR))],
        );
        let report = parsed(&run(&dir, &["check"]));
        assert!(
            findings(&report, "status.delivery-agreement").is_empty(),
            "{name}: {report}"
        );
        // And code evidence delivers it.
        let dir = repo(
            name,
            &declared(),
            &[(
                1,
                milestone(
                    "Planned",
                    kind,
                    "- It is built. — evidence: code:src/a.rs\n",
                ),
            )],
        );
        let text = apply(&dir);
        assert!(text.contains("Status: Done"), "{name}: {text}");
    }
}

#[test]
fn a_config_without_kind_ignores_the_field() {
    // `Kind` is only a field here: with no `kind` declared, a decision label
    // changes nothing and the result equals the no-kind control.
    let cfg = config(ITEMS, "Realized-by, Kind");
    let dir = repo(
        "undeclared",
        &cfg,
        &[(1, milestone("Planned", Some("decision"), ACCEPTED_ADR))],
    );
    let report = parsed(&run(&dir, &["check"]));
    assert!(
        findings(&report, "status.delivery-agreement").is_empty(),
        "{report}"
    );
}

#[test]
fn a_kind_outside_the_vocabulary_is_reported() {
    let dir = repo(
        "unknown-kind",
        &declared(),
        &[(1, milestone("Planned", Some("research"), ACCEPTED_ADR))],
    );
    let report = parsed(&run(&dir, &["check"]));
    let found = findings(&report, "status.done-items-shape");
    assert_eq!(found.len(), 1, "{report}");
    assert!(message(found[0]).contains("research"), "{report}");
    assert!(message(found[0]).contains("build, decision"), "{report}");
}

#[test]
fn invalid_kind_declarations_are_refused_at_load() {
    let cases: Vec<(String, &str)> = vec![
        (
            config(&items(KIND), "Realized-by"),
            "which the type does not declare",
        ),
        (
            config(
                &items(&KIND.replace("[Specified, Implemented, Verified]", "[Imaginary]")),
                "Realized-by, Kind",
            ),
            "not a declared embodiment tier",
        ),
        (
            config(
                &format!(
                    "{}{}",
                    ITEMS.replace(
                        "resolving_tiers: [Specified, Implemented, Verified]",
                        "resolving_tiers: [Implemented, Verified]"
                    ),
                    KIND
                ),
                "Realized-by, Kind",
            ),
            "not in `resolving_tiers`",
        ),
        (
            config(
                &items(&KIND.replace("{name: build}", "{name: decision}")),
                "Realized-by, Kind",
            ),
            "twice",
        ),
        (
            config(
                &items(&KIND.replace("field: Kind", "field: \"\"")),
                "Realized-by, Kind",
            ),
            "kind.field",
        ),
    ];
    for (cfg, want) in cases {
        let dir = common::TempDir::new("urzua-kind-badcfg");
        std::fs::create_dir_all(dir.join(".urzua")).unwrap();
        std::fs::write(dir.join(".urzua/config.yaml"), cfg).unwrap();
        git(&dir, &["init", "-q"]);
        let out = run(&dir, &["check"]);
        let text = format!(
            "{}{}",
            String::from_utf8_lossy(&out.stdout),
            String::from_utf8_lossy(&out.stderr)
        );
        assert_eq!(out.status.code(), Some(2), "{want}: {text}");
        assert!(text.contains(want), "wanted {want:?}: {text}");
    }
}
