//! `urzua export`'s view model and `urzua board`, on two fixture configurations
//! with entirely different type and status names and on this repository's own
//! records: states, containment, waits, edges, order, the page's record count,
//! escaping, well-formedness, the absence of any external address, and the
//! server.

mod common;

use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpStream;
use std::path::{Path, PathBuf};
use std::process::{Child, Stdio};
use std::time::{Duration, Instant};

use serde_json::Value;

fn write(dir: &Path, path: &str, content: &str) {
    let full = dir.join(path);
    std::fs::create_dir_all(full.parent().unwrap()).unwrap();
    std::fs::write(full, content).unwrap();
}

fn record(status_line: &str, extra: &str, heading: &str) -> String {
    format!("---\n{status_line}\n{extra}---\n{heading}\n\nText of the record.\n")
}

const STATUSES: &str = "    statuses:
      values: [Idea, Rolling, Shipped, Cancelled]
      success: [Shipped]
      failure: [Cancelled]
      progress: [Rolling, Shipped]
      initial: [Idea]
      transitions:
        Idea: [Rolling, Cancelled]
        Rolling: [Shipped, Cancelled]
";

/// Three levels of containment (`programme` > `stage` > `step`), a gate on
/// `stage`, a type with a gate and no transitions, and a pair ordering work.
fn chain_config() -> String {
    let level = |name: &str, prefix: &str, dir: &str, part_of: &str| {
        format!(
            "  {name}:
    dir: docs/{dir}
    prefix: {prefix}
    header_shape: yaml-frontmatter
    required_fields: [Status]
    known_fields: [Owner, Lane, In, Needs, Gates]
    narrative_fields: []
    pointer_fields: [{part_of}Needs, Gates]
    field_values:
      Lane: [north, south]
{STATUSES}"
        )
    };
    format!(
        "schema_version: 2
export:
  link_prefix: https://example.test/blob/main/
record_types:
{}{}{}  charter:
    dir: docs/charters
    prefix: CHR
    header_shape: yaml-frontmatter
    required_fields: [Status]
    pointer_fields: []
    narrative_fields: []
    statuses:
      values: [Draft, Signed]
      success: [Signed]
authority:
  deciders:
    leads: {{logins: [someone]}}
  gates:
    - {{type: stage, to: [Shipped], by: leads}}
    - {{type: charter, to: [Signed], by: leads}}
relation_pairs:
  - {{name: order, forward: Needs, inverse: Gates, reciprocity: none, cross_type: true, acyclic: true}}
roadmap:
  pairs: [order]
  work_types: [programme, stage, step, charter]
",
        level("programme", "PRG", "programmes", ""),
        level("stage", "STG", "stages", "{name: In, targets: [programme], part_of: true}, "),
        level("step", "STP", "steps", "{name: In, targets: [stage], part_of: true}, "),
    )
}

fn chain_records(dir: &Path) {
    let r = |path: &str, status: &str, extra: &str, heading: &str| {
        write(
            dir,
            path,
            &record(&format!("Status: {status}"), extra, heading),
        );
    };
    r(
        "docs/programmes/PRG-1-alpha.md",
        "Rolling",
        "Owner: ann\nLane: north\n",
        "# 1 — Alpha",
    );
    r(
        "docs/programmes/PRG-2-beta.md",
        "Idea",
        "Lane: south\n",
        "# PRG-2: Beta",
    );
    r(
        "docs/programmes/PRG-3-gamma.md",
        "Shipped",
        "",
        "# 3 — Gamma",
    );
    r(
        "docs/programmes/PRG-4-hostile.md",
        "Idea",
        "Owner: javascript:alert(1)\nLane: north\n",
        "# 4 — <script>alert(\"x\")</script> & 'quoted' \"double\"",
    );
    r(
        "docs/programmes/PRG-5-no-title.md",
        "Idea",
        "",
        "Plain words, no heading.",
    );
    r(
        "docs/stages/STG-1-first.md",
        "Shipped",
        "In: PRG-1\n",
        "# 1 — First",
    );
    r(
        "docs/stages/STG-2-second.md",
        "Rolling",
        "In: PRG-1\n",
        "# 2 — Second",
    );
    r(
        "docs/stages/STG-3-third.md",
        "Idea",
        "In: PRG-1\nNeeds: STG-2\n",
        "# 3 — Third",
    );
    r(
        "docs/stages/STG-4-fourth.md",
        "Rolling",
        "In: PRG-2\n",
        "# 4 — Fourth",
    );
    r(
        "docs/stages/STG-5-fifth.md",
        "Idea",
        "In: PRG-3\n",
        "# 5 — Fifth",
    );
    r("docs/stages/STG-6-loose.md", "Idea", "", "# 6 — Loose");
    r("docs/steps/STP-1-a.md", "Rolling", "In: STG-2\n", "# 1 — A");
    r("docs/steps/STP-2-b.md", "Shipped", "In: STG-2\n", "# 2 — B");
    r(
        "docs/steps/STP-3-c.md",
        "Cancelled",
        "In: STG-2\n",
        "# 3 — C",
    );
    r(
        "docs/steps/STP-10-d.md",
        "Idea",
        "In: STG-2\nNeeds: STP-1, STP-2\n",
        "# 10 — D",
    );
    r("docs/steps/STP-11-e.md", "Idea", "", "# 11 — E");
    r("docs/charters/CHR-1-terms.md", "Draft", "", "# 1 — Terms");
    write(
        dir,
        "docs/stages/STG-7-unreadable.md",
        "no header here\n# 7 — Broken\n",
    );
}

fn repo(name: &str, config: &str, populate: impl FnOnce(&Path)) -> common::TempDir {
    let dir = common::TempDir::new(&format!("urzua-board-{name}"));
    write(&dir, ".urzua/config.yaml", config);
    populate(&dir);
    common::git(&dir, &["init", "-q", "-b", "main"]);
    common::commit_all(&dir, 0);
    dir
}

fn chain(name: &str) -> common::TempDir {
    repo(name, &chain_config(), chain_records)
}

const FLAT_CONFIG: &str = "schema_version: 2
record_types:
  ticket:
    dir: docs/tickets
    prefix: TCK
    header_shape: yaml-frontmatter
    required_fields: [Status]
    known_fields: [Needs, Gates, Area]
    narrative_fields: []
    pointer_fields: [Needs, Gates]
    field_values:
      Area: [web, cli]
    statuses:
      values: [Todo, Doing, Done, Dropped]
      success: [Done]
      failure: [Dropped]
      progress: [Doing]
  memo:
    dir: docs/memos
    prefix: MEM
    header_shape: yaml-frontmatter
    required_fields: [Topic]
    known_fields: [Related]
    narrative_fields: []
    pointer_fields: [Related]
