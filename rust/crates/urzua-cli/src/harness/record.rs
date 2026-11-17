//! `urzua harness record`: the shared steps of the generated drafting and
//! lifecycle commands, in the binary so nothing executable is installed in the
//! repository.
//!
//! - `new <type> <title>`: scaffold through `urzua new`, snapshot the scaffold.
//! - `finish <record>`: refuse a change to anything the scaffold decided, stage,
//!   regenerate indexes, run `urzua check`.
//! - `amend-finish <record>`: an amendment only adds; stage, run `urzua check`.
//! - `repin <record> <reason>`, `reverify <record> <reason>`: only with a reason.
//! - `done <record>`: `urzua fix --apply`, the engine's computed value.
//!
//! It holds no schema knowledge: a record's shape comes from `urzua new` and the
//! configuration; the verdict comes from `urzua check`.

use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode, Stdio};

const P: &str = "urzua harness record";

fn fail(code: u8, message: &str) -> ExitCode {
    eprintln!("{P}: {message}");
    ExitCode::from(code)
}

/// A run of urzua: (exit code, stdout). `URZUA_BIN` (or `URZUA`) names another
/// build, as the drafting helper script honoured; otherwise this binary.
fn urzua(args: &[&str]) -> Result<(i32, String), String> {
    let named = ["URZUA_BIN", "URZUA"]
        .iter()
        .find_map(|v| std::env::var_os(v).filter(|b| !b.is_empty()));
    let exe = match named {
        Some(b) => PathBuf::from(b),
        None => std::env::current_exe().map_err(|e| e.to_string())?,
    };
    let out = Command::new(exe)
        .args(args)
        .stderr(Stdio::inherit())
        .output()
        .map_err(|e| e.to_string())?;
    Ok((
        out.status.code().unwrap_or(1),
        String::from_utf8_lossy(&out.stdout).trim_end_matches('\n').to_string(),
    ))
}

fn git(args: &[&str]) -> Option<String> {
    let out = Command::new("git").args(args).stderr(Stdio::null()).output().ok()?;
    out.status
        .success()
        .then(|| String::from_utf8_lossy(&out.stdout).trim_end_matches('\n').to_string())
}

fn git_add(path: &str) -> Result<(), String> {
    git(&["add", "--", path])
        .map(|_| ())
        .ok_or_else(|| format!("git add {path} failed"))
}

/// Every value of `key` in a JSON document, depth first.
fn values_of(doc: &serde_json::Value, key: &str, out: &mut Vec<serde_json::Value>) {
    match doc {
        serde_json::Value::Object(o) => {
            for (k, v) in o {
                if k == key {
                    out.push(v.clone());
                }
                values_of(v, key, out);
            }
        }
        serde_json::Value::Array(a) => a.iter().for_each(|v| values_of(v, key, out)),
        _ => {}
    }
}

fn first_of(text: &str, key: &str) -> Option<serde_json::Value> {
    let doc: serde_json::Value = serde_json::from_str(text).ok()?;
    let mut found = Vec::new();
    values_of(&doc, key, &mut found);
    found.into_iter().next()
}

/// The header lines of a record: between a first line `---` and the next `---`.
fn header(text: &str) -> Vec<&str> {
    let mut lines = text.lines();
    if lines.next() != Some("---") {
        return Vec::new();
    }
    lines.take_while(|l| *l != "---").collect()
}

fn snapshots(root: &Path) -> PathBuf {
    root.join(".urzua/cache/draft")
}

fn scaffold_of(root: &Path, path: &str) -> PathBuf {
    let name = Path::new(path).file_name().unwrap_or_default().to_string_lossy();
    snapshots(root).join(format!("{name}.scaffold"))
}

fn identity() -> Option<String> {
    std::env::var("URZUA_BY")
        .ok()
        .filter(|b| !b.is_empty())
        .or_else(|| git(&["config", "user.name"]).filter(|b| !b.is_empty()))
}

