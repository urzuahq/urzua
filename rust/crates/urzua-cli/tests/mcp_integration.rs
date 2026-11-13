//! `urzua mcp` (SPEC-26, ADR-99): a read-only MCP server over stdio, driven as a
//! real process with real git-repo fixtures. Read-only is proved here, not
//! asserted: no write tool exists, the working tree and `.git` hash the same
//! after every call, stdout carries only JSON-RPC frames, and a scan of the
//! server's sources finds no write API (a planted source shows it can fail).

mod common;

use std::collections::BTreeMap;
use std::hash::{DefaultHasher, Hash, Hasher};
use std::io::Write;
use std::path::Path;
use std::process::{Command, Stdio};

use serde_json::{json, Value};

const CONFIG: &str = "schema_version: 2\nrules: {header.required-fields: error, section.required: error}\nrecord_types:\n  adr:\n    dir: \"docs/adr\"\n    required_fields: [\"Status\", \"Date\"]\n    known_fields: [\"Superseded-by\", \"Supersedes\"]\n    pointer_fields: [\"Superseded-by\", \"Supersedes\"]\n    sections:\n      from: headings\n      required:\n        - {name: Context}\n";

fn git(dir: &Path, args: &[&str]) {
    let status = Command::new("git")
        .args(args)
        .current_dir(dir)
        .status()
        .expect("git should run");
    assert!(status.success(), "git {args:?} failed");
}

fn adr(status: &str, extra: &str, n: u32) -> String {
    format!("---\nStatus: {status}\nDate: 2026-01-01\n{extra}---\n# ADR-{n}: Decision {n}\n\n## Context\n\nreason {n}\n")
}

const EXPLAIN_CONFIG: &str = "schema_version: 2\nrules: {header.required-fields: error}\nrecord_types:\n  adr:\n    dir: \"docs/adr\"\n    required_fields: [\"Status\"]\n    known_fields: [\"Realized-by\"]\n";

/// One ADR whose `Realized-by` names `src/a.rs`, plus that file and an ungoverned one.
fn explain_fixture(name: &str) -> common::TempDir {
    let dir = common::TempDir::new(&format!("urzua-mcp-{name}"));
    for sub in ["docs/adr", ".urzua", "src"] {
        std::fs::create_dir_all(dir.join(sub)).unwrap();
    }
    git(&dir, &["init", "-q"]);
    git(&dir, &["config", "user.email", "test@example.com"]);
    git(&dir, &["config", "user.name", "test"]);
    std::fs::write(dir.join(".urzua/config.yaml"), EXPLAIN_CONFIG).unwrap();
    std::fs::write(dir.join("src/a.rs"), "fn a() {}\n").unwrap();
    std::fs::write(dir.join("src/b.rs"), "fn b() {}\n").unwrap();
    std::fs::write(
        dir.join("docs/adr/ADR-1-a.md"),
        "---\nStatus: Accepted\nRealized-by: code:src/a.rs\n---\n# ADR-1: A\n",
    )
    .unwrap();
    git(&dir, &["add", "-A"]);
    git(&dir, &["commit", "-q", "-m", "fixture"]);
    settle(&dir);
    dir
}

/// ADR-1 Accepted, ADR-2 Rejected, ADR-3 Superseded by ADR-4, ADR-5 broken (no Date).
fn fixture(name: &str) -> common::TempDir {
    let dir = common::TempDir::new(&format!("urzua-mcp-{name}"));
    std::fs::create_dir_all(dir.join("docs/adr")).unwrap();
    std::fs::create_dir_all(dir.join(".urzua")).unwrap();
    git(&dir, &["init", "-q"]);
    git(&dir, &["config", "user.email", "test@example.com"]);
    git(&dir, &["config", "user.name", "test"]);
    std::fs::write(dir.join(".urzua/config.yaml"), CONFIG).unwrap();
    let files = [
        ("ADR-1-a.md", adr("Accepted", "", 1)),
        ("ADR-2-b.md", adr("Rejected", "", 2)),
        ("ADR-3-c.md", adr("Superseded", "Superseded-by: ADR-4\n", 3)),
        ("ADR-4-d.md", adr("Accepted", "Supersedes: ADR-3\n", 4)),
        (
            "ADR-5-e.md",
            "---\nStatus: Proposed\n---\n# ADR-5: Decision 5\n\n## Context\n\nreason\n".to_string(),
        ),
    ];
    for (file, text) in files {
        std::fs::write(dir.join("docs/adr").join(file), text).unwrap();
    }
    git(&dir, &["add", "-A"]);
    git(&dir, &["commit", "-q", "-m", "fixture"]);
    settle(&dir);
    dir
}