relation_pairs:
  - {name: order, forward: Needs, inverse: Gates, reciprocity: none, cross_type: true, acyclic: true}
roadmap:
  pairs: [order]
  work_types: [ticket]
";

fn flat(name: &str) -> common::TempDir {
    repo(name, FLAT_CONFIG, |dir| {
        let t = |n: u32, status: &str, extra: &str| {
            write(
                dir,
                &format!("docs/tickets/TCK-{n}-t.md"),
                &record(
                    &format!("Status: {status}"),
                    extra,
                    &format!("# {n} — Ticket {n}"),
                ),
            );
        };
        t(1, "Done", "Area: web\n");
        t(2, "Doing", "");
        t(3, "Todo", "Needs: TCK-2\n");
        t(4, "Todo", "Area: [web, cli]\n");
        t(5, "Dropped", "");
        t(6, "Todo", "Needs: TCK-5\n");
        write(
            dir,
            "docs/memos/MEM-1-m.md",
            &record("Topic: x", "Related: TCK-4\n", "# 1 — Memo"),
        );
        write(
            dir,
            "docs/memos/MEM-2-n.md",
            &record("Topic: y", "Related: MEM-1\n", "# 2 — Other memo"),
        );
    })
}

fn run(dir: &Path, args: &[&str]) -> std::process::Output {
    common::urzua()
        .args(args)
        .current_dir(dir)
        .output()
        .unwrap()
}

fn json(out: &std::process::Output) -> Value {
    serde_json::from_slice(&out.stdout)
        .unwrap_or_else(|e| panic!("not JSON ({e}): {}", String::from_utf8_lossy(&out.stdout)))
}

fn export(dir: &Path) -> Value {
    let out = run(dir, &["export", "--format", "json"]);
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    json(&out)
}

fn by_id<'a>(export: &'a Value, id: &str) -> &'a Value {
    export["records"]
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["id"] == id)
        .unwrap_or_else(|| panic!("no record {id}"))
}

fn ids(value: &Value) -> Vec<String> {
    value
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_str().unwrap().to_string())
        .collect()
}

fn board_page(dir: &Path) -> (String, Value) {
    let target = dir.join("out/board.html");
    let out = run(dir, &["board", "--out", target.to_str().unwrap()]);
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    (std::fs::read_to_string(&target).unwrap(), json(&out))
}

// ---------------------------------------------------------------- the view model

#[test]
fn a_title_is_read_the_way_the_title_checks_read_it_and_falls_back_to_the_id() {
    let dir = chain("title");
    let e = export(&dir);
    assert_eq!(by_id(&e, "PRG-1")["title"], "Alpha", "number form");
    assert_eq!(by_id(&e, "PRG-2")["title"], "Beta", "own id form");
    assert_eq!(by_id(&e, "PRG-1")["title_is_fallback"], false);
    let none = by_id(&e, "PRG-5");
    assert_eq!(none["title"], "PRG-5");
    assert_eq!(none["title_is_fallback"], true);
}

#[test]
fn a_state_is_decided_by_a_declared_list_and_says_which() {
    let dir = chain("state");
    let e = export(&dir);
    let expect = [
        (
            "PRG-1",
            "under-way",
            "record_types.programme.statuses.progress",
        ),
        (
            "PRG-2",
            "not-started",
            "record_types.programme.statuses.progress",
        ),
        (
            "PRG-3",
            "finished",
            "record_types.programme.statuses.success",
        ),
        ("STP-3", "dropped", "record_types.step.statuses.failure"),
        ("STP-2", "finished", "record_types.step.statuses.success"),
        ("CHR-1", "open", "no-progress-declared"),
    ];
    for (id, state, by) in expect {
        let r = by_id(&e, id);
        assert_eq!(
            (r["state"].as_str(), r["state_decided_by"].as_str()),
            (Some(state), Some(by)),
            "{id}"
        );
    }
    let flat = flat("state-flat");
    let e = export(&flat);
    assert_eq!(by_id(&e, "MEM-1")["state"], "none");
    assert_eq!(
        by_id(&e, "MEM-1")["state_decided_by"],
        "no-statuses-declared"
    );
    assert_eq!(by_id(&e, "TCK-2")["state"], "under-way");
}

#[test]
fn without_a_progress_set_a_record_that_is_neither_finished_nor_dropped_is_open() {
    let config = chain_config().replace("      progress: [Rolling, Shipped]\n", "");
    let dir = repo("open", &config, chain_records);
    let e = export(&dir);
    for (id, state, by) in [
        ("PRG-1", "open", "no-progress-declared"),
        ("STG-2", "open", "no-progress-declared"),
        (
            "PRG-3",
            "finished",
            "record_types.programme.statuses.success",
        ),
        ("STP-3", "dropped", "record_types.step.statuses.failure"),
        ("STG-7", "open", "no-status"),
    ] {
        let r = by_id(&e, id);
        assert_eq!(
            (r["state"].as_str(), r["state_decided_by"].as_str()),
            (Some(state), Some(by)),
            "{id}"
        );
    }
    assert_eq!(e["type_roles"]["programme"]["progress_declared"], false);
    assert_eq!(
        by_id(&e, "STG-2")["children_by_state"]["open"],
        2,
        "STP-1 and STP-10"
    );
    assert_eq!(
        by_id(&e, "PRG-2")["state_disagrees_with_children"],
        false,
        "nothing says an open parent has not begun"
    );
    assert_eq!(
        by_id(&e, "PRG-3")["state_disagrees_with_children"],
        true,
        "finished, child open"
    );
    assert!(e["records"]
        .as_array()
        .unwrap()
        .iter()
        .all(|r| r["state"] != "under-way" && r["state"] != "not-started"));
    let (html, _) = board_page(&dir);
    assert!(html.contains("Under way (0)"));
    assert!(
        html.contains("Nothing can be listed as under way for charter, programme, stage, step"),
        "{}",
        html.split("Under way (0)")
            .nth(1)
            .unwrap()
            .chars()
            .take(400)
            .collect::<String>()
    );
    assert!(html.contains("statuses.progress"));
}

#[test]
fn a_finished_status_in_the_progress_list_is_finished_not_under_way() {
    let dir = chain("progress");
    let e = export(&dir);
    assert_eq!(by_id(&e, "STP-2")["status"], "Shipped");
    assert_eq!(by_id(&e, "STP-2")["state"], "finished");
}

