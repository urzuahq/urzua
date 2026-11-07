//! MILE-89, ADR-93: with `new.check_open_prs`, `urzua new` fetches pull-request
//! head refs with plain git into `refs/urzua/pr/<segment>/*`, skips the numbers
//! found there and in sibling worktrees, serializes concurrent runs with a lock
//! and falls back to local refs when offline; the opt-in `identity.open-pr-collision`
//! rule flags a shared id under `--base`. Real git fixtures (a bare origin that
//! publishes `refs/pull/<n>/head` the way a host does), real binary.
//!
//! Verifies: SPEC-12, SPEC-3, SPEC-2

mod common;

use std::path::{Path, PathBuf};
use std::process::Command;

fn git(dir: &Path, args: &[&str]) -> String {
    let out = Command::new("git")
        .args(["-c", "user.email=t@example.com", "-c", "user.name=t"])
        .args(args)
        .current_dir(dir)
        .output()
        .expect("git should run");
    assert!(
        out.status.success(),
        "git {args:?} failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout).trim().to_string()
}

fn urzua_in(dir: &Path, args: &[&str]) -> std::process::Output {
    common::urzua()
        .args(args)
        .current_dir(dir)
        .env("GIT_ALLOW_PROTOCOL", "file:ext")
        .output()
        .expect("urzua binary should run")
}

fn config(new_block: &str, rules: &str) -> String {
    format!(
        "schema_version: 2\n{new_block}rules:\n  header.required-fields: error\n{rules}\
         record_types:\n  bug:\n    dir: docs/bug\n    prefix: BUG\n    required_fields: [Status]\n    header_shape: yaml-frontmatter\n"
    )
}

fn bug(n: u32, stable_id: &str) -> String {
    format!("---\nStable-Id: {stable_id}\nStatus: Open\n---\n# BUG-{n} - x\n")
}

struct World {
    _root: common::TempDir,
    origin: PathBuf,
    work: PathBuf,
}

/// A bare origin holding `main` with BUG-1, and a clone `work` of it.
fn world(name: &str, new_block: &str, rules: &str) -> World {
    let root = common::TempDir::new(&format!("urzua-open-pr-{name}"));
    let origin = root.join("origin.git");
    let work = root.join("work");
    std::fs::create_dir_all(&origin).unwrap();
    git(&origin, &["init", "-q", "--bare", "-b", "main"]);
    git(
        root.path(),
        &["clone", "-q", origin.to_str().unwrap(), "work"],
    );
    git(&work, &["checkout", "-q", "-b", "main"]);
    std::fs::create_dir_all(work.join(".urzua")).unwrap();
    std::fs::create_dir_all(work.join("docs/bug")).unwrap();
    std::fs::write(work.join(".urzua/config.yaml"), config(new_block, rules)).unwrap();
    std::fs::write(
        work.join("docs/bug/BUG-1-one.md"),
        bug(1, "01AAAAAAAAAAAAAAAAAAAAAAAA"),
    )
    .unwrap();
    git(&work, &["add", "-A"]);
    git(&work, &["commit", "-q", "-m", "base"]);
    git(&work, &["push", "-q", "origin", "main"]);
    World {
        _root: root,
        origin,
        work,
    }
}

/// Publishes `files` (path, content) as a pull request head ref on the origin, as a
/// host does for `refs/pull/<n>/head`, from a throwaway clone off `main`.
fn publish_pull(w: &World, refname: &str, files: &[(&str, String)]) {
    let tmp = common::TempDir::new("urzua-open-pr-pub");
    git(
        tmp.path(),
        &["clone", "-q", w.origin.to_str().unwrap(), "p"],
    );
    let p = tmp.join("p");
    git(&p, &["checkout", "-q", "-B", "main", "origin/main"]);
    for (path, content) in files {
        std::fs::create_dir_all(p.join(path).parent().unwrap()).unwrap();
        std::fs::write(p.join(path), content).unwrap();
    }
    git(&p, &["add", "-A"]);
    git(&p, &["commit", "-q", "-m", "pull request"]);
    git(&p, &["push", "-q", "origin", &format!("HEAD:{refname}")]);
}

fn new_report(w: &World, extra: &[&str]) -> (i32, serde_json::Value, String) {
    let mut args = vec!["new", "bug", "Fresh", "--by", "tester"];
    args.extend_from_slice(extra);
    let out = urzua_in(&w.work, &args);
    let v: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap_or_else(|e| {
        panic!(
            "stdout is not JSON ({e}): {} / {}",
            String::from_utf8_lossy(&out.stdout),
            String::from_utf8_lossy(&out.stderr)
        )
    });
    (
        out.status.code().unwrap(),
        v,
        String::from_utf8_lossy(&out.stderr).to_string(),
    )
}

const ON: &str = "new:\n  check_open_prs: true\n";

fn notice(v: &serde_json::Value) -> Vec<&serde_json::Value> {
    v.get("notices")
        .and_then(|n| n.as_array())
        .into_iter()
        .flatten()
        .filter(|n| n["subject"] == "new.pull-refs")
        .collect()
}

#[test]
fn a_number_claimed_only_on_a_pull_ref_is_skipped_and_reported() {
    let w = world("skip", ON, "");
    publish_pull(
        &w,
        "refs/pull/7/head",
        &[(
            "docs/bug/BUG-2-other.md",
            bug(2, "01BBBBBBBBBBBBBBBBBBBBBBBB"),
        )],
    );
    let (code, v, stderr) = new_report(&w, &[]);
    assert_eq!(code, 0, "{v} {stderr}");
    assert_eq!(v["display_number"], 3, "{v} {stderr}");
    let p = &v["pull_refs"];
    assert_eq!(p["fetched"], true, "{v}");
    assert_eq!(p["remote"], "origin", "{v}");
    assert_eq!(p["refs"], 1, "{v}");
    assert!(p["reason"].is_null(), "{v}");
    let skipped = p["skipped_numbers"].as_array().unwrap();
    assert_eq!(skipped.len(), 1, "{v}");
    assert_eq!(skipped[0]["number"], 2, "{v}");
    assert!(
        skipped[0]["held_by"][0]
            .as_str()
            .unwrap()
            .contains("refs/urzua/pr/origin/7"),
        "{v}"
    );
    // The ref lives in the private namespace, and the lock is gone.
    assert!(git(&w.work, &["for-each-ref", "refs/urzua/pr"]).contains("refs/urzua/pr/origin/7"));
    assert!(!w.work.join(".git/urzua-new.lock").exists());
}

#[test]
fn with_the_option_off_no_remote_is_touched_and_the_report_says_disabled() {
    let w = world("off", "", "");
    publish_pull(
        &w,
        "refs/pull/7/head",
        &[(
            "docs/bug/BUG-2-other.md",
            bug(2, "01BBBBBBBBBBBBBBBBBBBBBBBB"),
        )],
    );
    let (code, v, _) = new_report(&w, &[]);
    assert_eq!(code, 0, "{v}");
    assert_eq!(v["display_number"], 2, "the pull ref was not fetched: {v}");
    assert_eq!(v["pull_refs"]["fetched"], false, "{v}");
    assert_eq!(v["pull_refs"]["reason"], "disabled", "{v}");
    assert_eq!(git(&w.work, &["for-each-ref", "refs/urzua"]), "");
}

#[test]
fn each_pattern_has_its_own_namespace_so_two_patterns_yielding_123_do_not_overwrite() {
    let w = world(
        "patterns",
        "new:\n  check_open_prs: true\n  pull_ref_patterns: [\"refs/pull/*/head\", \"refs/merge-requests/*/head\"]\n",
        "",
    );
    publish_pull(
        &w,
        "refs/pull/123/head",
        &[("docs/bug/BUG-5-a.md", bug(5, "01CCCCCCCCCCCCCCCCCCCCCCCC"))],
    );
    publish_pull(
        &w,
        "refs/merge-requests/123/head",
        &[("docs/bug/BUG-6-b.md", bug(6, "01DDDDDDDDDDDDDDDDDDDDDDDD"))],
    );
    let (code, v, stderr) = new_report(&w, &[]);
    assert_eq!(code, 0, "{v} {stderr}");
    assert_eq!(
        v["display_number"], 7,
        "both numbers are skipped: {v} {stderr}"
    );
    let a = git(&w.work, &["rev-parse", "refs/urzua/pr/0/123"]);
    let b = git(&w.work, &["rev-parse", "refs/urzua/pr/1/123"]);
    assert_ne!(a, b, "neither pattern overwrote the other");
}

#[test]
fn an_unreachable_or_missing_remote_is_a_notice_and_never_a_failure() {
    let w = world(
        "offline",
        "new:\n  check_open_prs: true\n  remote: nowhere\n",
        "",
    );
    let (code, v, _) = new_report(&w, &[]);
    assert_eq!(code, 0, "{v}");
    assert_eq!(v["display_number"], 2, "{v}");
    assert_eq!(v["pull_refs"]["fetched"], false, "{v}");
    assert_eq!(v["pull_refs"]["reason"], "offline", "{v}");
    let n = notice(&v);
    assert!(
        n.iter().any(|n| n["severity"] == "warning"
            && n["message"]
                .as_str()
                .unwrap()
                .contains("unfetched pull request")),
        "{v}"
    );

    // A remote that exists but cannot be reached.
    let w = world("unreachable", ON, "");
    git(
        &w.work,
        &["remote", "set-url", "origin", "/nonexistent/repo.git"],
    );
    let (code, v, _) = new_report(&w, &[]);
    assert_eq!(code, 0, "{v}");
    assert_eq!(v["pull_refs"]["reason"], "offline", "{v}");
}

#[test]
fn a_hung_fetch_is_killed_at_the_timeout_and_reported() {
    let w = world(
        "timeout",
        "new:\n  check_open_prs: true\n  fetch_timeout_seconds: 1\n",
        "",
    );
    git(&w.work, &["remote", "set-url", "origin", "ext::sleep 30"]);
    let started = std::time::Instant::now();
    let (code, v, _) = new_report(&w, &[]);
    assert!(
        started.elapsed().as_secs() < 20,
        "the hung fetch was killed"
    );
    assert_eq!(code, 0, "{v}");
    assert_eq!(v["pull_refs"]["reason"], "timeout", "{v}");
    assert_eq!(v["display_number"], 2, "{v}");
}

#[test]
fn refs_left_by_an_earlier_fetch_still_reserve_their_numbers_when_offline() {
    let w = world("earlier", ON, "");
    publish_pull(
        &w,
        "refs/pull/7/head",
        &[(
            "docs/bug/BUG-2-other.md",
            bug(2, "01BBBBBBBBBBBBBBBBBBBBBBBB"),
        )],
    );
    let (_, v, _) = new_report(&w, &[]);
    assert_eq!(v["display_number"], 3, "{v}");
    git(
        &w.work,
        &["remote", "set-url", "origin", "/nonexistent/repo.git"],
    );
    let (code, v, _) = new_report(&w, &[]);
    assert_eq!(code, 0, "{v}");
    assert_eq!(v["pull_refs"]["reason"], "offline", "{v}");
    // BUG-2 is held on the pull ref, BUG-3 is in the working tree now.
    assert_eq!(v["display_number"], 4, "{v}");
}

#[test]
fn a_number_in_a_sibling_worktree_untracked_is_skipped() {
    let w = world("worktree", ON, "");
    let sibling = w.work.parent().unwrap().join("sibling");
    git(
        &w.work,
        &[
            "worktree",
            "add",
            "-q",
            "-b",
            "side",
            sibling.to_str().unwrap(),
        ],
    );
    std::fs::write(
        sibling.join("docs/bug/BUG-2-drafting.md"),
        bug(2, "01EEEEEEEEEEEEEEEEEEEEEEEE"),
    )
    .unwrap();
    let (code, v, stderr) = new_report(&w, &[]);
    assert_eq!(code, 0, "{v} {stderr}");
    assert_eq!(v["display_number"], 3, "{v} {stderr}");
    assert_eq!(v["pull_refs"]["worktrees_scanned"], 1, "{v}");
    let held = v["pull_refs"]["skipped_numbers"][0]["held_by"][0]
        .as_str()
        .unwrap();
    assert!(held.contains("sibling"), "{v}");
}

#[test]
fn a_listed_worktree_whose_directory_is_missing_is_skipped_with_a_notice() {
    let w = world("worktree-gone", ON, "");
    let sibling = w.work.parent().unwrap().join("gone");
    git(
        &w.work,
        &[
            "worktree",
            "add",
            "-q",
            "-b",
            "side",
            sibling.to_str().unwrap(),
        ],
    );
    std::fs::remove_dir_all(&sibling).unwrap();
    let (code, v, _) = new_report(&w, &[]);
    assert_eq!(code, 0, "{v}");
    assert_eq!(v["display_number"], 2, "{v}");
    assert!(
        notice(&v)
            .iter()
            .any(|n| n["message"].as_str().unwrap().contains("gone")),
        "{v}"
    );
}

#[test]
fn the_report_states_that_separate_clones_are_not_seen() {
    let w = world("limit", ON, "");
    let (_, v, _) = new_report(&w, &[]);
    assert!(
        notice(&v).iter().any(|n| n["severity"] == "info"
            && n["message"].as_str().unwrap().contains("separate clone")),
        "{v}"
    );
}

#[test]
fn two_runs_started_together_take_two_different_numbers() {
    let w = world("concurrent", ON, "");
    let spawn = || {
        common::urzua()
            .args(["new", "bug", "Same time", "--by", "tester"])
            .current_dir(&w.work)
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .unwrap()
    };
    let (a, b) = (spawn(), spawn());
    let (a, b) = (a.wait_with_output().unwrap(), b.wait_with_output().unwrap());
    let number = |o: &std::process::Output| -> u64 {
        assert_eq!(
            o.status.code(),
            Some(0),
            "{}",
            String::from_utf8_lossy(&o.stderr)
        );
        let v: serde_json::Value = serde_json::from_slice(&o.stdout).unwrap();
        v["display_number"].as_u64().unwrap()
    };
    let mut numbers = vec![number(&a), number(&b)];
    numbers.sort();
    assert_eq!(numbers, vec![2, 3]);
}

#[test]
fn a_stale_lock_whose_pid_is_gone_is_removed() {
    let w = world("stale", ON, "");
    let mut dead = Command::new("true").spawn().unwrap();
    let pid = dead.id();
    dead.wait().unwrap();
    let lock = w.work.join(".git/urzua-new.lock");
    std::fs::write(&lock, format!("{pid}\n")).unwrap();
    let (code, v, _) = new_report(&w, &[]);
    assert_eq!(code, 0, "{v}");
    assert!(!lock.exists());
}

#[test]
fn a_held_lock_times_out_naming_its_path_and_holder_and_writes_nothing() {
    let w = world(
        "held",
        "new:\n  check_open_prs: true\n  lock_timeout_seconds: 1\n",
        "",
    );
    let lock = w.work.join(".git/urzua-new.lock");
    std::fs::write(&lock, format!("{}\n", std::process::id())).unwrap();
    let (code, v, _) = new_report(&w, &[]);
    assert_eq!(code, 2, "{v}");
    let text = v.to_string();
    assert!(text.contains("urzua-new.lock"), "{text}");
    assert!(text.contains(&std::process::id().to_string()), "{text}");
    assert!(lock.exists(), "a lock this run did not take is left alone");
    assert!(!w.work.join("docs/bug/BUG-2-fresh.md").exists());
}

#[test]
fn the_new_keys_are_validated_at_load() {
    for (block, needle) in [
        ("new:\n  check_open_prs: \"yes\"\n", "check_open_prs"),
        ("new:\n  nope: 1\n", "nope"),
        ("new:\n  pull_ref_patterns: []\n", "pull_ref_patterns"),
        (
            "new:\n  pull_ref_patterns: [\"refs/pull/head\"]\n",
            "pull_ref_patterns",
        ),
        (
            "new:\n  pull_ref_patterns: [\"heads/*\"]\n",
            "pull_ref_patterns",
        ),
        (
            "new:\n  fetch_timeout_seconds: 0\n",
            "fetch_timeout_seconds",
        ),
        ("new:\n  remote: \"-x\"\n", "remote"),
    ] {
        let w = world("validate", "", "");
        std::fs::write(w.work.join(".urzua/config.yaml"), config(block, "")).unwrap();
        let (code, v, _) = new_report(&w, &[]);
        assert_eq!(code, 2, "{block}: {v}");
        assert!(v.to_string().contains(needle), "{block}: {v}");
    }
}

// ---- identity.open-pr-collision ------------------------------------------------

const RULE: &str = "identity.open-pr-collision";

/// `work` on branch `mine` holding a BUG-2 with `mine_id`, the origin publishing a pull
/// request that holds a BUG-2 with `theirs_id`; the pull refs fetched by hand
/// (`check` never fetches).
fn collision_world(name: &str, mine_id: &str, theirs_id: &str, fetch: bool) -> World {
    let w = world(name, "", &format!("  {RULE}: warn\n"));
    publish_pull(
        &w,
        "refs/pull/9/head",
        &[("docs/bug/BUG-2-theirs.md", bug(2, theirs_id))],
    );
    if fetch {
        git(
            &w.work,
            &[
                "fetch",
                "-q",
                "--no-tags",
                "origin",
                "+refs/pull/*/head:refs/urzua/pr/origin/*",
            ],
        );
    }
    git(&w.work, &["checkout", "-q", "-b", "mine"]);
    std::fs::write(w.work.join("docs/bug/BUG-2-mine.md"), bug(2, mine_id)).unwrap();
    git(&w.work, &["add", "-A"]);
    git(&w.work, &["commit", "-q", "-m", "mine"]);
    w
}

fn check(w: &World, args: &[&str]) -> (i32, serde_json::Value) {
    let out = urzua_in(&w.work, &[&["check"], args].concat());
    (
        out.status.code().unwrap(),
        serde_json::from_slice(&out.stdout).expect("JSON"),
    )
}

fn rule_findings(v: &serde_json::Value) -> Vec<&serde_json::Value> {
    v["findings"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|f| f["rule"] == RULE)
        .collect()
}

fn execution(v: &serde_json::Value) -> &serde_json::Value {
    v["rules_executed"]
        .as_array()
        .unwrap()
        .iter()
        .find(|e| e["rule"] == RULE)
        .unwrap_or_else(|| panic!("{RULE} not executed: {v}"))
}

#[test]
fn two_pull_refs_holding_one_number_are_flagged_against_the_record_a_diff_adds() {
    // The simulated collision: this branch and a pull request each filed BUG-2.
    let w = collision_world(
        "collide",
        "01MINEMINEMINEMINEMINEMINE",
        "01THEIRSTHEIRSTHEIRSTHEIR",
        true,
    );
    let (code, v) = check(&w, &["--base", "main"]);
    let f = rule_findings(&v);
    assert_eq!(f.len(), 1, "{v}");
    assert_eq!(f[0]["severity"], "warning", "{v}");
    assert_eq!(f[0]["file"], "docs/bug/BUG-2-mine.md", "{v}");
    let message = f[0]["message"].as_str().unwrap();
    assert!(message.contains("refs/urzua/pr/origin/9"), "{message}");
    assert!(message.contains("docs/bug/BUG-2-theirs.md"), "{message}");
    assert_eq!(execution(&v)["status"], "ran", "{v}");
    assert_eq!(execution(&v)["population"]["eligible"], 1, "{v}");
    assert_eq!(code, 0, "a warning gates only under fail_on: warning");
}

#[test]
fn a_head_carrying_the_records_own_stable_id_is_the_same_record_not_a_collision() {
    let same = "01SAMESAMESAMESAMESAMESAME";
    let w = collision_world("same-id", same, same, true);
    let (_, v) = check(&w, &["--base", "main"]);
    assert!(rule_findings(&v).is_empty(), "{v}");
    assert_eq!(execution(&v)["status"], "ran", "{v}");
}

#[test]
fn without_base_the_rule_says_it_judges_a_diff() {
    let w = collision_world(
        "nobase",
        "01MINEMINEMINEMINEMINEMINE",
        "01THEIRSTHEIRSTHEIRSTHEIR",
        true,
    );
    let (_, v) = check(&w, &[]);
    assert!(rule_findings(&v).is_empty(), "{v}");
    assert_eq!(execution(&v)["status"], "not-applicable", "{v}");
}

#[test]
fn with_no_pull_refs_fetched_the_rule_notices_and_counts_zero_refs() {
    let w = collision_world(
        "norefs",
        "01MINEMINEMINEMINEMINEMINE",
        "01THEIRSTHEIRSTHEIRSTHEIR",
        false,
    );
    let (_, v) = check(&w, &["--base", "main"]);
    assert!(rule_findings(&v).is_empty(), "{v}");
    let e = execution(&v);
    assert_eq!(e["status"], "ran", "{v}");
    assert_eq!(e["population"]["unit"], "pull-ref", "{v}");
    assert_eq!(e["population"]["eligible"], 0, "{v}");
    assert_eq!(e["population"]["examined"], 0, "{v}");
    let notices: Vec<_> = v["notices"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|n| n["subject"] == RULE)
        .collect();
    assert_eq!(notices.len(), 1, "{v}");
    assert!(
        notices[0]["message"]
            .as_str()
            .unwrap()
            .contains("git fetch"),
        "{v}"
    );
    // And check did not fetch.
    assert_eq!(git(&w.work, &["for-each-ref", "refs/urzua"]), "");
}

#[test]
fn the_rule_is_off_unless_configured() {
    let w = collision_world(
        "off",
        "01MINEMINEMINEMINEMINEMINE",
        "01THEIRSTHEIRSTHEIRSTHEIR",
        true,
    );
    std::fs::write(w.work.join(".urzua/config.yaml"), config("", "")).unwrap();
    let (_, v) = check(&w, &["--base", "main"]);
    assert!(rule_findings(&v).is_empty(), "{v}");
    assert!(v["rules_executed"]
        .as_array()
        .unwrap()
        .iter()
        .all(|e| e["rule"] != RULE || e["status"] == "not-enabled"));
}
