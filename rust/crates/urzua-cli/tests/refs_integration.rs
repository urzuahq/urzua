//! `citation.issue-exists` and `urzua refs refresh` (MILE-143) end to end: the
//! real binary against an isolated git fixture. `check` is proved to stay
//! offline; `refresh` is exercised against a loopback server that replays the
//! recorded GitHub responses in `tests/fixtures/github`, never the network.

mod common;

use std::io::{Read, Write};
use std::net::TcpListener;
use std::path::Path;
use std::process::Command;
use std::sync::{Arc, Mutex};

fn git(dir: &Path, args: &[&str]) {
    let status = Command::new("git")
        .args(args)
        .current_dir(dir)
        .status()
        .expect("git should run");
    assert!(status.success(), "git {args:?} failed");
}

const RULE: &str = "{ citation.issue-exists: { level: warn, reference_sections: [\"References\"], skip_revision_log: true, issue_refs: { snapshot: \".urzua/known-refs.json\", repository: \"urzuahq/urzua\", forms: [number, url] } } }";

fn fixture(name: &str, rules: &str, body: &str, snapshot: Option<&str>) -> common::TempDir {
    let dir = common::TempDir::new(&format!("urzua-refs-{name}"));
    std::fs::create_dir_all(dir.join("docs/adr")).unwrap();
    std::fs::create_dir_all(dir.join(".urzua")).unwrap();
    git(&dir, &["init", "-q"]);
    git(&dir, &["config", "user.email", "test@example.com"]);
    git(&dir, &["config", "user.name", "test"]);
    std::fs::write(
        dir.join(".urzua/config.yaml"),
        format!(
            "schema_version: 2\nrules: {rules}\n\nrecord_types:\n  adr:\n    dir: \"docs/adr\"\n    prefix: \"ADR\"\n    required_fields: [\"Status\"]\n"
        ),
    )
    .unwrap();
    std::fs::write(dir.join("docs/adr/ADR-1-x.md"), body).unwrap();
    if let Some(snapshot) = snapshot {
        std::fs::write(dir.join(".urzua/known-refs.json"), snapshot).unwrap();
    }
    git(&dir, &["add", "-A"]);
    git(&dir, &["commit", "-q", "-m", "fixture"]);
    dir
}

fn urzua(dir: &Path, args: &[&str], envs: &[(&str, &str)]) -> (i32, serde_json::Value, String) {
    let mut command = common::urzua();
    command
        .args(args)
        .current_dir(dir)
        .env_remove("GITHUB_TOKEN")
        .env_remove("GH_TOKEN")
        .env_remove("URZUA_GITHUB_API");
    for (k, v) in envs {
        command.env(k, v);
    }
    let output = command.output().expect("urzua binary should run");
    let parsed = serde_json::from_slice(&output.stdout).unwrap_or_else(|e| {
        panic!(
            "must print JSON ({e}): {} / {}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        )
    });
    (
        output.status.code().unwrap_or(-1),
        parsed,
        String::from_utf8_lossy(&output.stderr).into_owned(),
    )
}

fn findings_of<'a>(report: &'a serde_json::Value, id: &str) -> Vec<&'a serde_json::Value> {
    report["findings"]
        .as_array()
        .expect("findings")
        .iter()
        .filter(|f| f["rule"] == id)
        .collect()
}

const SNAPSHOT: &str = "{\n  \"schema_version\": 1,\n  \"repository\": \"urzuahq/urzua\",\n  \"generated_at\": \"2026-11-04\",\n  \"issues\": [3],\n  \"pulls\": [7, 12]\n}\n";
const BODY: &str = "---\nStatus: Accepted\n---\n# 1 — X\n\n## References\n\n- #7, #12 and #3, but not #99\n- https://github.com/urzuahq/urzua/pull/12 and https://github.com/urzuahq/urzua/issues/500\n\n> **Revision log**\n>\n> | Date | Change | Class |\n> |---|---|---|\n> | 2026-01-01 | Closed #4000. | **substantive** |\n";

