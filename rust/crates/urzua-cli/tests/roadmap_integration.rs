//! MILE-182: `urzua roadmap` (`ADR-82`, `RFC-48`). Waves, the ready set, the
//! critical path and the notes, read from fixtures whose answer is worked out by
//! hand in each test.
//!
//! Verifies: MILE-182

mod common;

use std::path::Path;
use std::process::Command;

fn config(extra: &str) -> String {
    format!(
        "schema_version: 2
roadmap:
  pairs: [sequencing]
{extra}record_types:
  milestone:
    dir: docs/milestones
    prefix: MILE
    required_fields: [Status, Phase]
    known_fields: [Track, Depends-on, Blocks, Blocked-on]
    pointer_fields: [Depends-on, Blocks]
    narrative_fields: [Blocked-on]
    statuses:
      values: [Planned, Done, WontDo]
      success: [Done]
      failure: [WontDo]
  rfc:
    dir: docs/rfc
    prefix: RFC
    required_fields: [Date]
relation_pairs:
  - {{name: sequencing, forward: Depends-on, inverse: Blocks, reciprocity: none, cross_type: true, acyclic: true}}
rules:
  header.required-fields: error
"
    )
}

/// `(number, status, extra header lines)`.
type Milestone<'a> = (u32, &'a str, &'a str);

fn repo(name: &str, config: &str, milestones: &[Milestone], rfcs: &[u32]) -> common::TempDir {
    let dir = common::TempDir::new(&format!("urzua-roadmap-{name}"));
    std::fs::create_dir_all(dir.join("docs/milestones")).unwrap();
    std::fs::create_dir_all(dir.join("docs/rfc")).unwrap();
    std::fs::create_dir_all(dir.join(".urzua")).unwrap();
    std::fs::write(dir.join(".urzua/config.yaml"), config).unwrap();
    for (n, status, extra) in milestones {
        let phase = if n % 2 == 0 { "1" } else { "10" };
        std::fs::write(
            dir.join(format!("docs/milestones/MILE-{n}-m.md")),
            format!("---\nStatus: {status}\nPhase: {phase}\n{extra}---\n# {n} — m\n"),
        )
        .unwrap();
    }
    for n in rfcs {
        std::fs::write(
            dir.join(format!("docs/rfc/RFC-{n}-r.md")),
            format!("---\nDate: 2026-11-05\n---\n# {n} — r\n"),
        )
        .unwrap();
    }
    for args in [&["init", "-q"][..], &["add", "-A"][..]] {
        assert!(Command::new("git")
            .args(args)
            .current_dir(&dir)
            .env("GIT_CONFIG_GLOBAL", "/dev/null")
            .status()
            .unwrap()
            .success());
    }
    dir
}

fn run(dir: &Path, args: &[&str]) -> (i32, String) {
    let out = Command::new(env!("CARGO_BIN_EXE_urzua"))
        .args(args)
        .current_dir(dir)
        .output()
        .expect("urzua binary should run");
    (
        out.status.code().unwrap(),
        String::from_utf8(out.stdout).unwrap(),
    )
}

fn roadmap(dir: &Path) -> serde_json::Value {
    let (code, text) = run(dir, &["roadmap"]);
    assert_eq!(code, 0, "{text}");
    let json: serde_json::Value =
        serde_json::from_str(&text).unwrap_or_else(|e| panic!("not JSON ({e}): {text}"));
    assert_eq!(json["status"], "ok", "{text}");
    json["roadmap"].clone()
}

fn strings(value: &serde_json::Value) -> Vec<&str> {
    value
        .as_array()
        .unwrap_or_else(|| panic!("not an array: {value}"))
        .iter()
        .map(|v| v.as_str().unwrap())
        .collect()
}

fn dep(ids: &[u32]) -> String {
    let list: Vec<String> = ids.iter().map(|n| format!("MILE-{n}")).collect();
    format!("Depends-on: {}\n", list.join(", "))
}

fn notes<'a>(roadmap: &'a serde_json::Value, kind: &str) -> Vec<&'a serde_json::Value> {
    roadmap["notes"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|n| n["kind"] == kind)
        .collect()
}

