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
