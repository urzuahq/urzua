//! `Finding.suggested_action` (`ADR-73`, `MILE-44`) against the real binary:
//! the shape on the wire, the exemption reasons `urzua rules` shows, and the
//! promise a `run-command` action makes -- that running it would produce the
//! repair the finding stands for.

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

fn urzua(dir: &Path, args: &[&str]) -> std::process::Output {
    common::urzua()
        .args(args)
        .current_dir(dir)
        .output()
        .expect("urzua binary should run")
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
        .unwrap()
        .iter()
        .filter(|f| f["rule"] == rule)
        .collect()
}

fn on<'a>(found: &[&'a serde_json::Value], file: &str) -> &'a serde_json::Value {
    found
        .iter()
        .copied()
        .find(|f| f["file"].as_str().unwrap().ends_with(file))
        .unwrap_or_else(|| panic!("no finding on {file}: {found:?}"))
}

// ---------------------------------------------------------------- the wire shape

const SHAPE_CONFIG: &str = "schema_version: 2
rules:
  field.quality: error
  status.forbidden-field-present: error
  relation.pair-reciprocity: error
  identity.collision: error
  config.pointer-declaration-missing: error
relation_pairs:
  - name: blocks
    forward: Blocks
    inverse: Blocked-by
    reciprocity: required
record_types:
  note:
    dir: docs/notes
    prefix: NOTE
    header_shape: yaml-frontmatter
    required_fields: [Status, Summary]
    known_fields: [Blocks, Blocked-by, Closed-on]
    pointer_fields: [Blocks]
    statuses:
      values: [Open, Closed]
      success: [Closed]
      failure: []
      forbid:
        - {when: [Open], fields: [Closed-on]}
";

fn shape_repo(name: &str) -> common::TempDir {
    let dir = common::TempDir::new(&format!("urzua-action-shape-{name}"));
    for d in ["docs/notes", ".urzua"] {
        std::fs::create_dir_all(dir.join(d)).unwrap();
    }
    git(&dir, &["init", "-q"]);
    git(&dir, &["config", "user.email", "test@example.com"]);
    git(&dir, &["config", "user.name", "test"]);
    std::fs::write(dir.join(".urzua/config.yaml"), SHAPE_CONFIG).unwrap();
    // NOTE-1: a blank required field, a forbidden field, and a claim NOTE-2 does not echo.
    std::fs::write(
        dir.join("docs/notes/NOTE-1-a.md"),
        "---\nStatus: Open\nSummary: \nBlocks: NOTE-2\nClosed-on: 2026-01-01\n---\n# 1 - A\n",
    )
    .unwrap();
    std::fs::write(
        dir.join("docs/notes/NOTE-2-b.md"),
        "---\nStatus: Open\nSummary: b\nBlocks: \u{2014}\n---\n# 2 - B\n",
    )
    .unwrap();
    // Two records claiming one identifier: collision has no action by design.
    std::fs::write(
        dir.join("docs/notes/NOTE-3-c.md"),
        "---\nStatus: Open\nSummary: c\nBlocks: \u{2014}\n---\n# 3 - C\n",
    )
    .unwrap();
    std::fs::write(
        dir.join("docs/notes/NOTE-3-d.md"),
        "---\nStatus: Open\nSummary: d\nBlocks: \u{2014}\n---\n# 3 - D\n",
    )
    .unwrap();
    git(&dir, &["add", "-A"]);
    git(&dir, &["commit", "-q", "-m", "fixture"]);
    dir
}

#[test]
fn a_finding_carries_the_action_its_variant_declares() {
    let dir = shape_repo("a_finding_ca");
    let report = parsed(&urzua(&dir, &["check"]));

    let quality = findings(&report, "field.quality");
    assert_eq!(
        on(&quality, "NOTE-1-a.md")["suggested_action"],
        serde_json::json!({
            "kind": "edit-field",
            "op": "set",
            "target": { "file": "docs/notes/NOTE-1-a.md", "field": "Summary" }
        }),
        "{report}"
    );

    let forbidden = findings(&report, "status.forbidden-field-present");
    assert_eq!(
        on(&forbidden, "NOTE-1-a.md")["suggested_action"],
        serde_json::json!({
            "kind": "edit-field",
            "op": "remove",
            "target": { "file": "docs/notes/NOTE-1-a.md", "field": "Closed-on" }
        }),
        "{report}"
    );

    // The finding sits on the claimant; the edit belongs on the counterpart,
    // and carries the id to append.
    let pair = findings(&report, "relation.pair-reciprocity");
    assert_eq!(
        on(&pair, "NOTE-1-a.md")["suggested_action"],
        serde_json::json!({
            "kind": "edit-field",
            "op": "append",
            "target": { "file": "docs/notes/NOTE-2-b.md", "field": "Blocked-by" },
            "value": "NOTE-1"
        }),
        "{report}"
    );
}

