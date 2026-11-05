//! MILE-185 / ADR-74: scoped evidence pins. Real git-repo fixtures, real
//! binary; all output is JSON. A scope that stops resolving is reported by
//! `embodiment.locator-exists`, never passed.

use std::path::{Path, PathBuf};
use std::process::Command;

fn git(dir: &Path, args: &[&str]) {
    assert!(
        Command::new("git")
            .args(args)
            .current_dir(dir)
            .status()
            .expect("git should run")
            .success(),
        "git {args:?} failed"
    );
}

fn run_urzua(dir: &Path, args: &[&str]) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_urzua"))
        .args(args)
        .current_dir(dir)
        .output()
        .expect("urzua binary should run")
}

fn json(out: &std::process::Output) -> serde_json::Value {
    serde_json::from_slice(&out.stdout)
        .unwrap_or_else(|e| panic!("stdout is not JSON ({e}): {out:?}"))
}

const RECORD_PATH: &str = "docs/adr/ADR-1-first.md";
const ZEROS: &str = "0000000000000000000000000000000000000000000000000000000000000000";

const RESOLVERS: &str = r#"
locator_resolvers:
  - name: rust
    glob: "**/*.rs"
    start: '^\s*(pub\s+)?(fn|struct)\s+{symbol}\b'
    end: brace_balanced
    leading: '^\s*(#\[|///)'
    container:
      start: '^\s*impl(<.*?>)?\s+{symbol}(<[^{]*>)?\s*(where\b.*|\{.*)?$'
      end: brace_balanced
      spans: all
    member:
      start: '^\s*(pub\s+)?fn\s+{symbol}\b'
      end: brace_balanced
"#;

fn config(resolvers: &str) -> String {
    format!(
        "schema_version: 2\nrules:\n  embodiment.locator-exists: error\n{resolvers}\nrecord_types:\n  adr:\n    dir: \"docs/adr\"\n    required_fields: []\n    known_fields: [\"Embodiment\", \"Realized-by\"]\n"
    )
}

fn record(realized_by: &str) -> String {
    format!(
        "---\nEmbodiment: 'Implemented'\nRealized-by: '{realized_by}'\n---\n# ADR-1: First\n\n> **Revision log**\n>\n> | Date | Change | Class |\n> |---|---|---|\n> | 2026-01-01 | Initial. | **structural** |\n"
    )
}

const SOURCE: &str = "/// Alpha.\nfn alpha() {\n    1;\n}\n\nstruct Beta {\n    b: u8,\n}\n\nimpl Beta {\n    fn go(&self) {}\n}\n";

fn repo(name: &str, resolvers: &str, realized_by: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("urzua-scoped-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    for d in ["docs/adr", "src", ".urzua"] {
        std::fs::create_dir_all(dir.join(d)).unwrap();
    }
    git(&dir, &["init", "-q"]);
    git(&dir, &["config", "user.email", "test@example.com"]);
    git(&dir, &["config", "user.name", "test"]);
    git(&dir, &["config", "commit.gpgsign", "false"]);
    std::fs::write(dir.join(".urzua/config.yaml"), config(resolvers)).unwrap();
    std::fs::write(dir.join("src/a.rs"), SOURCE).unwrap();
    std::fs::write(dir.join(RECORD_PATH), record(realized_by)).unwrap();
    git(&dir, &["add", "-A"]);
    git(&dir, &["commit", "-q", "-m", "fixture"]);
    dir
}

fn exists_messages(dir: &Path) -> Vec<String> {
    let parsed = json(&run_urzua(dir, &["check", "docs/"]));
    parsed["findings"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|f| f["rule"] == "embodiment.locator-exists")
        .map(|f| f["message"].as_str().unwrap().to_string())
        .collect()
}

#[test]
fn symbols_members_and_ranges_that_resolve_draw_no_finding() {
    let loc = format!(
        "code:src/a.rs#alpha@sha256:{ZEROS}, code:src/a.rs#Beta@sha256:{ZEROS}, code:src/a.rs#Beta::go@sha256:{ZEROS}, code:src/a.rs#L2-L4@sha256:{ZEROS}"
    );
    let dir = repo("resolve", RESOLVERS, &loc);
    assert_eq!(exists_messages(&dir), Vec::<String>::new());
}

