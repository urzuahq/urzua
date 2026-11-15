//! MILE-201: the `attestation` configuration (`ADR-89` section 5, `SPEC-3`,
//! `SPEC-8`). `identity: gh-verified | asserted` and `require_verified` decide
//! how `fix --apply` and `fix --rebaseline --apply` resolve who attests, and
//! the identity source and the actor kind land in the revision row. Real git
//! fixtures, real binary, a stub `gh` on PATH; all output is JSON.

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

fn commit_all(dir: &Path) {
    git(dir, &["add", "-A"]);
    git(dir, &["commit", "-q", "-m", "fixture"]);
}

/// Runs urzua with a `gh` on PATH that answers `login`, or fails when `None`
/// (not logged in), so the result never depends on the machine's own `gh`.
fn run(dir: &Path, args: &[&str], login: Option<&str>) -> std::process::Output {
    use std::os::unix::fs::PermissionsExt;
    let stub = common::TempDir::new("urzua-attest-stub-gh");
    let body = match login {
        Some(l) => format!("#!/bin/sh\necho '{l}'\n"),
        None => "#!/bin/sh\nexit 1\n".to_string(),
    };
    std::fs::write(stub.join("gh"), body).unwrap();
    std::fs::set_permissions(stub.join("gh"), std::fs::Permissions::from_mode(0o755)).unwrap();
    let path = format!(
        "{}:{}",
        stub.display(),
        std::env::var("PATH").unwrap_or_default()
    );
    common::urzua()
        .args(args)
        .env("PATH", path)
        .current_dir(dir)
        .output()
        .expect("urzua binary should run")
}

fn json(out: &std::process::Output) -> serde_json::Value {
    serde_json::from_slice(&out.stdout)
        .unwrap_or_else(|e| panic!("stdout is not JSON ({e}): {out:?}"))
}

const BASE: &str = "schema_version: 2\nrules: {embodiment.consistency: error}\n\nrecord_types:\n  adr:\n    dir: \"docs/adr\"\n    required_fields: []\n";
const V1: &str = "fn a() {}\n";
const V2: &str = "fn a() { let _ = 1; }\n";
const RECORD: &str = "docs/adr/ADR-1-a.md";
const LOG: &str = "\n> **Revision log**\n>\n> | Date | Change | Class |\n> |---|---|---|\n> | 2026-01-01 | Initial. | **structural** |\n";

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
    let out = child.wait_with_output().unwrap();
    String::from_utf8(out.stdout)
        .unwrap()
        .split_whitespace()
        .next()
        .unwrap()
        .to_string()
}

fn repo(name: &str, attestation: &str) -> common::TempDir {
    let dir = common::TempDir::new(&format!("urzua-attest-{name}"));
    std::fs::create_dir_all(dir.join("docs/adr")).unwrap();
    std::fs::create_dir_all(dir.join("src")).unwrap();
    std::fs::create_dir_all(dir.join(".urzua")).unwrap();
    git(&dir, &["init", "-q"]);
    git(&dir, &["config", "user.email", "test@example.com"]);
    git(&dir, &["config", "user.name", "git-name"]);
    std::fs::write(
        dir.join(".urzua/config.yaml"),
        format!("{BASE}{attestation}"),
    )
    .unwrap();
    std::fs::write(dir.join("src/a.rs"), V1).unwrap();
    dir
}

/// A record pinning `src/a.rs`, which then changes: `--rebaseline` has a stale
/// pin to re-pin.
fn pinned(name: &str, attestation: &str) -> common::TempDir {
    let dir = repo(name, attestation);
    std::fs::write(
        dir.join(RECORD),
        format!(
            "---\nEmbodiment: Implemented\nRealized-by: 'code:src/a.rs@sha256:{}'\n---\n# ADR-1: T\n{LOG}",
            sha256(V1)
        ),
    )
    .unwrap();
    commit_all(&dir);
    std::fs::write(dir.join("src/a.rs"), V2).unwrap();
    commit_all(&dir);
    dir
}