#[test]
fn waiting_and_ready_are_separate_and_a_record_can_be_under_way_and_waiting() {
    let dir = chain("waiting");
    let e = export(&dir);
    let d = by_id(&e, "STP-10");
    assert_eq!(ids(&d["waiting_on"]), ["STP-1"], "STP-2 is finished");
    assert_eq!(d["ready"], false);
    assert_eq!(by_id(&e, "STG-3")["state"], "not-started");
    assert_eq!(
        e["type_roles"]["stage"]["waits_through"],
        serde_json::json!(["Needs"])
    );
    assert_eq!(ids(&by_id(&e, "STG-3")["waiting_on"]), ["STG-2"]);
    assert_eq!(by_id(&e, "STP-1")["ready"], true);
    assert_eq!(by_id(&e, "PRG-3")["ready"], false, "finished");
    assert_eq!(by_id(&e, "STP-3")["ready"], false, "dropped");
    assert_eq!(by_id(&e, "STP-11")["ready"], true);

    let flat = flat("waiting-flat");
    let e = export(&flat);
    assert_eq!(ids(&by_id(&e, "TCK-3")["waiting_on"]), ["TCK-2"]);
    assert_eq!(
        ids(&by_id(&e, "TCK-6")["waiting_on"]),
        ["TCK-5"],
        "a dropped record is not finished"
    );
    assert_eq!(by_id(&e, "TCK-6")["ready"], false);
    assert_eq!(by_id(&e, "TCK-4")["ready"], true);
    assert_eq!(
        by_id(&e, "TCK-2")["ready"],
        true,
        "under way and waiting on nothing"
    );
    assert_eq!(by_id(&e, "MEM-1")["ready"], false, "not a work type");
    assert_eq!(by_id(&e, "TCK-1")["ready"], false, "finished");
}

#[test]
fn containment_follows_only_the_declared_part_of_field_to_three_levels() {
    let dir = chain("tree");
    let e = export(&dir);
    assert_eq!(by_id(&e, "STP-1")["parent"], "STG-2");
    assert_eq!(by_id(&e, "STG-2")["parent"], "PRG-1");
    assert_eq!(by_id(&e, "PRG-1")["parent"], Value::Null);
    assert_eq!(
        ids(&by_id(&e, "PRG-1")["children"]),
        ["STG-1", "STG-2", "STG-3"]
    );
    assert_eq!(
        ids(&by_id(&e, "STG-2")["children"]),
        ["STP-1", "STP-2", "STP-3", "STP-10"],
        "numeric order"
    );
    let counts = &by_id(&e, "STG-2")["children_by_state"];
    assert_eq!(counts["finished"], 1);
    assert_eq!(counts["dropped"], 1);
    assert_eq!(counts["under-way"], 1);
    assert_eq!(counts["not-started"], 1);
    assert_eq!(counts["none"], 0);
    assert_eq!(by_id(&e, "STG-6")["parent"], Value::Null);
    assert_eq!(e["type_roles"]["step"]["part_of"], "In");
    assert_eq!(e["type_roles"]["programme"]["part_of"], Value::Null);
}

#[test]
fn a_reference_with_targets_but_no_part_of_is_not_containment() {
    let config = chain_config().replace(", part_of: true", "");
    let dir = repo("no-part-of", &config, chain_records);
    let e = export(&dir);
    for r in e["records"].as_array().unwrap() {
        assert_eq!(r["parent"], Value::Null, "{}", r["id"]);
        assert_eq!(r["children"], serde_json::json!([]), "{}", r["id"]);
    }
    assert_eq!(
        by_id(&e, "STP-1")["refers_to"]["In"],
        serde_json::json!(["STG-2"])
    );
}

#[test]
fn a_parent_and_its_children_disagree_when_one_contradicts_the_other() {
    let dir = chain("disagree");
    let e = export(&dir);
    assert_eq!(
        by_id(&e, "PRG-2")["state_disagrees_with_children"],
        true,
        "not started, child under way"
    );
    assert_eq!(
        by_id(&e, "PRG-3")["state_disagrees_with_children"],
        true,
        "finished, child not finished"
    );
    assert_eq!(by_id(&e, "PRG-1")["state_disagrees_with_children"], false);
    assert_eq!(by_id(&e, "STG-2")["state_disagrees_with_children"], false);
    assert_eq!(
        by_id(&e, "STP-1")["state_disagrees_with_children"],
        false,
        "no children"
    );
}

#[test]
fn a_gate_with_a_declared_transition_reserves_the_move_for_a_person() {
    let dir = chain("gates");
    let e = export(&dir);
    assert_eq!(
        by_id(&e, "STG-2")["gated_moves"],
        serde_json::json!([{"to": "Shipped", "by": "leads"}])
    );
    assert_eq!(
        by_id(&e, "STG-4")["gated_moves"].as_array().unwrap().len(),
        1
    );
    assert_eq!(
        by_id(&e, "STG-3")["gated_moves"],
        serde_json::json!([]),
        "Idea moves to ungated statuses"
    );
    assert_eq!(
        by_id(&e, "STG-1")["gated_moves"],
        serde_json::json!([]),
        "finished"
    );
    assert_eq!(
        e["type_roles"]["charter"]["gated_without_transitions"],
        true
    );
    assert_eq!(e["type_roles"]["stage"]["gated_without_transitions"], false);
}

#[test]
fn the_edges_are_the_ones_graph_prints_with_their_inverse() {
    let dir = chain("edges");
    let e = export(&dir);
    let graph = json(&run(&dir, &["graph"]));
    for edge in graph["edges"].as_array().unwrap() {
        let from = by_id(&e, edge["from"].as_str().unwrap());
        let to = edge["to"].as_str().unwrap();
        let field = edge["relation"].as_str().unwrap();
        assert!(
            ids(&from["refers_to"][field]).contains(&to.to_string()),
            "{edge} missing from refers_to"
        );
        assert!(
            ids(&by_id(&e, to)["referred_by"][field])
                .contains(&edge["from"].as_str().unwrap().to_string()),
            "{edge} missing from referred_by"
        );
    }
    let referred = ids(&by_id(&e, "STG-2")["referred_by"]["In"]);
    assert_eq!(referred, ["STP-1", "STP-2", "STP-3", "STP-10"]);
    assert_eq!(
        by_id(&e, "STP-10")["refers_to"]["Needs"],
        serde_json::json!(["STP-1", "STP-2"])
    );
}