/// Gives every working file an old modification time and refreshes git's index.
/// A file written in the same instant as the index is "racily clean", and any git
/// command that compares the working tree (`git diff`, which `check --base` runs)
/// then rewrites the index to record that it is not. That is git tidying its own
/// cache, not the server writing, but it would make a byte-for-byte hash of `.git`
/// depend on timing, so the fixture starts with nothing for git to tidy.
fn settle(dir: &Path) {
    fn age(dir: &Path) {
        let old = std::time::UNIX_EPOCH + std::time::Duration::from_secs(1_600_000_000);
        for entry in std::fs::read_dir(dir).unwrap() {
            let path = entry.unwrap().path();
            if path.file_name().is_some_and(|n| n == ".git") {
                continue;
            }
            if path.is_dir() {
                age(&path);
            } else {
                std::fs::File::options()
                    .write(true)
                    .open(&path)
                    .unwrap()
                    .set_modified(old)
                    .unwrap();
            }
        }
    }
    age(dir);
    git(dir, &["update-index", "-q", "--refresh"]);
}

/// A fingerprint of every file and directory under `dir`, `.git` included: path to content hash (a directory, which has no content, is `path/` to 0).
fn tree_hash(dir: &Path) -> BTreeMap<String, u64> {
    fn walk(dir: &Path, out: &mut BTreeMap<String, u64>, root: &Path) {
        for entry in std::fs::read_dir(dir).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                // A directory is an entry of its own, so an empty one is seen.
                let key = format!("{}/", path.strip_prefix(root).unwrap().display());
                out.insert(key, 0);
                walk(&path, out, root);
            } else {
                let key = path.strip_prefix(root).unwrap().display().to_string();
                let mut hasher = DefaultHasher::new();
                std::fs::read(&path).unwrap_or_default().hash(&mut hasher);
                out.insert(key, hasher.finish());
            }
        }
    }
    let mut files = BTreeMap::new();
    walk(dir, &mut files, dir);
    files
}

/// Fails naming every path that was added, removed or changed.
fn assert_tree_unchanged(before: &BTreeMap<String, u64>, dir: &Path, what: &str) {
    let after = tree_hash(dir);
    let changed: Vec<&String> = before
        .keys()
        .chain(after.keys())
        .filter(|k| before.get(*k) != after.get(*k))
        .collect();
    assert!(changed.is_empty(), "{what} changed the tree: {changed:?}");
}

/// Runs `urzua mcp` in `dir`, feeds it the frames, and returns the raw stdout
/// lines and the process output.
fn session(dir: &Path, frames: &[Value]) -> (Vec<Value>, std::process::Output) {
    let mut child = common::urzua()
        .args(["mcp"])
        .current_dir(dir)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("urzua binary should run");
    let mut stdin = child.stdin.take().unwrap();
    for frame in frames {
        writeln!(stdin, "{frame}").unwrap();
    }
    drop(stdin);
    let out = child.wait_with_output().unwrap();
    let replies = String::from_utf8(out.stdout.clone())
        .unwrap()
        .lines()
        .map(|line| {
            serde_json::from_str(line).unwrap_or_else(|e| panic!("not a frame ({e}): {line}"))
        })
        .collect();
    (replies, out)
}

