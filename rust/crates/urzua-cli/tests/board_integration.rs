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
        ("STG-7", "none", "header-unreadable"),
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
    assert!(html.contains("<h3>Under way<i>0</i></h3>"));
    let under_way = section_of(&html, "under-way");
    assert!(
        under_way.contains("<p class=\"empty\">No record is under way.</p>"),
        "{under_way}"
    );
    let note = about_of(&html);
    assert!(
        !note.contains("charter declares no <code>statuses.progress</code>"),
        "a type that declares no initial status is not named: {note}"
    );
    for name in ["programme", "stage", "step"] {
        assert!(
            note.contains(&format!("{name} declares no <code>statuses.progress</code>: until it does, its records in Rolling show as open")),
            "{note}"
        );
    }
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
    assert_eq!(by_id(&e, "STP-1")["ready"], false, "under way is not ready");
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
        false,
        "under way, so not work to pick up"
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
        "parent_ambiguous",
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
        } else if !VOID.contains(&name.as_str()) && !tag.ends_with("/>") {
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

/// Every `link` the export gives its records: the only addresses outside the
/// page that the page may carry.
fn links(export: &Value) -> Vec<String> {
    export["records"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|r| r["link"].as_str().map(str::to_string))
        .collect()
}

fn html_escape(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}

/// The page's one `<link>`: the favicon, an inline SVG address that names no
/// other file. Returns the whole tag so the scan below can leave it out after
/// checking it.
fn icon_link(html: &str) -> Option<String> {
    let start = html.find("<link")?;
    let end = start + html[start..].find('>')? + 1;
    let tag = &html[start..end];
    let prefix = "<link rel=\"icon\" href=\"data:image/svg+xml,";
    assert!(
        tag.starts_with(prefix),
        "the only link is the favicon: {tag}"
    );
    assert_eq!(html.matches("<link").count(), 1);
    let data = &tag[prefix.len()..tag.len() - 2];
    assert!(tag.ends_with("\">"));
    for bad in ["\"", "'", " ", "<", ">", "#", "script", "href", "url"] {
        assert!(!data.contains(bad), "{bad} in the favicon: {data}");
    }
    assert_eq!(data.matches("%3Crect").count(), 5, "five squares: {data}");
    assert!(data.starts_with("%3Csvg%20xmlns=%22http%3A//www.w3.org/2000/svg%22"));
    Some(tag.to_string())
}

fn assert_self_contained(html: &str, links: &[String]) {
    assert_page(html, false, links);
}

/// A served page: the same, plus the one request it may make, to its own
/// server, and nothing else.
fn assert_served(html: &str, links: &[String]) {
    assert_page(html, true, links);
}

fn assert_page(html: &str, served: bool, links: &[String]) {
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
    let mut scanned = html.to_string();
    let icon = icon_link(html);
    if let Some(link) = &icon {
        scanned = scanned.replace(link, "");
    }
    for value in attribute_values(&scanned, "href")
        .into_iter()
        .chain(attribute_values(&scanned, "src"))
    {
        let record_link = links.iter().any(|l| html_escape(l) == value);
        assert!(
            value.starts_with('#')
                || value.starts_with("record/")
                || value == "check"
                || record_link,
            "address {value}"
        );
        if record_link {
            scanned = scanned.replace(&format!(" href=\"{value}\""), "");
        }
    }
    for forbidden in [
        "http://",
        "https://",
        "XMLHttpRequest",
        "WebSocket",
        "EventSource",
        "sendBeacon",
        "@import",
        "url(",
        "<link",
        "<img",
        "<iframe",
        "<form",
    ] {
        assert!(
            !scanned.contains(forbidden),
            "the page contains {forbidden}"
        );
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
    if served {
        assert!(html.contains("script-src") && html.contains("connect-src 'self';"));
        assert_eq!(count(html, "connect-src"), 1);
        assert_eq!(count(script, "fetch("), 1, "one fetch, taking a path");
        assert!(script.contains("get('./')") && script.contains("get('stamp')"));
        assert!(
            !script.contains("http") && !script.contains("//"),
            "no other origin"
        );
    } else {
        assert!(
            !html.contains("connect-src"),
            "a file may not connect anywhere"
        );
        assert!(!script.contains("fetch(") && !script.contains("setInterval"));
        assert!(!html.contains("id=\"refresh\"") && !html.contains("id=\"stamp\""));
    }
}

#[test]
fn the_page_holds_every_record_and_is_a_self_contained_well_formed_document() {
    for (name, dir) in [("chain", chain("page-chain")), ("flat", flat("page-flat"))] {
        let e = export(&dir);
        let (html, answer) = board_page(&dir);
        assert_self_contained(&html, &links(&e));
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
        assert!(html.contains("working tree"));
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

fn section_of<'a>(html: &'a str, group: &str) -> &'a str {
    html.split(&format!("data-sec=\"{group}\""))
        .nth(1)
        .unwrap_or_else(|| panic!("no section {group}"))
        .split("</section>")
        .next()
        .unwrap()
}

/// The "About this board" panel: how each part of the page follows from the
/// configuration, and what each type declares.
fn about_of(html: &str) -> &str {
    html.split("id=\"about\"")
        .nth(1)
        .expect("an about panel")
        .split("</section>")
        .next()
        .unwrap()
}

#[test]
fn what_needs_a_person_is_the_gated_moves_the_transitions_allow() {
    let dir = chain("person");
    let (html, _) = board_page(&dir);
    let needs = section_of(&html, "needs");
    assert!(needs.contains("<h3>Needs you<i>2</i></h3>"), "{needs}");
    for id in ["STG-2", "STG-4"] {
        assert!(needs.contains(&format!(">{id}</a>")), "{id}");
    }
    assert!(!needs.contains(">STG-1</a>") && !needs.contains(">STG-3</a>"));
    assert!(needs.contains("Rolling → Shipped"), "From → To: {needs}");
    assert!(
        needs.contains("<span class=\"x k\">leads</span>"),
        "the group is a chip: {needs}"
    );
    assert!(
        needs.contains("charter declares a gate but no transitions, so the next move of its records cannot be read."),
        "{needs}"
    );
    let under_way = section_of(&html, "under-way");
    assert!(
        !under_way.contains("charter"),
        "a type that declares no initial status is not named: {under_way}"
    );
    assert!(
        !under_way.contains("programme"),
        "a type with a progress set is not named: {under_way}"
    );
    let flat = flat("person-flat");
    let (html, _) = board_page(&flat);
    assert!(!html.contains("data-sec=\"needs\""), "no gate, no section");
    assert!(!html.contains("declares a gate but no transitions"));
}

#[test]
fn work_is_a_tree_of_one_line_rows_with_progress_folds_and_a_group_for_records_with_no_parent() {
    let dir = chain("tree-page");
    let (html, _) = board_page(&dir);
    assert!(html.contains("<h3>Work<i>2 parents · "));
    assert!(html.contains("<h3>No parent<i>"));
    let prg1 = entry(&html, "r-PRG-1");
    assert!(prg1.contains("1 of 3 finished"), "{prg1}");
    assert!(
        prg1.contains("<details class=\"kd\" open>"),
        "a parent with an active child starts open: {prg1}"
    );
    assert!(
        prg1.contains("<span class=\"ch tri\">"),
        "a parent has a disclosure triangle"
    );
    assert!(entry(&html, "r-STG-2").contains("1 of 4 finished"));
    assert_eq!(count(&html, " · disagrees"), 2);
    assert!(html.contains("data-f=\"open\""));
    // PRG-1's row holds STG-2's, which holds STP-1.
    let prg1 = html.find("id=\"r-PRG-1\"").unwrap();
    let stg2 = html.find("id=\"r-STG-2\"").unwrap();
    let stp1 = html.find("id=\"r-STP-1\"").unwrap();
    assert!(prg1 < stg2 && stg2 < stp1);
    // STG-2's finished and dropped children are one muted row, opening in place.
    let fold = html
        .find("1 finished, 1 dropped, collapsed")
        .expect("the children's fold");
    assert!(fold > stp1);
    assert!(html[fold..].contains("id=\"r-STP-3\"") && html[fold..].contains("id=\"r-STP-2\""));
    // A finished parent is in the work that is finished and dropped.
    let closed = html.split("<h3>Finished and dropped<i>").nth(1).unwrap();
    assert!(closed.contains("id=\"r-PRG-3\""));
    // The unreadable header is shown, not skipped.
    let unreadable = html
        .split("<h3>Headers that could not be read<i>1</i></h3>")
        .nth(1)
        .unwrap();
    assert!(unreadable.contains("id=\"r-STG-7\""));
}

#[test]
fn a_row_is_one_line_with_the_status_as_written_beside_a_dot_that_says_its_state() {
    let dir = chain("row");
    let (html, _) = board_page(&dir);
    let alpha = entry(&html, "r-PRG-1");
    assert!(alpha.contains("<span class=\"id\">PRG-1</span>"), "{alpha}");
    assert!(alpha.contains("<span class=\"t\">Alpha</span>"));
    assert!(
        alpha.contains(
            "<span class=\"d o\" role=\"img\" aria-label=\"under way\" title=\"under way\"></span><span class=\"st\">Rolling</span>"
        ),
        "{alpha}"
    );
    assert!(
        alpha.contains("title=\"Lane\">north</span>"),
        "a declared-value field is a chip: {alpha}"
    );
    let finished = entry(&html, "r-STP-2");
    assert!(finished.contains(
        "<span class=\"d g\" role=\"img\" aria-label=\"finished\" title=\"finished\"></span><span class=\"st\">Shipped</span>"
    ));
    let dropped = entry(&html, "r-STP-3");
    assert!(dropped.contains(
        "<span class=\"d r\" role=\"img\" aria-label=\"dropped\" title=\"dropped\"></span>"
    ));
    let ready = entry(&html, "r-STP-11");
    assert!(ready.contains("<span class=\"d b\" role=\"img\" aria-label=\"ready (not started)\" title=\"ready (not started)\"></span><span class=\"st\">Idea</span>"), "{ready}");
    let open = entry(&html, "r-CHR-1");
    assert!(
        open.contains("<span class=\"d n\" role=\"img\" aria-label=\"open\" title=\"open\"></span>")
            || open.contains("<span class=\"d b\" role=\"img\" aria-label=\"ready (open)\" title=\"ready (open)\"></span>"),
        "{open}"
    );
    let waiting = entry(&html, "r-STP-10");
    assert!(
        waiting.contains("<span class=\"x w\">waits on <a href=\"#r-STP-1\">STP-1</a></span>"),
        "{waiting}"
    );
    // Colour is a dot and the waits-on text: no badge, no filled mark.
    for gone in [
        "class=\"mark",
        "m-waiting",
        "m-ready",
        "box-shadow",
        "gradient",
    ] {
        assert!(!html.contains(gone), "{gone}");
    }
}

#[test]
fn without_containment_work_is_grouped_by_type_and_other_types_are_rows_under_their_own_name() {
    let dir = flat("flat-page");
    let (html, _) = board_page(&dir);
    assert!(html.contains("declares no containment"));
    assert!(!html.contains("<h3>Work<i>"));
    assert!(
        html.contains("<h3 class=\"ty\">ticket<i>"),
        "the type name as declared"
    );
    assert!(html.contains("<h3 class=\"ty\">memo<i>2</i></h3>"));
    assert!(
        entry(&html, "r-MEM-1").contains("1 refer"),
        "a row says how many records refer to it"
    );
    assert!(html.contains("waits on <a href=\"#r-TCK-2\">TCK-2</a>"));
    let ticket = entry(&html, "r-TCK-4");
    assert!(
        ticket.contains("title=\"Area\">web, cli</span>"),
        "a declared-value field is a chip: {ticket}"
    );
    assert!(html.contains("Related: <a href=\"#r-TCK-4\">TCK-4</a>"));
    // Finished tickets are one muted row; the type has fewer than eleven memos, so they stay rows.
    assert!(
        html.contains("2 finished, collapsed") || html.contains("1 finished, 1 dropped, collapsed")
    );
}

#[test]
fn a_type_with_more_than_ten_records_folds_its_finished_ones() {
    let dir = repo("many", FLAT_CONFIG, |dir| {
        for n in 1..=12 {
            let status = if n <= 5 { "Done" } else { "Todo" };
            write(
                dir,
                &format!("docs/memos/MEM-{n}-m.md"),
                &record("Topic: x", "", &format!("# {n} — Memo {n}")),
            );
            write(
                dir,
                &format!("docs/tickets/TCK-{n}-t.md"),
                &record(
                    &format!("Status: {status}"),
                    "",
                    &format!("# {n} — Ticket {n}"),
                ),
            );
        }
    });
    let (html, _) = board_page(&dir);
    let memos = html
        .split("<h3 class=\"ty\">memo<i>12</i></h3>")
        .nth(1)
        .unwrap()
        .split("</section>")
        .next()
        .unwrap();
    assert_eq!(count(memos, " data-record=\""), 12);
    // Memos declare no statuses, so none is finished; tickets are work and always fold.
    assert!(!memos.contains("collapsed"));
    let tickets = html
        .split("<h3 class=\"ty\">ticket<i>12</i></h3>")
        .nth(1)
        .unwrap()
        .split("</section>")
        .next()
        .unwrap();
    assert!(tickets.contains("5 finished, collapsed"), "{tickets}");
}

const CHECKLIST_CONFIG: &str = "schema_version: 2
record_types:
  job:
    dir: docs/jobs
    prefix: JOB
    header_shape: yaml-frontmatter
    required_fields: [Status]
    known_fields: [Embodiment, Realized-by]
    pointer_fields: []
    narrative_fields: []
    sections:
      from: headings
      depth: 2
      required:
        - {name: Checks}
    statuses:
      values: [Todo, Doing, Done]
      success: [Done]
      progress: [Doing]
      delivery:
        delivered_tiers: [Implemented]
        unevidenced_tiers: [Not started]
        not_done: [Todo, Doing]
        set_status: Done
        items:
          section: Checks
          clause_separator: \" — \"
          evidence_marker: \"evidence:\"
          waiver_marker: \"waived:\"
          partial_status: Doing
";

fn checklist_repo(name: &str) -> common::TempDir {
    repo(name, CHECKLIST_CONFIG, |dir| {
        write(dir, "src/a.rs", "fn a() {}\n");
        write(
            dir,
            "docs/jobs/JOB-1-j.md",
            "---\nStatus: Doing\n---\n# 1 — Job\n\n## Checks\n\n- First item — evidence: code:src/a.rs\n- Second item\n- Third item\n",
        );
    })
}

#[test]
fn a_declared_checklist_shows_its_count_in_the_row_and_its_items_in_the_opened_row() {
    let dir = checklist_repo("checklist");
    let (html, _) = board_page(&dir);
    let job = entry(&html, "r-JOB-1");
    assert!(
        job.contains("title=\"checklist items resolved\">1/3</span>"),
        "{job}"
    );
    assert!(
        job.contains("<code>[x]</code> First item — code:src/a.rs"),
        "{job}"
    );
    assert!(
        job.contains("<code>[ ]</code> Second item") && job.contains("<code>[ ]</code> Third item"),
        "{job}"
    );
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
        step.contains("<div>part of <a href=\"#r-STG-2\">STG-2</a></div>"),
        "{step}"
    );
    assert!(
        step.contains("<div class=\"w\">waits on <a href=\"#r-STP-1\">STP-1</a></div>"),
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
        ticket.contains("waits on <a href=\"#r-TCK-2\">TCK-2</a>")
            && !ticket.contains("<div>Needs:"),
        "{ticket}"
    );
    // A finished record shows no wait, so its field is the one place its dependencies appear.
    let (html, _) = board_page(&chain("once-finished"));
    assert!(!entry(&html, "r-STP-2").contains("waits on"));
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
    assert_self_contained(&html, &links(&export(&dir)));
    assert!(html.contains("&lt;script&gt;alert(&quot;x&quot;)&lt;/script&gt; &amp; &#39;quoted&#39; &quot;double&quot;"));
    assert!(html.contains("&lt;img src=x onerror=alert(1)&gt;"));
    assert!(!html.contains("<script>alert"));
    assert!(!html.contains("<img"));
    assert!(html.contains("Owner: javascript:alert(1)"), "shown as text");
    assert!(!html.contains("href=\"javascript"));
    assert!(!html.contains("javascript:alert(2)\">"));
}

#[test]
fn the_script_keeps_the_open_state_it_had_before_a_filter_and_restores_it_when_cleared() {
    let (html, _) = board_page(&flat("restore"));
    let script = html
        .split("<script>")
        .nth(1)
        .unwrap()
        .split("</script>")
        .next()
        .unwrap();
    assert!(
        script.contains("saved = snapshot()"),
        "taken when a filter starts"
    );
    assert!(
        script.contains("restore(saved); saved = null;"),
        "put back when it is cleared"
    );
    let dir = flat("restore-served");
    let served = Server::start(&dir);
    let (_, page) = served.get("/");
    assert!(
        page.contains("var open = saved !== null ? saved : snapshot();"),
        "a swap during a filter keeps the state from before it"
    );
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
        Server::start_with(dir, &[], &[])
    }

    /// A server with more arguments and test-seam delays in its environment.
    fn start_with(dir: &Path, args: &[&str], envs: &[(&str, &str)]) -> Server {
        let mut child = common::urzua()
            .args(["board", "--serve", "--port", "0"])
            .args(args)
            .envs(envs.iter().copied())
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

    fn get_with_host(&self, path: &str, host: Option<&str>) -> String {
        let host = host.map_or(String::new(), |h| format!("Host: {h}\r\n"));
        self.request(&format!(
            "GET {path} HTTP/1.1\r\n{host}Connection: close\r\n\r\n"
        ))
        .0
    }

    fn get(&self, path: &str) -> (String, String) {
        self.request(&format!(
            "GET {path} HTTP/1.1\r\nHost: a-proxy.example\r\nConnection: close\r\n\r\n"
        ))
    }

    fn interrupt_and_wait(&mut self) -> bool {
        let pid = self.child.id().to_string();
        assert!(common::command_with_stub_gh("kill")
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
    assert_served(&body, &links(&export(&dir)));
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
        body.contains("check summary: <a href=\"check\">/check</a>"),
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

fn stamp(server: &Server) -> Value {
    let (head, body) = server.get("/stamp");
    assert!(head.starts_with("HTTP/1.1 200"), "{head}");
    let lower = head.to_lowercase();
    assert!(
        lower.contains("cache-control: no-store")
            && lower.contains("content-type: application/json"),
        "{head}"
    );
    serde_json::from_str(&body).unwrap_or_else(|e| panic!("not JSON ({e}): {body}"))
}

#[test]
fn the_stamp_changes_with_a_record_or_the_configuration_and_is_stable_otherwise() {
    let dir = chain("stamp");
    let mut server = Server::start(&dir);
    let first = stamp(&server);
    let digest = first["digest"].as_str().unwrap().to_string();
    assert_eq!(digest.len(), 64);
    assert_eq!(first["branch"], "main");
    assert_eq!(first["commit"].as_str().unwrap().len(), 40);

    // Reading the page, the check and a record's text changes nothing.
    let _ = server.get("/");
    let _ = server.get("/record/PRG-1");
    assert_eq!(stamp(&server), first, "stable while nothing changes");

    // A record file changes it, and the page built next reflects the change and carries the new digest.
    write(
        &dir,
        "docs/programmes/PRG-1-alpha.md",
        &record(
            "Status: Rolling",
            "Owner: ann\nLane: north\n",
            "# 1 — Alpha, edited",
        ),
    );
    let after_record = stamp(&server);
    assert_ne!(after_record["digest"], first["digest"]);
    assert_eq!(
        after_record["commit"], first["commit"],
        "the commit did not move"
    );
    let (_, page) = server.get("/");
    assert!(
        page.contains("Alpha, edited"),
        "the page reflects the change"
    );
    assert!(page.contains(&format!(
        "data-d=\"{}\"",
        after_record["digest"].as_str().unwrap()
    )));

    // A new record file, a removed one, and the configuration each change it.
    write(
        &dir,
        "docs/steps/STP-99-new.md",
        &record("Status: Idea", "In: STG-2\n", "# 99 — New"),
    );
    let after_new = stamp(&server);
    assert_ne!(after_new["digest"], after_record["digest"]);
    std::fs::remove_file(dir.join("docs/steps/STP-99-new.md")).unwrap();
    assert_ne!(stamp(&server)["digest"], after_new["digest"]);
    let before_config = stamp(&server);
    let config = std::fs::read_to_string(dir.join(".urzua/config.yaml")).unwrap();
    std::fs::write(
        dir.join(".urzua/config.yaml"),
        format!("{config}# a comment\n"),
    )
    .unwrap();
    assert_ne!(stamp(&server)["digest"], before_config["digest"]);

    // The server's rules: GET only, a query is ignored, no other path.
    let (head, _) = server.request("POST /stamp HTTP/1.1\r\nHost: x\r\nContent-Length: 0\r\n\r\n");
    assert!(
        head.starts_with("HTTP/1.1 405") && head.contains("Allow: GET"),
        "{head}"
    );
    assert!(server.get("/stamp?now=1").0.starts_with("HTTP/1.1 200"));
    assert!(server.get("/stamp/x").0.starts_with("HTTP/1.1 404"));
    assert!(server.interrupt_and_wait());
}

#[test]
fn a_served_page_updates_itself_and_a_file_does_not() {
    let dir = chain("live");
    let (file, _) = board_page(&dir);
    assert_self_contained(&file, &links(&export(&dir)));
    assert!(
        file.contains("static file, generated 20"),
        "the header says it is a file"
    );
    assert!(
        !file.contains("Refresh")
            && !file.contains("setInterval")
            && !file.contains("visibilitychange")
    );

    let server = Server::start(&dir);
    let (_, page) = server.get("/");
    assert_served(&page, &links(&export(&dir)));
    assert!(page.contains("<button type=\"button\" id=\"refresh\" class=\"rf\">Refresh</button>"));
    assert!(
        page.contains("id=\"upd\"") && page.contains("title=\"20") && page.contains(">updated 20"),
        "the exact time, for a reader without script"
    );
    let script = page
        .split("<script>")
        .nth(1)
        .unwrap()
        .split("</script>")
        .next()
        .unwrap();
    assert!(script.contains("15000"), "polls every 15 seconds");
    assert!(
        script.contains("visibilitychange") && script.contains("document.hidden"),
        "and only while the tab is visible"
    );
    assert!(script.contains("not updating: last read"));
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
    assert!(page.contains(".warn") && page.contains("id=\"board\""));
    // The swap replaces the main element's children with nodes of the fetched document.
    assert!(script.contains("replaceChildren") && script.contains("importNode"));
}

/// The tags of `html` in order, as `(name, is_closing, attributes)`; text and
/// the contents of `script` and `style` are skipped.
fn tags(html: &str) -> Vec<(String, bool, String)> {
    let mut out = Vec::new();
    let mut at = 0;
    while let Some(open) = html[at..].find('<') {
        let start = at + open;
        let rest = &html[start..];
        if rest.starts_with("<!") {
            at = start + rest.find('>').unwrap() + 1;
            continue;
        }
        let closing = rest.starts_with("</");
        let body = &rest[if closing { 2 } else { 1 }..];
        let name: String = body
            .chars()
            .take_while(|c| c.is_ascii_alphanumeric())
            .collect();
        let mut end = 0;
        let mut quote = false;
        for (i, c) in body.char_indices() {
            match c {
                '"' => quote = !quote,
                '>' if !quote => {
                    end = i;
                    break;
                }
                _ => {}
            }
        }
        let attrs = body[name.len()..end].to_string();
        at = start + (if closing { 2 } else { 1 }) + end + 1;
        if !closing && (name == "script" || name == "style") {
            at += html[at..].find(&format!("</{name}>")).unwrap();
        }
        out.push((name, closing, attrs));
    }
    out
}

/// For each element whose open tag has `class="<class>"`, whether it is shown
/// by default: every `details` around it is open.
fn shown_by_default(html: &str, class: &str) -> Vec<bool> {
    const VOID: [&str; 6] = ["meta", "br", "input", "link", "img", "hr"];
    let mut stack: Vec<(String, bool)> = Vec::new();
    let mut found = Vec::new();
    for (name, closing, attrs) in tags(html) {
        if closing {
            stack.pop();
            continue;
        }
        if attrs.contains(&format!("class=\"{class}\"")) {
            found.push(
                stack
                    .iter()
                    .filter(|(n, _)| n == "details")
                    .all(|(_, open)| *open),
            );
        }
        if !VOID.contains(&name.as_str()) {
            let open = name == "details" && attrs.split_whitespace().any(|a| a == "open");
            stack.push((name, open));
        }
    }
    found
}

#[test]
fn a_records_own_detail_is_closed_by_default_and_apart_from_its_children() {
    let dir = chain("detail");
    let (html, _) = board_page(&dir);
    let records = count(&html, " data-record=\"");
    // Every record has its own detail disclosure, and none of them starts open.
    assert_eq!(count(&html, "<details class=\"det\">"), records);
    assert!(!html.contains("<details class=\"det\" open"));
    // No record's text or reference lines are inside anything that is open by default.
    let more = shown_by_default(&html, "more");
    assert_eq!(more.len(), records, "one detail area per record");
    assert!(
        more.iter().all(|open| !open),
        "a record's detail area is shown by default"
    );
    assert!(
        html.contains("Text of the record."),
        "the text is on the page, behind the detail"
    );
    // A parent's children are a separate disclosure, open only for an active child.
    assert!(html.contains("<details class=\"kd\" open>"));
    let prg1 = entry(&html, "r-PRG-1");
    assert!(prg1.contains("<summary class=\"rs\">") && prg1.contains("<span class=\"ch tri\">"));
    assert!(
        !prg1.contains("class=\"more\""),
        "the children list does not hold the parent's own detail"
    );
    // The control is a summary with an accessible name, and a word for wide screens.
    assert!(html.contains("<summary aria-label=\"Details of PRG-1\" title=\"Details\">"));
    assert!(html.contains("<span class=\"dw\">details</span><span class=\"dn\">›</span>"));
}

#[test]
fn ready_lists_leaves_only_and_a_parent_stays_in_work_with_its_count() {
    let dir = chain("ready");
    let e = export(&dir);
    assert_eq!(
        by_id(&e, "PRG-2")["ready"],
        true,
        "the export's key is unchanged"
    );
    let (html, _) = board_page(&dir);
    let ready = section_of(&html, "ready");
    assert!(
        !ready.contains(">PRG-2</a>") && !ready.contains(">STG-2</a>"),
        "a parent is not listed as ready: {ready}"
    );
    assert!(ready.contains(">STP-11</a>"), "a leaf is");
    let leaves = e["records"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|r| {
            r["ready"] == true
                && r["header_readable"] == true
                && r["children"].as_array().unwrap().is_empty()
        })
        .count();
    assert!(
        ready.contains(&format!("<i>{leaves}</i>")),
        "{leaves}: {}",
        &ready[..120]
    );
    assert!(html.contains(&format!(">Ready<i>{leaves}</i></button>")));
    assert!(!entry(&html, "r-PRG-2").contains("data-flags=\"ready\""));
    assert!(entry(&html, "r-PRG-2").contains("0 of 1 finished"));
}

#[test]
fn the_three_lists_at_the_top_show_eight_rows_then_one_more_row_that_opens_in_place() {
    let dir = repo("eight", &chain_config(), |dir| {
        chain_records(dir);
        for n in 20..30 {
            write(
                dir,
                &format!("docs/stages/STG-{n}-g.md"),
                &record("Status: Rolling", "", &format!("# {n} — Gated {n}")),
            );
            write(
                dir,
                &format!("docs/stages/STG-{}-i.md", n + 20),
                &record("Status: Idea", "", &format!("# {} — Idle {n}", n + 20)),
            );
        }
    });
    let (html, _) = board_page(&dir);
    for group in ["needs", "under-way", "ready"] {
        let list = section_of(&html, group);
        assert_eq!(
            count(list, "<li class=\"br\">")
                - count(
                    list.split("<li id=\"more-").nth(1).unwrap_or(""),
                    "<li class=\"br\">"
                ),
            8,
            "{group}: eight rows before the fold"
        );
        assert!(
            list.contains(&format!("<li id=\"more-{group}\" class=\"fold\">")),
            "{group}"
        );
        assert!(list.contains(" more</summary>"));
    }
    assert!(
        section_of(&html, "needs").contains(">4 more<")
            || section_of(&html, "needs").contains("4 more</summary>")
    );
}

#[test]
fn the_header_area_has_one_short_line_for_the_check_and_none_for_anything_else() {
    let dir = chain("header");
    let (file, _) = board_page(&dir);
    let server = Server::start(&dir);
    let (_, served) = server.get("/");
    for (name, html) in [("file", &file), ("served", &served)] {
        let top = html
            .split("<main id=\"board\">")
            .nth(1)
            .unwrap()
            .split("<p id=\"shown\"")
            .next()
            .unwrap();
        let lines: Vec<&str> = top.lines().filter(|l| l.contains("<p")).collect();
        assert!(!lines.is_empty() && lines.len() <= 2, "{name}: {lines:?}");
        for line in lines {
            assert!(
                line.len() < 150 && line.matches(". ").count() == 0,
                "{name}: one short line: {line}"
            );
        }
    }
    assert!(served.contains(
        "check summary: <a href=\"check\">/check</a> (runs urzua check, which takes a while)"
    ));
    let _ = server.get("/check");
    let (_, again) = server.get("/");
    assert!(
        again.contains("(runs urzua check, about "),
        "after one run the page says how long it took"
    );
}

fn status_line(head: &str) -> &str {
    head.lines().next().unwrap_or_default()
}

// ---------------------------------------------------------------- fixes from review

#[test]
fn a_record_whose_header_cannot_be_read_is_not_work_in_the_export() {
    let dir = repo("unreadable", &chain_config(), |dir| {
        chain_records(dir);
        write(
            dir,
            "docs/steps/STP-20-under-broken.md",
            &record("Status: Idea", "In: STG-7\n", "# 20 — Under a broken stage"),
        );
    });
    let e = export(&dir);
    let broken = by_id(&e, "STG-7");
    assert_eq!(broken["header_readable"], false);
    assert_eq!(broken["state"], "none");
    assert_eq!(broken["state_decided_by"], "header-unreadable");
    assert_eq!(broken["status"], Value::Null);
    assert_eq!(broken["ready"], false);
    assert_eq!(broken["waiting_on"], serde_json::json!([]));
    assert_eq!(broken["parent"], Value::Null);
    assert_eq!(
        broken["children"],
        serde_json::json!([]),
        "no children derived from it"
    );
    assert_eq!(
        by_id(&e, "STP-20")["parent"],
        Value::Null,
        "nor a parent derived to it"
    );
    assert!(e["records"]
        .as_array()
        .unwrap()
        .iter()
        .all(|r| r["header_readable"] == true || r["ready"] == false));
}

#[test]
fn ready_is_work_someone_can_pick_up() {
    let dir = chain("ready-meaning");
    let e = export(&dir);
    assert_eq!(by_id(&e, "STP-1")["state"], "under-way");
    assert_eq!(by_id(&e, "STP-1")["ready"], false, "under way");
    assert_eq!(by_id(&e, "PRG-1")["ready"], false, "under way");
    assert_eq!(
        by_id(&e, "STP-11")["ready"],
        true,
        "not started, waits on nothing"
    );
    assert_eq!(
        by_id(&e, "CHR-1")["ready"],
        true,
        "open, in a work type that declares statuses"
    );
    assert_eq!(by_id(&e, "STP-10")["ready"], false, "waits");
    let flat = flat("ready-meaning-flat");
    let e = export(&flat);
    assert_eq!(by_id(&e, "MEM-1")["state"], "none");
    assert_eq!(
        by_id(&e, "MEM-1")["ready"],
        false,
        "a type with no statuses is never ready"
    );
    // The Ready list and the Under way list share no record.
    let (html, _) = board_page(&chain("ready-meaning-page"));
    let ready = section_of(&html, "ready");
    let under = section_of(&html, "under-way");
    for id in ["PRG-1", "STG-2", "STG-4", "STP-1"] {
        assert!(
            under.contains(&format!(">{id}</a>")) && !ready.contains(&format!(">{id}</a>")),
            "{id}"
        );
    }
}

#[test]
fn an_unstructured_wait_does_not_make_a_record_wait() {
    let config = FLAT_CONFIG.replace(
        "known_fields: [Needs, Gates, Area]",
        "known_fields: [Needs, Gates, Area, Blocked-on]",
    );
    let dir = repo("free-text", &config, |dir| {
        write(
            dir,
            "docs/tickets/TCK-1-t.md",
            &record(
                "Status: Todo",
                "Blocked-on: waiting for a review of TCK-2\n",
                "# 1 — T",
            ),
        );
        write(
            dir,
            "docs/tickets/TCK-2-t.md",
            &record("Status: Todo", "", "# 2 — T"),
        );
    });
    let e = export(&dir);
    assert_eq!(by_id(&e, "TCK-1")["waiting_on"], serde_json::json!([]));
    assert_eq!(by_id(&e, "TCK-1")["ready"], true);
    let roadmap = json(&run(&dir, &["roadmap"]));
    assert_eq!(
        roadmap["roadmap"]["records"]["TCK-1"]["blocked"], false,
        "the roadmap does not count it either"
    );
}

#[test]
fn a_part_of_field_naming_two_records_gives_no_parent_and_says_so() {
    let dir = repo("ambiguous", &chain_config(), |dir| {
        chain_records(dir);
        write(
            dir,
            "docs/steps/STP-20-two.md",
            &record("Status: Idea", "In: STG-1, STG-2\n", "# 20 — Two parents"),
        );
    });
    let e = export(&dir);
    let two = by_id(&e, "STP-20");
    assert_eq!(two["parent"], Value::Null);
    assert_eq!(ids(&two["parent_ambiguous"]), ["STG-1", "STG-2"]);
    for stage in ["STG-1", "STG-2"] {
        assert!(
            !ids(&by_id(&e, stage)["children"]).contains(&"STP-20".to_string()),
            "{stage}"
        );
    }
    assert_eq!(
        by_id(&e, "STP-1")["parent_ambiguous"],
        serde_json::json!([])
    );
    let (html, _) = board_page(&dir);
    let no_parent = html
        .split("<h3>No parent<i>")
        .nth(1)
        .unwrap()
        .split("</section>")
        .next()
        .unwrap();
    assert!(
        no_parent.contains("id=\"r-STP-20\""),
        "in the no-parent group"
    );
    assert!(entry(&html, "r-STP-20").contains("names 2 parents"));
}

#[test]
fn children_by_state_counts_exactly_the_children_listed() {
    let dir = repo("child-counts", &chain_config(), |dir| {
        chain_records(dir);
        write(
            dir,
            "docs/steps/loose-note.md",
            &record("Status: Idea", "In: STG-2\n", "# A file with no id"),
        );
    });
    let e = export(&dir);
    for r in e["records"].as_array().unwrap() {
        let total: u64 = r["children_by_state"]
            .as_object()
            .unwrap()
            .values()
            .map(|n| n.as_u64().unwrap())
            .sum();
        assert_eq!(
            total as usize,
            r["children"].as_array().unwrap().len(),
            "{}",
            r["path"]
        );
    }
}

#[test]
fn a_part_of_loop_counts_the_parents_that_are_drawn() {
    let config = "schema_version: 2
record_types:
  node:
    dir: docs/nodes
    prefix: NODE
    header_shape: yaml-frontmatter
    required_fields: [Status]
    known_fields: [Up]
    narrative_fields: []
    pointer_fields:
      - {name: Up, targets: [node], part_of: true}
    statuses:
      values: [Open, Closed]
      success: [Closed]
";
    let dir = repo("loop", config, |dir| {
        write(
            dir,
            "docs/nodes/NODE-1-a.md",
            &record("Status: Open", "Up: NODE-2\n", "# 1 — A"),
        );
        write(
            dir,
            "docs/nodes/NODE-2-b.md",
            &record("Status: Open", "Up: NODE-1\n", "# 2 — B"),
        );
        write(
            dir,
            "docs/nodes/NODE-3-c.md",
            &record("Status: Open", "Up: NODE-1\n", "# 3 — C"),
        );
    });
    let (html, _) = board_page(&dir);
    assert_eq!(
        count(&html, " data-record=\""),
        3,
        "every record is still on the page once"
    );
    assert!(
        html.contains("<h3>Work<i>1 parent · 3 records</i></h3>"),
        "{}",
        html.split("<h3>Work")
            .nth(1)
            .unwrap()
            .chars()
            .take(80)
            .collect::<String>()
    );
}

#[test]
fn an_export_can_leave_out_all_text_and_the_checklist_evaluation() {
    let dir = checklist_repo("export-flags");
    let args = |extra: &[&str]| {
        let mut all = vec!["export", "--format", "json"];
        all.extend_from_slice(extra);
        json(&run(&dir, &all))
    };
    let full = args(&[]);
    let job = by_id(&full, "JOB-1");
    assert!(job["raw_text"].is_string());
    assert!(job["sections"][0]["content"].is_string());
    assert_eq!(job["sections"][0]["items"].as_array().unwrap().len(), 3);

    let no_raw = args(&["--no-raw-text"]);
    let job = by_id(&no_raw, "JOB-1");
    assert!(job.get("raw_text").is_none());
    assert!(
        job["sections"][0]["content"].is_string(),
        "--no-raw-text keeps section content"
    );

    let no_text = args(&["--no-text"]);
    let job = by_id(&no_text, "JOB-1");
    assert!(job.get("raw_text").is_none());
    assert!(
        job["sections"][0].get("content").is_none(),
        "--no-text leaves section content out too"
    );
    for key in [
        "name", "heading", "line", "end_line", "ticked", "unticked", "items",
    ] {
        assert!(job["sections"][0].get(key).is_some(), "{key}");
    }

    let no_items = args(&["--no-checklist-items"]);
    let job = by_id(&no_items, "JOB-1");
    assert!(job["sections"][0].get("items").is_none());
    assert!(job["raw_text"].is_string(), "and nothing else changes");
    assert_eq!(job["state"], by_id(&full, "JOB-1")["state"]);
}

#[test]
fn a_link_prefix_with_a_quote_or_a_second_slash_is_refused_and_a_path_is_encoded() {
    for bad in [
        "https://h.test/\"><script>alert(1)</script>",
        "//evil.example/",
        "https://h.test/'x",
        "https://h.test/a b",
    ] {
        let config = chain_config().replace("https://example.test/blob/main/", bad);
        let dir = repo("bad-link", &config, chain_records);
        let out = run(&dir, &["export", "--format", "json"]);
        assert_eq!(out.status.code(), Some(2), "{bad}");
        assert!(
            String::from_utf8_lossy(&out.stdout).contains("link_prefix"),
            "{bad}"
        );
    }
    let dir = repo("odd-name", &chain_config(), |dir| {
        chain_records(dir);
        write(
            dir,
            "docs/programmes/PRG-9-a#b?c%d e.md",
            &record("Status: Idea", "", "# 9 — Odd name"),
        );
    });
    let e = export(&dir);
    assert_eq!(
        by_id(&e, "PRG-9")["link"],
        "https://example.test/blob/main/docs/programmes/PRG-9-a%23b%3Fc%25d%20e.md"
    );
}

#[test]
fn an_idle_connection_is_dropped_in_two_seconds_and_a_full_server_refuses_at_once() {
    let dir = chain("idle");
    let server = Server::start(&dir);
    // One idle client is closed by the server within its head budget.
    let mut idle = TcpStream::connect(("127.0.0.1", server.port)).unwrap();
    idle.set_read_timeout(Some(Duration::from_secs(5))).unwrap();
    let started = Instant::now();
    let mut buffer = [0u8; 16];
    let read = idle.read(&mut buffer);
    assert!(matches!(read, Ok(0)) || read.is_err(), "{read:?}");
    assert!(
        started.elapsed() < Duration::from_millis(3500),
        "{:?}",
        started.elapsed()
    );

    // More idle connections than the server takes at once: a real request is answered or refused, never queued.
    let held: Vec<TcpStream> = (0..48)
        .map(|_| TcpStream::connect(("127.0.0.1", server.port)).unwrap())
        .collect();
    std::thread::sleep(Duration::from_millis(300));
    let started = Instant::now();
    let (head, _) = server.get("/stamp");
    assert!(
        started.elapsed() < Duration::from_millis(1500),
        "queued behind idle clients: {:?}",
        started.elapsed()
    );
    let status = status_line(&head);
    assert!(status.contains("200") || status.contains("503"), "{status}");
    // Those past the cap were refused outright.
    let refused = held
        .into_iter()
        .filter_map(|mut s| {
            s.set_read_timeout(Some(Duration::from_millis(200))).ok()?;
            let mut text = String::new();
            let _ = s.read_to_string(&mut text);
            text.starts_with("HTTP/1.1 503").then_some(())
        })
        .count();
    assert!(refused > 0, "no connection was refused with 503");
    // And once they have timed out, the server answers again.
    std::thread::sleep(Duration::from_millis(2600));
    assert!(status_line(&server.get("/stamp").0).contains("200"));
}

#[test]
fn check_runs_once_at_a_time_is_cached_and_never_delays_the_page_or_the_stamp() {
    let dir = chain("check-gate");
    let server = Server::start_with(&dir, &[], &[("URZUA_BOARD_TEST_CHECK_DELAY_MS", "2500")]);
    let started = Instant::now();
    let (texts, quick) = std::thread::scope(|scope| {
        let runs: Vec<_> = (0..3)
            .map(|_| scope.spawn(|| server.get("/check").1))
            .collect();
        std::thread::sleep(Duration::from_millis(400));
        // While a run is in flight, the stamp and the page answer at once.
        let t = Instant::now();
        let stamp_head = server.get("/stamp").0;
        let page_head = server.get("/").0;
        let quick = (t.elapsed(), stamp_head, page_head);
        (
            runs.into_iter()
                .map(|r| r.join().unwrap())
                .collect::<Vec<_>>(),
            quick,
        )
    });
    assert!(
        quick.0 < Duration::from_millis(2000),
        "the stamp and the page waited on the check: {:?}",
        quick.0
    );
    assert!(status_line(&quick.1).contains("200") && status_line(&quick.2).contains("200"));
    assert_eq!(texts[0], texts[1]);
    assert_eq!(
        texts[1], texts[2],
        "requests that arrive during a run share its result"
    );
    assert!(
        started.elapsed() < Duration::from_millis(5800),
        "three runs ran one after another: {:?}",
        started.elapsed()
    );
    // The result is served from the cache while the digest is the same.
    let t = Instant::now();
    assert_eq!(server.get("/check").1, texts[0]);
    assert!(
        t.elapsed() < Duration::from_millis(800),
        "{:?}",
        t.elapsed()
    );
    // A change to a record makes the next request run again.
    write(
        &dir,
        "docs/steps/STP-1-a.md",
        &record("Status: Idea", "In: STG-2\n", "# 1 — A, changed"),
    );
    let t = Instant::now();
    let again = server.get("/check").1;
    assert!(
        t.elapsed() >= Duration::from_millis(2300),
        "{:?}",
        t.elapsed()
    );
    assert!(again.starts_with("urzua check: "));
}

#[test]
fn the_page_and_a_records_text_are_cached_against_the_digest() {
    let dir = chain("page-cache");
    let server = Server::start_with(&dir, &[], &[("URZUA_BOARD_TEST_BUILD_DELAY_MS", "1500")]);
    // A record's text does not build the view model.
    let t = Instant::now();
    let (head, text) = server.get("/record/PRG-1");
    assert!(
        status_line(&head).contains("200") && text.contains("Alpha"),
        "{head}"
    );
    assert!(
        t.elapsed() < Duration::from_millis(1200),
        "{:?}",
        t.elapsed()
    );
    // The first page builds; the second, unchanged, comes from memory.
    let t = Instant::now();
    let (_, first) = server.get("/");
    assert!(
        t.elapsed() >= Duration::from_millis(1400),
        "{:?}",
        t.elapsed()
    );
    let t = Instant::now();
    let (_, second) = server.get("/");
    assert!(
        t.elapsed() < Duration::from_millis(1000),
        "{:?}",
        t.elapsed()
    );
    assert_eq!(
        first.split("data-d=\"").nth(1).unwrap()[..64].to_string(),
        second.split("data-d=\"").nth(1).unwrap()[..64].to_string()
    );
    // A change to a record invalidates it, and the new page shows the change.
    write(
        &dir,
        "docs/programmes/PRG-1-alpha.md",
        &record(
            "Status: Rolling",
            "Owner: ann\nLane: north\n",
            "# 1 — Alpha, cached no more",
        ),
    );
    let t = Instant::now();
    let (_, third) = server.get("/");
    assert!(
        t.elapsed() >= Duration::from_millis(1400),
        "{:?}",
        t.elapsed()
    );
    assert!(third.contains("Alpha, cached no more"));
    let (_, text) = server.get("/record/PRG-1");
    assert!(
        text.contains("Alpha, cached no more"),
        "the text read after a change is the new text"
    );
}

#[test]
fn a_request_for_another_host_is_refused_when_allowed_hosts_are_given() {
    let dir = chain("hosts");
    let open = Server::start(&dir);
    assert!(
        status_line(&open.get_with_host("/stamp", Some("anything.example"))).contains("200"),
        "no check by default"
    );
    assert!(status_line(&open.get_with_host("/stamp", None)).contains("200"));
    let (head, _) = open.get("/stamp");
    assert!(
        !head.to_lowercase().contains("access-control-allow-origin"),
        "{head}"
    );
    drop(open);

    let guarded = Server::start_with(
        &dir,
        &[
            "--allowed-host",
            "Board.Example",
            "--allowed-host",
            "other.test",
        ],
        &[],
    );
    for host in [
        "board.example",
        "board.example:8443",
        "BOARD.EXAMPLE",
        "other.test",
        "localhost",
        "localhost:3000",
        "127.0.0.1",
        "127.0.0.1:18000",
    ] {
        assert!(
            status_line(&guarded.get_with_host("/stamp", Some(host))).contains("200"),
            "{host}"
        );
    }
    for host in [
        "evil.example",
        "board.example.evil.test",
        "evil.example:80",
        "[::1]:80",
        "127.0.0.2",
    ] {
        assert!(
            status_line(&guarded.get_with_host("/stamp", Some(host))).contains("403"),
            "{host}"
        );
    }
    assert!(
        status_line(&guarded.get_with_host("/", None)).contains("403"),
        "no Host header at all"
    );
    assert!(
        status_line(&guarded.get_with_host("/record/PRG-1", Some("evil.example"))).contains("403")
    );
    assert!(status_line(&guarded.get_with_host("/check", Some("evil.example"))).contains("403"));
    let out = run(&dir, &["board", "--allowed-host", "x.test"]);
    assert_eq!(out.status.code(), Some(2), "--allowed-host needs --serve");
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
    assert_self_contained(&html, &links(&e));
    assert_eq!(count(&html, " data-record=\""), records.len());
    assert!(html.len() < 4 * 1024 * 1024, "{} bytes", html.len());
}

// ---------------------------------------------------------------- the additive guarantee

const NEW_RECORD_KEYS: [&str; 18] = [
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
    "parent_ambiguous",
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
    let out = common::command_with_stub_gh(&bin)
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

// ---------------------------------------------------------------- first-use fixes

fn narrow_rules(html: &str) -> &str {
    html.split("@media (max-width: 460px) {")
        .nth(1)
        .and_then(|rest| rest.split("\n}\n").next())
        .expect("a narrow-screen block")
}

const NARROW_NEEDS: [&str; 5] = [
    ".rs, .br { flex-wrap: wrap;",
    "-webkit-line-clamp: 3",
    "white-space: normal",
    ".s { flex: 1 0 100%;",
    ".s .x { max-width: none; overflow: visible;",
];

const NARROW_NEVER: [&str; 3] = [".s .x { display: none", "max-width: 12ch", "nowrap"];

const WIDE_TITLE_RULE: &str =
    ".t { flex: 1 1 0; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }";

#[test]
fn at_phone_width_a_title_wraps_and_the_detail_drops_to_a_line_under_it() {
    let dir = chain("narrow");
    let (html, _) = board_page(&dir);
    let narrow = narrow_rules(&html);
    for needed in NARROW_NEEDS {
        assert!(narrow.contains(needed), "{needed}: {narrow}");
    }
    for never in NARROW_NEVER {
        assert!(!narrow.contains(never), "{never}: {narrow}");
    }
    let wide = html.split("@media (max-width: 460px)").next().unwrap();
    assert!(
        wide.contains(WIDE_TITLE_RULE),
        "the wide layout keeps one line"
    );
    let waiting = entry(&html, "r-STP-10");
    let after_title = waiting.split("<span class=\"t\"").nth(1).unwrap();
    assert!(
        after_title.contains("</span><span class=\"s\"><span class=\"x w\">waits on"),
        "the detail is in the row's own right-hand group, which the narrow rules put under the title: {after_title}"
    );
}

#[test]
fn a_list_row_says_what_it_waits_on_and_what_it_is_part_of() {
    let dir = chain("list-context");
    let (html, _) = board_page(&dir);
    let under_way = section_of(&html, "under-way");
    let stage = under_way
        .split("<li class=\"br\">")
        .find(|r| r.contains(">STG-2</a>"))
        .unwrap();
    assert!(
        stage.contains("<span class=\"x\">part of <a href=\"#r-PRG-1\">PRG-1</a></span>"),
        "{stage}"
    );
    let e = export(&dir);
    let ready = section_of(&html, "ready");
    for id in ids(&by_id(&e, "STG-2")["children"])
        .iter()
        .filter(|c| by_id(&e, c)["ready"] == true)
    {
        let parent = by_id(&e, id)["parent"].as_str().unwrap().to_string();
        let row = ready
            .split("<li class=\"br\">")
            .find(|r| r.contains(&format!(">{id}</a>")))
            .unwrap_or_else(|| panic!("{id} in Ready: {ready}"));
        assert!(
            row.contains(&format!("part of <a href=\"#r-{parent}\">{parent}</a>")),
            "{row}"
        );
    }
    let flat = repo("list-waits", FLAT_CONFIG, |dir| {
        write(
            dir,
            "docs/tickets/TCK-1-a.md",
            &record("Status: Doing", "", "# 1 — A"),
        );
        write(
            dir,
            "docs/tickets/TCK-2-b.md",
            &record("Status: Doing", "Needs: TCK-1\n", "# 2 — B"),
        );
    });
    let (html, _) = board_page(&flat);
    let row = section_of(&html, "under-way")
        .split("<li class=\"br\">")
        .find(|r| r.contains(">TCK-2</a>"))
        .unwrap()
        .to_string();
    assert!(
        row.contains("<span class=\"x w\">waits on <a href=\"#r-TCK-1\">TCK-1</a></span>"),
        "{row}"
    );
    assert!(
        !row.contains("part of"),
        "a record with no parent claims none: {row}"
    );
}

#[test]
fn the_details_panel_links_to_the_records_file_once_and_only_where_the_export_has_a_link() {
    let dir = chain("file-link");
    let (html, _) = board_page(&dir);
    let e = export(&dir);
    let mut with_link = 0;
    for record in e["records"].as_array().unwrap() {
        let Some(link) = record["link"].as_str() else {
            continue;
        };
        with_link += 1;
        let anchor = format!("<a href=\"{link}\" target=\"_blank\" rel=\"noopener noreferrer\">");
        assert_eq!(count(&html, &anchor), 1, "{link}");
    }
    assert!(with_link > 0);
    assert_eq!(count(&html, "rel=\"noopener noreferrer\""), with_link);

    let flat = flat("file-link-none");
    let (html, _) = board_page(&flat);
    assert!(!html.contains("noopener") && !html.contains("Open the record's file"));
    assert!(!html.contains("http://") && !html.contains("https://"));

    let escaped = repo(
        "file-link-escaped",
        &chain_config().replace(
            "https://example.test/blob/main/",
            "https://example.test/view?a=1&b=2&path=",
        ),
        chain_records,
    );
    let (html, _) = board_page(&escaped);
    let link = export(&escaped)["records"][0]["link"]
        .as_str()
        .unwrap()
        .to_string();
    assert!(link.contains("&b=2"));
    assert_eq!(
        count(&html, &format!("href=\"{}\"", link.replace('&', "&amp;"))),
        1
    );
}

fn two_type_config(extra_type_statuses: &str) -> String {
    format!(
        "schema_version: 2
record_types:
  plain:
    dir: docs/plain
    prefix: PLN
    header_shape: yaml-frontmatter
    required_fields: [Status]
    narrative_fields: []
    pointer_fields: []
    statuses:
      values: [Idea, Shipped, Cancelled]
      success: [Shipped]
      failure: [Cancelled]
      initial: [Idea]
      transitions:
        Idea: [Shipped, Cancelled]
  staged:
    dir: docs/staged
    prefix: STD
    header_shape: yaml-frontmatter
    required_fields: [Status]
    narrative_fields: []
    pointer_fields: []
{extra_type_statuses}"
    )
}

#[test]
fn the_progress_note_names_only_a_type_with_statuses_in_between_and_the_json_answer_says_it_too() {
    let in_between = "    statuses:
      values: [Idea, Rolling, Reviewing, Shipped]
      success: [Shipped]
      initial: [Idea]
      transitions:
        Idea: [Rolling]
        Rolling: [Reviewing]
        Reviewing: [Shipped]
";
    let dir = repo("progress-gap", &two_type_config(in_between), |dir| {
        write(
            dir,
            "docs/plain/PLN-1-a.md",
            &record("Status: Idea", "", "# 1 — A"),
        );
        write(
            dir,
            "docs/staged/STD-1-a.md",
            &record("Status: Rolling", "", "# 1 — A"),
        );
    });
    let (html, answer) = board_page(&dir);
    let note = about_of(&html);
    assert!(
        note.contains("staged declares no <code>statuses.progress</code>: until it does, its records in Rolling and Reviewing show as open and a parent&#39;s state cannot disagree with its children."),
        "{note}"
    );
    assert!(
        !note.contains("plain declares no <code>statuses.progress</code>"),
        "a type with nothing in between is not named: {note}"
    );
    assert!(
        !section_of(&html, "under-way").contains("statuses.progress"),
        "the Under way list no longer carries the note"
    );
    let notices = answer["notices"].as_array().unwrap();
    assert_eq!(notices.len(), 1, "{notices:?}");
    assert_eq!(notices[0]["subject"], "board.progress");
    assert_eq!(notices[0]["severity"], "info");
    assert_eq!(notices[0]["detail"]["type"], "staged");
    assert_eq!(
        notices[0]["detail"]["statuses"],
        serde_json::json!(["Rolling", "Reviewing"])
    );
    assert!(notices[0]["message"]
        .as_str()
        .unwrap()
        .contains("until it does, its records in Rolling and Reviewing show as open"));

    let declared = in_between.replace(
        "      initial: [Idea]",
        "      progress: [Rolling, Reviewing]\n      initial: [Idea]",
    );
    let dir = repo("progress-declared", &two_type_config(&declared), |dir| {
        write(
            dir,
            "docs/staged/STD-1-a.md",
            &record("Status: Rolling", "", "# 1 — A"),
        );
    });
    let (html, answer) = board_page(&dir);
    assert!(!about_of(&html).contains("until it does"));
    assert!(answer.get("notices").is_none(), "{answer}");

    let nothing_to_declare = "    statuses:
      values: [Idea, Shipped]
      success: [Shipped]
      initial: [Idea]
      transitions:
        Idea: [Shipped]
";
    let dir = repo(
        "progress-none",
        &two_type_config(nothing_to_declare),
        |dir| {
            write(
                dir,
                "docs/staged/STD-1-a.md",
                &record("Status: Idea", "", "# 1 — A"),
            );
        },
    );
    let (html, answer) = board_page(&dir);
    assert!(!about_of(&html).contains("until it does"));
    assert!(answer.get("notices").is_none(), "{answer}");
    assert_eq!(
        export(&dir)["type_roles"]["staged"]["unplaced_statuses"],
        serde_json::json!([])
    );
}

#[test]
fn a_fold_names_the_records_it_hides_capped_and_counts_those_below_them() {
    let dir = repo("fold-names", &chain_config(), |dir| {
        let r = |path: &str, status: &str, extra: &str, heading: &str| {
            write(
                dir,
                path,
                &record(&format!("Status: {status}"), extra, heading),
            );
        };
        r("docs/programmes/PRG-1-a.md", "Rolling", "", "# 1 — A");
        r(
            "docs/stages/STG-1-a.md",
            "Shipped",
            "In: PRG-1\n",
            "# 1 — A",
        );
        r("docs/steps/STP-1-a.md", "Shipped", "In: STG-1\n", "# 1 — A");
        r(
            "docs/steps/STP-2-a.md",
            "Cancelled",
            "In: STG-1\n",
            "# 2 — B",
        );
        for n in 2..=8 {
            r(
                &format!("docs/stages/STG-{n}-a.md"),
                "Shipped",
                "In: PRG-1\n",
                &format!("# {n} — S"),
            );
        }
    });
    let (html, _) = board_page(&dir);
    assert!(
        html.contains("8 finished, collapsed: STG-1, STG-2, STG-3, STG-4, STG-5 +3 more; 2 records under them"),
        "{}",
        html.split("collapsed").next().unwrap_or_default().chars().rev().take(300).collect::<String>().chars().rev().collect::<String>()
    );
    assert!(html.contains("1 finished, 1 dropped, collapsed: STP-1, STP-2</span>"));
    let one = repo("fold-one", &chain_config(), |dir| {
        write(
            dir,
            "docs/programmes/PRG-1-a.md",
            &record("Status: Rolling", "", "# 1 — A"),
        );
        write(
            dir,
            "docs/stages/STG-1-a.md",
            &record("Status: Shipped", "In: PRG-1\n", "# 1 — A"),
        );
        write(
            dir,
            "docs/steps/STP-1-a.md",
            &record("Status: Shipped", "In: STG-1\n", "# 1 — A"),
        );
    });
    let (html, _) = board_page(&one);
    assert!(html.contains("1 finished, collapsed: STG-1; 1 record under them"));
}

#[test]
fn the_read_stamp_says_apart_when_the_configuration_has_uncommitted_changes() {
    let dir = chain("config-stamp");
    let first = export(&dir);
    assert_eq!(first["read"]["uncommitted_config_changes"], false);
    let (page, answer) = board_page(&dir);
    assert_eq!(answer["read"]["uncommitted_config_changes"], false);
    assert!(!page.contains("uncommitted configuration changes"));

    let config = std::fs::read_to_string(dir.join(".urzua/config.yaml")).unwrap();
    write(&dir, ".urzua/config.yaml", &format!("{config}\n# edited\n"));
    let changed = export(&dir);
    assert_eq!(changed["read"]["uncommitted_config_changes"], true);
    assert_eq!(
        changed["read"]["uncommitted_record_changes"], false,
        "the existing key keeps its meaning"
    );
    let (page, answer) = board_page(&dir);
    assert_eq!(answer["read"]["uncommitted_config_changes"], true);
    assert!(
        page.contains("working tree, uncommitted configuration changes</span>"),
        "{}",
        page.split("<header")
            .nth(1)
            .unwrap()
            .split("</header>")
            .next()
            .unwrap()
    );

    common::commit_all(&dir, 1);
    assert_eq!(export(&dir)["read"]["uncommitted_config_changes"], false);

    let time = |value: &Value| value["read"]["time"].as_str().unwrap().to_string();
    assert_eq!(
        time(&first).len(),
        20,
        "the stamp is to the second, so two runs within a second print one time"
    );
}

#[test]
fn a_record_link_may_be_any_prefix_the_configuration_allows_and_nothing_else_is() {
    for (prefix, starts) in [
        ("http://example.test/raw/", "http://example.test/raw/docs/"),
        ("/files/", "/files/docs/"),
    ] {
        let dir = repo(
            "link-shapes",
            &chain_config().replace("https://example.test/blob/main/", prefix),
            chain_records,
        );
        let e = export(&dir);
        let linked = links(&e);
        assert!(!linked.is_empty() && linked.iter().all(|l| l.starts_with(starts)));
        let (html, _) = board_page(&dir);
        assert_self_contained(&html, &linked);
        assert_eq!(
            count(&html, "rel=\"noopener noreferrer\""),
            linked.len(),
            "{prefix}"
        );
    }
}

#[test]
#[should_panic(expected = "address https://evil.test/x")]
fn an_address_the_export_did_not_give_is_refused_by_the_page_check() {
    let dir = chain("foreign-link");
    let (html, _) = board_page(&dir);
    let foreign = html.replace(
        "</main>",
        "<a href=\"https://evil.test/x\" target=\"_blank\" rel=\"noopener noreferrer\">x</a></main>",
    );
    assert_self_contained(&foreign, &links(&export(&dir)));
}

fn export_from(cwd: &Path, args: &[&str]) -> Value {
    let out = run(cwd, args);
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    json(&out)
}

#[test]
fn the_configuration_stamp_survives_a_relative_or_linked_config_and_is_null_when_it_cannot_be_placed(
) {
    let dir = chain("config-path");
    let config = std::fs::read_to_string(dir.join(".urzua/config.yaml")).unwrap();
    write(&dir, ".urzua/config.yaml", &format!("{config}\n# edited\n"));
    let sub = dir.join("docs");
    let relative = export_from(
        &sub,
        &[
            "export",
            "--format",
            "json",
            "--config",
            "../.urzua/config.yaml",
        ],
    );
    assert_eq!(relative["read"]["uncommitted_config_changes"], true);

    let dotted = export_from(
        &sub,
        &[
            "export",
            "--format",
            "json",
            "--config",
            "./programmes/../../.urzua/config.yaml",
        ],
    );
    assert_eq!(dotted["read"]["uncommitted_config_changes"], true);

    std::os::unix::fs::symlink(dir.join(".urzua/config.yaml"), dir.join("alias.yaml")).unwrap();
    let linked = export_from(
        &sub,
        &["export", "--format", "json", "--config", "../alias.yaml"],
    );
    assert_eq!(linked["read"]["uncommitted_config_changes"], true);

    let outside = common::TempDir::new("urzua-board-config-outside");
    write(
        &outside,
        "config.yaml",
        &std::fs::read_to_string(dir.join(".urzua/config.yaml")).unwrap(),
    );
    let elsewhere = export_from(
        &dir,
        &[
            "export",
            "--format",
            "json",
            "--config",
            outside.join("config.yaml").to_str().unwrap(),
        ],
    );
    assert!(
        elsewhere["read"]["uncommitted_config_changes"].is_null(),
        "a configuration outside the repository is not a guess of false: {}",
        elsewhere["read"]
    );
    let (page, _) = {
        let target = dir.join("out/outside.html");
        let out = run(
            &dir,
            &[
                "board",
                "--out",
                target.to_str().unwrap(),
                "--config",
                outside.join("config.yaml").to_str().unwrap(),
            ],
        );
        assert!(out.status.success());
        (std::fs::read_to_string(&target).unwrap(), json(&out))
    };
    assert!(!page.contains("uncommitted configuration changes"));
}

#[test]
fn a_type_that_declares_no_initial_status_is_never_named_by_the_progress_note() {
    let statuses = "    statuses:
      values: [Open, Shipped]
      success: [Shipped]
";
    let dir = repo("progress-no-initial", &two_type_config(statuses), |dir| {
        write(
            dir,
            "docs/staged/STD-1-a.md",
            &record("Status: Open", "", "# 1 — A"),
        );
    });
    let (html, answer) = board_page(&dir);
    assert!(
        !about_of(&html).contains("until it does"),
        "{}",
        about_of(&html)
    );
    assert!(answer.get("notices").is_none(), "{answer}");
    assert_eq!(
        export(&dir)["type_roles"]["staged"]["unplaced_statuses"],
        serde_json::json!([])
    );
}

#[test]
fn a_backtick_in_a_type_or_status_name_does_not_turn_the_note_into_code() {
    let statuses = "    statuses:
      values: [Idea, \"Ro`lling\", Shipped]
      success: [Shipped]
      initial: [Idea]
      transitions:
        Idea: [\"Ro`lling\"]
        \"Ro`lling\": [Shipped]
";
    let dir = repo("progress-tick", &two_type_config(statuses), |dir| {
        write(
            dir,
            "docs/staged/STD-1-a.md",
            &record("Status: Idea", "", "# 1 — A"),
        );
    });
    let (html, answer) = board_page(&dir);
    let note = about_of(&html)
        .split("staged declares")
        .nth(1)
        .and_then(|rest| rest.split("children.").next())
        .expect("the staged sentence");
    assert!(
        note.contains("its records in Ro&#39;lling show as open")
            && note.matches("<code>").count() == 1,
        "{note}"
    );
    assert_eq!(
        answer["notices"][0]["detail"]["statuses"],
        serde_json::json!(["Ro`lling"]),
        "the notice keeps the name as declared"
    );
}

// ---------------------------------------------------------------- the application shell and the prompt

fn prompt_of(dir: &Path) -> String {
    let out = run(dir, &["board", "--prompt"]);
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let answer = json(&out);
    assert_eq!(answer["status"], "ok");
    answer["prompt"].as_str().expect("a prompt").to_string()
}

/// The words in backticks on the first line of `prompt` that starts with `label`.
fn keys_named(prompt: &str, label: &str) -> Vec<String> {
    let line = prompt
        .lines()
        .find(|l| l.starts_with(label))
        .unwrap_or_else(|| panic!("no line {label}"));
    line[label.len()..]
        .split('`')
        .skip(1)
        .step_by(2)
        .map(str::to_string)
        .collect()
}

fn object_keys(value: &Value) -> std::collections::BTreeSet<String> {
    value.as_object().unwrap().keys().cloned().collect()
}

#[test]
fn the_prompt_is_written_from_the_configuration_and_names_exactly_the_keys_an_export_has() {
    let dir = chain("prompt");
    let prompt = prompt_of(&dir);
    assert!(
        !dir.join(".urzua/cache/board.html").exists(),
        "the prompt writes no file"
    );
    assert!(prompt.lines().count() <= 60, "{prompt}");
    assert!(prompt.trim_end().ends_with("\nBuild me a view that ..."));
    for needed in [
        "urzua export --format json",
        "--no-text",
        "--no-raw-text",
        "--no-checklist-items",
        "`urzua board` is the plain default",
        "stable input",
    ] {
        assert!(prompt.contains(needed), "{needed}: {prompt}");
    }
    let e = export(&dir);
    for t in e["types"].as_array().unwrap() {
        let name = t["type"].as_str().unwrap();
        assert!(prompt.contains(&format!("- {name} (")), "type {name}");
    }
    assert!(prompt.contains("- programme (docs/programmes)"));
    assert!(prompt.contains("Lane takes north, south"), "{prompt}");
    assert!(
        prompt.contains("statuses Idea, Rolling, Shipped, Cancelled; initial: Idea; success: Shipped; failure: Cancelled; progress: Rolling, Shipped"),
        "{prompt}"
    );
    assert!(
        prompt.contains(
            "- charter (docs/charters): fields Status; statuses Draft, Signed; success: Signed"
        ),
        "{prompt}"
    );

    let top = keys_named(&prompt, "Top-level keys:");
    assert_eq!(object_keys(&e), top.iter().cloned().collect(), "top level");
    let mut seen = std::collections::BTreeSet::new();
    for record in e["records"].as_array().unwrap() {
        seen.extend(object_keys(record));
    }
    let per_record = keys_named(&prompt, "Keys of each entry in `records`:");
    assert_eq!(
        seen,
        per_record.iter().cloned().collect(),
        "a record's keys: a key the prompt names that no record has, or one it leaves out"
    );
}

#[test]
fn the_prompt_is_the_same_in_the_page_and_a_flag_that_takes_no_other() {
    let dir = flat("prompt-page");
    let prompt = prompt_of(&dir);
    assert!(
        prompt.contains("- ticket (docs/tickets): fields Area, Gates, Needs, Status;"),
        "{prompt}"
    );
    assert!(prompt.contains("- memo (docs/memos): fields Related, Topic; declares no statuses"));
    let (html, _) = board_page(&dir);
    let textarea = html
        .split("<textarea id=\"prompt\"")
        .nth(1)
        .unwrap()
        .split_once('>')
        .unwrap()
        .1
        .split("</textarea>")
        .next()
        .unwrap();
    assert_eq!(textarea, html_escape(&prompt));
    assert!(html.contains("id=\"copy\"") && html.contains("urzua board --prompt"));
    for args in [
        &["board", "--prompt", "--serve"][..],
        &["board", "--prompt", "--out", "x.html"][..],
        &["board", "--prompt", "--port", "8080"][..],
        &["board", "--prompt", "--allowed-host", "box.example"][..],
    ] {
        let out = run(&dir, args);
        assert_eq!(out.status.code(), Some(2), "{args:?}");
    }
}

#[test]
fn a_page_is_drawn_from_the_one_configuration_its_records_were_read_with() {
    let dir = flat("one-config");
    let target = dir.join("out/board.html");
    let mut child = common::urzua()
        .args(["board", "--out", target.to_str().unwrap()])
        .env("URZUA_BOARD_TEST_BUILD_DELAY_MS", "1500")
        .current_dir(&*dir)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();
    std::thread::sleep(Duration::from_millis(600));
    let config = dir.join(".urzua/config.yaml");
    let edited = std::fs::read_to_string(&config).unwrap().replace(
        "values: [Todo, Doing, Done, Dropped]",
        "values: [Todo, Doing, Done, Dropped, Parked]",
    );
    std::fs::write(&config, edited).unwrap();
    assert!(child.wait().unwrap().success());
    let html = std::fs::read_to_string(&target).unwrap();
    assert!(
        !html.contains("Parked"),
        "the page names a status the records were not read with"
    );
}

fn rendered_shell(dir: &Path) -> String {
    let (html, _) = board_page(dir);
    assert_self_contained(&html, &links(&export(dir)));
    html
}

#[test]
fn the_page_is_a_shell_with_a_top_bar_tabs_a_sidebar_and_a_footer() {
    for (name, dir) in [
        ("chain", chain("shell-chain")),
        ("flat", flat("shell-flat")),
    ] {
        let html = rendered_shell(&dir);
        let e = export(&dir);
        for needed in [
            "<header class=\"top\" id=\"top\">",
            "<nav id=\"tabs\" class=\"tabs\"",
            "<nav id=\"side\" class=\"side\"",
            "<main id=\"board\">",
            "<aside id=\"panel\"",
            "<footer class=\"foot\">",
            "id=\"q\"",
            "id=\"theme\"",
            "data-open=\"build-view\"",
            "data-open=\"about\"",
            "id=\"empty\"",
        ] {
            assert!(html.contains(needed), "{name}: {needed}");
        }
        assert!(
            html.contains(&format!(
                "urzua {} · generated from this repository's configuration",
                env!("CARGO_PKG_VERSION")
            )),
            "{name}: the footer"
        );
        let side = html
            .split("<nav id=\"side\"")
            .nth(1)
            .unwrap()
            .split("</nav>")
            .next()
            .unwrap();
        for t in e["types"].as_array().unwrap() {
            let ty = t["type"].as_str().unwrap();
            assert!(
                side.contains(&format!(
                    "data-t=\"{ty}\"><span>{ty}</span><i>{}</i>",
                    t["records"]
                )),
                "{name}: the sidebar lists {ty} with its count: {side}"
            );
        }
        let tabs = html
            .split("<nav id=\"tabs\"")
            .nth(1)
            .unwrap()
            .split("</nav>")
            .next()
            .unwrap();
        for key in ["", "needs", "under-way", "ready"] {
            assert!(tabs.contains(&format!("data-f=\"{key}\"")), "{name}: {key}");
        }
        assert!(tabs.contains(&format!("All<i>{}</i>", e["record_count"])));
        assert!(!html.contains("class=\"bar\"") && !html.contains("class=\"seg\""));
    }
}

#[test]
fn the_mark_is_the_five_squares_inline_and_the_favicon_and_blue_is_the_accent() {
    let html = rendered_shell(&chain("brand"));
    let svg = "<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 30 20\" role=\"img\" aria-label=\"urzua\"><rect x=\"0\" y=\"0\" width=\"10\" height=\"10\" fill=\"#0F5BF0\"/><rect x=\"0\" y=\"10\" width=\"10\" height=\"10\" fill=\"#12B550\"/><rect x=\"10\" y=\"10\" width=\"10\" height=\"10\" fill=\"#FFD21F\"/><rect x=\"20\" y=\"10\" width=\"10\" height=\"10\" fill=\"#FF7A1A\"/><rect x=\"20\" y=\"0\" width=\"10\" height=\"10\" fill=\"#E5233D\"/></svg>";
    assert!(
        html.contains(&svg.replace("http://", "http&#58;//")),
        "the mark, with its namespace as a character reference"
    );
    assert_eq!(count(&html, "<svg"), 1);
    assert!(html.contains("<link rel=\"icon\" href=\"data:image/svg+xml,"));
    let style = html
        .split("<style>")
        .nth(1)
        .unwrap()
        .split("</style>")
        .next()
        .unwrap();
    assert!(!style.contains("gradient"), "no gradients");
    assert!(style.contains("--blue: #0F5BF0") && style.contains("--link: #0F5BF0"));
    assert!(
        !style.contains("color: #FFD21F") && !style.contains("--link: #FFD21F"),
        "yellow is never text"
    );
    assert!(style.contains(":focus-visible") && style.contains("prefers-color-scheme: dark"));
    assert!(style.contains(":root[data-theme=\"dark\"]"));
    assert!(style.contains("@media (min-width: 1000px)"));
}

#[test]
fn the_script_has_the_keys_the_theme_choice_and_the_panel_and_stays_without_markup() {
    let html = rendered_shell(&flat("shell-script"));
    let script = html
        .split("<script>")
        .nth(1)
        .unwrap()
        .split("</script>")
        .next()
        .unwrap();
    for needed in [
        "e.key === '/'",
        "e.key === 'Escape'",
        "localStorage.setItem('urzua-board-theme'",
        "localStorage.getItem('urzua-board-theme'",
        "(min-width: 1000px)",
        "navigator.clipboard.writeText",
        "document.execCommand('copy')",
        "cloneNode(true)",
    ] {
        assert!(script.contains(needed), "{needed}");
    }
    for call in ["localStorage.setItem", "localStorage.getItem"] {
        let at = script.find(call).unwrap();
        assert!(
            script[..at].rfind("try").is_some_and(|t| at - t < 120),
            "{call} sits inside a try"
        );
    }
}

#[test]
fn a_view_with_nothing_in_it_says_so_in_one_sentence_instead_of_an_empty_card() {
    let dir = repo("nothing", FLAT_CONFIG, |dir| {
        write(
            dir,
            "docs/tickets/TCK-1-a.md",
            &record("Status: Todo", "", "# 1 — A"),
        );
    });
    let html = rendered_shell(&dir);
    let under_way = section_of(&html, "under-way");
    assert!(
        under_way.contains("<p class=\"empty\">No record is under way.</p>")
            && !under_way.contains("<ul"),
        "{under_way}"
    );
    assert!(!under_way.contains("class=\"card\""));
    assert!(html.contains("<p id=\"empty\" class=\"empty\" hidden>No record matches.</p>"));
}

#[test]
fn about_this_board_says_what_each_part_follows_from_and_what_each_type_declares() {
    let dir = chain("about");
    let html = rendered_shell(&dir);
    let about = about_of(&html);
    for needed in [
        "<code>part_of</code>",
        "<code>statuses.progress</code>",
        "<code>authority.gates</code>",
        "<code>statuses.transitions</code>",
        "<code>roadmap.pairs</code>",
        "Declared by: stage (In), step (In)",
        "<dt>programme</dt><dd>A work type. A record is under way in Rolling and Shipped.",
        "A gate reserves the move to Shipped for <code>leads</code>.",
        "<dt>charter</dt><dd>A work type. It declares no <code>initial</code> status",
        "It declares no <code>statuses.transitions</code>, so its next move cannot be read.",
    ] {
        assert!(about.contains(needed), "{needed}: {about}");
    }
    for ty in export(&dir)["types"].as_array().unwrap() {
        let name = ty["type"].as_str().unwrap();
        assert!(about.contains(&format!("<dt>{name}</dt>")), "{name}");
    }
    assert!(html.contains("<a href=\"#about\" data-open=\"about\">About this board</a>"));
    let flat_html = rendered_shell(&flat("about-flat"));
    assert!(
        about_of(&flat_html).contains("No type declares a <code>part_of</code> field"),
        "no containment, no tree"
    );
}

#[test]
fn deep_links_still_name_a_row_and_the_page_is_no_larger_than_its_frame_needs() {
    let dir = chain("deep");
    let html = rendered_shell(&dir);
    assert!(html.contains("<li id=\"r-STG-1\""));
    assert!(html.contains("href=\"#r-STG-1\""));
    let empty = repo(
        "empty",
        "schema_version: 2
record_types:
  ticket:
    dir: docs/tickets
    prefix: TCK
    header_shape: yaml-frontmatter
    required_fields: [Status]
    narrative_fields: []
    pointer_fields: []
    statuses:
      values: [Todo, Doing, Done, Dropped]
      success: [Done]
      failure: [Dropped]
      progress: [Doing]
",
        |_| {},
    );
    let bare = rendered_shell(&empty);
    const BEFORE_THE_SHELL: usize = 10_633;
    assert!(
        bare.len() <= BEFORE_THE_SHELL + 40 * 1024,
        "the frame of a page with no record is {} bytes, {} more than it was",
        bare.len(),
        bare.len() - BEFORE_THE_SHELL
    );
}

/// Writes the pages the phone-width check measures into `URZUA_BOARD_REVIEW_DIR`:
/// the three-level fixture as it is, both fixtures, the chain one with a record that waits on many and one with
/// long ids. Run with `--ignored`; it asserts nothing.
#[test]
#[ignore]
fn write_pages_for_the_phone_width_check() {
    let Some(out) = std::env::var_os("URZUA_BOARD_REVIEW_DIR") else {
        return;
    };
    let out = PathBuf::from(out);
    std::fs::create_dir_all(&out).unwrap();
    std::fs::write(
        out.join("fixture.html"),
        board_page(&chain("review-plain")).0,
    )
    .unwrap();
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