#[test]
fn fields_are_a_map_and_a_list_valued_field_is_an_array() {
    let dir = chain("fields");
    let e = export(&dir);
    let alpha = &by_id(&e, "PRG-1")["fields"];
    assert_eq!(alpha["Owner"], "ann");
    assert_eq!(
        alpha["Lane"],
        serde_json::json!(["north"]),
        "a declared vocabulary reads as a list"
    );
    assert_eq!(
        by_id(&e, "STP-10")["fields"]["Needs"],
        serde_json::json!(["STP-1", "STP-2"])
    );
    let flat = flat("fields-flat");
    let e = export(&flat);
    assert_eq!(
        by_id(&e, "TCK-4")["fields"]["Area"],
        serde_json::json!(["web", "cli"])
    );
    let header: Vec<&str> = by_id(&e, "TCK-4")["header"]
        .as_array()
        .unwrap()
        .iter()
        .map(|f| f["name"].as_str().unwrap())
        .collect();
    assert_eq!(header, ["Status", "Area"], "the header list is unchanged");
}

#[test]
fn a_link_exists_only_when_a_prefix_is_declared() {
    let dir = chain("link");
    let e = export(&dir);
    assert_eq!(
        by_id(&e, "PRG-1")["link"],
        "https://example.test/blob/main/docs/programmes/PRG-1-alpha.md"
    );
    let flat = flat("link-flat");
    let e = export(&flat);
    assert!(by_id(&e, "TCK-1").get("link").is_none());
}

#[test]
fn a_bad_link_prefix_or_part_of_declaration_is_refused_at_load() {
    let dir = repo(
        "bad-prefix",
        &chain_config().replace("https://example.test/blob/main/", "ftp://x/ y"),
        chain_records,
    );
    assert_eq!(
        run(&dir, &["export", "--format", "json"]).status.code(),
        Some(2)
    );
    let no_targets = chain_config().replace(
        "{name: In, targets: [stage], part_of: true}",
        "{name: In, part_of: true}",
    );
    let dir = repo("no-targets", &no_targets, chain_records);
    let out = run(&dir, &["export", "--format", "json"]);
    assert_eq!(out.status.code(), Some(2));
    assert!(
        String::from_utf8_lossy(&out.stdout).contains("without targets"),
        "{}",
        String::from_utf8_lossy(&out.stdout)
    );
}

#[test]
fn records_are_in_numeric_order_within_a_type() {
    let dir = chain("order");
    let e = export(&dir);
    let steps: Vec<String> = e["records"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|r| r["type"] == "step")
        .map(|r| r["id"].as_str().unwrap().to_string())
        .collect();
    assert_eq!(steps, ["STP-1", "STP-2", "STP-3", "STP-10", "STP-11"]);
}

#[test]
fn sections_carry_checkbox_counts_and_the_header_list_and_text_are_unchanged() {
    let config = FLAT_CONFIG.replace(
        "    statuses:\n      values: [Todo",
        "    sections:\n      from: headings\n      depth: 2\n      required:\n        - {name: Checks}\n    statuses:\n      values: [Todo",
    );
    let dir = repo("sections-flat", &config, |dir| {
        write(
            dir,
            "docs/tickets/TCK-1-t.md",
            "---\nStatus: Todo\n---\n# 1 — T\n\n## Checks\n\n- [x] one\n- [ ] two\n- [ ] three\n",
        );
        write(
            dir,
            "docs/memos/MEM-1-m.md",
            "---\nTopic: x\n---\n# 1 — M\n",
        );
    });
    let e = export(&dir);
    let t = by_id(&e, "TCK-1");
    let checks = &t["sections"][0];
    assert_eq!(
        (checks["ticked"].as_u64(), checks["unticked"].as_u64()),
        (Some(1), Some(2))
    );
    assert!(t["raw_text"].as_str().unwrap().contains("- [x] one"));
    for key in ["name", "heading", "line", "end_line", "content"] {
        assert!(checks.get(key).is_some(), "{key}");
    }
}

#[test]
fn the_export_without_its_new_keys_has_the_keys_it_had() {
    let dir = chain("additive");
    let e = export(&dir);
    let new_record_keys = [
        "title",
        "title_is_fallback",
        "fields",
        "status",
        "state",
        "state_decided_by",
        "waiting_on",
        "ready",
        "gated_moves",
        "parent",
        "children",
        "children_by_state",
        "state_disagrees_with_children",
        "refers_to",
        "unresolved_refs",
        "referred_by",
        "link",
    ];
    for r in e["records"].as_array().unwrap() {
        let mut keys: Vec<&str> = r.as_object().unwrap().keys().map(String::as_str).collect();
        keys.retain(|k| !new_record_keys.contains(k));
        keys.sort_unstable();
        let expected = [
            "header",
            "header_readable",
            "id",
            "path",
            "preamble",
            "raw_text",
            "sections",
            "stable_id",
            "type",
        ];
        assert_eq!(keys, expected, "{}", r["path"]);
    }
    let mut top: Vec<&str> = e.as_object().unwrap().keys().map(String::as_str).collect();
    top.retain(|k| !["type_roles", "read"].contains(k) && !k.starts_with("next_action"));
    top.sort_unstable();
    assert_eq!(
        top,
        ["record_count", "records", "roadmap", "status", "types"]
    );
    assert_eq!(e["types"][0].as_object().unwrap().len(), 3);
    let without = run(&dir, &["export", "--format", "json", "--no-raw-text"]);
    let lean = json(&without);
    assert!(lean["records"][0].get("raw_text").is_none());
    assert_eq!(lean["record_count"], e["record_count"]);
}

#[test]
fn the_read_stamp_names_the_commit_the_branch_and_uncommitted_record_changes() {
    let dir = chain("stamp");
    let e = export(&dir);
    let head = String::from_utf8(
        common::isolated_git()
            .args(["rev-parse", "HEAD"])
            .current_dir(&dir)
            .output()
            .unwrap()
            .stdout,
    )
    .unwrap();
    assert_eq!(e["read"]["commit"], head.trim());
    assert_eq!(e["read"]["branch"], "main");
    assert_eq!(e["read"]["uncommitted_record_changes"], false);
    let time = e["read"]["time"].as_str().unwrap();
    assert!(time.len() == 20 && time.ends_with('Z'), "{time}");
    write(
        &dir,
        "docs/steps/STP-1-a.md",
        &record("Status: Idea", "In: STG-2\n", "# 1 — A"),
    );
    assert_eq!(export(&dir)["read"]["uncommitted_record_changes"], true);
}

// ---------------------------------------------------------------- the page