#[test]
fn a_configuration_edit_names_the_repository_relative_config_and_a_key_path() {
    let dir = shape_repo("a_configurat");
    let report = parsed(&urzua(&dir, &["check"]));
    let missing = findings(&report, "config.pointer-declaration-missing");
    assert_eq!(missing.len(), 1, "{report}");
    assert_eq!(missing[0]["file"], ".urzua/config.yaml", "{report}");
    // `pointer_fields` is declared and `narrative_fields` is not: the missing
    // half is set to the empty list.
    assert_eq!(
        missing[0]["suggested_action"],
        serde_json::json!({
            "kind": "edit-config",
            "op": "set",
            "target": {
                "file": ".urzua/config.yaml",
                "key_path": ["record_types", "note", "narrative_fields"]
            },
            "value": []
        }),
        "{report}"
    );
}

/// Applies one `edit-config` suggested action to the configuration file, the
/// way an agent following it would: `set` replaces the value at the key path,
/// `append` pushes onto the list there (creating it when absent).
fn apply_config_action(dir: &Path, action: &serde_json::Value) {
    let path = dir.join(".urzua/config.yaml");
    let mut doc: yaml_serde::Value =
        yaml_serde::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
    let key_path: Vec<&str> = action["target"]["key_path"]
        .as_array()
        .unwrap()
        .iter()
        .map(|k| k.as_str().unwrap())
        .collect();
    let (last, parents) = key_path.split_last().unwrap();
    let mut node = &mut doc;
    for key in parents {
        node = node.get_mut(*key).expect("the key path resolves");
    }
    let map = node.as_mapping_mut().unwrap();
    let value: yaml_serde::Value = yaml_serde::to_value(&action["value"]).unwrap();
    match action["op"].as_str().unwrap() {
        "set" => {
            map.insert(yaml_serde::Value::from(*last), value);
        }
        "append" => {
            let entry = map
                .entry(yaml_serde::Value::from(*last))
                .or_insert_with(|| yaml_serde::Value::Sequence(Vec::new()));
            entry.as_sequence_mut().unwrap().push(value);
        }
        other => panic!("unexpected op {other}"),
    }
    std::fs::write(&path, yaml_serde::to_string(&doc).unwrap()).unwrap();
}

#[test]
fn a_known_fields_suggestion_for_a_type_declaring_none_leaves_the_config_clean() {
    let dir = common::TempDir::new("urzua-action-known-fields");
    for d in ["docs/notes", ".urzua"] {
        std::fs::create_dir_all(dir.join(d)).unwrap();
    }
    git(&dir, &["init", "-q"]);
    git(&dir, &["config", "user.email", "test@example.com"]);
    git(&dir, &["config", "user.name", "test"]);
    // `note` declares no `known_fields`; its records use `Owner` and `Tags`
    // beyond the required `Status`, and the pointer field `Feeds-into` is
    // declared nowhere.
    std::fs::write(
        dir.join(".urzua/config.yaml"),
        "schema_version: 2
rules:
  config.pointer-field-not-known: error
  header.field-set-consistency: error
record_types:
  note:
    dir: docs/notes
    prefix: NOTE
    header_shape: yaml-frontmatter
    required_fields: [Status]
    pointer_fields: [Feeds-into]
    narrative_fields: []
",
    )
    .unwrap();
    std::fs::write(
        dir.join("docs/notes/NOTE-1-a.md"),
        "---\nStatus: Open\nOwner: a\nFeeds-into: NOTE-2\n---\n# 1 - A\n",
    )
    .unwrap();
    std::fs::write(
        dir.join("docs/notes/NOTE-2-b.md"),
        "---\nStatus: Open\nTags: x\n---\n# 2 - B\n",
    )
    .unwrap();
    git(&dir, &["add", "-A"]);
    git(&dir, &["commit", "-q", "-m", "init"]);

    let report = parsed(&urzua(&dir, &["check"]));
    let missing = findings(&report, "config.pointer-field-not-known");
    assert_eq!(missing.len(), 1, "{report}");
    let action = &missing[0]["suggested_action"];
    assert_eq!(
        *action,
        serde_json::json!({
            "kind": "edit-config",
            "op": "set",
            "target": {
                "file": ".urzua/config.yaml",
                "key_path": ["record_types", "note", "known_fields"]
            },
            "value": ["Feeds-into", "Owner", "Tags"]
        }),
        "{report}"
    );

    apply_config_action(&dir, action);
    let after = parsed(&urzua(&dir, &["check"]));
    assert_eq!(after["findings"], serde_json::json!([]), "{after}");
}