#[test]
fn a_cited_number_missing_from_the_snapshot_warns_and_the_notice_names_its_date() {
    let dir = fixture("missing", RULE, BODY, Some(SNAPSHOT));
    let (code, report, _) = urzua(&dir, &["check", "docs/"], &[]);
    let found = findings_of(&report, "citation.issue-exists");
    let messages: Vec<&str> = found
        .iter()
        .map(|f| f["message"].as_str().unwrap())
        .collect();
    assert_eq!(messages.len(), 2, "{report}");
    assert!(messages[0].contains("#99"), "{messages:?}");
    assert!(messages[1].contains("/issues/500"), "{messages:?}");
    assert!(messages.iter().all(|m| m.contains("2026-11-04")));
    assert_eq!(found[0]["severity"], "warning");
    assert_eq!(
        code, 0,
        "a warning does not fail the default gate: {report}"
    );
    let notices = report["notices"].to_string();
    assert!(
        notices.contains("generated 2026-11-04") && notices.contains("1 issues, 2 pull requests"),
        "{notices}"
    );
    let pop = report["rules_executed"]
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["rule"] == "citation.issue-exists")
        .unwrap()["population"]
        .clone();
    assert_eq!(pop["unit"], "citation");
    assert_eq!(pop["examined"], 6);
}

#[test]
fn a_missing_or_unreadable_snapshot_is_a_finding_never_a_pass() {
    let absent = fixture("absent", RULE, BODY, None);
    let (code, report, _) = urzua(&absent, &["check", "docs/"], &[]);
    let found = findings_of(&report, "citation.issue-exists");
    assert_eq!(found.len(), 1, "{report}");
    assert_eq!(found[0]["file"], ".urzua/known-refs.json");
    assert!(found[0]["message"]
        .as_str()
        .unwrap()
        .contains("does not exist"));
    assert_eq!(code, 0, "warn level: {report}");
    assert_eq!(report["status"], "findings-present");

    for (name, content, needle) in [
        ("garbage", "not json", "not a valid snapshot"),
        (
            "elsewhere",
            "{\"schema_version\":1,\"repository\":\"a/b\",\"generated_at\":\"2026-01-01\",\"issues\":[],\"pulls\":[]}",
            "describes 'a/b'",
        ),
    ] {
        let dir = fixture(name, RULE, BODY, Some(content));
        let (_, report, _) = urzua(&dir, &["check", "docs/"], &[]);
        let found = findings_of(&report, "citation.issue-exists");
        assert_eq!(found.len(), 1, "{name}: {report}");
        assert!(
            found[0]["message"].as_str().unwrap().contains(needle),
            "{name}: {report}"
        );
    }
}

#[test]
fn check_never_reaches_the_network() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    listener.set_nonblocking(true).unwrap();
    let base = format!("http://{}", listener.local_addr().unwrap());
    let dir = fixture("offline", RULE, BODY, Some(SNAPSHOT));
    let (_, report, _) = urzua(
        &dir,
        &["check", "docs/"],
        &[("URZUA_GITHUB_API", &base), ("GITHUB_TOKEN", "tok")],
    );
    assert_eq!(findings_of(&report, "citation.issue-exists").len(), 2);
    assert!(
        listener.accept().is_err(),
        "check opened a connection to the API"
    );
}

#[test]
fn the_rule_is_opt_in() {
    let dir = fixture("optin", "{}", BODY, None);
    let (_, report, _) = urzua(&dir, &["check", "docs/"], &[]);
    assert!(findings_of(&report, "citation.issue-exists").is_empty());
    assert!(!report["notices"].to_string().contains("known-refs"));
}

/// Replays `responses` in order, one per connection, substituting `{{BASE}}`,
/// and records each request's head.
fn serve(responses: Vec<String>) -> (String, Arc<Mutex<Vec<String>>>) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let base = format!("http://{}", listener.local_addr().unwrap());
    let seen = Arc::new(Mutex::new(Vec::new()));
    let (b, s) = (base.clone(), seen.clone());
    std::thread::spawn(move || {
        for raw in responses {
            let Ok((mut stream, _)) = listener.accept() else {
                return;
            };
            let mut head = Vec::new();
            let mut buf = [0u8; 1024];
            while !head.windows(4).any(|w| w == b"\r\n\r\n") {
                match stream.read(&mut buf) {
                    Ok(0) | Err(_) => break,
                    Ok(n) => head.extend_from_slice(&buf[..n]),
                }
            }
            s.lock()
                .unwrap()
                .push(String::from_utf8_lossy(&head).into_owned());
            let _ = stream.write_all(raw.replace("{{BASE}}", &b).as_bytes());
        }
    });
    (base, seen)
}