fn call(id: u64, tool: &str, arguments: Value) -> Value {
    json!({"jsonrpc": "2.0", "id": id, "method": "tools/call", "params": {"name": tool, "arguments": arguments}})
}

/// The one reply to a single call.
fn call_one(dir: &Path, tool: &str, arguments: Value) -> Value {
    let (replies, _) = session(dir, &[call(1, tool, arguments)]);
    assert_eq!(replies.len(), 1, "one request, one reply: {replies:?}");
    replies.into_iter().next().unwrap()
}

fn ids(report: &Value) -> Vec<String> {
    report["records"]
        .as_array()
        .unwrap()
        .iter()
        .map(|r| r["id"].as_str().unwrap().to_string())
        .collect()
}

#[test]
fn initialize_and_tools_list_advertise_exactly_the_read_only_tools_built() {
    let dir = fixture("list");
    let (replies, _) = session(
        &dir,
        &[
            json!({"jsonrpc": "2.0", "id": 1, "method": "initialize", "params": {"protocolVersion": "2025-06-18"}}),
            json!({"jsonrpc": "2.0", "method": "notifications/initialized"}),
            json!({"jsonrpc": "2.0", "id": 2, "method": "tools/list"}),
        ],
    );
    assert_eq!(replies.len(), 2, "the notification is not answered");
    assert_eq!(replies[0]["result"]["protocolVersion"], "2025-06-18");
    assert_eq!(replies[0]["result"]["capabilities"], json!({"tools": {}}));
    let tools = replies[1]["result"]["tools"].as_array().unwrap();
    let names: Vec<&str> = tools.iter().map(|t| t["name"].as_str().unwrap()).collect();
    assert_eq!(
        names,
        [
            "urzua_check",
            "urzua_list_records",
            "urzua_show_record",
            "urzua_get_findings_for_path",
            "urzua_get_graph"
        ]
    );
    for tool in tools {
        assert_eq!(tool["annotations"]["readOnlyHint"], true);
        assert_eq!(tool["inputSchema"]["additionalProperties"], false);
    }
}

#[test]
fn tool_names_carry_a_read_word_and_no_write_word() {
    // The harness reads an MCP call as a write by the words in its tool name
    // (SPEC-19); the names are load-bearing, so a write word here is a bug.
    let dir = fixture("names");
    let (replies, _) = session(
        &dir,
        &[json!({"jsonrpc": "2.0", "id": 1, "method": "tools/list"})],
    );
    let write_words = [
        "write", "create", "set", "update", "move", "delete", "remove", "apply", "fix", "edit",
        "patch", "install",
    ];
    // The hook leaves a tool alone only when its name carries one of these (and no
    // write word); a tool without one is judged as an edit when it is given a path.
    let read_words = [
        "read", "get", "list", "search", "find", "stat", "view", "show", "cat", "fetch", "query",
        "describe", "info", "tree", "diff", "status", "log", "count", "check", "lookup", "inspect",
    ];
    for tool in replies[0]["result"]["tools"].as_array().unwrap() {
        let name = tool["name"].as_str().unwrap();
        let words: Vec<&str> = name.split('_').collect();
        for word in &words {
            assert!(
                !write_words.contains(word),
                "tool `{name}` has the write word `{word}`"
            );
        }
        assert!(
            words.iter().any(|w| read_words.contains(w)),
            "tool `{name}` has no read word, so the editor hook would judge it as an edit"
        );
    }
}

