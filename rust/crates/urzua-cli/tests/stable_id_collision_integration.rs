//! `identity.collision` also reports two records that carry the same
//! `Stable-Id` (external review, item 1.8): the rename pairing and the diff gate
//! match a record by it, so it must name one record. Real git fixtures and the
//! real binary; all output is JSON.

mod common;

use common::{commit_all, git};
use std::path::Path;

const CONFIG: &str = "schema_version: 2\nrules:\n  identity.collision: error\nrecord_types:\n  adr:\n    dir: docs/adr\n    prefix: ADR\n    header_shape: yaml-frontmatter\n    required_fields: [Status]\n  rfc:\n    dir: docs/rfc\n    prefix: RFC\n    header_shape: yaml-frontmatter\n    required_fields: [Status]\n";
const SID_A: &str = "01AAAAAAAAAAAAAAAAAAAAAAAA";
const SID_B: &str = "01BBBBBBBBBBBBBBBBBBBBBBBB";

fn record(n: u32, stable_id: &str) -> String {
    format!("---\nStable-Id: {stable_id}\nStatus: Accepted\n---\n# {n}: T\n")
}

fn fixture(name: &str, records: &[(&str, String)]) -> common::TempDir {
    let dir = common::TempDir::new(&format!("urzua-sid-{name}"));
    for d in ["docs/adr", "docs/rfc", ".urzua"] {
        std::fs::create_dir_all(dir.join(d)).unwrap();
    }
    git(&dir, &["init", "-q"]);
    git(&dir, &["config", "user.email", "test@example.com"]);
    git(&dir, &["config", "user.name", "test"]);
    std::fs::write(dir.join(".urzua/config.yaml"), CONFIG).unwrap();
    for (path, text) in records {
        std::fs::write(dir.join(path), text).unwrap();
    }
    commit_all(&dir, 0);
    dir
}

fn check(dir: &Path) -> serde_json::Value {
    let out = common::urzua()
        .arg("check")
        .current_dir(dir)
        .output()
        .unwrap();
    serde_json::from_slice(&out.stdout).unwrap_or_else(|e| panic!("json: {e}: {out:?}"))
}

fn collisions(report: &serde_json::Value) -> Vec<&serde_json::Value> {
    report["findings"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|f| f["rule"] == "identity.collision")
        .collect()
}

#[test]
fn two_records_sharing_a_stable_id_are_each_reported_naming_the_other() {
    let dir = fixture(
        "dup",
        &[
            ("docs/adr/ADR-1-a.md", record(1, SID_A)),
            ("docs/adr/ADR-2-b.md", record(2, SID_A)),
            ("docs/adr/ADR-3-c.md", record(3, SID_B)),
        ],
    );
    let report = check(&dir);
    let found = collisions(&report);
    assert_eq!(found.len(), 2, "{report}");
    let files: Vec<&str> = found.iter().map(|f| f["file"].as_str().unwrap()).collect();
    assert!(files.contains(&"docs/adr/ADR-1-a.md") && files.contains(&"docs/adr/ADR-2-b.md"));
    let message = found[0]["message"].as_str().unwrap();
    assert!(
        message.contains("Stable-Id") && message.contains(SID_A),
        "{message}"
    );
    assert!(message.contains("docs/adr/ADR-1-a.md") && message.contains("docs/adr/ADR-2-b.md"));
    assert_eq!(report["blocking"], true, "{report}");
}

#[test]
fn a_stable_id_shared_across_two_record_types_is_reported() {
    let dir = fixture(
        "cross-type",
        &[
            ("docs/adr/ADR-1-a.md", record(1, SID_A)),
            ("docs/rfc/RFC-1-a.md", record(1, SID_A)),
        ],
    );
    assert_eq!(collisions(&check(&dir)).len(), 2);
}

#[test]
fn distinct_stable_ids_and_records_with_none_are_not_reported() {
    let dir = fixture(
        "distinct",
        &[
            ("docs/adr/ADR-1-a.md", record(1, SID_A)),
            ("docs/adr/ADR-2-b.md", record(2, SID_B)),
            (
                "docs/adr/ADR-3-c.md",
                "---\nStatus: Accepted\n---\n# 3: T\n".to_string(),
            ),
            (
                "docs/adr/ADR-4-d.md",
                "---\nStatus: Accepted\n---\n# 4: T\n".to_string(),
            ),
        ],
    );
    assert!(collisions(&check(&dir)).is_empty());
}