/// `urzua check <path>`, printed; `Err` when it examined no record.
fn check_record(path: &str, done: &str) -> Result<ExitCode, ExitCode> {
    let (rc, report) = match urzua(&["check", path]) {
        Ok(r) => r,
        Err(e) => return Err(fail(2, &format!("could not run urzua check: {e}"))),
    };
    println!("{report}");
    let examined = first_of(&report, "files_examined")
        .and_then(|v| v.as_u64())
        .unwrap_or(0);
    if examined < 1 {
        return Err(fail(1, &format!("urzua check examined no records; {path} is NOT verified")));
    }
    eprintln!("{P}: {done}; urzua check examined {examined} record(s)");
    Ok(ExitCode::from(u8::try_from(rc).unwrap_or(1)))
}

fn new(root: &Path, ty: &str, title: &str) -> ExitCode {
    let (rc, out) = match urzua(&["new", ty, title]) {
        Ok(r) => r,
        Err(e) => return fail(2, &format!("could not run urzua new: {e}")),
    };
    println!("{out}");
    if rc != 0 {
        return ExitCode::from(u8::try_from(rc).unwrap_or(1));
    }
    let Some(path) = first_of(&out, "path").and_then(|p| p.as_str().map(str::to_string)) else {
        return fail(1, "urzua new reported no record path");
    };
    if !Path::new(&path).is_file() {
        return fail(1, "urzua new reported no record path");
    }
    let copied = std::fs::create_dir_all(snapshots(root))
        .and_then(|()| std::fs::copy(&path, scaffold_of(root, &path)));
    match copied {
        Ok(_) => ExitCode::SUCCESS,
        Err(e) => fail(1, &format!("could not snapshot the scaffold of {path}: {e}")),
    }
}

/// The header problems of `now` against the scaffold: a field renamed, added,
/// removed or moved, or a value the scaffold set (anything but `null`) changed.
fn header_problems(scaffold: &str, now: &str) -> Vec<String> {
    let (old, new) = (header(scaffold), header(now));
    let key = |l: &str| l.split(": ").next().unwrap_or("").to_string();
    let value = |l: &str| match l.find(':') {
        Some(at) => l[at + 1..].trim_start_matches(' ').to_string(),
        None => l.to_string(),
    };
    let mut problems = Vec::new();
    for i in 0..old.len().max(new.len()) {
        let (o, n) = (old.get(i).copied().unwrap_or(""), new.get(i).copied().unwrap_or(""));
        if key(o) != key(n) {
            problems.push(format!("header field {} became {}", key(o), key(n)));
        } else if value(n) != value(o) && value(o) != "null" {
            problems.push(format!(
                "scaffolded value of {} was changed ({} -> {})",
                key(o),
                value(o),
                value(n)
            ));
        }
    }
    if problems.is_empty() && old.len() != new.len() {
        problems.push("the header's field count differs from the scaffold".to_string());
    }
    problems
}

fn finish(root: &Path, path: &str) -> ExitCode {
    let scaffold = scaffold_of(root, path);
    let Ok(now) = std::fs::read_to_string(path) else {
        return fail(2, &format!("{path} does not exist"));
    };
    let Ok(before) = std::fs::read_to_string(&scaffold) else {
        return fail(2, &format!("{path} was not scaffolded by '{P} new'"));
    };
    // Never decide: only a field the scaffold left null may change. A scaffolded
    // state, date or id is a decision or an identity, not content.
    let problems = header_problems(&before, &now);
    if !problems.is_empty() {
        for p in &problems {
            eprintln!("{p}");
        }
        return fail(1, "refusing; a drafting command fills null fields and the body only");
    }
    if let Err(e) = git_add(path) {
        return fail(1, &e);
    }
    let (rc, idx) = match urzua(&["index", "--apply"]) {
        Ok(r) => r,
        Err(e) => return fail(2, &format!("could not run urzua index: {e}")),
    };
    if rc != 0 {
        return ExitCode::from(u8::try_from(rc).unwrap_or(1));
    }
    if let Ok(doc) = serde_json::from_str::<serde_json::Value>(&idx) {
        let mut paths = Vec::new();
        values_of(&doc, "path", &mut paths);
        for p in paths.iter().filter_map(|p| p.as_str()) {
            if Path::new(p).is_file() {
                let _ = git_add(p);
            }
        }
    }
    match check_record(path, &format!("wrote {path}")) {
        Ok(code) => {
            let _ = std::fs::remove_file(&scaffold);
            code
        }
        Err(code) => code,
    }
}