#[test]
fn list_records_serves_a_rejected_decision_and_a_superseded_one_with_its_chain() {
    let dir = fixture("rejected");
    let reply = call_one(
        &dir,
        "urzua_list_records",
        json!({"type": "adr", "status": ["Rejected", "Superseded"]}),
    );
    let result = &reply["result"];
    assert_eq!(result["isError"], false);
    let report = &result["structuredContent"];
    assert_eq!(ids(report), ["ADR-2", "ADR-3"]);
    let superseded = &report["records"][1];
    let header: BTreeMap<&str, &str> = superseded["header"]
        .as_array()
        .unwrap()
        .iter()
        .map(|f| (f["name"].as_str().unwrap(), f["value"].as_str().unwrap()))
        .collect();
    assert_eq!(header["Status"], "Superseded");
    assert_eq!(header["Superseded-by"], "ADR-4");
    for record in report["records"].as_array().unwrap() {
        for text in ["raw_text", "sections", "preamble"] {
            assert!(record.get(text).is_none(), "a listing carries no `{text}`");
        }
    }
}

#[test]
fn list_records_filters_generically_and_says_when_a_filter_matched_nothing() {
    let dir = fixture("filters");
    let all = call_one(&dir, "urzua_list_records", json!({}));
    assert_eq!(ids(&all["result"]["structuredContent"]).len(), 5);
    let none = call_one(
        &dir,
        "urzua_list_records",
        json!({"type": "nope", "status": ["Accepted"]}),
    );
    let report = &none["result"]["structuredContent"];
    assert_eq!(ids(report).len(), 0);
    let subjects: Vec<&str> = report["notices"]
        .as_array()
        .unwrap()
        .iter()
        .map(|n| n["subject"].as_str().unwrap())
        .collect();
    assert!(subjects.contains(&"list-records.type"), "{subjects:?}");
    let unseen = call_one(&dir, "urzua_list_records", json!({"status": ["Retired"]}));
    assert_eq!(
        unseen["result"]["structuredContent"]["notices"][0]["subject"],
        "list-records.status"
    );
}

#[test]
fn check_returns_the_cli_report_as_structured_content_and_identical_text() {
    let dir = fixture("check");
    let reply = call_one(&dir, "urzua_check", json!({}));
    let result = &reply["result"];
    assert_eq!(
        result["isError"], false,
        "findings are a report, not a failure"
    );
    let text: Value = serde_json::from_str(result["content"][0]["text"].as_str().unwrap()).unwrap();
    assert_eq!(text, result["structuredContent"]);
    let cli = common::urzua()
        .args(["check"])
        .current_dir(&dir)
        .output()
        .unwrap();
    let cli: Value = serde_json::from_slice(&cli.stdout).unwrap();
    for key in ["status", "files_examined", "findings", "rules_executed"] {
        assert_eq!(
            result["structuredContent"][key], cli[key],
            "`{key}` differs from `urzua check`"
        );
    }
    let rules: Vec<&str> = cli["findings"]
        .as_array()
        .unwrap()
        .iter()
        .map(|f| f["rule"].as_str().unwrap())
        .collect();
    assert!(
        rules.contains(&"header.required-fields"),
        "the broken ADR-5 is reported: {rules:?}"
    );
}

#[test]
fn check_narrows_by_rules_and_path_and_reports_a_could_not_run_as_an_error_result() {
    let dir = fixture("narrow");
    let narrowed = call_one(
        &dir,
        "urzua_check",
        json!({"rules": ["header.required-fields"], "paths": ["docs/adr/ADR-1-a.md"]}),
    );
    let report = &narrowed["result"]["structuredContent"];
    assert_eq!(report["rules_executed"].as_array().unwrap().len(), 1);
    assert_eq!(report["findings"].as_array().unwrap().len(), 0);
    let failed = call_one(&dir, "urzua_check", json!({"rules": ["not.a.rule"]}));
    assert_eq!(failed["result"]["isError"], true);
    assert_eq!(failed["result"]["structuredContent"]["status"], "not-run");
    let escaping = call_one(&dir, "urzua_check", json!({"paths": ["../outside"]}));
    assert_eq!(escaping["error"]["code"], -32602);
}