fn notes_repo(prefix: &str, config: &str, records: &[(&str, &str)]) -> common::TempDir {
    let dir = common::TempDir::new(prefix);
    for d in ["docs/notes", "docs/refs", ".urzua"] {
        std::fs::create_dir_all(dir.join(d)).unwrap();
    }
    git(&dir, &["init", "-q"]);
    git(&dir, &["config", "user.email", "test@example.com"]);
    git(&dir, &["config", "user.name", "test"]);
    std::fs::write(dir.join(".urzua/config.yaml"), config).unwrap();
    for (path, body) in records {
        std::fs::write(dir.join(path), body).unwrap();
    }
    git(&dir, &["add", "-A"]);
    git(&dir, &["commit", "-q", "-m", "init"]);
    dir
}

#[test]
fn a_target_status_suggestion_for_a_type_declaring_no_known_fields_leaves_the_config_clean() {
    let dir = notes_repo(
        "urzua-action-target-status",
        "schema_version: 2
rules:
  relation.target-status-undeclared: error
  header.field-set-consistency: error
record_types:
  note:
    dir: docs/notes
    prefix: NOTE
    header_shape: yaml-frontmatter
    required_fields: [Date]
    pointer_fields: [Feeds-into]
    narrative_fields: []
    known_fields: [Feeds-into]
  ref:
    dir: docs/refs
    prefix: REF
    header_shape: yaml-frontmatter
    required_fields: [Date]
    pointer_fields: []
    narrative_fields: []
",
        &[
            (
                "docs/notes/NOTE-1-a.md",
                "---\nDate: 2026-01-01\nFeeds-into: REF-1\n---\n# 1 - A\n",
            ),
            (
                "docs/refs/REF-1-b.md",
                "---\nDate: 2026-01-01\nOwner: a\n---\n# 1 - B\n",
            ),
            (
                "docs/refs/REF-2-c.md",
                "---\nDate: 2026-01-01\nTags: x\n---\n# 2 - C\n",
            ),
        ],
    );
    let report = parsed(&urzua(&dir, &["check"]));
    let missing = findings(&report, "relation.target-status-undeclared");
    assert_eq!(missing.len(), 1, "{report}");
    let action = &missing[0]["suggested_action"];
    assert_eq!(
        *action,
        serde_json::json!({
            "kind": "edit-config",
            "op": "set",
            "target": {
                "file": ".urzua/config.yaml",
                "key_path": ["record_types", "ref", "known_fields"]
            },
            "value": ["Owner", "Status", "Tags"]
        }),
        "{report}"
    );
    apply_config_action(&dir, action);
    let after = parsed(&urzua(&dir, &["check"]));
    assert!(
        findings(&after, "header.field-set-consistency").is_empty()
            && findings(&after, "relation.target-status-undeclared").is_empty(),
        "{after}"
    );
}