fn assert_well_formed(html: &str) {
    const VOID: [&str; 6] = ["meta", "br", "input", "link", "img", "hr"];
    let bytes = html.as_bytes();
    let mut stack: Vec<String> = Vec::new();
    let mut i = 0;
    assert!(html.starts_with("<!doctype html>"));
    while i < bytes.len() {
        if bytes[i] != b'<' {
            i += 1;
            continue;
        }
        let rest = &html[i..];
        if rest.starts_with("<!doctype") {
            i += rest.find('>').unwrap() + 1;
            continue;
        }
        if rest.starts_with("<!--") {
            i += rest.find("-->").expect("comment closes") + 3;
            continue;
        }
        let closing = rest.starts_with("</");
        let name_start = if closing { 2 } else { 1 };
        let name: String = rest[name_start..]
            .chars()
            .take_while(|c| c.is_ascii_alphanumeric())
            .collect();
        assert!(
            !name.is_empty(),
            "a `<` that opens no tag at byte {i}: {}",
            &rest[..rest.len().min(60)]
        );
        let mut j = name_start + name.len();
        let mut quote: Option<char> = None;
        let rb = rest.as_bytes();
        while j < rb.len() {
            let c = rb[j] as char;
            match quote {
                Some(q) if c == q => quote = None,
                Some(_) => {}
                None if c == '"' => quote = Some('"'),
                None if c == '\'' => panic!("single-quoted attribute near {}", &rest[..j]),
                None if c == '>' => break,
                None => {}
            }
            j += 1;
        }
        assert!(
            quote.is_none() && j < rb.len(),
            "an unterminated tag at byte {i}"
        );
        let tag = &rest[..=j];
        if closing {
            assert_eq!(
                stack.pop().as_deref(),
                Some(name.as_str()),
                "mismatched </{name}> at byte {i}"
            );
        } else if !VOID.contains(&name.as_str()) {
            stack.push(name.clone());
        }
        i += j + 1;
        if !closing && (name == "script" || name == "style") && !tag.ends_with("/>") {
            let end = html[i..]
                .find(&format!("</{name}>"))
                .expect("raw text element closes");
            i += end;
        }
    }
    assert!(stack.is_empty(), "unclosed: {stack:?}");
}

fn count(html: &str, needle: &str) -> usize {
    html.matches(needle).count()
}

fn attribute_values(html: &str, attribute: &str) -> Vec<String> {
    let needle = format!(" {attribute}=\"");
    let mut out = Vec::new();
    let mut rest = html;
    while let Some(at) = rest.find(&needle) {
        let after = &rest[at + needle.len()..];
        let end = after.find('"').unwrap();
        out.push(after[..end].to_string());
        rest = &after[end..];
    }
    out
}

const BASE64: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

fn csp_source(content: &str) -> String {
    let hex = urzua_io::sha256_hex_of_str(content);
    let bytes: Vec<u8> = (0..32)
        .map(|i| u8::from_str_radix(&hex[2 * i..2 * i + 2], 16).unwrap())
        .collect();
    let mut out = String::new();
    for chunk in bytes.chunks(3) {
        let n = chunk
            .iter()
            .enumerate()
            .fold(0u32, |n, (i, b)| n | (u32::from(*b) << (16 - 8 * i)));
        for i in 0..4 {
            out.push(if i <= chunk.len() {
                BASE64[(n >> (18 - 6 * i) & 63) as usize] as char
            } else {
                '='
            });
        }
    }
    format!("'sha256-{out}'")
}

fn assert_self_contained(html: &str) {
    assert_well_formed(html);
    assert!(html.contains("<meta charset=\"utf-8\">"));
    assert!(html.contains("<meta name=\"viewport\""));
    assert!(html.contains("http-equiv=\"Content-Security-Policy\""));
    assert!(html.contains("default-src 'none'"));
    assert_eq!(
        count(html, "<script"),
        1,
        "one inline script, nothing from a record"
    );
    for value in attribute_values(html, "href")
        .into_iter()
        .chain(attribute_values(html, "src"))
    {
        assert!(
            value.starts_with('#') || value.starts_with("record/") || value == "check",
            "address {value}"
        );
    }
    for forbidden in [
        "http://",
        "https://",
        "fetch(",
        "XMLHttpRequest",
        "WebSocket",
        "@import",
        "url(",
        "<link",
        "<img",
        "<iframe",
        "<form",
    ] {
        assert!(!html.contains(forbidden), "the page contains {forbidden}");
    }
    let script = html
        .split("<script>")
        .nth(1)
        .unwrap()
        .split("</script>")
        .next()
        .unwrap();
    let style = html
        .split("<style>")
        .nth(1)
        .unwrap()
        .split("</style>")
        .next()
        .unwrap();
    assert!(
        html.contains(&format!("script-src {}", csp_source(script))),
        "the script's hash is the policy's"
    );
    assert!(
        html.contains(&format!("style-src {}", csp_source(style))),
        "the style's hash is the policy's"
    );
    assert!(
        !html.contains(" style=\""),
        "an inline style attribute would be blocked"
    );
}

#[test]
fn the_page_holds_every_record_and_is_a_self_contained_well_formed_document() {
    for (name, dir) in [("chain", chain("page-chain")), ("flat", flat("page-flat"))] {
        let e = export(&dir);
        let (html, answer) = board_page(&dir);
        assert_self_contained(&html);
        let records = e["record_count"].as_u64().unwrap() as usize;
        assert_eq!(
            count(&html, " data-record=\""),
            records,
            "{name}: every record is an entry, once"
        );
        assert_eq!(answer["record_count"], e["record_count"]);
        assert_eq!(answer["status"], "ok");
        assert!(answer["bytes"].as_u64().unwrap() as usize == html.len());
        assert!(answer["read"]["commit"].is_string());
        assert!(html.contains("This page shows the working tree"));
        assert!(html.contains("urzua check:"));
    }
}

#[test]
fn the_page_writes_to_the_cache_by_default_and_stdout_stays_json() {
    let dir = flat("default-path");
    let out = run(&dir, &["board"]);
    let answer = json(&out);
    assert_eq!(
        answer["file"],
        dir.join(".urzua/cache/board.html").to_str().unwrap()
    );
    assert!(dir.join(".urzua/cache/board.html").is_file());
}

#[test]
fn what_waits_on_a_person_is_the_gated_moves_the_transitions_allow() {
    let dir = chain("person");
    let (html, _) = board_page(&dir);
    let section = html
        .split("Waiting on a person (")
        .nth(1)
        .unwrap()
        .split("<h3>Under way")
        .next()
        .unwrap();
    assert!(section.starts_with("2)"), "{section}");
    for id in ["STG-2", "STG-4"] {
        assert!(section.contains(&format!(">{id}</a>")), "{id}");
    }
    assert!(!section.contains(">STG-1</a>") && !section.contains(">STG-3</a>"));
    assert!(
        section.contains("The type charter declares a gate but no transitions"),
        "{section}"
    );
    let flat = flat("person-flat");
    let (html, _) = board_page(&flat);
    assert!(html.contains("Waiting on a person (0)"));
    assert!(!html.contains("declares a gate but no transitions"));
}