/// The problems of an amendment: every removed line is a change to existing
/// text, and it adds a dated amendment section and a revision-log row.
fn amendment_problems(diff: &str, path: &str) -> Vec<String> {
    let mut in_hunk = false;
    let mut removed = 0;
    for line in diff.lines() {
        if line.starts_with("@@") {
            in_hunk = true;
        } else if in_hunk && line.starts_with('-') {
            removed += 1;
        }
    }
    if removed != 0 {
        return vec![format!("refusing; an amendment only adds lines, and {removed} existing line(s) of {path} were changed or removed")];
    }
    if !diff.lines().any(|l| l.starts_with("+## Amendment (")) {
        return vec![format!("refusing; no added '## Amendment (YYYY-MM-DD): ...' section in {path}")];
    }
    if !diff.lines().any(|l| l.starts_with("+> |")) {
        return vec![format!("refusing; no added revision-log row in {path}")];
    }
    Vec::new()
}

fn amend_finish(path: &str) -> ExitCode {
    if !Path::new(path).is_file() {
        return fail(2, &format!("{path} does not exist"));
    }
    let Some(diff) = git(&["diff", "-U0", "HEAD", "--", path]) else {
        return fail(2, &format!("git diff of {path} failed"));
    };
    if let Some(problem) = amendment_problems(&diff, path).first() {
        return fail(1, problem);
    }
    if let Err(e) = git_add(path) {
        return fail(1, &e);
    }
    check_record(path, &format!("amended {path}")).unwrap_or_else(|c| c)
}

fn needs_reason(reason: &str, what: &str) -> Option<ExitCode> {
    reason.trim().is_empty().then(|| fail(1, &format!("refusing; {what}")))
}

fn repin(path: &str, reason: &str) -> ExitCode {
    if let Some(c) = needs_reason(reason, "a re-pin needs a non-empty reason that says what actually changed in the pinned code, not that the pin was stale") {
        return c;
    }
    let Some(by) = identity() else {
        return fail(2, "no identity; set URZUA_BY to the person who authorised the re-pin");
    };
    eprintln!("{P}: the stale pins of {path} (look at the diff of each pinned file before going on):");
    if let Ok((_, review)) = urzua(&["fix", "--rebaseline", "--ids", path]) {
        eprintln!("{review}");
    }
    let (rc, out) = match urzua(&["fix", "--rebaseline", "--apply", "--ids", path, "--by", &by, "--actor", "agent", "--reason", reason]) {
        Ok(r) => r,
        Err(e) => return fail(2, &format!("could not run urzua fix: {e}")),
    };
    println!("{out}");
    if !out.contains("\"status\": \"applied\"") {
        return fail(1, &format!("nothing was re-pinned for {path}"));
    }
    if rc != 0 {
        return ExitCode::from(u8::try_from(rc).unwrap_or(1));
    }
    if let Err(e) = git_add(path) {
        return fail(1, &e);
    }
    eprintln!("Re-pin: {path} -- {reason}");
    eprintln!("{P}: list the line above in the pull request body for the reviewer");
    ExitCode::SUCCESS
}

fn reverify(path: &str, reason: &str) -> ExitCode {
    if let Some(c) = needs_reason(reason, "a renewal needs a non-empty reason that says what you checked, not that the claim was old") {
        return c;
    }
    let Some(by) = identity() else {
        return fail(2, "no identity; set URZUA_BY to the person who authorised the renewal");
    };
    eprintln!("{P}: the review of {path} (read every locator and span it lists before going on):");
    let review = urzua(&["reverify", "--ids", path]).map(|r| r.1).unwrap_or_default();
    eprintln!("{review}");
    let (rc, out) = match urzua(&["reverify", "--apply", "--ids", path, "--by", &by, "--actor", "agent", "--reason", reason]) {
        Ok(r) => r,
        Err(e) => return fail(2, &format!("could not run urzua reverify: {e}")),
    };
    println!("{out}");
    if !out.contains("\"status\": \"applied\"") {
        return fail(1, &format!("nothing was renewed for {path}"));
    }
    if rc != 0 {
        return ExitCode::from(u8::try_from(rc).unwrap_or(1));
    }
    if let Err(e) = git_add(path) {
        return fail(1, &e);
    }
    if review.contains("\"check\": \"will-pin\"") {
        eprintln!("Pinned: {path} -- {reason}");
    }
    eprintln!("Re-verified: {path} -- {reason}");
    eprintln!("{P}: list every Pinned and Re-verified line above in the pull request body for the reviewer");
    ExitCode::SUCCESS
}