#[test]
fn waves_ready_blocks_and_the_critical_path_match_the_hand_computed_answer() {
    // 1 Done. 2 <- 1. 3 <- 2. 4 <- 2. 5 <- 3, 4. 6 alone. 10 <- 5.
    let d2 = dep(&[1]);
    let d3 = dep(&[2]);
    let d5 = dep(&[3, 4]);
    let d10 = dep(&[5]);
    let dir = repo(
        "answer",
        &config(""),
        &[
            (1, "Done", ""),
            (2, "Planned", &d2),
            (3, "Planned", &d3),
            (4, "Planned", &d3),
            (5, "Planned", &d5),
            (6, "Planned", ""),
            (10, "Planned", &d10),
        ],
        &[],
    );
    let r = roadmap(&dir);
    assert_eq!(r["schema"], 1);
    assert_eq!(
        r["waves"],
        serde_json::json!([
            ["MILE-2", "MILE-6"],
            ["MILE-3", "MILE-4"],
            ["MILE-5"],
            ["MILE-10"]
        ])
    );
    assert_eq!(strings(&r["ready"]), ["MILE-2", "MILE-6"]);
    assert_eq!(
        strings(&r["critical_path"]),
        ["MILE-2", "MILE-3", "MILE-5", "MILE-10"],
        "the tie between 3 and 4 goes to the smaller id"
    );
    let two = &r["records"]["MILE-2"];
    assert_eq!(two["wave"], 0);
    assert_eq!(two["ready"], true);
    assert_eq!(two["blocked"], false);
    assert_eq!(strings(&two["blocked_by"]), Vec::<&str>::new());
    assert_eq!(
        strings(&two["blocks"]),
        ["MILE-3", "MILE-4", "MILE-5", "MILE-10"],
        "transitive, in natural id order"
    );
    let five = &r["records"]["MILE-5"];
    assert_eq!(strings(&five["blocked_by"]), ["MILE-3", "MILE-4"]);
    assert_eq!(five["blocked"], true);
    assert_eq!(five["ready"], false);
    assert_eq!(five["wave"], 2);
    assert_eq!(five["status"], "Planned");
    assert_eq!(five["type"], "milestone");
    assert!(
        r["records"].get("MILE-1").is_none(),
        "a Done record is not open"
    );
    assert_eq!(r["needs_replan"], serde_json::json!([]));
    assert_eq!(r["cycles"], serde_json::json!([]));
}

#[test]
fn ids_and_map_keys_come_out_in_natural_order() {
    let dir = repo(
        "natural",
        &config(""),
        &[
            (12, "Planned", ""),
            (5, "Planned", ""),
            (100, "Planned", ""),
        ],
        &[],
    );
    let (_, text) = run(&dir, &["roadmap"]);
    let at = |id: &str| text.find(&format!("\"{id}\": {{")).unwrap();
    assert!(
        at("MILE-5") < at("MILE-12") && at("MILE-12") < at("MILE-100"),
        "{text}"
    );
    let r = roadmap(&dir);
    assert_eq!(strings(&r["ready"]), ["MILE-5", "MILE-12", "MILE-100"]);
}

#[test]
fn a_blocker_in_a_failure_terminal_keeps_its_dependents_out_of_every_list() {
    let d3 = dep(&[2]);
    let d4 = dep(&[3]);
    let d5 = dep(&[1]);
    let dir = repo(
        "failure",
        &config(""),
        &[
            (1, "Done", ""),
            (2, "WontDo", ""),
            (3, "Planned", &d3),
            (4, "Planned", &d4),
            (5, "Planned", &d5),
        ],
        &[],
    );
    let r = roadmap(&dir);
    assert_eq!(strings(&r["ready"]), ["MILE-5"]);
    assert_eq!(r["waves"], serde_json::json!([["MILE-5"]]));
    // MILE-5's only dependency is finished, so no open record has an included
    // dependency: there is no chain, and the critical path is empty.
    assert_eq!(r["critical_path"], serde_json::json!([]));
    assert_eq!(
        r["needs_replan"],
        serde_json::json!([
            {"record": "MILE-3", "reason": "failed-dependency", "via": "MILE-2"},
            {"record": "MILE-4", "reason": "depends-on-excluded", "via": "MILE-3"}
        ])
    );
    let three = &r["records"]["MILE-3"];
    assert_eq!(three["wave"], serde_json::Value::Null);
    assert_eq!(three["blocked"], true, "a failed dependency still blocks");
    assert_eq!(strings(&three["blocked_by"]), ["MILE-2"]);
    assert_eq!(strings(&three["blocks"]), ["MILE-4"]);
    assert!(r["records"].get("MILE-2").is_none());
}