#[test]
fn work_is_a_tree_with_parents_counts_marks_and_a_group_for_records_with_no_parent() {
    let dir = chain("tree-page");
    let (html, _) = board_page(&dir);
    assert!(html.contains("<h3>Parents ("));
    assert!(html.contains("<h3>No parent ("));
    assert!(
        html.contains("1 finished, 1 dropped, 1 under way, 1 not started"),
        "counts beside the parent"
    );
    assert_eq!(count(&html, "status and children disagree"), 2);
    assert!(html.contains("<option value=\"open\">open</option>"));
    // PRG-1's section holds STG-2's, which holds STP-1.
    let prg1 = html.find("id=\"r-PRG-1\"").unwrap();
    let stg2 = html.find("id=\"r-STG-2\"").unwrap();
    let stp1 = html.find("id=\"r-STP-1\"").unwrap();
    assert!(prg1 < stg2 && stg2 < stp1);
    // Finished and dropped records are under their own count, collapsed.
    let closed = html.split("id=\"closed-records\"").nth(1).unwrap();
    assert!(closed.contains("records finished or dropped"));
    assert!(closed.contains("id=\"r-PRG-3\"") && closed.contains("id=\"r-STP-3\""));
    assert!(!html.contains("<details class=\"group\" id=\"closed-records\" open"));
    // The unreadable header is shown, not skipped.
    let unreadable = html.split("id=\"unreadable\"").nth(1).unwrap();
    assert!(unreadable.contains("id=\"r-STG-7\""));
}

#[test]
fn without_containment_work_is_grouped_by_type_and_other_types_are_compact_rows() {
    let dir = flat("flat-page");
    let (html, _) = board_page(&dir);
    assert!(html.contains("declares no containment"));
    assert!(!html.contains("<h3>Parents"));
    assert!(html.contains("<h3>ticket ("));
    let other = html
        .split("id=\"other\"")
        .nth(1)
        .unwrap()
        .split("id=\"closed\"")
        .next()
        .unwrap();
    assert!(other.contains("<h3>memo (2)</h3>"));
    assert!(
        other.contains("1 refer to it"),
        "a row says how many records refer to it: {other}"
    );
    assert!(html.contains("waits on: <a href=\"#r-TCK-2\">TCK-2</a>"));
    assert!(
        html.contains("Area: web, cli"),
        "a declared-value field shows"
    );
    assert!(html.contains("Related: <a href=\"#r-TCK-4\">TCK-4</a>"));
}

fn entry<'a>(html: &'a str, anchor: &str) -> &'a str {
    let at = html.find(&format!("id=\"{anchor}\"")).unwrap();
    let rest = &html[at..];
    let end = rest[1..].find("<li ").map_or(rest.len(), |i| i + 1);
    &rest[..end]
}

#[test]
fn the_containment_field_and_the_waiting_pairs_field_are_shown_once() {
    let (html, _) = board_page(&chain("once"));
    let step = entry(&html, "r-STP-10");
    assert!(
        step.contains("part of: <a href=\"#r-STG-2\">STG-2</a>"),
        "{step}"
    );
    assert!(
        step.contains("waits on: <a href=\"#r-STP-1\">STP-1</a>"),
        "{step}"
    );
    assert!(
        !step.contains("<div>In:"),
        "the containment field is not listed again: {step}"
    );
    assert!(
        !step.contains("<div>Needs:"),
        "the waiting pair's field is not listed again: {step}"
    );
    let (html, _) = board_page(&flat("once-flat"));
    let ticket = entry(&html, "r-TCK-3");
    assert!(
        ticket.contains("waits on: <a href=\"#r-TCK-2\">TCK-2</a>")
            && !ticket.contains("<div>Needs:"),
        "{ticket}"
    );
    // A finished record shows no wait, so its field is the one place its dependencies appear.
    let (html, _) = board_page(&chain("once-finished"));
    let closed = html.split("id=\"closed-records\"").nth(1).unwrap();
    assert!(!entry(closed, "r-STP-2").contains("waits on"));
}

#[test]
fn everything_from_a_record_is_escaped_and_nothing_becomes_a_link() {
    let dir = chain("escape");
    write(
        &dir,
        "docs/programmes/PRG-4-hostile.md",
        &record(
            "Status: Idea",
            "Owner: javascript:alert(1)\nLane: north\n",
            "# 4 — <script>alert(\"x\")</script> & 'quoted' \"double\"\n\n<img src=x onerror=alert(1)> and [a link](javascript:alert(2))",
        ),
    );
    common::commit_all(&dir, 0);
    let (html, _) = board_page(&dir);
    assert_self_contained(&html);
    assert!(html.contains("&lt;script&gt;alert(&quot;x&quot;)&lt;/script&gt; &amp; &#39;quoted&#39; &quot;double&quot;"));
    assert!(html.contains("&lt;img src=x onerror=alert(1)&gt;"));
    assert!(!html.contains("<script>alert"));
    assert!(!html.contains("<img"));
    assert!(html.contains("Owner: javascript:alert(1)"), "shown as text");
    assert!(!html.contains("href=\"javascript"));
    assert!(!html.contains("javascript:alert(2)\">"));
}

#[test]
fn the_script_only_filters_and_builds_no_html() {
    let dir = flat("script");
    let (html, _) = board_page(&dir);
    let script = html
        .split("<script>")
        .nth(1)
        .unwrap()
        .split("</script>")
        .next()
        .unwrap();
    for forbidden in [
        "innerHTML",
        "outerHTML",
        "insertAdjacentHTML",
        "document.write",
        "eval(",
        "Function(",
        "location",
    ] {
        assert!(!script.contains(forbidden), "{forbidden}");
    }
}

// ---------------------------------------------------------------- the server

struct Server {
    child: Child,
    port: u16,
}

impl Server {
    fn start(dir: &Path) -> Server {
        let mut child = common::urzua()
            .args(["board", "--serve", "--port", "0"])
            .current_dir(dir)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        let mut line = String::new();
        BufReader::new(child.stdout.as_mut().unwrap())
            .read_line(&mut line)
            .unwrap();
        let v: Value = serde_json::from_str(&line)
            .unwrap_or_else(|e| panic!("not one JSON line ({e}): {line}"));
        assert_eq!(v["status"], "ok");
        let port = v["port"].as_u64().unwrap() as u16;
        assert_eq!(v["address"], format!("127.0.0.1:{port}"));
        assert_eq!(v["listening"], format!("http://127.0.0.1:{port}/"));
        Server { child, port }
    }