fn recorded(name: &str) -> String {
    std::fs::read_to_string(
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/github")
            .join(name),
    )
    .unwrap()
}

#[test]
fn refresh_pages_through_the_recorded_listing_and_writes_a_sorted_snapshot() {
    let dir = fixture("refresh", RULE, BODY, None);
    let (base, seen) = serve(
        [
            "issues-page1.http",
            "issues-page2.http",
            "issues-page3.http",
        ]
        .iter()
        .map(|n| recorded(n))
        .collect(),
    );
    let (code, report, _) = urzua(
        &dir,
        &["refs", "refresh"],
        &[
            ("URZUA_GITHUB_API", &base),
            ("GITHUB_TOKEN", "secret-token"),
        ],
    );
    assert_eq!(code, 0, "{report}");
    assert_eq!(report["status"], "ok");
    assert_eq!(report["outcome"], "created");
    assert_eq!(
        (report["issues"].as_u64(), report["pulls"].as_u64()),
        (Some(1), Some(8))
    );
    assert_eq!(report["newest"], 224);
    assert!(report["notices"].is_null(), "a token was set: {report}");

    let written = std::fs::read_to_string(dir.join(".urzua/known-refs.json")).unwrap();
    let snapshot: serde_json::Value = serde_json::from_str(&written).unwrap();
    assert_eq!(snapshot["repository"], "urzuahq/urzua");
    assert_eq!(snapshot["issues"], serde_json::json!([217]));
    assert_eq!(
        snapshot["pulls"],
        serde_json::json!([216, 218, 219, 220, 221, 222, 223, 224])
    );
    assert!(written.ends_with("}\n"));

    let requests = seen.lock().unwrap();
    assert_eq!(requests.len(), 3);
    assert!(requests[0].contains("/repos/urzuahq/urzua/issues?state=all&per_page=100&page=1"));
    assert!(requests[2].contains("page=3"));
    assert!(requests.iter().all(|r| r
        .to_ascii_lowercase()
        .contains("authorization: bearer secret-token")));
    drop(requests);

    // The refreshed snapshot now answers `check`: 224 and below are known.
    let cites =
        "---\nStatus: Accepted\n---\n# 1 — X\n\n## References\n\n- #224 and #217 and #225\n";
    std::fs::write(dir.join("docs/adr/ADR-1-x.md"), cites).unwrap();
    git(&dir, &["add", "-A"]);
    git(&dir, &["commit", "-q", "-m", "cite"]);
    let (_, check, _) = urzua(&dir, &["check", "docs/"], &[]);
    let found = findings_of(&check, "citation.issue-exists");
    assert_eq!(found.len(), 1, "{check}");
    assert!(found[0]["message"].as_str().unwrap().contains("#225"));
}

#[test]
fn a_second_refresh_with_the_same_answers_leaves_the_file_current() {
    let dir = fixture("again", RULE, BODY, None);
    let pages = || {
        [
            "issues-page1.http",
            "issues-page2.http",
            "issues-page3.http",
        ]
        .iter()
        .map(|n| recorded(n))
        .collect::<Vec<_>>()
    };
    let (base, _) = serve(pages());
    urzua(&dir, &["refs", "refresh"], &[("URZUA_GITHUB_API", &base)]);
    let first = std::fs::read(dir.join(".urzua/known-refs.json")).unwrap();
    let (base, seen) = serve(pages());
    let (_, report, _) = urzua(&dir, &["refs", "refresh"], &[("URZUA_GITHUB_API", &base)]);
    assert_eq!(report["outcome"], "current", "{report}");
    assert_eq!(
        first,
        std::fs::read(dir.join(".urzua/known-refs.json")).unwrap()
    );
    assert!(
        !seen.lock().unwrap()[0]
            .to_ascii_lowercase()
            .contains("authorization"),
        "no token was set"
    );
    assert!(report["notices"].to_string().contains("unauthenticated"));
}