#[test]
fn a_scope_that_no_longer_resolves_is_reported_with_its_reason() {
    for (locator, reason) in [
        ("#gone", "matches nothing"),
        ("#Beta::gone", "member 'gone' not found"),
        ("#L2-L99", "beyond the end"),
    ] {
        let dir = repo(
            "unresolved",
            RESOLVERS,
            &format!("code:src/a.rs{locator}@sha256:{ZEROS}"),
        );
        let m = exists_messages(&dir);
        assert_eq!(m.len(), 1, "{locator}: {m:?}");
        assert!(m[0].contains(reason) && m[0].contains("src/a.rs"), "{m:?}");
    }
}

#[test]
fn a_symbol_in_a_file_type_with_no_resolver_is_reported_unsupported() {
    let dir = repo(
        "no-resolver",
        "",
        &format!("code:src/a.rs#alpha@sha256:{ZEROS}"),
    );
    let m = exists_messages(&dir);
    assert_eq!(m.len(), 1, "{m:?}");
    assert!(m[0].contains("no locator resolver"), "{m:?}");
    // A line range needs none.
    let dir = repo(
        "range-only",
        "",
        &format!("code:src/a.rs#L2-L4@sha256:{ZEROS}"),
    );
    assert_eq!(exists_messages(&dir), Vec::<String>::new());
}

#[test]
fn a_scope_without_a_pin_and_a_commit_anchor_are_reported() {
    let dir = repo("unpinned", RESOLVERS, "code:src/a.rs#alpha");
    let m = exists_messages(&dir);
    assert_eq!(m.len(), 1, "{m:?}");
    assert!(m[0].contains("without a pin"), "{m:?}");
    let dir = repo("commit", RESOLVERS, "code:src/a.rs#alpha@abcdef1");
    let m = exists_messages(&dir);
    assert_eq!(m.len(), 1, "{m:?}");
    assert!(m[0].contains("commit anchor not yet supported"), "{m:?}");
}

#[test]
fn a_malformed_locator_names_the_grammar_not_a_missing_file() {
    let dir = repo("malformed", RESOLVERS, "code:src/a.rs#L0");
    let m = exists_messages(&dir);
    assert_eq!(m.len(), 1, "{m:?}");
    assert!(
        m[0].contains("does not parse") && m[0].contains("grammar"),
        "{m:?}"
    );
}

#[test]
fn an_invalid_resolver_declaration_is_a_config_load_error() {
    for (what, bad) in [
        (
            "placeholder",
            "  - {name: r, glob: '**/*.rs', start: '^fn', end: indent}\n",
        ),
        (
            "pattern",
            "  - {name: r, glob: '**/*.rs', start: '^({symbol}', end: indent}\n",
        ),
        (
            "end kind",
            "  - {name: r, glob: '**/*.rs', start: '^{symbol}', end: nope}\n",
        ),
        (
            "unknown key",
            "  - {name: r, glob: '**/*.rs', start: '^{symbol}', end: indent, extra: 1}\n",
        ),
    ] {
        let dir = repo(
            "bad-resolver",
            &format!("locator_resolvers:\n{bad}"),
            "code:src/a.rs",
        );
        let out = run_urzua(&dir, &["check", "docs/"]);
        assert_eq!(out.status.code(), Some(2), "{what}: {out:?}");
        assert!(
            String::from_utf8_lossy(&out.stdout).contains("locator_resolvers")
                || String::from_utf8_lossy(&out.stdout).contains("could not parse config"),
            "{what}: {out:?}"
        );
    }
}

// ---- drift, explain and re-baselining (ADR-74 points 7 and 8) ------------------

fn config_with(consistency: &str) -> String {
    format!(
        "schema_version: 2\nrules:\n  embodiment.consistency: {consistency}\n  embodiment.locator-exists: error\n{RESOLVERS}\nrecord_types:\n  adr:\n    dir: \"docs/adr\"\n    required_fields: []\n    known_fields: [\"Embodiment\", \"Realized-by\", \"Last-audited\"]\n"
    )
}

fn commit_all(dir: &Path) {
    git(dir, &["add", "-A"]);
    git(dir, &["commit", "-q", "-m", "fixture"]);
}

/// The pin `urzua explain` prints for `path#scope`.
fn explained_pin(dir: &Path, arg: &str) -> String {
    let out = run_urzua(dir, &["explain", arg]);
    assert!(out.status.success(), "{out:?}");
    json(&out)["scope"]["pin"].as_str().unwrap().to_string()
}