#[test]
fn a_companion_suggestion_for_a_type_declaring_no_known_fields_leaves_the_config_clean() {
    let dir = notes_repo(
        "urzua-action-companion",
        "schema_version: 2
rules:
  config.companion-declaration-missing: error
  header.field-set-consistency: error
record_types:
  note:
    dir: docs/notes
    prefix: NOTE
    header_shape: yaml-frontmatter
    required_fields: [Status, Embodiment]
    pointer_fields: []
    narrative_fields: []
    companion_fields:
      - [embodiment_state, embodiment_locator]
",
        &[
            (
                "docs/notes/NOTE-1-a.md",
                "---\nStatus: Open\nEmbodiment: Not started\nOwner: a\n---\n# 1 - A\n",
            ),
            (
                "docs/notes/NOTE-2-b.md",
                "---\nStatus: Open\nEmbodiment: Not started\nTags: x\n---\n# 2 - B\n",
            ),
        ],
    );
    let report = parsed(&urzua(&dir, &["check"]));
    let missing = findings(&report, "config.companion-declaration-missing");
    assert_eq!(missing.len(), 1, "{report}");
    let action = &missing[0]["suggested_action"];
    assert_eq!(
        *action,
        serde_json::json!({
            "kind": "edit-config",
            "op": "set",
            "target": {
                "file": ".urzua/config.yaml",
                "key_path": ["record_types", "note", "known_fields"]
            },
            "value": ["Owner", "Realized-by", "Tags"]
        }),
        "{report}"
    );
    apply_config_action(&dir, action);
    let after = parsed(&urzua(&dir, &["check"]));
    assert!(
        findings(&after, "header.field-set-consistency").is_empty()
            && findings(&after, "config.companion-declaration-missing").is_empty(),
        "{after}"
    );
}

#[test]
fn an_exempt_variant_omits_the_key_and_no_action_holds_a_null() {
    let dir = shape_repo("an_exempt_va");
    let report = parsed(&urzua(&dir, &["check"]));
    let collisions = findings(&report, "identity.collision");
    assert!(!collisions.is_empty(), "{report}");
    for f in collisions {
        assert!(
            f.as_object().unwrap().get("suggested_action").is_none(),
            "an exempt variant has no suggested_action key: {f}"
        );
    }
    fn no_null(v: &serde_json::Value) {
        match v {
            serde_json::Value::Null => panic!("a null inside a suggested_action"),
            serde_json::Value::Array(a) => a.iter().for_each(no_null),
            serde_json::Value::Object(o) => o.values().for_each(no_null),
            _ => {}
        }
    }
    for f in report["findings"].as_array().unwrap() {
        if let Some(action) = f.get("suggested_action") {
            no_null(action);
            let kind = action["kind"].as_str().unwrap();
            assert!(
                [
                    "edit-field",
                    "edit-section",
                    "edit-config",
                    "add-record",
                    "run-command"
                ]
                .contains(&kind),
                "{f}"
            );
        }
    }
}

#[test]
fn a_waived_finding_carries_its_action_unchanged() {
    let dir = shape_repo("a_waived_fin");
    std::fs::create_dir_all(dir.join("docs/waivers")).unwrap();
    let cfg = std::fs::read_to_string(dir.join(".urzua/config.yaml")).unwrap();
    std::fs::write(
        dir.join(".urzua/config.yaml"),
        format!(
            "{cfg}  waiver:\n    dir: docs/waivers\n    prefix: WAIVER\n    header_shape: yaml-frontmatter\n    required_fields: [Rule, Scope, Reason]\n"
        ),
    )
    .unwrap();
    std::fs::write(
        dir.join("docs/waivers/WAIVER-1-q.md"),
        "---\nRule: field.quality\nScope: docs/notes/NOTE-1-a.md\nReason: tracked\n---\n# 1 - W\n",
    )
    .unwrap();
    git(&dir, &["add", "-A"]);
    git(&dir, &["commit", "-q", "-m", "waive"]);
    let report = parsed(&urzua(&dir, &["check"]));
    let quality = findings(&report, "field.quality");
    let waived = on(&quality, "NOTE-1-a.md");
    assert!(waived.get("waived").is_some(), "{report}");
    assert_eq!(waived["suggested_action"]["kind"], "edit-field", "{report}");
}

// ------------------------------------------------------- `urzua rules` exemptions