/// A record whose `Embodiment` is stale against its drifted evidence: plain
/// `fix` has a Tier 1 repair to apply.
fn drifted(name: &str, attestation: &str) -> common::TempDir {
    let dir = repo(name, attestation);
    std::fs::write(
        dir.join(RECORD),
        format!("---\nEmbodiment: Implemented\nRealized-by: code:src/a.rs\n---\n# ADR-1: T\n{LOG}"),
    )
    .unwrap();
    commit_all(&dir);
    std::fs::write(dir.join("src/a.rs"), V2).unwrap();
    commit_all(&dir);
    dir
}

fn rebaseline(extra: &[&str]) -> Vec<String> {
    let mut args = vec![
        "fix".to_string(),
        "--rebaseline".into(),
        "--apply".into(),
        "--ids".into(),
        RECORD.into(),
        "--reason".into(),
        "The helper now discards its result.".into(),
    ];
    args.extend(extra.iter().map(|s| s.to_string()));
    args
}

fn tier1(extra: &[&str]) -> Vec<String> {
    let mut args = vec![
        "fix".to_string(),
        "--apply".into(),
        "--ids".into(),
        RECORD.into(),
    ];
    args.extend(extra.iter().map(|s| s.to_string()));
    args
}

fn go(dir: &Path, args: &[String], login: Option<&str>) -> std::process::Output {
    let refs: Vec<&str> = args.iter().map(String::as_str).collect();
    run(dir, &refs, login)
}

fn last_row(dir: &Path) -> String {
    std::fs::read_to_string(dir.join(RECORD))
        .unwrap()
        .lines()
        .rfind(|l| l.starts_with("> | 20"))
        .unwrap()
        .to_string()
}

#[test]
fn an_undeclared_config_keeps_the_gh_login_and_records_it_as_gh_verified() {
    let dir = pinned("default", "");
    let out = go(
        &dir,
        &rebaseline(&["--by", "typed", "--actor", "agent"]),
        Some("gh-login"),
    );
    assert_eq!(out.status.code(), Some(0), "{out:?}");
    let v = json(&out);
    assert_eq!(v["status"], "applied", "{v}");
    let row = last_row(&dir);
    assert!(
        row.contains(
            "Re-baselined (attested by gh-login, identity: gh-verified, actor: agent): re-pinned"
        ),
        "{row}"
    );
    // Today's notice stays, and the replaced --by is now a field of the report.
    assert!(
        v["notices"][0]["message"]
            .as_str()
            .unwrap()
            .contains("typed"),
        "{v}"
    );
    assert_eq!(v["identity"]["name"], "gh-login", "{v}");
    assert_eq!(v["identity"]["source"], "gh-verified", "{v}");
    assert_eq!(v["identity"]["actor"], "agent", "{v}");
    assert_eq!(v["identity"]["replaced_by"], "typed", "{v}");
}

#[test]
fn an_undeclared_config_without_gh_takes_by_as_given_and_says_asserted() {
    let dir = drifted("default-nogh", "");
    let out = go(&dir, &tier1(&["--by", "typed"]), None);
    assert_eq!(out.status.code(), Some(0), "{out:?}");
    let row = last_row(&dir);
    assert!(
        row.contains(
            "Tool-authored (by typed, identity: asserted, actor: unspecified): recomputed"
        ),
        "{row}"
    );
    assert!(json(&out)["identity"]["replaced_by"].is_null());
}

#[test]
fn without_by_and_gh_the_git_config_name_is_recorded_as_git_config() {
    let dir = drifted("git-config", "");
    let out = go(&dir, &tier1(&[]), None);
    assert_eq!(out.status.code(), Some(0), "{out:?}");
    assert!(
        last_row(&dir).contains("(by git-name, identity: git-config, actor: unspecified)"),
        "{}",
        last_row(&dir)
    );
}

