//! `git` is started in one place: `urzua-io`'s runner (`urzua_io::git`). A second
//! production site that builds its own `Command::new("git")` would word spawn
//! failures and non-zero exits its own way and wire its own pipes, which is the
//! duplication the runner removed.

use std::path::{Path, PathBuf};

fn rust_files(dir: &Path, out: &mut Vec<PathBuf>) {
    for entry in std::fs::read_dir(dir).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() {
            rust_files(&path, out);
        } else if path.extension().is_some_and(|e| e == "rs") {
            out.push(path);
        }
    }
}

/// Every production source file that starts a `git` process, as paths relative
/// to the crates directory. Whitespace is ignored so a call split across lines
/// is still found.
fn files_spawning_git() -> Vec<String> {
    let crates = Path::new(env!("CARGO_MANIFEST_DIR")).join("..");
    let mut files = Vec::new();
    for krate in std::fs::read_dir(&crates).unwrap() {
        let src = krate.unwrap().path().join("src");
        if src.is_dir() {
            rust_files(&src, &mut files);
        }
    }
    let mut hits = Vec::new();
    for file in files {
        let text = std::fs::read_to_string(&file).unwrap();
        // Production code only: a `#[cfg(test)]` module runs to the end of the
        // file in this codebase.
        let production = text.split("#[cfg(test)]").next().unwrap();
        let code: String = production
            .lines()
            .map(|l| l.split("//").next().unwrap())
            .flat_map(|l| l.chars().filter(|c| !c.is_whitespace()))
            .collect();
        if code.contains(r#"Command::new("git")"#) {
            let rel = file.strip_prefix(&crates).unwrap_or(&file);
            hits.push(rel.display().to_string().replace('\\', "/"));
        }
    }
    hits
}

#[test]
fn git_is_spawned_in_one_place() {
    let hits = files_spawning_git();
    assert_eq!(
        hits,
        vec!["urzua-io/src/git.rs".to_string()],
        "production files that start git themselves; use urzua_io::git::GitRun: {hits:?}"
    );
}