#[test]
fn a_rate_limited_refresh_exits_2_and_leaves_the_existing_snapshot_untouched() {
    let dir = fixture("limited", RULE, BODY, Some(SNAPSHOT));
    let (base, _) = serve(vec![
        recorded("issues-page1.http"),
        recorded("rate-limited.http"),
    ]);
    let (code, report, _) = urzua(&dir, &["refs", "refresh"], &[("URZUA_GITHUB_API", &base)]);
    assert_eq!(code, 2, "{report}");
    assert_eq!(report["status"], "not-run");
    assert!(report["error"].as_str().unwrap().contains("rate limit"));
    assert_eq!(
        std::fs::read_to_string(dir.join(".urzua/known-refs.json")).unwrap(),
        SNAPSHOT
    );
    assert!(!dir.join(".urzua/known-refs.json.tmp").exists());
}

#[test]
fn refresh_without_a_network_fails_clearly_and_writes_nothing() {
    let dir = fixture("offline-refresh", RULE, BODY, None);
    // A port nothing listens on.
    let closed = {
        let l = TcpListener::bind("127.0.0.1:0").unwrap();
        format!("http://{}", l.local_addr().unwrap())
    };
    let (code, report, _) = urzua(&dir, &["refs", "refresh"], &[("URZUA_GITHUB_API", &closed)]);
    assert_eq!(code, 2, "{report}");
    assert!(report["error"]
        .as_str()
        .unwrap()
        .contains("could not reach"));
    assert!(!dir.join(".urzua/known-refs.json").exists());
}

#[test]
fn refresh_needs_the_rule_declared() {
    let dir = fixture("undeclared", "{}", BODY, None);
    let (code, report, _) = urzua(&dir, &["refs", "refresh"], &[]);
    assert_eq!(code, 2, "{report}");
    assert!(report["error"].as_str().unwrap().contains("issue_refs"));
}

#[test]
fn a_declared_header_field_is_checked_end_to_end_and_only_when_declared() {
    let body =
        "---\nStatus: Accepted\nFound-in: 'PR #99 review'\n---\n# 1\n\n## References\n\n- none\n";
    let with_fields = RULE.replace(
        "forms: [number, url] }",
        "forms: [number, url], fields: [Status, Found-in] }",
    );
    let dir = fixture("fieldcheck", &with_fields, body, Some(SNAPSHOT));
    // Found-in is not declared by the fixture's type: refused at load.
    let (code, report, _) = urzua(&dir, &["check", "docs/"], &[]);
    assert_ne!(code, 0, "{report}");
    assert!(report.to_string().contains("Found-in"), "{report}");

    let cfg = std::fs::read_to_string(dir.join(".urzua/config.yaml")).unwrap();
    std::fs::write(
        dir.join(".urzua/config.yaml"),
        cfg.replace(
            "required_fields: [\"Status\"]",
            "required_fields: [\"Status\", \"Found-in\"]",
        ),
    )
    .unwrap();
    let (_, report, _) = urzua(&dir, &["check", "docs/"], &[]);
    let found = findings_of(&report, "citation.issue-exists");
    assert_eq!(found.len(), 1, "{report}");
    assert!(found[0]["message"].as_str().unwrap().contains("Found-in"));
    assert_eq!(found[0]["line"], 3);

    // Default off: the same record with no `fields` is clean.
    let cfg = std::fs::read_to_string(dir.join(".urzua/config.yaml")).unwrap();
    std::fs::write(
        dir.join(".urzua/config.yaml"),
        cfg.replace(", fields: [Status, Found-in]", ""),
    )
    .unwrap();
    let (_, report, _) = urzua(&dir, &["check", "docs/"], &[]);
    assert!(
        findings_of(&report, "citation.issue-exists").is_empty(),
        "{report}"
    );
}

#[test]
fn the_snapshot_notice_is_said_only_when_the_citation_rule_is_selected_observed_failing() {
    let rules = RULE.replacen("{ ", "{ header.required-fields: warn, ", 1);
    let dir = fixture("notice-filter", &rules, BODY, Some(SNAPSHOT));
    let (_, other, _) = urzua(&dir, &["check", "--rules", "header.required-fields"], &[]);
    assert!(
        !other["notices"].to_string().contains("known-refs"),
        "{other}"
    );
    let (_, own, _) = urzua(&dir, &["check", "--rules", "citation.issue-exists"], &[]);
    assert!(own["notices"].to_string().contains("known-refs"), "{own}");
    let (_, all, _) = urzua(&dir, &["check"], &[]);
    assert!(all["notices"].to_string().contains("known-refs"), "{all}");
}