#[test]
fn require_verified_without_a_gh_answer_is_not_run_and_writes_nothing() {
    let strict = "attestation:\n  identity: gh-verified\n  require_verified: true\n";
    for (label, make, args) in [
        (
            "rebaseline",
            pinned as fn(&str, &str) -> common::TempDir,
            rebaseline(&["--by", "typed"]),
        ),
        (
            "tier1",
            drifted as fn(&str, &str) -> common::TempDir,
            tier1(&["--by", "typed"]),
        ),
    ] {
        let dir = make(&format!("strict-{label}"), strict);
        let before = std::fs::read_to_string(dir.join(RECORD)).unwrap();
        let out = go(&dir, &args, None);
        assert_eq!(out.status.code(), Some(2), "{label}: {out:?}");
        let v = json(&out);
        assert_eq!(v["status"], "not-run", "{label}: {v}");
        assert!(
            v["error"].as_str().unwrap().contains("require_verified"),
            "{label}: {v}"
        );
        assert_eq!(
            std::fs::read_to_string(dir.join(RECORD)).unwrap(),
            before,
            "{label}"
        );
        assert!(!dir.join(".urzua/cache/fix.lock").exists(), "{label}");
    }
}

#[test]
fn require_verified_with_a_gh_answer_applies_as_gh_verified() {
    let strict = "attestation:\n  require_verified: true\n";
    let dir = pinned("strict-ok", strict);
    let out = go(&dir, &rebaseline(&["--actor", "human"]), Some("gh-login"));
    assert_eq!(out.status.code(), Some(0), "{out:?}");
    assert!(last_row(&dir).contains("attested by gh-login, identity: gh-verified"));
}

#[test]
fn asserted_records_by_as_given_with_the_verified_login_beside_it() {
    let asserted = "attestation:\n  identity: asserted\n";
    for (label, make, args, prefix) in [
        (
            "rebaseline",
            pinned as fn(&str, &str) -> common::TempDir,
            rebaseline(&["--by", "the-maintainer"]),
            "Re-baselined (attested by ",
        ),
        (
            "tier1",
            drifted as fn(&str, &str) -> common::TempDir,
            tier1(&["--by", "the-maintainer"]),
            "Tool-authored (by ",
        ),
    ] {
        let dir = make(&format!("asserted-{label}"), asserted);
        let out = go(&dir, &args, Some("gh-login"));
        assert_eq!(out.status.code(), Some(0), "{label}: {out:?}");
        let v = json(&out);
        assert_eq!(v["identity"]["name"], "the-maintainer", "{label}: {v}");
        assert_eq!(v["identity"]["source"], "asserted", "{label}: {v}");
        assert_eq!(v["identity"]["verified_login"], "gh-login", "{label}: {v}");
        assert!(
            v.get("notices").is_none(),
            "{label}: no replacement to warn of: {v}"
        );
        let row = last_row(&dir);
        assert!(
            row.contains(&format!(
                "{prefix}the-maintainer, identity: asserted, actor: unspecified, verified login: gh-login)"
            )),
            "{label}: {row}"
        );
    }
}

#[test]
fn asserted_without_a_gh_answer_records_no_verified_login() {
    let dir = pinned("asserted-nogh", "attestation:\n  identity: asserted\n");
    let out = go(&dir, &rebaseline(&["--by", "the-maintainer"]), None);
    assert_eq!(out.status.code(), Some(0), "{out:?}");
    let row = last_row(&dir);
    assert!(
        row.contains("(attested by the-maintainer, identity: asserted, actor: unspecified):"),
        "{row}"
    );
}

#[test]
fn actor_is_recorded_by_both_commands_and_is_the_callers_claim() {
    for actor in ["agent", "human"] {
        let dir = pinned(&format!("actor-{actor}-rb"), "");
        let out = go(&dir, &rebaseline(&["--actor", actor]), Some("gh-login"));
        assert_eq!(out.status.code(), Some(0), "{out:?}");
        assert!(
            last_row(&dir).contains(&format!("identity: gh-verified, actor: {actor})")),
            "{}",
            last_row(&dir)
        );
        assert_eq!(json(&out)["identity"]["actor"], actor);

        let dir = drifted(&format!("actor-{actor}-t1"), "");
        let out = go(&dir, &tier1(&["--actor", actor]), Some("gh-login"));
        assert_eq!(out.status.code(), Some(0), "{out:?}");
        assert!(
            last_row(&dir).contains(&format!("identity: gh-verified, actor: {actor})")),
            "{}",
            last_row(&dir)
        );
    }
}

