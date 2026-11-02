//! SPEC-4's corpus acceptance suite (MILE-101): discovers case files under
//! `tests/acceptance/corpora/<corpus-id>/expectations/*.yaml`, materializes
//! each case's corpus as an isolated git repo, runs the real `urzua` binary
//! and compares the result with the case's declared expectation.
//!
//! A custom harness (`harness = false`), so the report is always printed.
//!
//! Private corpora: `URZUA_ACCEPTANCE_EXTRA` is a `:`-separated list of
//! directories. Each is either one corpus (has `expectations/`) or a root of
//! corpus directories. Unset, the runner says so and skips them.
//!
//! `known_gap: {ref, reason}` marks a case whose expectation is the CORRECT
//! behaviour that does not hold yet. The case is still run: failing is the
//! expected outcome (reported KNOWN-GAP); passing is a failure of the suite,
//! because the gap closed and the marker must go. A gap can never go stale.

use serde::Deserialize;
use std::path::{Path, PathBuf};
use std::process::Command;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Case {
    id: String,
    /// A corpus directory name, or `@repo` for this repository (in place).
    corpus: String,
    #[serde(default)]
    setup: Vec<Vec<String>>,
    invocation: Vec<String>,
    expect_exit: i32,
    #[serde(default)]
    expect_stdout_contains: Vec<String>,
    #[serde(default)]
    expect_json: Vec<JsonCheck>,
    #[serde(default)]
    expect_findings: Vec<Matcher>,
    #[serde(default)]
    expect_no_findings: Vec<Matcher>,
    known_gap: Option<KnownGap>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct KnownGap {
    #[serde(rename = "ref")]
    reference: String,
    reason: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct JsonCheck {
    at: String,
    equals: Option<serde_json::Value>,
    gt: Option<f64>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Matcher {
    rule: Option<String>,
    file: Option<String>,
    severity: Option<String>,
}

#[derive(Deserialize)]
struct Manifest {
    #[serde(default)]
    fixtures: Vec<Fixture>,
    #[serde(default)]
    in_place: bool,
}

#[derive(Deserialize)]
struct Fixture {
    path: String,
    irregularity: String,
}

enum Outcome {
    Pass,
    KnownGap(String),
    Fail(Vec<String>),
}

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../..")
}

fn git(dir: &Path, args: &[&str]) -> Result<(), String> {
    let out = Command::new("git")
        .args(args)
        .current_dir(dir)
        .output()
        .map_err(|e| format!("git {args:?}: {e}"))?;
    if out.status.success() {
        Ok(())
    } else {
        Err(format!(
            "git {args:?}: {}",
            String::from_utf8_lossy(&out.stderr)
        ))
    }
}

/// Files under `dir` (relative, sorted), skipping `expectations/` and
/// `manifest.yaml` at the corpus root.
fn corpus_files(dir: &Path) -> Vec<String> {
    fn walk(base: &Path, dir: &Path, out: &mut Vec<String>) {
        for e in std::fs::read_dir(dir).unwrap().flatten() {
            let p = e.path();
            let rel = p
                .strip_prefix(base)
                .unwrap()
                .to_string_lossy()
                .replace('\\', "/");
            if rel == "expectations" || rel == "manifest.yaml" {
                continue;
            }
            if p.is_dir() {
                walk(base, &p, out);
            } else {
                out.push(rel);
            }
        }
    }
    let mut out = Vec::new();
    walk(dir, dir, &mut out);
    out.sort();
    out
}

fn copy_corpus(src: &Path, dest: &Path) -> Result<(), String> {
    for rel in corpus_files(src) {
        let to = dest.join(&rel);
        std::fs::create_dir_all(to.parent().unwrap()).map_err(|e| e.to_string())?;
        std::fs::copy(src.join(&rel), &to).map_err(|e| e.to_string())?;
    }
    Ok(())
}

/// The manifest must account for every fixture file and name no phantom one,
/// so "what irregularity is this built to exhibit" cannot silently rot.
fn check_manifest(corpus_dir: &Path) -> Vec<String> {
    let mut problems = Vec::new();
    let text = match std::fs::read_to_string(corpus_dir.join("manifest.yaml")) {
        Ok(t) => t,
        Err(e) => return vec![format!("manifest.yaml unreadable: {e}")],
    };
    let manifest: Manifest = match yaml_serde::from_str(&text) {
        Ok(m) => m,
        Err(e) => return vec![format!("manifest.yaml invalid: {e}")],
    };
    if manifest.in_place {
        return problems;
    }
    let on_disk = corpus_files(corpus_dir);
    for f in &manifest.fixtures {
        if f.irregularity.trim().is_empty() {
            problems.push(format!("manifest: '{}' has an empty irregularity", f.path));
        }
        if !on_disk.contains(&f.path) {
            problems.push(format!("manifest names '{}' which does not exist", f.path));
        }
    }
    for p in &on_disk {
        if !manifest.fixtures.iter().any(|f| &f.path == p) {
            problems.push(format!("fixture '{p}' is not described in manifest.yaml"));
        }
    }
    problems
}

fn matches(f: &serde_json::Value, m: &Matcher) -> bool {
    let field = |k: &str| f.get(k).and_then(|v| v.as_str()).unwrap_or("");
    m.rule.as_deref().is_none_or(|r| field("rule") == r)
        && m.severity.as_deref().is_none_or(|s| field("severity") == s)
        && m.file.as_deref().is_none_or(|p| field("file").ends_with(p))
}

/// Run one case; returns the list of unmet expectations (empty = all met).
fn evaluate(case: &Case, corpus_dir: &Path) -> Result<Vec<String>, String> {
    let (cwd, temp) = if case.corpus == "@repo" {
        (repo_root(), None)
    } else {
        let t = std::env::temp_dir().join(format!(
            "urzua-acceptance-{}-{}",
            std::process::id(),
            case.id
        ));
        let _ = std::fs::remove_dir_all(&t);
        std::fs::create_dir_all(&t).map_err(|e| e.to_string())?;
        copy_corpus(corpus_dir, &t)?;
        git(&t, &["init", "-q"])?;
        git(&t, &["config", "user.email", "acceptance@example.com"])?;
        git(&t, &["config", "user.name", "acceptance"])?;
        git(&t, &["add", "-A"])?;
        git(&t, &["commit", "-q", "-m", "fixture"])?;
        (t.clone(), Some(t))
    };
    let run = |args: &[String]| {
        Command::new(env!("CARGO_BIN_EXE_urzua"))
            .args(args)
            .current_dir(&cwd)
            .output()
            .map_err(|e| e.to_string())
    };
    for step in &case.setup {
        let out = run(step)?;
        if !out.status.success() {
            return Err(format!("setup {step:?} exited {:?}", out.status.code()));
        }
    }
    let out = run(&case.invocation)?;
    if let Some(t) = temp {
        let _ = std::fs::remove_dir_all(t);
    }

    let stdout = String::from_utf8_lossy(&out.stdout).to_string();
    let mut unmet = Vec::new();
    let code = out.status.code().unwrap_or(-1);
    if code != case.expect_exit {
        unmet.push(format!("exit {code}, expected {}", case.expect_exit));
    }
    for s in &case.expect_stdout_contains {
        if !stdout.contains(s) {
            unmet.push(format!("stdout lacks {s:?}"));
        }
    }
    let json: Option<serde_json::Value> = serde_json::from_str(&stdout).ok();
    let needs_json = !case.expect_json.is_empty()
        || !case.expect_findings.is_empty()
        || !case.expect_no_findings.is_empty();
    if needs_json && json.is_none() {
        unmet.push("stdout is not JSON".to_string());
    }
    if let Some(j) = &json {
        for c in &case.expect_json {
            match j.pointer(&c.at) {
                None => unmet.push(format!("{} absent from output", c.at)),
                Some(v) => {
                    if let Some(eq) = &c.equals {
                        if v != eq {
                            unmet.push(format!("{} is {v}, expected {eq}", c.at));
                        }
                    }
                    if let Some(gt) = c.gt {
                        if v.as_f64().is_none_or(|n| n <= gt) {
                            unmet.push(format!("{} is {v}, expected > {gt}", c.at));
                        }
                    }
                }
            }
        }
        let findings = j
            .get("findings")
            .and_then(|f| f.as_array())
            .cloned()
            .unwrap_or_default();
        for m in &case.expect_findings {
            if !findings.iter().any(|f| matches(f, m)) {
                unmet.push(format!(
                    "no finding matching rule={:?} file={:?} severity={:?}",
                    m.rule, m.file, m.severity
                ));
            }
        }
        for m in &case.expect_no_findings {
            if let Some(f) = findings.iter().find(|f| matches(f, m)) {
                unmet.push(format!("unexpected finding: {}", f));
            }
        }
    }
    Ok(unmet)
}

fn run_case(case: &Case, corpus_dir: &Path) -> Outcome {
    let unmet = match evaluate(case, corpus_dir) {
        Ok(u) => u,
        Err(e) => return Outcome::Fail(vec![format!("harness error: {e}")]),
    };
    match (&case.known_gap, unmet.is_empty()) {
        (None, true) => Outcome::Pass,
        (None, false) => Outcome::Fail(unmet),
        (Some(g), false) if !g.reason.trim().is_empty() && !g.reference.trim().is_empty() => {
            Outcome::KnownGap(format!("{} -- {}", g.reference, unmet.join("; ")))
        }
        (Some(_), false) => Outcome::Fail(vec!["known_gap needs a ref and a reason".into()]),
        (Some(g), true) => Outcome::Fail(vec![format!(
            "known gap {} has closed: the case now passes, remove its known_gap",
            g.reference
        )]),
    }
}

/// Corpus directories under a root, or the dir itself if it is one corpus.
fn corpus_dirs(root: &Path) -> Vec<PathBuf> {
    if root.join("expectations").is_dir() {
        return vec![root.to_path_buf()];
    }
    let mut dirs: Vec<PathBuf> = std::fs::read_dir(root)
        .map(|rd| {
            rd.flatten()
                .map(|e| e.path())
                .filter(|p| p.join("expectations").is_dir())
                .collect()
        })
        .unwrap_or_default();
    dirs.sort();
    dirs
}

fn main() {
    let filter: Option<String> = std::env::args().skip(1).find(|a| !a.starts_with('-'));
    let mut roots = vec![Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/acceptance/corpora")];
    match std::env::var("URZUA_ACCEPTANCE_EXTRA") {
        Ok(v) if !v.trim().is_empty() => {
            for d in v.split(':').filter(|d| !d.is_empty()) {
                let p = PathBuf::from(d);
                if p.is_dir() {
                    roots.push(p);
                } else {
                    println!("acceptance: URZUA_ACCEPTANCE_EXTRA entry {d:?} is not a directory");
                    std::process::exit(1);
                }
            }
        }
        _ => println!("acceptance: URZUA_ACCEPTANCE_EXTRA not set; skipping private corpora"),
    }

    let (mut pass, mut gaps, mut fail, mut total) = (0, 0, 0, 0);
    let mut seen_ids = std::collections::HashSet::new();
    for root in &roots {
        for dir in corpus_dirs(root) {
            let name = dir.file_name().unwrap().to_string_lossy().to_string();
            for p in check_manifest(&dir) {
                println!("FAIL  [{name}] {p}");
                fail += 1;
            }
            let mut files: Vec<PathBuf> = std::fs::read_dir(dir.join("expectations"))
                .unwrap()
                .flatten()
                .map(|e| e.path())
                .filter(|p| p.extension().is_some_and(|x| x == "yaml"))
                .collect();
            files.sort();
            for f in files {
                let case: Case = match std::fs::read_to_string(&f)
                    .map_err(|e| e.to_string())
                    .and_then(|t| yaml_serde::from_str(&t).map_err(|e| e.to_string()))
                {
                    Ok(c) => c,
                    Err(e) => {
                        println!("FAIL  {}: unreadable case: {e}", f.display());
                        fail += 1;
                        continue;
                    }
                };
                if !seen_ids.insert(format!("{name}/{}", case.id)) {
                    println!("FAIL  [{name}] duplicate case id {}", case.id);
                    fail += 1;
                    continue;
                }
                if filter
                    .as_ref()
                    .is_some_and(|s| !case.id.contains(s.as_str()))
                {
                    continue;
                }
                total += 1;
                match run_case(&case, &dir) {
                    Outcome::Pass => {
                        pass += 1;
                        println!("ok         [{name}] {}", case.id);
                    }
                    Outcome::KnownGap(why) => {
                        gaps += 1;
                        println!("KNOWN-GAP  [{name}] {} ({why})", case.id);
                    }
                    Outcome::Fail(why) => {
                        fail += 1;
                        println!("FAIL       [{name}] {}", case.id);
                        for w in why {
                            println!("             - {w}");
                        }
                    }
                }
            }
        }
    }
    println!("acceptance: {total} cases, {pass} ok, {gaps} known-gap, {fail} failed");
    if fail > 0 || total == 0 {
        if total == 0 {
            println!("acceptance: no cases ran");
        }
        std::process::exit(1);
    }
}