    fn request(&self, raw: &str) -> (String, String) {
        let mut stream = TcpStream::connect(("127.0.0.1", self.port)).unwrap();
        stream
            .set_read_timeout(Some(Duration::from_secs(120)))
            .unwrap();
        stream.write_all(raw.as_bytes()).unwrap();
        let mut response = String::new();
        let _ = stream.read_to_string(&mut response);
        let (head, body) = response.split_once("\r\n\r\n").unwrap_or((&response, ""));
        (head.to_string(), body.to_string())
    }

    fn get(&self, path: &str) -> (String, String) {
        self.request(&format!(
            "GET {path} HTTP/1.1\r\nHost: a-proxy.example\r\nConnection: close\r\n\r\n"
        ))
    }

    fn interrupt_and_wait(&mut self) -> bool {
        let pid = self.child.id().to_string();
        assert!(std::process::Command::new("kill")
            .args(["-INT", &pid])
            .status()
            .unwrap()
            .success());
        let started = Instant::now();
        while started.elapsed() < Duration::from_secs(20) {
            if self.child.try_wait().unwrap().is_some() {
                return true;
            }
            std::thread::sleep(Duration::from_millis(50));
        }
        false
    }
}

impl Drop for Server {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

#[test]
fn the_server_answers_get_only_re_reads_every_request_and_stops_on_interrupt() {
    let dir = chain("serve");
    let mut server = Server::start(&dir);

    let (head, body) = server.get("/");
    assert!(head.starts_with("HTTP/1.1 200"), "{head}");
    assert!(
        head.to_lowercase().contains("cache-control: no-store"),
        "{head}"
    );
    assert!(
        head.to_lowercase().contains("content-type: text/html"),
        "{head}"
    );
    assert_self_contained(&body);
    assert_eq!(count(&body, " data-record=\""), 18);
    assert!(body.contains("Alpha"));

    write(
        &dir,
        "docs/programmes/PRG-1-alpha.md",
        &record(
            "Status: Rolling",
            "Owner: ann\nLane: north\n",
            "# 1 — Alpha renamed between requests",
        ),
    );
    let (_, body) = server.get("/");
    assert!(
        body.contains("Alpha renamed between requests"),
        "the second request reads the changed record"
    );

    assert!(
        body.contains("<a href=\"check\">Run it</a>"),
        "the served page links the check instead of running it"
    );
    assert!(!body.contains("urzua check: findings-present") && !body.contains("urzua check: ok"));
    let (head, line) = server.get("/check");
    assert!(
        head.starts_with("HTTP/1.1 200") && head.to_lowercase().contains("cache-control: no-store"),
        "{head}"
    );
    assert!(
        line.starts_with("urzua check: ")
            && line.contains("(computed 20")
            && line.contains(" took "),
        "{line}"
    );

    let (head, text) = server.get("/record/PRG-1");
    assert!(
        head.starts_with("HTTP/1.1 200") && text.contains("Alpha renamed between requests"),
        "{head}"
    );
    for path in [
        "/nope",
        "/record/",
        "/record/..%2f..%2fetc%2fpasswd",
        "/record/NOPE-9",
        "/../../etc/passwd",
        "//",
        "/index.html",
    ] {
        assert!(server.get(path).0.starts_with("HTTP/1.1 404"), "{path}");
    }
    for method in ["POST", "PUT", "DELETE", "HEAD", "OPTIONS"] {
        let (head, _) = server.request(&format!(
            "{method} / HTTP/1.1\r\nHost: x\r\nContent-Length: 0\r\n\r\n"
        ));
        assert!(head.starts_with("HTTP/1.1 405"), "{method}: {head}");
        assert!(head.contains("Allow: GET"));
    }
    let long = format!("GET /{} HTTP/1.1\r\n\r\n", "a".repeat(20_000));
    assert!(server.request(&long).0.starts_with("HTTP/1.1 400"));

    // A client that sends nothing cannot hold the server.
    let stalled = TcpStream::connect(("127.0.0.1", server.port)).unwrap();
    let started = Instant::now();
    let (head, _) = server.get("/record/PRG-1");
    assert!(head.starts_with("HTTP/1.1 200"), "{head}");
    assert!(started.elapsed() < Duration::from_secs(60));
    drop(stalled);

    let port = server.port;
    assert!(
        server.interrupt_and_wait(),
        "the server did not stop on SIGINT"
    );
    assert!(
        TcpStream::connect(("127.0.0.1", port)).is_err(),
        "the port is released"
    );
}

#[test]
fn the_server_listens_on_the_loopback_address_only_and_refuses_a_flag_misuse() {
    let dir = flat("serve-flags");
    let server = Server::start(&dir);
    assert!(server.get("/").0.starts_with("HTTP/1.1 200"));
    drop(server);
    let out = run(&dir, &["board", "--port", "9"]);
    assert_eq!(out.status.code(), Some(2), "--port needs --serve");
    let out = run(&dir, &["board", "--serve", "--out", "x.html"]);
    assert_eq!(
        out.status.code(),
        Some(2),
        "--out and --serve exclude each other"
    );
}

// ---------------------------------------------------------------- this repository's records

fn own_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../..")
}

#[test]
fn this_repositorys_records_have_a_state_each_and_the_page_holds_every_one() {
    let root = own_root();
    let lean = run(&root, &["export", "--format", "json", "--no-raw-text"]);
    let e = json(&lean);
    let records = e["records"].as_array().unwrap();
    assert!(records.len() > 600);
    let states = [
        "finished",
        "dropped",
        "under-way",
        "not-started",
        "open",
        "none",
    ];
    for r in records {
        assert!(
            states.contains(&r["state"].as_str().unwrap()),
            "{}",
            r["path"]
        );
        assert!(r["title"].is_string());
        assert!(r.get("raw_text").is_none());
    }
    let numbers: Vec<u64> = records
        .iter()
        .filter(|r| r["type"] == "milestone")
        .map(|r| {
            r["id"]
                .as_str()
                .unwrap()
                .rsplit('-')
                .next()
                .unwrap()
                .parse()
                .unwrap()
        })
        .collect();
    assert!(
        numbers.windows(2).all(|w| w[0] < w[1]),
        "milestones are in numeric order"
    );
    for r in records.iter().filter(|r| r["ready"] == true) {
        assert!(e["type_roles"][r["type"].as_str().unwrap()]["work"] == true);
        assert!(r["waiting_on"].as_array().unwrap().is_empty());
        assert!(
            ["not-started", "under-way", "open", "none"].contains(&r["state"].as_str().unwrap())
        );
    }
    let head = String::from_utf8(
        common::isolated_git()
            .args(["rev-parse", "HEAD"])
            .current_dir(&root)
            .output()
            .unwrap()
            .stdout,
    )
    .unwrap();
    assert_eq!(e["read"]["commit"], head.trim());

    let target = common::TempFile::new("urzua-board-own");
    let out = run(&root, &["board", "--out", target.to_str().unwrap()]);
    let answer = json(&out);
    assert_eq!(answer["record_count"], e["record_count"]);
    let html = std::fs::read_to_string(&*target).unwrap();
    assert_self_contained(&html);
    assert_eq!(count(&html, " data-record=\""), records.len());
    assert!(html.len() < 4 * 1024 * 1024, "{} bytes", html.len());
}