fn drift_messages(dir: &Path) -> Vec<String> {
    let parsed = json(&run_urzua(dir, &["check", "docs/"]));
    parsed["findings"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|f| f["rule"] == "embodiment.consistency")
        .map(|f| f["message"].as_str().unwrap().to_string())
        .collect()
}

fn pinned_repo(name: &str, consistency: &str, scope: &str) -> PathBuf {
    let dir = repo(name, "", "code:src/a.rs");
    std::fs::write(dir.join(".urzua/config.yaml"), config_with(consistency)).unwrap();
    let pin = explained_pin(&dir, &format!("src/a.rs{scope}"));
    std::fs::write(dir.join(RECORD_PATH), record(&format!("code:{pin}"))).unwrap();
    commit_all(&dir);
    dir
}

#[test]
fn explain_prints_the_resolved_span_and_a_pin_ready_to_paste() {
    let dir = repo("explain", RESOLVERS, "code:src/a.rs");
    let out = run_urzua(&dir, &["explain", "src/a.rs#Beta::go"]);
    let parsed = json(&out);
    let scope = &parsed["scope"];
    assert_eq!(
        (scope["start"].as_u64(), scope["end"].as_u64()),
        (Some(11), Some(11))
    );
    assert_eq!(scope["text"], "    fn go(&self) {}");
    assert_eq!(scope["resolver"], "rust");
    let hash = scope["sha256"].as_str().unwrap();
    assert_eq!(scope["pin"], format!("src/a.rs#Beta::go@sha256:{hash}"));
    // The leading doc line is part of `alpha`.
    let parsed = json(&run_urzua(&dir, &["explain", "src/a.rs#alpha"]));
    assert_eq!(parsed["scope"]["start"], 1);
    assert_eq!(parsed["scope"]["end"], 4);
    // A scope that does not resolve is a could-not-run with the reason.
    let out = run_urzua(&dir, &["explain", "src/a.rs#gone"]);
    assert_eq!(out.status.code(), Some(2), "{out:?}");
    assert!(String::from_utf8_lossy(&out.stdout).contains("matches nothing"));
    // Without a scope it is the old query.
    let parsed = json(&run_urzua(&dir, &["explain", "src/a.rs"]));
    assert!(parsed.get("scope").is_none(), "{parsed}");
}

#[test]
fn explain_names_the_scope_each_citing_record_pins() {
    let dir = repo(
        "explain-cited",
        RESOLVERS,
        &format!("code:src/a.rs#alpha@sha256:{ZEROS}"),
    );
    let parsed = json(&run_urzua(&dir, &["explain", "src/a.rs"]));
    let g = &parsed["governing_records"][0];
    assert_eq!(g["scope"], "alpha", "{parsed}");
}

#[test]
fn an_edit_elsewhere_in_the_file_is_not_drift_and_an_edit_to_the_item_is() {
    {
        let consistency = "error";
        let dir = pinned_repo("drift", consistency, "#alpha");
        // Unrelated edit, even with the default (no grace) window.
        let src = std::fs::read_to_string(dir.join("src/a.rs")).unwrap();
        std::fs::write(
            dir.join("src/a.rs"),
            src.replace("b: u8,", "b: u8,\n    c: u8,"),
        )
        .unwrap();
        commit_all(&dir);
        assert_eq!(drift_messages(&dir), Vec::<String>::new());
        // An edit to the pinned item.
        std::fs::write(
            dir.join("src/a.rs"),
            std::fs::read_to_string(dir.join("src/a.rs"))
                .unwrap()
                .replace("    1;", "    2;"),
        )
        .unwrap();
        commit_all(&dir);
        assert!(
            drift_messages(&dir)
                .iter()
                .any(|m| m.contains("Drift detected")),
            "{:?}",
            drift_messages(&dir)
        );
    }
}