fn done(path: &str) -> ExitCode {
    let Some(by) = identity() else {
        return fail(2, "no identity; set URZUA_BY to the person who authorised this");
    };
    let (rc, out) = match urzua(&["fix", "--apply", "--ids", path, "--by", &by, "--actor", "agent"]) {
        Ok(r) => r,
        Err(e) => return fail(2, &format!("could not run urzua fix: {e}")),
    };
    println!("{out}");
    if !out.contains("\"computed_value\"") {
        return fail(1, &format!("the engine computed no value for {path} from its evidence, so nothing was set (add the evidence first, on each item of the record's completion criteria that the change delivered; the notices above name any item still unresolved; this command never writes a value itself)"));
    }
    if rc != 0 {
        return ExitCode::from(u8::try_from(rc).unwrap_or(1));
    }
    if let Err(e) = git_add(path) {
        return fail(1, &e);
    }
    check_record(path, &format!("{path} updated by the engine")).unwrap_or_else(|c| c)
}

/// `step` and its arguments, run from the repository's top directory.
pub fn run_record(step: &str, args: &[String]) -> ExitCode {
    let Some(root) = git(&["rev-parse", "--show-toplevel"]).map(PathBuf::from) else {
        return fail(2, "not inside a git repository");
    };
    if std::env::set_current_dir(&root).is_err() {
        return fail(2, &format!("cannot enter {}", root.display()));
    }
    let a: Vec<&str> = args.iter().map(String::as_str).collect();
    match (step, a.as_slice()) {
        ("new", [ty, title]) => new(&root, ty, title),
        ("finish", [path]) => finish(&root, path),
        ("amend-finish", [path]) => amend_finish(path),
        ("repin", [path, rest @ ..]) if rest.len() <= 1 => repin(path, rest.first().unwrap_or(&"")),
        ("reverify", [path, rest @ ..]) if rest.len() <= 1 => reverify(path, rest.first().unwrap_or(&"")),
        ("done", [path]) => done(path),
        _ => fail(2, "usage: urzua harness record new <type> <title> | finish <record> | amend-finish <record> | repin <record> <reason> | reverify <record> <reason> | done <record>"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SCAFFOLD: &str = "---\nStatus: Draft\nOwner: null\nDate: 2026-11-17\n---\n# x\n";

    #[test]
    fn only_a_null_field_may_be_filled() {
        assert!(header_problems(SCAFFOLD, &SCAFFOLD.replace("Owner: null", "Owner: ann")).is_empty());
        let changed = header_problems(SCAFFOLD, &SCAFFOLD.replace("Draft", "Accepted"));
        assert_eq!(changed, ["scaffolded value of Status was changed (Draft -> Accepted)"]);
        let added = header_problems(SCAFFOLD, &SCAFFOLD.replace("Date:", "Extra: 1\nDate:"));
        assert!(added[0].starts_with("header field Date became Extra"), "{added:?}");
        let removed = header_problems(SCAFFOLD, &SCAFFOLD.replace("Owner: null\n", ""));
        assert!(!removed.is_empty());
    }

    #[test]
    fn an_amendment_only_adds() {
        let ok = "--- a/x\n+++ b/x\n@@ -9,0 +10,3 @@\n+## Amendment (2026-11-17): y\n+> | 2026-11-17 | z |\n";
        assert!(amendment_problems(ok, "x").is_empty());
        let removes = format!("{ok}@@ -3 +3 @@\n-old\n+new\n");
        assert!(amendment_problems(&removes, "x")[0].contains("1 existing line"));
        assert!(amendment_problems("@@ -1,0 +2 @@\n+> | r |\n", "x")[0].contains("no added '## Amendment"));
        assert!(amendment_problems("@@ -1,0 +2 @@\n+## Amendment (d): t\n", "x")[0].contains("revision-log row"));
    }
}