#[test]
fn a_write_attempt_is_refused_and_the_tree_is_unchanged() {
    let dir = fixture("write");
    let before = tree_hash(&dir);
    for tool in [
        "urzua_write_record",
        "urzua_fix_apply",
        "urzua_new",
        "urzua_index_apply",
    ] {
        let reply = call_one(
            &dir,
            tool,
            json!({"path": "docs/adr/ADR-9-x.md", "content": "x"}),
        );
        assert_eq!(
            reply["error"]["code"], -32602,
            "{tool} must be refused: {reply}"
        );
        assert!(reply.get("result").is_none());
    }
    // A declared tool is not given an argument it does not declare either.
    let reply = call_one(&dir, "urzua_list_records", json!({"apply": true}));
    assert_eq!(reply["error"]["code"], -32602);
    assert_tree_unchanged(&before, &dir, "a refused write attempt");
}

#[test]
fn the_working_tree_and_git_hash_the_same_after_every_tool() {
    let dir = fixture("hash");
    let table = [
        ("urzua_check", json!({})),
        ("urzua_check", json!({"base": "HEAD"})),
        ("urzua_check", json!({"rules": ["header.required-fields"]})),
        ("urzua_list_records", json!({})),
        (
            "urzua_list_records",
            json!({"type": "adr", "status": ["Rejected"]}),
        ),
    ];
    for (tool, arguments) in table {
        let before = tree_hash(&dir);
        let reply = call_one(&dir, tool, arguments.clone());
        assert!(reply.get("result").is_some(), "{tool} {arguments}: {reply}");
        assert_eq!(
            tree_hash(&dir),
            before,
            "{tool} {arguments} changed the tree"
        );
    }
}

#[test]
fn stdout_carries_only_json_rpc_frames_even_for_bad_input() {
    let dir = fixture("frames");
    let mut child = common::urzua()
        .args(["mcp"])
        .current_dir(&dir)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let mut stdin = child.stdin.take().unwrap();
    for line in [
        "not json",
        "[]",
        "{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"nope\"}",
        "",
        "{\"jsonrpc\":\"2.0\",\"id\":2,\"method\":\"ping\"}",
    ] {
        writeln!(stdin, "{line}").unwrap();
    }
    drop(stdin);
    let out = child.wait_with_output().unwrap();
    assert!(out.status.success());
    let lines: Vec<String> = String::from_utf8(out.stdout)
        .unwrap()
        .lines()
        .map(str::to_string)
        .collect();
    assert_eq!(lines.len(), 4);
    let codes: Vec<i64> = lines
        .iter()
        .map(|l| {
            let v: Value = serde_json::from_str(l).unwrap();
            assert_eq!(v["jsonrpc"], "2.0");
            v["error"]["code"].as_i64().unwrap_or(0)
        })
        .collect();
    assert_eq!(codes, [-32700, -32600, -32601, 0]);
}

#[test]
fn a_server_that_cannot_read_the_corpus_exits_2_with_json_on_stderr_and_nothing_on_stdout() {
    let dir = common::TempDir::new("urzua-mcp-noconfig");
    git(&dir, &["init", "-q"]);
    let (replies, out) = session(&dir, &[]);
    assert!(replies.is_empty());
    assert_eq!(out.status.code(), Some(2));
    let stderr: Value = serde_json::from_slice(&out.stderr).unwrap();
    assert_eq!(stderr["status"], "not-run");
}

#[test]
fn root_selects_the_repository_and_as_of_is_refused() {
    let dir = fixture("root");
    let elsewhere = common::TempDir::new("urzua-mcp-elsewhere");
    let mut child = common::urzua()
        .args(["mcp", "--root"])
        .arg(dir.path())
        .current_dir(&elsewhere)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();
    writeln!(
        child.stdin.take().unwrap(),
        "{}",
        call(1, "urzua_list_records", json!({}))
    )
    .unwrap();
    let out = child.wait_with_output().unwrap();
    let reply: Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(ids(&reply["result"]["structuredContent"]).len(), 5);
    let refused = common::urzua()
        .args(["--as-of", "2026-01-01", "mcp"])
        .current_dir(&dir)
        .output()
        .unwrap();
    assert_eq!(refused.status.code(), Some(2));
}

