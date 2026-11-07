//! MILE-196: `roadmap.work_types` (`ADR-82`, Amendment of 2026-11-05). A record
//! type the repository does not list is a fact nobody builds: it gates its
//! dependents but is never ordered itself. The fixtures name the non-work type
//! `fact`, never `condition`, because the engine knows no type name (`ADR-64`).
//!
//! Verifies: MILE-196

mod common;

use std::path::Path;
use std::process::Command;

/// `work_line` is the `work_types` line, or empty for the key absent.
fn config(work_line: &str) -> String {
    format!(
        "schema_version: 2
roadmap:
  pairs: [sequencing]
{work_line}record_types:
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
relation_pairs:
  - {{name: sequencing, forward: Depends-on, inverse: Blocks, reciprocity: none, cross_type: true, acyclic: true}}
rules:
  header.required-fields: error
  sequence.cycle: error
  sequence.status-agreement: error
"
    )
}

/// `(id, status, extra header lines)`; the prefix picks the directory.
type Rec<'a> = (&'a str, &'a str, &'a str);

fn repo(name: &str, config: &str, records: &[Rec]) -> common::TempDir {
    let dir = common::TempDir::new(&format!("urzua-worktypes-{name}"));
    std::fs::create_dir_all(dir.join("docs/tasks")).unwrap();
    std::fs::create_dir_all(dir.join("docs/facts")).unwrap();
    std::fs::create_dir_all(dir.join(".urzua")).unwrap();
    std::fs::write(dir.join(".urzua/config.yaml"), config).unwrap();
    for (id, status, extra) in records {
        let sub = if id.starts_with("TASK") {
            "tasks"
        } else {
            "facts"
        };
        std::fs::write(
            dir.join(format!("docs/{sub}/{id}-r.md")),
            format!("---\nStatus: {status}\n{extra}---\n# r\n"),
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
    let out = common::urzua()
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

fn flat_waves(r: &serde_json::Value) -> Vec<&str> {
    r["waves"]
        .as_array()
        .unwrap()
        .iter()
        .flat_map(|w| strings(w))
        .collect()
}

const WORK: &str = "  work_types: [task]\n";

// ---- load errors

fn load_error(name: &str, line: &str, needle: &str) {
    let dir = repo(name, &config(line), &[("TASK-1", "Planned", "")]);
    let (code, text) = run(&dir, &["roadmap"]);
    assert_eq!(code, 2, "{text}");
    assert!(text.contains("roadmap:"), "{text}");
    assert!(text.contains("work_types"), "{text}");
    assert!(text.contains(needle), "{text}");
}

#[test]
fn an_unknown_type_name_is_a_load_error_naming_it() {
    load_error("unknown", "  work_types: [task, nonesuch]\n", "nonesuch");
}

#[test]
fn a_type_listed_twice_is_a_load_error_naming_it() {
    load_error("dup", "  work_types: [task, task]\n", "'task'");
}

#[test]
fn a_blank_name_or_one_with_surrounding_whitespace_is_a_load_error() {
    load_error("blank", "  work_types: [task, \"\"]\n", "blank");
    load_error("space", "  work_types: [\" task\"]\n", "' task'");
}

#[test]
fn an_empty_list_is_a_load_error() {
    load_error("empty", "  work_types: []\n", "empty");
}

#[test]
fn work_types_with_no_pairs_is_accepted_and_inert() {
    let cfg = config(WORK)
        .replace("  pairs: [sequencing]\n", "  pairs: []\n")
        .replace(
            "  sequence.cycle: error\n  sequence.status-agreement: error\n",
            "",
        );
    let dir = repo("nopairs", &cfg, &[("TASK-1", "Planned", "")]);
    let r = roadmap(&dir);
    assert_eq!(r["notes"][0]["kind"], "no-sequencing-pair", "{r}");
}

// ---- absent means no change

const CORPUS: &[Rec] = &[
    ("TASK-1", "Planned", ""),
    ("TASK-2", "Planned", "Depends-on: FACT-1\n"),
    ("TASK-3", "Planned", "Depends-on: TASK-2\n"),
    ("FACT-1", "Open", "Depends-on: TASK-1\n"),
];

#[test]
fn with_the_key_absent_every_type_is_work_and_there_is_no_waiting_on() {
    let dir = repo("absent", &config(""), CORPUS);
    let r = roadmap(&dir);
    assert!(r.get("waiting_on").is_none(), "{r}");
    assert_eq!(r["records"]["FACT-1"]["wave"], 1, "{r}");
    assert!(r["records"].get("FACT-1").is_some());
    let (_, export) = run(&dir, &["export", "--format", "json"]);
    assert!(!export.contains("waiting_on"), "{export}");
}

#[test]
fn listing_every_type_changes_nothing_but_adds_an_empty_waiting_on() {
    let absent = repo("absent2", &config(""), CORPUS);
    let all = repo("all", &config("  work_types: [task, fact]\n"), CORPUS);
    let mut declared = roadmap(&all);
    let waiting = declared.as_object_mut().unwrap().remove("waiting_on");
    assert_eq!(waiting, Some(serde_json::json!([])));
    assert_eq!(declared, roadmap(&absent));
}

// ---- a non-work record gates and is never ordered

#[test]
fn a_non_work_record_is_in_no_list_and_still_blocks_its_dependents() {
    let dir = repo("gate", &config(WORK), CORPUS);
    let r = roadmap(&dir);
    // TASK-1 is open work; FACT-1 depends on it; TASK-2 on FACT-1; TASK-3 on TASK-2.
    assert_eq!(
        r["waves"],
        serde_json::json!([["TASK-1"], ["TASK-2"], ["TASK-3"]]),
        "TASK-2 waits on TASK-1 through FACT-1: {r}"
    );
    assert!(!flat_waves(&r).contains(&"FACT-1"));
    assert_eq!(strings(&r["ready"]), ["TASK-1"]);
    assert_eq!(strings(&r["critical_path"]), ["TASK-1", "TASK-2", "TASK-3"]);
    assert!(r["records"].get("FACT-1").is_none(), "{r}");
    assert!(r["groups"].as_array().unwrap().is_empty());
    let t2 = &r["records"]["TASK-2"];
    assert_eq!(t2["blocked"], true);
    assert_eq!(t2["ready"], false);
    assert_eq!(strings(&t2["blocked_by"]), ["FACT-1"]);
    // `blocks` reaches work through the non-work record.
    assert_eq!(
        strings(&r["records"]["TASK-1"]["blocks"]),
        ["TASK-2", "TASK-3"]
    );
    assert_eq!(
        r["waiting_on"],
        serde_json::json!([{
            "record": "FACT-1", "type": "fact", "status": "Open",
            "blocks": ["TASK-2", "TASK-3"]
        }])
    );
}

#[test]
fn a_dependent_of_an_open_fact_is_blocked_and_in_wave_zero_but_not_ready() {
    let dir = repo(
        "wave0",
        &config(WORK),
        &[
            ("FACT-1", "Open", ""),
            ("TASK-1", "Planned", "Depends-on: FACT-1\n"),
        ],
    );
    let r = roadmap(&dir);
    assert_eq!(r["waves"], serde_json::json!([["TASK-1"]]));
    assert!(r["ready"].as_array().unwrap().is_empty(), "{r}");
    assert_eq!(r["records"]["TASK-1"]["blocked"], true);
    assert_eq!(r["records"]["TASK-1"]["wave"], 0);
}

#[test]
fn a_met_fact_releases_its_dependent_and_drops_out_of_waiting_on() {
    let dir = repo(
        "met",
        &config(WORK),
        &[
            ("FACT-1", "Met", ""),
            ("TASK-1", "Planned", "Depends-on: FACT-1\n"),
        ],
    );
    let r = roadmap(&dir);
    assert_eq!(strings(&r["ready"]), ["TASK-1"]);
    assert_eq!(r["records"]["TASK-1"]["blocked"], false);
    assert_eq!(r["waiting_on"], serde_json::json!([]));
}

#[test]
fn a_dropped_fact_sends_its_dependent_to_needs_replan_and_is_listed() {
    let dir = repo(
        "gone",
        &config(WORK),
        &[
            ("FACT-1", "Gone", ""),
            ("TASK-1", "Planned", "Depends-on: FACT-1\n"),
            ("TASK-2", "Planned", "Depends-on: TASK-1\n"),
        ],
    );
    let r = roadmap(&dir);
    assert_eq!(
        r["needs_replan"],
        serde_json::json!([
            {"record": "TASK-1", "reason": "failed-dependency", "via": "FACT-1"},
            {"record": "TASK-2", "reason": "depends-on-excluded", "via": "TASK-1"}
        ])
    );
    assert!(r["waves"].as_array().unwrap().is_empty());
    assert_eq!(r["waiting_on"][0]["record"], "FACT-1");
    assert_eq!(r["waiting_on"][0]["status"], "Gone");
}

#[test]
fn exclusion_propagates_through_a_fact_that_waits_on_failed_work() {
    let dir = repo(
        "viafact",
        &config(WORK),
        &[
            ("TASK-1", "Dropped", ""),
            ("FACT-1", "Open", "Depends-on: TASK-1\n"),
            ("TASK-2", "Planned", "Depends-on: FACT-1\n"),
        ],
    );
    let r = roadmap(&dir);
    // The fact is not a record of needs_replan; its dependent is, via the fact.
    assert_eq!(
        r["needs_replan"],
        serde_json::json!([
            {"record": "TASK-2", "reason": "depends-on-excluded", "via": "FACT-1"}
        ])
    );
}

#[test]
fn a_fact_is_not_counted_by_nothing_open_or_unstructured_wait() {
    let cfg = config(WORK).replace(
        "    known_fields: [Depends-on, Blocks]\n    pointer_fields: [Depends-on, Blocks]\n    statuses:\n      values: [Open",
        "    known_fields: [Depends-on, Blocks, Waiting]\n    pointer_fields: [Depends-on, Blocks]\n    narrative_fields: [Waiting]\n    statuses:\n      values: [Open",
    );
    let dir = repo(
        "notes",
        &cfg,
        &[
            ("FACT-1", "Open", "Waiting: the vendor answers\n"),
            ("TASK-1", "Done", ""),
        ],
    );
    let r = roadmap(&dir);
    let kinds: Vec<&str> = r["notes"]
        .as_array()
        .unwrap()
        .iter()
        .map(|n| n["kind"].as_str().unwrap())
        .collect();
    assert!(kinds.contains(&"nothing-open"), "{r}");
    assert!(!kinds.contains(&"unstructured-wait"), "{r}");
}

#[test]
fn a_fact_is_not_in_groups() {
    let cfg = config("  group_by: [Status]\n  work_types: [task]\n");
    let dir = repo(
        "groups",
        &cfg,
        &[("FACT-1", "Open", ""), ("TASK-1", "Planned", "")],
    );
    let r = roadmap(&dir);
    assert_eq!(
        r["groups"],
        serde_json::json!([{"field": "Status", "value": "Planned", "records": ["TASK-1"]}])
    );
}

// ---- contraction: the worked example of the amendment

fn contracted(fact_status: &str, name: &str) -> serde_json::Value {
    // W (work, open) <- C (non-work) <- R (work): C depends on W, R on C.
    let dir = repo(
        name,
        &config(WORK),
        &[
            ("TASK-1", "Planned", ""),
            ("FACT-1", fact_status, "Depends-on: TASK-1\n"),
            ("TASK-2", "Planned", "Depends-on: FACT-1\n"),
        ],
    );
    roadmap(&dir)
}

#[test]
fn the_worked_example_contracts_waves_and_the_critical_path_through_a_fact() {
    for status in ["Open", "Met"] {
        let r = contracted(status, &format!("chain-{status}"));
        assert_eq!(
            r["waves"],
            serde_json::json!([["TASK-1"], ["TASK-2"]]),
            "{status}: {r}"
        );
        assert_eq!(
            strings(&r["critical_path"]),
            ["TASK-1", "TASK-2"],
            "{status}"
        );
        assert_eq!(
            strings(&r["records"]["TASK-1"]["blocks"]),
            ["TASK-2"],
            "{status}"
        );
        assert_eq!(strings(&r["ready"]), ["TASK-1"], "{status}");
    }
    // Open still blocks the dependent, Met releases it (the work above it is
    // what keeps it out of wave 0).
    let open = contracted("Open", "chain-open2");
    assert_eq!(open["records"]["TASK-2"]["blocked"], true);
    let met = contracted("Met", "chain-met2");
    assert_eq!(met["records"]["TASK-2"]["blocked"], false);
    // Ready is a subset of wave 0 and R is not ready behind open work.
    assert_eq!(met["records"]["TASK-2"]["ready"], false);
}

#[test]
fn the_worked_example_with_a_dropped_fact_sends_the_dependent_to_needs_replan() {
    let r = contracted("Gone", "chain-gone");
    assert_eq!(
        r["needs_replan"],
        serde_json::json!([
            {"record": "TASK-2", "reason": "failed-dependency", "via": "FACT-1"}
        ])
    );
    assert_eq!(r["waves"], serde_json::json!([["TASK-1"]]));
}

#[test]
fn contraction_runs_through_a_chain_of_facts() {
    let dir = repo(
        "twofacts",
        &config(WORK),
        &[
            ("TASK-1", "Planned", ""),
            ("FACT-1", "Met", "Depends-on: TASK-1\n"),
            ("FACT-2", "Met", "Depends-on: FACT-1\n"),
            ("TASK-2", "Planned", "Depends-on: FACT-2\n"),
        ],
    );
    let r = roadmap(&dir);
    assert_eq!(r["waves"], serde_json::json!([["TASK-1"], ["TASK-2"]]));
    assert_eq!(strings(&r["critical_path"]), ["TASK-1", "TASK-2"]);
    assert_eq!(strings(&r["records"]["TASK-1"]["blocks"]), ["TASK-2"]);
}

// ---- check never reads work_types

fn findings(dir: &Path) -> Vec<String> {
    let (_, text) = run(dir, &["check", "docs/"]);
    let json: serde_json::Value = serde_json::from_str(&text).unwrap();
    let mut out: Vec<String> = json["findings"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|f| f["rule"] == "sequence.cycle" || f["rule"] == "sequence.status-agreement")
        .map(|f| format!("{} {} {}", f["rule"], f["file"], f["message"]))
        .collect();
    out.sort();
    out
}

#[test]
fn sequence_cycle_and_status_agreement_give_the_same_findings_with_and_without_work_types() {
    let records: &[Rec] = &[
        // A loop through a fact and a task.
        ("TASK-1", "Planned", "Depends-on: FACT-1\n"),
        ("FACT-1", "Open", "Depends-on: TASK-1\n"),
        // Done work behind an Open fact: status-agreement.
        ("FACT-2", "Open", ""),
        ("TASK-2", "Done", "Depends-on: FACT-2\n"),
    ];
    let without = findings(&repo("chk-a", &config(""), records));
    let with = findings(&repo("chk-b", &config(WORK), records));
    assert!(
        without.iter().any(|f| f.starts_with("\"sequence.cycle\"")),
        "{without:?}"
    );
    assert!(
        without
            .iter()
            .any(|f| f.starts_with("\"sequence.status-agreement\"")),
        "{without:?}"
    );
    assert_eq!(without, with);
    // The roadmap still lists the loop, fact included.
    let r = roadmap(&repo("chk-c", &config(WORK), records));
    assert_eq!(r["cycles"], serde_json::json!([["FACT-1", "TASK-1"]]));
}

// ---- this repository: the key absent changes nothing

fn copy_tree(from: &Path, to: &Path) {
    std::fs::create_dir_all(to).unwrap();
    for entry in std::fs::read_dir(from).unwrap().flatten() {
        let target = to.join(entry.file_name());
        if entry.file_type().unwrap().is_dir() {
            copy_tree(&entry.path(), &target);
        } else {
            std::fs::copy(entry.path(), target).unwrap();
        }
    }
}

/// `text` without the `"waiting_on": []` entry the declared key adds, and the
/// comma that led to it.
fn without_empty_waiting_on(text: &str) -> String {
    let entry = "\"waiting_on\": []";
    assert_eq!(text.matches(entry).count(), 1, "one empty waiting_on");
    let at = text.find(entry).unwrap();
    let head = text[..at].trim_end();
    let head = head.strip_suffix(',').expect("a comma before the entry");
    format!("{head}{}", &text[at + entry.len()..])
}

/// MILE-196: with `roadmap.work_types` absent, every record type is work, which
/// is what the roadmap read before the key existed. On this repository's own
/// corpus the key-absent `roadmap` and `export --format json` are byte for byte
/// the output of a config that lists every record type (the same reading,
/// spelled out), less the one `waiting_on` entry that declaring the key adds.
/// A byte that moved with the key absent would show here; the comparison runs
/// over the 400 records this repository holds, not over a fixture.
#[test]
fn on_this_repository_the_absent_key_is_byte_identical_to_every_type_being_work() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../..");
    let dir = common::TempDir::new("urzua-worktypes-own-corpus");
    copy_tree(&root.join("docs"), &dir.join("docs"));
    copy_tree(&root.join(".urzua"), &dir.join(".urzua"));
    assert!(Command::new("git")
        .args(["init", "-q"])
        .current_dir(&dir)
        .status()
        .unwrap()
        .success());

    let own = std::fs::read_to_string(dir.join(".urzua/config.yaml")).unwrap();
    let parsed: yaml_serde::Value = yaml_serde::from_str(&own).unwrap();
    let types: Vec<String> = parsed["record_types"]
        .as_mapping()
        .expect("record_types")
        .keys()
        .map(|k| k.as_str().unwrap().to_string())
        .collect();
    assert!(types.len() > 3, "{types:?}");

    // The same config twice: the key removed, and the key naming every type.
    let absent: String = own
        .lines()
        .filter(|l| !l.trim_start().starts_with("work_types:"))
        .map(|l| format!("{l}\n"))
        .collect();
    let all = absent.replacen(
        "\nroadmap:\n",
        &format!("\nroadmap:\n  work_types: [{}]\n", types.join(", ")),
        1,
    );
    assert_ne!(absent, all, "the roadmap block was found");

    let mut outputs = Vec::new();
    for config in [&absent, &all] {
        std::fs::write(dir.join(".urzua/config.yaml"), config).unwrap();
        let mut pair = Vec::new();
        for args in [&["roadmap"][..], &["export", "--format", "json"][..]] {
            let (code, text) = run(&dir, args);
            assert_eq!(code, 0, "{args:?}: {}", &text[..text.len().min(400)]);
            pair.push(text);
        }
        outputs.push(pair);
    }
    for (i, what) in ["roadmap", "export --format json"].iter().enumerate() {
        let (without_key, with_key) = (&outputs[0][i], &outputs[1][i]);
        assert!(
            !without_key.contains("\"waiting_on\": ["),
            "{what}: no waiting_on with the key absent"
        );
        assert!(
            without_empty_waiting_on(with_key) == *without_key,
            "{what}: the key absent differs from every type being work"
        );
    }
}
