//! The records gate on the tree a release leaves behind.
//!
//! `knope prepare-release` deletes every `.changeset/*.md` fragment. A record whose
//! evidence names one reports `embodiment.locator-exists` (and a Done milestone,
//! `status.delivery-agreement`) on the release pull request, where the only repair is
//! an amendment of `main`. So this repository's own corpus is checked here with the
//! fragments already deleted, and the pull request that cites a fragment fails first.

mod common;

use std::path::Path;
use std::process::Command;

/// A throwaway checkout of this repository's `HEAD`, with the uncommitted
/// changes laid over it, removed (worktree registration included) when dropped.
struct Checkout {
    root: std::path::PathBuf,
    dir: common::TempDir,
}

impl Drop for Checkout {
    fn drop(&mut self) {
        let _ = Command::new("git")
            .args(["worktree", "remove", "--force"])
            .arg(self.dir.path())
            .current_dir(&self.root)
            .output();
    }
}

fn git_out(dir: &Path, args: &[&str]) -> Vec<u8> {
    let out = Command::new("git")
        .args(args)
        .current_dir(dir)
        .output()
        .expect("git should run");
    assert!(out.status.success(), "git {args:?} failed: {out:?}");
    out.stdout
}

fn checkout_of_head() -> Checkout {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../..");
    let dir = common::TempDir::new("urzua-release-tree");
    common::git(
        &root,
        &["worktree", "add", "--detach", "--quiet"]
            .iter()
            .chain(&[dir.to_str().expect("utf-8 temp path"), "HEAD"])
            .copied()
            .collect::<Vec<_>>(),
    );
    let checkout = Checkout {
        root: root.clone(),
        dir,
    };
    overlay_uncommitted(&root, checkout.dir.path());
    checkout
}

/// What a developer has not committed yet is part of the tree being judged.
fn overlay_uncommitted(root: &Path, dir: &Path) {
    let patch = git_out(root, &["diff", "HEAD", "--binary"]);
    if !patch.is_empty() {
        let mut apply = Command::new("git")
            .args(["apply", "--whitespace=nowarn", "-"])
            .current_dir(dir)
            .stdin(std::process::Stdio::piped())
            .spawn()
            .expect("git apply should run");
        std::io::Write::write_all(&mut apply.stdin.take().unwrap(), &patch).unwrap();
        assert!(
            apply.wait().unwrap().success(),
            "the uncommitted diff applies"
        );
    }
    let untracked = git_out(root, &["ls-files", "-o", "--exclude-standard", "-z"]);
    for name in untracked.split(|b| *b == 0).filter(|n| !n.is_empty()) {
        let name = String::from_utf8(name.to_vec()).expect("utf-8 path");
        let to = dir.join(&name);
        std::fs::create_dir_all(to.parent().unwrap()).unwrap();
        std::fs::copy(root.join(&name), to).unwrap();
    }
}

/// Deletes the fragments as the release does (every markdown file of the
/// directory but its README) and returns how many it removed.
fn consume_fragments(dir: &Path) -> usize {
    let mut removed = 0;
    for entry in std::fs::read_dir(dir.join(".changeset")).expect(".changeset exists") {
        let path = entry.unwrap().path();
        let is_fragment = path.extension().is_some_and(|e| e == "md")
            && path.file_name().is_some_and(|n| n != "README.md");
        if is_fragment {
            std::fs::remove_file(&path).unwrap();
            removed += 1;
        }
    }
    removed
}

#[test]
fn this_repositorys_records_are_clean_once_the_release_has_consumed_the_changesets() {
    let checkout = checkout_of_head();
    let tree = checkout.dir.path();
    assert!(
        tree.join(".changeset/.gitkeep").exists(),
        "the changeset directory is kept by the release"
    );

    // On the release pull request itself none are left, which is the same tree.
    let removed = consume_fragments(tree);
    let out = common::urzua()
        .args(["--as-of", common::OWN_CORPUS_AS_OF, "check"])
        .current_dir(tree)
        .output()
        .expect("urzua binary should run");
    let report: serde_json::Value = serde_json::from_slice(&out.stdout)
        .unwrap_or_else(|e| panic!("stdout is not JSON ({e}): {out:?}"));
    let findings: Vec<String> = report["findings"]
        .as_array()
        .map(|all| {
            all.iter()
                .filter(|f| f["severity"] == "error" || f["severity"] == "warning")
                .map(|f| format!("{} {} {}", f["rule"], f["file"], f["message"]))
                .collect()
        })
        .unwrap_or_default();
    assert_eq!(
        out.status.code(),
        Some(0),
        "after the release deletes its {removed} changeset fragments the gate reports \
         (a record must not cite a `.changeset/` fragment as evidence; waive the Done item \
         as a process-outcome instead):\n{}",
        findings.join("\n")
    );
}