// ---------------------------------------------------------------------------
// The source scan: the server's code, and the CLI code its tools call, contain
// no write API.
// ---------------------------------------------------------------------------

/// APIs that write, rename, delete, change a file's metadata or spawn a process,
/// including the `OpenOptions` spellings (`File::options()` and its `write`,
/// `append`, `create`, `create_new` and `truncate` switches). The scan is textual
/// and deliberately broad: a legitimate need for one of these is a decision for
/// ADR-99's amendment, not an allowlist entry.
const FORBIDDEN: &[&str] = &[
    "fs::write",
    "fs::remove",
    "fs::rename",
    "fs::copy",
    "fs::create_dir",
    "fs::set_permissions",
    "fs::hard_link",
    "fs::soft_link",
    "File::create",
    "File::options",
    "OpenOptions",
    ".append(true)",
    ".write(true)",
    ".create(true)",
    ".create_new(true)",
    ".truncate(true)",
    "remove_file(",
    "remove_dir_all(",
    "remove_dir(",
    "create_dir_all(",
    "set_permissions(",
    "set_modified(",
    "set_times(",
    "set_len(",
    "symlink(",
    "write_atomic",
    "Command::new",
    "process::Command",
    ".write_all(",
];

/// `source` without its test code: every item that follows a `#[cfg(test)]`
/// attribute is removed (a `mod`'s braces to the matching one, any other item to
/// its `;` or its block), and so are `//` comments. Code after a test module is
/// kept, so a write placed after one is still seen.
fn production_code(source: &str) -> String {
    let mut rest = source;
    let mut kept = String::new();
    while let Some(at) = rest.find("#[cfg(test)]") {
        kept.push_str(&rest[..at]);
        let after = &rest[at + "#[cfg(test)]".len()..];
        let open = after.find('{');
        let semi = after.find(';');
        let skip = match (open, semi) {
            (Some(o), Some(s)) if s < o => s + 1,
            (Some(o), _) => {
                let mut depth = 0usize;
                let mut end = after.len();
                for (i, c) in after[o..].char_indices() {
                    match c {
                        '{' => depth += 1,
                        '}' => {
                            depth -= 1;
                            if depth == 0 {
                                end = o + i + 1;
                                break;
                            }
                        }
                        _ => {}
                    }
                }
                end
            }
            (None, Some(s)) => s + 1,
            (None, None) => after.len(),
        };
        rest = &after[skip..];
    }
    kept.push_str(rest);
    kept.lines()
        .filter(|line| !line.trim_start().starts_with("//"))
        .collect::<Vec<_>>()
        .join("\n")
}

/// The forbidden APIs in the production part of `source`.
fn forbidden_in(source: &str) -> Vec<&'static str> {
    let code = production_code(source);
    FORBIDDEN
        .iter()
        .copied()
        .filter(|api| code.contains(api))
        .collect()
}

/// The server's own sources, and the `urzua-cli` files its tools run through:
/// `check` (`commands/check.rs` and what it reads with), `export`, `explain` and `graph`. The
/// `urzua-io` and `urzua-core` crates are not scanned: `urzua-io` holds the
/// writers other commands use and runs `git`, so what the tools reach there is
/// covered by the tree-hash test instead (SPEC-26, "Read-only guarantee").
const SCANNED: &[&str] = &[
    "urzua-cli/src/commands/mcp.rs",
    "urzua-cli/src/commands/check.rs",
    "urzua-cli/src/commands/export.rs",
    "urzua-cli/src/commands/explain.rs",
    "urzua-cli/src/commands/graph.rs",
    "urzua-cli/src/session.rs",
    "urzua-cli/src/discovery.rs",
    "urzua-cli/src/file_texts.rs",
    "urzua-cli/src/diff_scope.rs",
];