#[test]
fn a_cycle_is_listed_once_in_ascending_order_and_never_looped_on() {
    let d2 = dep(&[11]);
    let d11 = dep(&[2]);
    let d12 = dep(&[2]);
    let self_loop = dep(&[7]);
    let dir = repo(
        "cycle",
        &config(""),
        &[
            (2, "Planned", &d2),
            (11, "Planned", &d11),
            (12, "Planned", &d12),
            (7, "Planned", &self_loop),
            (8, "Planned", ""),
        ],
        &[],
    );
    let r = roadmap(&dir);
    assert_eq!(
        r["cycles"],
        serde_json::json!([["MILE-2", "MILE-11"], ["MILE-7"]])
    );
    assert_eq!(
        r["needs_replan"],
        serde_json::json!([
            {"record": "MILE-2", "reason": "cycle", "via": null},
            {"record": "MILE-7", "reason": "cycle", "via": null},
            {"record": "MILE-11", "reason": "cycle", "via": null},
            {"record": "MILE-12", "reason": "depends-on-cycle", "via": "MILE-2"}
        ])
    );
    assert_eq!(r["waves"], serde_json::json!([["MILE-8"]]));
    assert_eq!(strings(&r["ready"]), ["MILE-8"]);
}

#[test]
fn a_loop_across_two_declared_pairs_is_a_cycle_that_check_and_the_roadmap_both_report() {
    let cfg = "schema_version: 2
roadmap:
  pairs: [sequencing, gating]
record_types:
  milestone:
    dir: docs/milestones
    prefix: MILE
    required_fields: [Status, Phase]
    known_fields: [Depends-on, Blocks, Gated-by, Gates]
    pointer_fields: [Depends-on, Blocks, Gated-by, Gates]
relation_pairs:
  - {name: sequencing, forward: Depends-on, inverse: Blocks, reciprocity: none, cross_type: true, acyclic: true}
  - {name: gating, forward: Gated-by, inverse: Gates, reciprocity: none, cross_type: true, acyclic: true}
rules:
  sequence.cycle: error
";
    let dir = repo(
        "crosspair",
        cfg,
        &[
            (2, "Planned", "Depends-on: MILE-3\n"),
            (3, "Planned", "Gated-by: MILE-2\n"),
        ],
        &[],
    );
    // `check` reads the same one graph (MILE-192), so the two agree: it
    // reports the loop on each member the roadmap lists.
    let (_, check) = run(&dir, &["check", "docs/"]);
    let check: serde_json::Value = serde_json::from_str(&check).unwrap();
    let found: Vec<&serde_json::Value> = check["findings"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|f| f["rule"] == "sequence.cycle")
        .collect();
    assert_eq!(found.len(), 2, "{check}");
    for f in found {
        assert!(
            f["message"].as_str().unwrap().contains("crosses"),
            "{check}"
        );
    }
    let r = roadmap(&dir);
    assert_eq!(r["cycles"], serde_json::json!([["MILE-2", "MILE-3"]]));
    assert_eq!(strings(&r["pairs"]), ["sequencing", "gating"]);
}

#[test]
fn the_critical_path_is_empty_when_no_open_record_has_an_included_dependency() {
    // A flat repository: three open records, none depending on another. Naming
    // one of them critical would present an arbitrary id as a chain.
    let dir = repo(
        "flat",
        &config(""),
        &[
            (1, "Planned", ""),
            (2, "Planned", ""),
            (3, "InProgress", ""),
        ],
        &[],
    );
    let r = roadmap(&dir);
    assert_eq!(strings(&r["ready"]), ["MILE-1", "MILE-2", "MILE-3"]);
    assert_eq!(r["critical_path"], serde_json::json!([]));
}

#[test]
fn the_critical_path_is_the_smallest_sequence_not_a_greedy_walk() {
    // 11 <- 12 (short); 11 <- 13 <- 14 <- 19, 100; 13 <- 15 <- 16; a tied chain 30..33.
    let (d12, d13, d14, d15, d16, d19) = (
        dep(&[11]),
        dep(&[11]),
        dep(&[13]),
        dep(&[13]),
        dep(&[15]),
        dep(&[14]),
    );
    let (d31, d32, d33, d100) = (dep(&[30]), dep(&[31]), dep(&[32]), dep(&[14]));
    let dir = repo(
        "tiebreak",
        &config(""),
        &[
            (11, "Planned", ""),
            (12, "Planned", &d12),
            (13, "Planned", &d13),
            (14, "Planned", &d14),
            (15, "Planned", &d15),
            (16, "Planned", &d16),
            (19, "Planned", &d19),
            (100, "Planned", &d100),
            (30, "Planned", ""),
            (31, "Planned", &d31),
            (32, "Planned", &d32),
            (33, "Planned", &d33),
        ],
        &[],
    );
    let r = roadmap(&dir);
    assert_eq!(
        strings(&r["critical_path"]),
        ["MILE-11", "MILE-13", "MILE-14", "MILE-19"],
        "greedy would start 11, 12; text order would end at 100"
    );
}