#[test]
fn an_unknown_actor_is_refused_and_writes_nothing() {
    let dir = pinned("actor-bad", "");
    let before = std::fs::read_to_string(dir.join(RECORD)).unwrap();
    let out = go(&dir, &rebaseline(&["--actor", "robot"]), Some("gh-login"));
    assert_eq!(out.status.code(), Some(2), "{out:?}");
    let err = json(&out)["error"].as_str().unwrap().to_string();
    assert!(err.contains("agent") && err.contains("human"), "{err}");
    assert_eq!(std::fs::read_to_string(dir.join(RECORD)).unwrap(), before);
}

#[test]
fn an_unknown_identity_is_a_load_error_naming_the_allowed_values() {
    let dir = pinned("bad-identity", "attestation:\n  identity: signed\n");
    let out = go(&dir, &["check".to_string()], None);
    assert_eq!(out.status.code(), Some(2), "{out:?}");
    let err = json(&out)["error"].as_str().unwrap().to_string();
    assert!(
        err.contains("gh-verified") && err.contains("asserted"),
        "{err}"
    );
}

#[test]
fn require_verified_with_asserted_is_a_load_error() {
    let dir = pinned(
        "bad-combination",
        "attestation:\n  identity: asserted\n  require_verified: true\n",
    );
    let out = go(&dir, &rebaseline(&["--by", "x"]), Some("gh-login"));
    assert_eq!(out.status.code(), Some(2), "{out:?}");
    assert!(json(&out)["error"]
        .as_str()
        .unwrap()
        .contains("require_verified"));
}

// ADR-101 (people own decisions): a gh-verified identity is a person's login,
// so a write with no `--actor` would read as that person's own attestation
// even when an agent ran it. It is refused, and writes nothing.

fn assert_refused_for_missing_actor(out: &std::process::Output) {
    assert_eq!(out.status.code(), Some(2), "{out:?}");
    let v = json(out);
    assert_eq!(v["status"], "not-run", "{v}");
    let err = v["error"].as_str().unwrap();
    assert!(
        err.contains("--actor") && err.contains("agent") && err.contains("human"),
        "{err}"
    );
}

#[test]
fn a_gh_verified_rebaseline_apply_without_actor_is_refused_and_writes_nothing() {
    let dir = pinned("noactor-rb", "");
    let before = std::fs::read_to_string(dir.join(RECORD)).unwrap();
    let out = go(&dir, &rebaseline(&[]), Some("gh-login"));
    assert_refused_for_missing_actor(&out);
    assert_eq!(std::fs::read_to_string(dir.join(RECORD)).unwrap(), before);
}

#[test]
fn a_gh_verified_fix_apply_without_actor_is_refused_and_writes_nothing() {
    let dir = drifted("noactor-t1", "");
    let before = std::fs::read_to_string(dir.join(RECORD)).unwrap();
    let out = go(&dir, &tier1(&[]), Some("gh-login"));
    assert_refused_for_missing_actor(&out);
    assert_eq!(std::fs::read_to_string(dir.join(RECORD)).unwrap(), before);
}

#[test]
fn an_asserted_or_git_config_identity_needs_no_actor() {
    // No gh answer: the identity is `--by` as asserted, not a verified login.
    let dir = drifted("noactor-asserted", "");
    let out = go(&dir, &tier1(&["--by", "typed"]), None);
    assert_eq!(out.status.code(), Some(0), "{out:?}");
    assert!(
        last_row(&dir).contains("actor: unspecified"),
        "{}",
        last_row(&dir)
    );
}