#[test]
fn rules_lists_each_variant_with_its_action_or_its_exemption_reason() {
    let dir = std::env::temp_dir();
    let rules = parsed(&urzua(&dir, &["rules"]));
    let all = rules["rules"].as_array().unwrap();
    assert_eq!(all.len(), 73, "every shipped rule is listed");
    let find = |id: &str| {
        all.iter()
            .find(|r| r["id"] == id)
            .unwrap_or_else(|| panic!("{id}"))
    };

    let required = find("header.required-fields")["actions"]
        .as_array()
        .unwrap();
    let variant = |name: &str| {
        required
            .iter()
            .find(|a| a["variant"] == name)
            .unwrap_or_else(|| panic!("{name}"))
    };
    assert_eq!(variant("field-missing")["kind"], "edit-field");
    assert_eq!(variant("field-missing")["op"], "set");
    assert_eq!(variant("duplicate-key")["op"], "remove");
    // An exemption shows its reason, and carries no kind.
    let region = variant("header-region-missing");
    assert!(
        region["exempt"]
            .as_str()
            .unwrap()
            .contains("no action kind"),
        "{region}"
    );
    assert!(region.get("kind").is_none(), "{region}");

    // A kind with no ops omits `op`.
    let index = find("index.freshness")["actions"].as_array().unwrap();
    let stale = index
        .iter()
        .find(|a| a["variant"] == "index-stale")
        .unwrap();
    assert_eq!(stale["kind"], "run-command");
    assert!(stale.get("op").is_none(), "{stale}");

    // Every rule lists at least one variant, and each states either an action
    // or a reason, never both and never neither.
    for rule in all {
        let actions = rule["actions"].as_array().unwrap();
        assert!(!actions.is_empty(), "{} lists no variant", rule["id"]);
        for a in actions {
            assert!(
                a.get("kind").is_some() != a.get("exempt").is_some(),
                "{} / {}: exactly one of kind and exempt",
                rule["id"],
                a["variant"]
            );
        }
    }
}

// ------------------------------------------------- run-command keeps its promise