#[test]
fn a_pair_nothing_carries_and_a_claim_that_resolves_to_nothing_are_notes() {
    let dir = repo(
        "unmatched",
        &config(""),
        &[(1, "Planned", ""), (2, "Planned", "")],
        &[],
    );
    let r = roadmap(&dir);
    let found = notes(&r, "pair-matches-nothing");
    assert_eq!(found.len(), 1, "{r}");
    assert_eq!(found[0]["pair"], "sequencing");
    assert_eq!(strings(&r["ready"]), ["MILE-1", "MILE-2"]);

    let dir = repo(
        "unresolved",
        &config(""),
        &[
            (1, "Planned", "Depends-on: MILE-99, MILE-2\n"),
            (2, "Planned", ""),
            (3, "Planned", "Depends-on: the adopter asks\n"),
        ],
        &[],
    );
    let r = roadmap(&dir);
    assert!(notes(&r, "pair-matches-nothing").is_empty(), "{r}");
    let found = notes(&r, "unresolved-claim");
    assert_eq!(found.len(), 2, "{r}");
    assert_eq!(found[0]["record"], "MILE-1");
    assert_eq!(found[0]["field"], "Depends-on");
    assert!(found[0]["message"].as_str().unwrap().contains("MILE-99"));
    assert_eq!(found[1]["record"], "MILE-3");
    // The resolvable half of MILE-1's claim still orders.
    assert_eq!(r["records"]["MILE-1"]["wave"], 1);
}

#[test]
fn no_declared_pair_is_a_stated_absence_not_an_empty_roadmap() {
    let cfg = config("").replace("roadmap:\n  pairs: [sequencing]\n", "");
    let dir = repo(
        "nopairs",
        &cfg,
        &[(1, "Planned", "Depends-on: MILE-2\n"), (2, "Planned", "")],
        &[],
    );
    let r = roadmap(&dir);
    assert_eq!(r["schema"], 1);
    assert_eq!(r["records"], serde_json::json!({}));
    assert_eq!(r["waves"], serde_json::json!([]));
    assert_eq!(r["ready"], serde_json::json!([]));
    assert_eq!(r["critical_path"], serde_json::json!([]));
    assert_eq!(r["pairs"], serde_json::json!([]));
    let found = notes(&r, "no-sequencing-pair");
    assert_eq!(found.len(), 1, "{r}");
    assert!(found[0]["message"].as_str().unwrap().contains("sequencing"));
    // The export carries the roadmap only when it is declared.
    let (_, export) = run(&dir, &["export", "--format", "json"]);
    let export: serde_json::Value = serde_json::from_str(&export).unwrap();
    assert!(export.get("roadmap").is_none(), "{export}");
}

#[test]
fn a_declared_roadmap_is_part_of_the_json_export() {
    let dir = repo(
        "export",
        &config(""),
        &[(1, "Planned", ""), (2, "Planned", &dep(&[1]))],
        &[],
    );
    let (code, export) = run(&dir, &["export", "--format", "json"]);
    assert_eq!(code, 0);
    let export: serde_json::Value = serde_json::from_str(&export).unwrap();
    assert_eq!(export["roadmap"], roadmap(&dir));
}

#[test]
fn groups_nest_in_group_by_order_with_the_null_bucket_last() {
    let cfg = config("  group_by: [Phase, Track]\n");
    let dir = repo(
        "groups",
        &cfg,
        &[
            (1, "Planned", "Track: x\n"),
            (2, "Planned", "Track: x\n"),
            (3, "Planned", ""),
            (4, "Planned", "Track: y\n"),
            (5, "Done", "Track: x\n"),
        ],
        &[],
    );
    let r = roadmap(&dir);
    // Phase is 1 for even ids and 10 for odd ones; 1 sorts before 10 as a number.
    assert_eq!(
        r["groups"],
        serde_json::json!([
            {"field": "Phase", "value": "1", "groups": [
                {"field": "Track", "value": "x", "records": ["MILE-2"]},
                {"field": "Track", "value": "y", "records": ["MILE-4"]}]},
            {"field": "Phase", "value": "10", "groups": [
                {"field": "Track", "value": "x", "records": ["MILE-1"]},
                {"field": "Track", "value": null, "records": ["MILE-3"]}]}
        ])
    );

    let dir = repo(
        "groups-unset",
        &config("  group_by: [Track]\n"),
        &[(1, "Planned", "")],
        &[],
    );
    let r = roadmap(&dir);
    assert_eq!(
        r["groups"],
        serde_json::json!([{"field": "Track", "value": null, "records": ["MILE-1"]}])
    );
    assert_eq!(notes(&r, "group-field-unset").len(), 1, "{r}");
}