// ---------------------------------------------------------------- the additive guarantee

const NEW_RECORD_KEYS: [&str; 17] = [
    "title",
    "title_is_fallback",
    "fields",
    "status",
    "state",
    "state_decided_by",
    "waiting_on",
    "ready",
    "gated_moves",
    "parent",
    "children",
    "children_by_state",
    "state_disagrees_with_children",
    "refers_to",
    "unresolved_refs",
    "referred_by",
    "link",
];

/// The id's number, so `MILE-2` sorts before `MILE-10`.
fn natural(id: &str) -> (String, u64, String) {
    match id.rsplit_once('-') {
        Some((prefix, n)) if n.parse::<u64>().is_ok() => {
            (prefix.to_string(), n.parse().unwrap(), String::new())
        }
        _ => (id.to_string(), 0, String::new()),
    }
}

/// The export of a released build, when one is on `PATH` (or named by
/// `URZUA_RELEASED_BIN`) and predates the view model. CI has none, and the
/// test then says so instead of passing silently.
fn released_export(root: &Path) -> Option<Value> {
    let bin = std::env::var("URZUA_RELEASED_BIN").unwrap_or_else(|_| "urzua".to_string());
    let out = std::process::Command::new(&bin)
        .args(["export", "--format", "json"])
        .current_dir(root)
        .output()
        .ok()?;
    let v: Value = serde_json::from_slice(&out.stdout).ok()?;
    (out.status.success() && v.get("type_roles").is_none()).then_some(v)
}

#[test]
fn this_repositorys_export_without_the_new_keys_equals_the_released_builds() {
    let root = own_root();
    let Some(old) = released_export(&root) else {
        eprintln!("skipped: no released `urzua` that predates the view model is on PATH (set URZUA_RELEASED_BIN)");
        return;
    };
    let new = export(&root);

    let strip = |records: &Value| -> Vec<Value> {
        records
            .as_array()
            .unwrap()
            .iter()
            .cloned()
            .map(|mut r| {
                for key in NEW_RECORD_KEYS {
                    r.as_object_mut().unwrap().remove(key);
                }
                if let Some(sections) = r["sections"].as_array_mut() {
                    for s in sections {
                        for key in ["ticked", "unticked", "items"] {
                            s.as_object_mut().unwrap().remove(key);
                        }
                    }
                }
                r
            })
            .collect()
    };
    let (old_records, new_records) = (strip(&old["records"]), strip(&new["records"]));

    // The one documented change: numeric order of ids within a type.
    let key = |r: &Value| {
        (
            r["type"].as_str().unwrap().to_string(),
            natural(r["id"].as_str().unwrap_or("")),
            r["path"].as_str().unwrap().to_string(),
        )
    };
    let mut expected_order: Vec<&Value> = old_records.iter().collect();
    expected_order.sort_by_key(|r| key(r));
    let new_order: Vec<&str> = new_records
        .iter()
        .map(|r| r["path"].as_str().unwrap())
        .collect();
    let expected: Vec<&str> = expected_order
        .iter()
        .map(|r| r["path"].as_str().unwrap())
        .collect();
    assert_eq!(
        new_order, expected,
        "records are in numeric order within a type, and in no other way reordered"
    );
    let old_order: Vec<&str> = old_records
        .iter()
        .map(|r| r["path"].as_str().unwrap())
        .collect();
    assert_ne!(
        old_order, new_order,
        "the released order is by path, so the numeric order differs on this repository"
    );

    // Everything else, record for record, keyed by path.
    let by_path = |records: &[Value]| -> std::collections::BTreeMap<String, Value> {
        records
            .iter()
            .map(|r| (r["path"].as_str().unwrap().to_string(), r.clone()))
            .collect()
    };
    let (old_map, new_map) = (by_path(&old_records), by_path(&new_records));
    assert_eq!(
        old_map.keys().collect::<Vec<_>>(),
        new_map.keys().collect::<Vec<_>>()
    );
    for (path, old_record) in &old_map {
        assert_eq!(old_record, &new_map[path], "{path}");
    }
    for key in ["status", "record_count", "types", "roadmap", "notices"] {
        assert_eq!(old.get(key), new.get(key), "top-level `{key}`");
    }
}

/// Writes the pages the phone-width check measures into `URZUA_BOARD_REVIEW_DIR`:
/// both fixtures, the chain one with a record that waits on many and one with
/// long ids. Run with `--ignored`; it asserts nothing.
#[test]
#[ignore]
fn write_pages_for_the_phone_width_check() {
    let Some(out) = std::env::var_os("URZUA_BOARD_REVIEW_DIR") else {
        return;
    };
    let out = PathBuf::from(out);
    let chain_dir = chain("review-chain");
    let many: Vec<String> = (1..=14).map(|n| format!("STP-{}", n + 1000)).collect();
    for n in 1..=14 {
        write(
            &chain_dir,
            &format!("docs/steps/STP-{}-x.md", n + 1000),
            &record(
                "Status: Rolling",
                "In: STG-2\n",
                &format!("# {} — Long running step number {n}", n + 1000),
            ),
        );
    }
    write(
        &chain_dir,
        "docs/steps/STP-2000-waits.md",
        &record(
            "Status: Idea",
            &format!("In: STG-2\nNeeds: {}\n", many.join(", ")),
            "# 2000 — Waits on fourteen",
        ),
    );
    common::commit_all(&chain_dir, 0);
    std::fs::create_dir_all(&out).unwrap();
    std::fs::write(out.join("chain.html"), board_page(&chain_dir).0).unwrap();
    std::fs::write(out.join("flat.html"), board_page(&flat("review-flat")).0).unwrap();
}