#[test]
fn a_scoped_pin_has_no_file_based_grace_window_but_an_audit_date_still_applies() {
    let with_grace = |dir: &Path, audit: Option<&str>| {
        let c =
            config_with("{level: error, drift_grace_days: 14, drift_audit_field: Last-audited}");
        std::fs::write(dir.join(".urzua/config.yaml"), c).unwrap();
        let rec = std::fs::read_to_string(dir.join(RECORD_PATH)).unwrap();
        let rec = match audit {
            Some(d) => rec.replace(
                "---\n# ADR-1",
                &format!("Last-audited: '{d}'\n---\n# ADR-1"),
            ),
            None => rec,
        };
        std::fs::write(dir.join(RECORD_PATH), rec).unwrap();
    };
    // The cited file changed just now (inside any file-based window): the
    // scoped pin is drift regardless.
    let dir = pinned_repo("grace-none", "error", "#alpha");
    std::fs::write(
        dir.join("src/a.rs"),
        std::fs::read_to_string(dir.join("src/a.rs"))
            .unwrap()
            .replace("    1;", "    2;"),
    )
    .unwrap();
    commit_all(&dir);
    with_grace(&dir, None);
    assert!(
        drift_messages(&dir)
            .iter()
            .any(|m| m.contains("Drift detected")),
        "{:?}",
        drift_messages(&dir)
    );
    // A fresh declared audit date holds it back, as for a file-level pin.
    let today =
        String::from_utf8(Command::new("date").arg("+%F").output().unwrap().stdout).unwrap();
    with_grace(&dir, Some(today.trim()));
    assert_eq!(drift_messages(&dir), Vec::<String>::new());
}

#[test]
fn a_scope_that_stops_resolving_is_reported_once_and_not_also_as_drift() {
    let dir = pinned_repo("gone", "error", "#alpha");
    std::fs::write(
        dir.join("src/a.rs"),
        std::fs::read_to_string(dir.join("src/a.rs"))
            .unwrap()
            .replace("fn alpha()", "fn renamed()"),
    )
    .unwrap();
    commit_all(&dir);
    let parsed = json(&run_urzua(&dir, &["check", "docs/"]));
    let rules: Vec<&str> = parsed["findings"]
        .as_array()
        .unwrap()
        .iter()
        .map(|f| f["rule"].as_str().unwrap())
        .collect();
    assert_eq!(rules, ["embodiment.locator-exists"], "{parsed}");
}

#[test]
fn rebaseline_shows_the_span_re_pins_in_its_spelling_and_lists_failed_pins() {
    let dir = pinned_repo("rebase", "error", "#alpha");
    let stale_record = std::fs::read_to_string(dir.join(RECORD_PATH)).unwrap();
    // A second pin on a scope that will not resolve.
    let with_gone = stale_record.replace(
        "Realized-by: 'code:",
        &format!("Realized-by: 'code:src/a.rs#gone@sha256:{ZEROS}, code:"),
    );
    std::fs::write(dir.join(RECORD_PATH), with_gone).unwrap();
    std::fs::write(
        dir.join("src/a.rs"),
        std::fs::read_to_string(dir.join("src/a.rs"))
            .unwrap()
            .replace("    1;", "    2;"),
    )
    .unwrap();
    commit_all(&dir);

    let listed = json(&run_urzua(&dir, &["fix", "--rebaseline"]));
    let pin = &listed["rebaselines"][0]["pins"][0];
    assert_eq!(pin["locator"], "src/a.rs#alpha", "{listed}");
    assert_eq!(pin["span"]["start"], 1);
    assert!(pin["span"]["text"].as_str().unwrap().contains("2;"));
    let failed = listed["failed_pins"].as_array().unwrap();
    assert_eq!(failed.len(), 1, "{listed}");
    assert_eq!(failed[0]["locator"], "src/a.rs#gone");
    assert!(failed[0]["reason"]
        .as_str()
        .unwrap()
        .contains("matches nothing"));

    let applied = run_urzua(
        &dir,
        &[
            "fix",
            "--rebaseline",
            "--apply",
            "--ids",
            RECORD_PATH,
            "--by",
            "ana",
            "--reason",
            "Read the item; the claim holds.",
        ],
    );
    let out = json(&applied);
    assert_eq!(out["failed_pins"].as_array().unwrap().len(), 1, "{out}");
    let rec = std::fs::read_to_string(dir.join(RECORD_PATH)).unwrap();
    let new_hash = explained_pin(&dir, "src/a.rs#alpha");
    assert!(rec.contains(&format!("code:{new_hash}")), "{rec}");
    // The unresolving pin is untouched, and the row names the lines.
    assert!(
        rec.contains(&format!("src/a.rs#gone@sha256:{ZEROS}")),
        "{rec}"
    );
    assert!(
        rec.contains("attested by") && rec.contains("(lines 1-4)"),
        "{rec}"
    );
}