#[test]
fn a_group_by_field_no_type_declares_is_a_load_error() {
    let dir = repo(
        "badgroup",
        &config("  group_by: [Nonesuch]\n"),
        &[(1, "Planned", "")],
        &[],
    );
    let (code, text) = run(&dir, &["roadmap"]);
    assert_eq!(code, 2, "{text}");
    assert!(text.contains("Nonesuch"), "{text}");
}

#[test]
fn a_roadmap_pair_that_is_not_acyclic_is_a_load_error() {
    let cfg = config("").replace(
        "reciprocity: none, cross_type: true, acyclic: true",
        "reciprocity: none, cross_type: true",
    );
    let dir = repo("notacyclic", &cfg, &[(1, "Planned", "")], &[]);
    let (code, text) = run(&dir, &["roadmap"]);
    assert_eq!(code, 2, "{text}");
    assert!(text.contains("acyclic"), "{text}");
}

#[test]
fn a_type_with_no_status_vocabulary_is_open_and_noted() {
    let dir = repo(
        "novocab",
        &config(""),
        &[(1, "Planned", "Depends-on: RFC-4\n")],
        &[4],
    );
    let r = roadmap(&dir);
    assert_eq!(r["records"]["RFC-4"]["status"], serde_json::Value::Null);
    assert_eq!(r["records"]["RFC-4"]["wave"], 0);
    assert_eq!(
        r["records"]["MILE-1"]["wave"], 1,
        "an RFC with no vocabulary blocks"
    );
    assert_eq!(strings(&r["critical_path"]), ["RFC-4", "MILE-1"]);
    let found = notes(&r, "no-status-vocabulary");
    assert_eq!(found.len(), 1, "{r}");
    assert_eq!(found[0]["record"], "RFC-4");
}

#[test]
fn a_prose_wait_with_no_dependency_is_noted_and_nothing_open_is_distinguished() {
    let dir = repo(
        "prose",
        &config(""),
        &[
            (1, "Planned", "Blocked-on: a real adopter asking for it\n"),
            (2, "Planned", "Blocked-on: —\n"),
            (3, "Planned", "Blocked-on: waiting\nDepends-on: MILE-2\n"),
        ],
        &[],
    );
    let r = roadmap(&dir);
    let found = notes(&r, "unstructured-wait");
    assert_eq!(found.len(), 1, "{r}");
    assert_eq!(found[0]["record"], "MILE-1");
    assert_eq!(found[0]["field"], "Blocked-on");
    assert_eq!(found[0]["pair"], serde_json::Value::Null);

    let dir = repo(
        "alldone",
        &config(""),
        &[(1, "Done", ""), (2, "WontDo", "")],
        &[],
    );
    let r = roadmap(&dir);
    assert_eq!(r["waves"], serde_json::json!([]));
    assert_eq!(notes(&r, "nothing-open").len(), 1, "{r}");
    assert!(notes(&r, "no-sequencing-pair").is_empty());
}

#[test]
fn notes_sort_by_kind_then_record_then_pair() {
    let dir = repo(
        "notesorder",
        &config(""),
        &[
            (12, "Planned", "Blocked-on: later\n"),
            (2, "Planned", "Blocked-on: sooner\n"),
            (3, "Planned", "Depends-on: MILE-98\n"),
        ],
        &[],
    );
    let r = roadmap(&dir);
    let order: Vec<(&str, &str)> = r["notes"]
        .as_array()
        .unwrap()
        .iter()
        .map(|n| {
            (
                n["kind"].as_str().unwrap(),
                n["record"].as_str().unwrap_or(""),
            )
        })
        .collect();
    assert_eq!(
        order,
        [
            ("unresolved-claim", "MILE-3"),
            ("unstructured-wait", "MILE-2"),
            ("unstructured-wait", "MILE-12")
        ]
    );
}