fn server_sources() -> Vec<(String, String)> {
    let crates = Path::new(env!("CARGO_MANIFEST_DIR")).join("..");
    let mut files: Vec<_> = SCANNED.iter().map(|f| crates.join(f)).collect();
    for entry in std::fs::read_dir(crates.join("urzua-mcp/src")).unwrap() {
        files.push(entry.unwrap().path());
    }
    files
        .into_iter()
        .map(|p| {
            (
                p.display().to_string(),
                std::fs::read_to_string(&p).unwrap(),
            )
        })
        .collect()
}

#[test]
fn the_server_sources_and_the_code_its_tools_call_contain_no_write_api() {
    let sources = server_sources();
    assert!(
        sources.len() > SCANNED.len(),
        "the scan must see every file it names"
    );
    for (path, text) in &sources {
        assert_eq!(
            forbidden_in(text),
            Vec::<&str>::new(),
            "{path} uses a write API"
        );
    }
}

#[test]
fn the_source_scan_fails_on_a_planted_write() {
    // ADR-55: a scan that cannot fail proves nothing.
    assert_eq!(
        forbidden_in("fn f() { std::fs::write(\"a\", \"b\").unwrap(); }"),
        ["fs::write"]
    );
    assert_eq!(
        forbidden_in("let c = std::process::Command::new(\"git\");"),
        ["Command::new", "process::Command"]
    );
    assert_eq!(
        forbidden_in("// fs::write is mentioned here only\nfn ok() {}"),
        Vec::<&str>::new()
    );
}

#[test]
fn the_source_scan_catches_open_options_style_writes() {
    for planted in [
        "let f = std::fs::File::options().append(true).open(p);",
        "let f = std::fs::OpenOptions::new().write(true).open(p);",
        "let f = options.create_new(true).open(p);",
        "let f = options.truncate(true).open(p);",
        "use std::fs::remove_file; remove_file(p);",
        "urzua_io::write_atomic(p, t);",
    ] {
        assert!(!forbidden_in(planted).is_empty(), "missed: {planted}");
    }
}

#[test]
fn a_cfg_test_item_hides_only_itself_not_the_code_after_it() {
    let tests_only = "fn ok() {}\n#[cfg(test)]\nmod t { fn f() { std::fs::write(1, 2); } }";
    assert_eq!(forbidden_in(tests_only), Vec::<&str>::new());
    let then_production = "#[cfg(test)]\nmod t { fn f() { let x = { 1 }; std::fs::write(1, 2); } }\nfn later() { std::fs::remove_file(p); }";
    assert_eq!(
        forbidden_in(then_production),
        ["fs::remove", "remove_file("]
    );
    let one_item = "#[cfg(test)]\nuse std::fs::File;\nfn later() { File::create(p); }";
    assert_eq!(forbidden_in(one_item), ["File::create"]);
}

#[test]
#[should_panic(expected = "changed the tree")]
fn the_tree_hash_sees_a_new_empty_directory() {
    let dir = fixture("emptydir");
    let before = tree_hash(&dir);
    std::fs::create_dir(dir.join("docs/empty")).unwrap();
    assert_tree_unchanged(&before, &dir, "a planted empty directory");
}

#[test]
fn a_bad_or_oversized_line_over_the_real_process_gets_an_error_and_the_server_keeps_serving() {
    let dir = fixture("badlines");
    let mut child = common::urzua()
        .args(["mcp"])
        .current_dir(&dir)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();
    let mut stdin = child.stdin.take().unwrap();
    stdin
        .write_all(b"{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"ping\",\"x\":\"\xff\"}\n")
        .unwrap();
    stdin.write_all(&vec![b'a'; 2 * 1024 * 1024]).unwrap();
    stdin
        .write_all(b"\n{\"jsonrpc\":\"2.0\",\"id\":3,\"method\":\"ping\"}\n")
        .unwrap();
    drop(stdin);
    let out = child.wait_with_output().unwrap();
    let replies: Vec<Value> = String::from_utf8(out.stdout)
        .unwrap()
        .lines()
        .map(|l| serde_json::from_str(l).unwrap())
        .collect();
    let codes: Vec<i64> = replies
        .iter()
        .map(|r| r["error"]["code"].as_i64().unwrap_or(0))
        .collect();
    assert_eq!(codes, [-32700, -32600, 0]);
    assert_eq!(replies[2]["id"], 3);
}