const COMMAND_CONFIG: &str = "schema_version: 2
rules:
  status.delivery-agreement: error
  embodiment.consistency: error
  index.freshness:
    level: error
    indexes:
      - {path: docs/milestones/README.md, type: milestone, format: \"- [{id}]({file}): {title}\", begin: \"<!-- begin -->\", end: \"<!-- end -->\"}
record_types:
  milestone:
    dir: docs/milestones
    prefix: MILE
    header_shape: yaml-frontmatter
    required_fields: [Status]
    known_fields: [Realized-by, Embodiment]
    statuses:
      values: [Planned, InProgress, Done, WontDo]
      success: [Done]
      failure: [WontDo]
      delivery:
        delivered_tiers: [Implemented, Verified]
        unevidenced_tiers: [\"Not started\"]
        not_done: [Planned, InProgress]
        set_status: Done
";

fn milestone(status: &str, extra: &str, n: u32) -> String {
    format!(
        "---\nStatus: {status}\n{extra}---\n# {n} - M\n\n> **Revision log**\n>\n> | Date | Change | Class |\n> |---|---|---|\n> | 2026-11-04 | Filed. | **substantive** |\n"
    )
}

fn command_repo(name: &str) -> common::TempDir {
    let dir = common::TempDir::new(&format!("urzua-action-{name}"));
    for d in ["docs/milestones", "src", ".urzua"] {
        std::fs::create_dir_all(dir.join(d)).unwrap();
    }
    git(&dir, &["init", "-q"]);
    git(&dir, &["config", "user.email", "test@example.com"]);
    git(&dir, &["config", "user.name", "test"]);
    std::fs::write(dir.join(".urzua/config.yaml"), COMMAND_CONFIG).unwrap();
    std::fs::write(dir.join("src/a.rs"), "fn a() {}\n").unwrap();
    // MILE-1: built, but its Status says not done: `fix` writes the Status.
    std::fs::write(
        dir.join("docs/milestones/MILE-1-a.md"),
        milestone("Planned", "Realized-by: code:src/a.rs\n", 1),
    )
    .unwrap();
    // MILE-2: states Specified while its code evidence computes Implemented.
    std::fs::write(
        dir.join("docs/milestones/MILE-2-b.md"),
        milestone(
            "InProgress",
            "Embodiment: Specified\nRealized-by: code:src/a.rs\n",
            2,
        ),
    )
    .unwrap();
    // MILE-3: a success Status with no evidence: nothing `fix` can write.
    std::fs::write(
        dir.join("docs/milestones/MILE-3-c.md"),
        milestone("Done", "", 3),
    )
    .unwrap();
    // MILE-4: states Verified with no evidence at all. `fix` does not judge a
    // record with neither a locator nor a citing back-pointer, so no command
    // would repair it.
    std::fs::write(
        dir.join("docs/milestones/MILE-4-d.md"),
        milestone("Planned", "Embodiment: Verified\n", 4),
    )
    .unwrap();
    // The index lists only MILE-1: stale.
    std::fs::write(
        dir.join("docs/milestones/README.md"),
        "# Milestones\n\n<!-- begin -->\n- [MILE-1](MILE-1-a.md): M\n<!-- end -->\n",
    )
    .unwrap();
    git(&dir, &["add", "-A"]);
    git(&dir, &["commit", "-q", "-m", "fixture"]);
    dir
}

/// The command a `run-command` action names, with the one flag the consumer
/// adds (`ADR-73`: `--by`, when no identity resolves).
fn command_of(finding: &serde_json::Value) -> Vec<String> {
    let action = &finding["suggested_action"];
    assert_eq!(action["kind"], "run-command", "{finding}");
    assert!(action.get("op").is_none(), "{finding}");
    let argv: Vec<String> = action["target"]["argv"]
        .as_array()
        .unwrap()
        .iter()
        .map(|a| a.as_str().unwrap().to_string())
        .collect();
    assert_eq!(argv[0], "urzua", "{finding}");
    argv
}

fn run_argv(dir: &Path, argv: &[String]) -> std::process::Output {
    let mut args: Vec<&str> = argv[1..].iter().map(String::as_str).collect();
    if argv[1] == "fix" {
        args.extend(["--by", "tester"]);
    }
    urzua(dir, &args)
}

#[test]
fn a_run_command_finding_goes_away_when_its_command_runs() {
    let dir = command_repo("promise");
    let report = parsed(&urzua(&dir, &["check"]));

    // Each finding that offers a command, with the file it is about.
    let offering: Vec<(&str, &str)> = vec![
        ("status.delivery-agreement", "MILE-1-a.md"),
        ("embodiment.consistency", "MILE-2-b.md"),
        ("index.freshness", "README.md"),
    ];
    for (rule, file) in &offering {
        let found = findings(&report, rule);
        let finding = on(&found, file);
        let argv = command_of(finding);
        // The command is concrete: it names the record it repairs.
        if argv[1] == "fix" {
            assert!(
                argv.iter().any(|a| a.ends_with(file)),
                "a fix command names its record: {argv:?}"
            );
        }

        // Run it in a fresh copy of the fixture: the finding must be gone and
        // the command must have succeeded.
        let fresh = command_repo(&format!("promise-{}", rule.replace('.', "-")));
        let out = run_argv(&fresh, &argv);
        assert!(
            out.status.success(),
            "{argv:?} failed: {}{}",
            String::from_utf8_lossy(&out.stdout),
            String::from_utf8_lossy(&out.stderr)
        );
        let after = parsed(&urzua(&fresh, &["check"]));
        assert!(
            !findings(&after, rule)
                .iter()
                .any(|f| f["file"].as_str().unwrap().ends_with(file)),
            "{argv:?} left the {rule} finding on {file}: {after}"
        );
    }
}

#[test]
fn a_finding_that_no_command_would_repair_offers_none() {
    let dir = command_repo("no-command");
    let report = parsed(&urzua(&dir, &["check"]));
    // MILE-3 is a success Status with no evidence: `fix` writes nothing for it.
    let delivery = findings(&report, "status.delivery-agreement");
    let unevidenced = on(&delivery, "MILE-3-c.md");
    assert!(
        unevidenced.get("suggested_action").is_none(),
        "no command would repair it, so none is offered: {unevidenced}"
    );
    // And `fix` indeed lists no repair for MILE-3.
    let fix = parsed(&urzua(&dir, &["fix"]));
    let repairs = fix["repairs"].as_array().unwrap();
    assert!(
        !repairs
            .iter()
            .any(|r| r["record"].as_str().unwrap().ends_with("MILE-3-c.md")),
        "{fix}"
    );

    // MILE-4 claims a tier no evidence supports. The finding is real, `fix`
    // writes nothing for it, so the action is an edit to the field and not a
    // command.
    let embodiment = findings(&report, "embodiment.consistency");
    let claim = on(&embodiment, "MILE-4-d.md");
    assert_eq!(
        claim["suggested_action"],
        serde_json::json!({
            "kind": "edit-field",
            "op": "set",
            "target": { "file": "docs/milestones/MILE-4-d.md", "field": "Embodiment" }
        }),
        "{report}"
    );
    assert!(
        !repairs
            .iter()
            .any(|r| r["record"].as_str().unwrap().ends_with("MILE-4-d.md")),
        "{fix}"
    );
}

#[test]
fn the_editor_hook_output_is_not_wired_to_findings() {
    // `fix --stdin-path` (ADR-69) reads `repairs`; it never carries findings or
    // a suggested_action.
    let dir = command_repo("hook");
    let mut child = common::urzua()
        .args(["fix", "--stdin-path", "docs/milestones/MILE-2-b.md"])
        .current_dir(&dir)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .spawn()
        .unwrap();
    {
        use std::io::Write;
        child
            .stdin
            .take()
            .unwrap()
            .write_all(
                milestone(
                    "InProgress",
                    "Embodiment: Specified\nRealized-by: code:src/a.rs\n",
                    2,
                )
                .as_bytes(),
            )
            .unwrap();
    }
    let out = child.wait_with_output().unwrap();
    let text = String::from_utf8_lossy(&out.stdout);
    assert!(!text.contains("suggested_action"), "{text}");
    assert!(!text.contains("\"findings\""), "{text}");
}

// ------------------------------------------------- next_action is runnable (MILE-72)

/// Runs a `next_action` string as a person would: through a shell, `urzua`
/// resolved on `PATH`, no gh and no flag the string does not carry.
fn run_next_action(dir: &std::path::Path, command: &str) -> std::process::Output {
    let bin = std::path::Path::new(env!("CARGO_TARGET_TMPDIR"))
        .join(format!("next-action-bin-{}", std::process::id()));
    std::fs::create_dir_all(&bin).unwrap();
    let link = bin.join("urzua");
    let _ = std::fs::remove_file(&link);
    std::os::unix::fs::symlink(env!("CARGO_BIN_EXE_urzua"), &link).unwrap();
    let path = std::env::join_paths([bin, common::stub_gh_dir().to_path_buf()])
        .map(|mut p| {
            p.push(":");
            p.push(std::env::var_os("PATH").unwrap_or_default());
            p
        })
        .unwrap();
    std::process::Command::new("sh")
        .arg("-c")
        .arg(command)
        .env("PATH", path)
        .current_dir(dir)
        .output()
        .expect("sh should run")
}

fn json_of(out: &std::process::Output) -> serde_json::Value {
    serde_json::from_slice(&out.stdout)
        .unwrap_or_else(|e| panic!("stdout is not JSON ({e}): {out:?}"))
}

fn urzua_json(dir: &std::path::Path, args: &[&str]) -> serde_json::Value {
    let out = common::urzua()
        .args(args)
        .current_dir(dir)
        .output()
        .unwrap();
    json_of(&out)
}

#[test]
fn the_next_action_of_a_check_runs_and_the_index_one_follows() {
    let dir = command_repo("next-check");
    let report = urzua_json(&dir, &["check"]);
    let next = report["next_action"].as_str().expect("a next_action");
    assert!(
        next.starts_with("urzua fix --apply --ids "),
        "fix is preferred and names its records: {next}"
    );
    assert!(!next.contains('<'), "no placeholder: {next}");
    let out = run_next_action(&dir, next);
    assert_ne!(out.status.code(), Some(2), "not-run: {out:?}");
    assert_eq!(json_of(&out)["status"], "applied", "{out:?}");

    // Nothing `fix` writes is left; the stale index is what remains.
    let report = urzua_json(&dir, &["check"]);
    let next = report["next_action"]
        .as_str()
        .expect("an index next_action");
    assert_eq!(next, "urzua index --apply", "{report}");
    let out = run_next_action(&dir, next);
    assert_ne!(out.status.code(), Some(2), "not-run: {out:?}");
    let after = urzua_json(&dir, &["check"]);
    assert!(
        after["findings"]
            .as_array()
            .unwrap()
            .iter()
            .all(|f| f["rule"] != "index.freshness"),
        "the index finding is gone: {after}"
    );
}

#[test]
fn the_next_action_of_a_fix_detect_run_runs() {
    let dir = command_repo("next-fix");
    let detect = urzua_json(&dir, &["fix"]);
    assert_eq!(detect["status"], "repairs-available", "{detect}");
    let next = detect["next_action"].as_str().expect("a next_action");
    assert!(next.starts_with("urzua fix --apply --ids "), "{next}");
    let out = run_next_action(&dir, next);
    assert_ne!(out.status.code(), Some(2), "not-run: {out:?}");
    let applied = json_of(&out);
    assert_eq!(applied["status"], "applied", "{applied}");
    // After an apply the next step is a check, which also runs.
    let check = applied["next_action"]
        .as_str()
        .expect("a check next_action");
    assert_eq!(check, "urzua check");
    let out = run_next_action(&dir, check);
    assert_ne!(out.status.code(), Some(2), "not-run: {out:?}");
}