#[test]
fn show_record_returns_the_record_with_its_sections_text_and_supersession_edges() {
    let dir = fixture("show");
    let reply = call_one(&dir, "urzua_show_record", json!({"id": "ADR-3"}));
    let result = &reply["result"];
    assert_eq!(result["isError"], false);
    let report = &result["structuredContent"];
    let text: Value = serde_json::from_str(result["content"][0]["text"].as_str().unwrap()).unwrap();
    assert_eq!(&text, report);
    let record = &report["record"];
    assert_eq!(record["id"], "ADR-3");
    assert!(record["raw_text"].as_str().unwrap().contains("reason 3"));
    assert_eq!(record["sections"][0]["name"], "Context");
    let edges = report["edges"].as_array().unwrap();
    assert!(
        edges
            .iter()
            .any(|e| e["from"] == "ADR-3" && e["to"] == "ADR-4"),
        "ADR-3 is superseded by ADR-4: {edges:?}"
    );
    assert!(edges
        .iter()
        .all(|e| e["from"] == "ADR-3" || e["to"] == "ADR-3"));
}

#[test]
fn show_record_of_an_unknown_id_is_an_error_result_not_an_empty_record() {
    let dir = fixture("showunknown");
    let reply = call_one(&dir, "urzua_show_record", json!({"id": "ADR-999"}));
    assert_eq!(reply["result"]["isError"], true);
    let report = &reply["result"]["structuredContent"];
    assert_eq!(report["status"], "not-run");
    assert!(report["error"].as_str().unwrap().contains("ADR-999"));
    assert!(report.get("record").is_none());
}

#[test]
fn findings_for_path_returns_the_records_that_govern_it() {
    let dir = explain_fixture("explain");
    let reply = call_one(
        &dir,
        "urzua_get_findings_for_path",
        json!({"path": "src/a.rs"}),
    );
    let report = &reply["result"]["structuredContent"];
    assert_eq!(reply["result"]["isError"], false, "{reply}");
    assert_eq!(report["path"], "src/a.rs");
    assert_eq!(report["governing_records"].as_array().unwrap().len(), 1);
    let cli = common::urzua()
        .args(["explain", "src/a.rs"])
        .current_dir(&dir)
        .output()
        .unwrap();
    let cli: Value = serde_json::from_slice(&cli.stdout).unwrap();
    assert_eq!(report["governing_records"], cli["governing_records"]);
    let ungoverned = call_one(
        &dir,
        "urzua_get_findings_for_path",
        json!({"path": "src/b.rs"}),
    );
    assert_eq!(
        ungoverned["result"]["structuredContent"]["governing_records"],
        json!([])
    );
    let escaping = call_one(&dir, "urzua_get_findings_for_path", json!({"path": "../x"}));
    assert_eq!(escaping["error"]["code"], -32602);
}

#[test]
fn graph_returns_the_cli_graph_report() {
    let dir = fixture("graph");
    let reply = call_one(&dir, "urzua_get_graph", json!({}));
    let report = &reply["result"]["structuredContent"];
    assert_eq!(reply["result"]["isError"], false);
    let cli = common::urzua()
        .args(["graph"])
        .current_dir(&dir)
        .output()
        .unwrap();
    let cli: Value = serde_json::from_slice(&cli.stdout).unwrap();
    assert_eq!(report["edges"], cli["edges"]);
    assert_eq!(report["population"], cli["population"]);
    assert!(!report["edges"].as_array().unwrap().is_empty());
    let extra = call_one(&dir, "urzua_get_graph", json!({"id": "ADR-1"}));
    assert_eq!(extra["error"]["code"], -32602);
}
