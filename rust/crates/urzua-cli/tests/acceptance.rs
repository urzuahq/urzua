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
//!
//! Mutation proof (MILE-170): SPEC-4 says "a case that passes against a build
//! with the rule removed is not a case". Every case therefore names the rules
//! it proves (`proves`, defaulting to the rules in `expect_findings` and
//! `expect_rules_ran`), and the runner re-runs it with each of those rules
//! switched off in the case's own config; the case must then FAIL. A case that
//! proves no rule says why in `mutation_exempt`. The rules proven this way are
//! reported against `ALL_RULES`, and the ones no case proves are listed, so a
//! gap in coverage is visible on every run instead of silent.

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
    /// Pairs of path prefixes whose findings must name the same non-empty set
    /// of rules: the same planted violations in two record types must be met
    /// by the same rules (SPEC-4's "Nth record type" claim).
    #[serde(default)]
    expect_mirrored: Vec<Mirror>,
    /// Notices that must appear in the output's `notices` array.
    #[serde(default)]
    expect_notices: Vec<NoticeMatcher>,
    /// Commits made after the fixture commit (which is tagged `base`), in
    /// order, so a case can run `check --base base` or exercise git history.
    /// Not allowed with `corpus: "@repo"`.
    #[serde(default)]
    commits: Vec<Commit>,
    /// Run in a `--depth 1` clone of the corpus repository once `commits` are
    /// made, so a case can assert how a truncated history is reported.
    #[serde(default)]
    shallow: bool,
    /// Rules that must appear in `rules_executed` with status `ran` and a
    /// non-empty examined population. Only the first is switched off for the
    /// mutation proof (the assertion is the same for each), which keeps a case
    /// over this repository's whole corpus to two runs.
    #[serde(default)]
    expect_rules_ran: Vec<String>,
    /// Rules this case proves, beyond those it names in `expect_findings` and
    /// `expect_rules_ran`. Each is switched off in turn and the case must fail.
    #[serde(default)]
    proves: Vec<String>,
    /// Why this case proves no rule (a positive control, a config error). A
    /// case with neither a rule to prove nor this reason is rejected.
    mutation_exempt: Option<String>,
    known_gap: Option<KnownGap>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct NoticeMatcher {
    subject: String,
    message_contains: Option<String>,
}

/// One commit made on top of the fixture commit. `write` maps a path to its
/// whole new content, `rename` moves a tracked file, `remove` deletes one.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Commit {
    #[serde(default)]
    write: std::collections::BTreeMap<String, String>,
    #[serde(default)]
    rename: Vec<Rename>,
    #[serde(default)]
    remove: Vec<String>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Rename {
    from: String,
    to: String,
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
struct Mirror {
    left: String,
    right: String,
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
    message_contains: Option<String>,
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

/// A case's outcome and the rules whose mutation proof held.
type CaseResult = (Outcome, Vec<String>);

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
        && m.message_contains
            .as_deref()
            .is_none_or(|t| field("message").contains(t))
}

/// Rules a case proves: the explicit list plus those it asserts on.
fn proven_rules(case: &Case) -> Vec<String> {
    let mut rules: Vec<String> = case.proves.clone();
    rules.extend(case.expect_rules_ran.iter().cloned());
    rules.extend(case.expect_findings.iter().filter_map(|m| m.rule.clone()));
    rules.sort();
    rules.dedup();
    rules
}

/// Config with `rule` switched off, as YAML text.
fn config_with_rule_off(text: &str, rule: &str) -> Result<String, String> {
    let mut doc: yaml_serde::Value = yaml_serde::from_str(text).map_err(|e| e.to_string())?;
    let root = doc.as_mapping_mut().ok_or("config is not a mapping")?;
    let key = yaml_serde::Value::String("rules".into());
    if !root.contains_key(&key) {
        root.insert(
            key.clone(),
            yaml_serde::Value::Mapping(yaml_serde::Mapping::new()),
        );
    }
    let rules = root
        .get_mut(&key)
        .and_then(|r| r.as_mapping_mut())
        .ok_or("`rules` is not a mapping")?;
    rules.insert(
        yaml_serde::Value::String(rule.into()),
        yaml_serde::Value::String("off".into()),
    );
    yaml_serde::to_string(&doc).map_err(|e| e.to_string())
}

/// Removes a case's temporary repository on every exit path, including the
/// early returns when copying, git setup or a setup step fails.
struct TempDir(PathBuf);

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

type SharedRun = std::sync::Arc<std::sync::OnceLock<(Option<i32>, String)>>;

fn repo_runs() -> &'static std::sync::Mutex<std::collections::HashMap<String, SharedRun>> {
    static RUNS: std::sync::OnceLock<
        std::sync::Mutex<std::collections::HashMap<String, SharedRun>>,
    > = std::sync::OnceLock::new();
    RUNS.get_or_init(Default::default)
}

struct TempFile(PathBuf);

impl Drop for TempFile {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.0);
    }
}

/// Run one case; returns the list of unmet expectations (empty = all met).
/// With `mutate`, the config the case runs under has that rule switched off
/// after `setup` has run, and a config the mutation breaks is a harness error
/// (a rejected config proves nothing about the rule).
fn evaluate(case: &Case, corpus_dir: &Path, mutate: Option<&str>) -> Result<Vec<String>, String> {
    let (cwd, temp) = if case.corpus == "@repo" {
        (repo_root(), None)
    } else {
        let t = std::env::temp_dir().join(format!(
            "urzua-acceptance-{}-{}{}",
            std::process::id(),
            case.id,
            mutate.map(|r| format!("-off-{r}")).unwrap_or_default()
        ));
        let _ = std::fs::remove_dir_all(&t);
        let guard = TempDir(t.clone());
        std::fs::create_dir_all(&t).map_err(|e| e.to_string())?;
        copy_corpus(corpus_dir, &t)?;
        git(&t, &["init", "-q"])?;
        git(&t, &["config", "user.email", "acceptance@example.com"])?;
        git(&t, &["config", "user.name", "acceptance"])?;
        git(&t, &["add", "-A"])?;
        git(&t, &["commit", "-q", "-m", "fixture"])?;
        git(&t, &["tag", "base"])?;
        for (n, c) in case.commits.iter().enumerate() {
            for r in &c.rename {
                if let Some(parent) = t.join(&r.to).parent() {
                    std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
                }
                git(&t, &["mv", &r.from, &r.to])?;
            }
            for p in &c.remove {
                git(&t, &["rm", "-q", p])?;
            }
            for (p, text) in &c.write {
                let to = t.join(p);
                std::fs::create_dir_all(to.parent().unwrap()).map_err(|e| e.to_string())?;
                std::fs::write(&to, text).map_err(|e| e.to_string())?;
            }
            git(&t, &["add", "-A"])?;
            git(&t, &["commit", "-q", "-m", &format!("change {}", n + 1)])?;
        }
        if case.shallow {
            let clone = PathBuf::from(format!("{}-shallow", t.display()));
            let _ = std::fs::remove_dir_all(&clone);
            let guard_clone = TempDir(clone.clone());
            let url = format!("file://{}", t.display());
            git(
                &std::env::temp_dir(),
                &[
                    "clone",
                    "-q",
                    "--depth",
                    "1",
                    &url,
                    &clone.to_string_lossy(),
                ],
            )?;
            drop(guard);
            (clone, Some(guard_clone))
        } else {
            (t, Some(guard))
        }
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
    let mut invocation = case.invocation.clone();
    let mut mutated_config = None;
    if let Some(rule) = mutate {
        let at = invocation.iter().position(|a| a == "--config");
        let source = match at {
            Some(i) => cwd.join(invocation.get(i + 1).ok_or("--config has no value")?),
            None => cwd.join(".urzua/config.yaml"),
        };
        let text = std::fs::read_to_string(&source)
            .map_err(|e| format!("mutation: cannot read {}: {e}", source.display()))?;
        let path = std::env::temp_dir().join(format!(
            "urzua-acceptance-{}-{}-off-{rule}.yaml",
            std::process::id(),
            case.id
        ));
        std::fs::write(&path, config_with_rule_off(&text, rule)?).map_err(|e| e.to_string())?;
        let value = path.to_string_lossy().to_string();
        match at {
            Some(i) => invocation[i + 1] = value,
            None => {
                invocation.insert(0, "--config".into());
                invocation.insert(1, value);
            }
        }
        mutated_config = Some(TempFile(path));
    }
    // Runs over this repository in place are read-only and many cases ask the
    // same question, so an identical (config, invocation) is run once.
    let shared = case.corpus == "@repo" && case.setup.is_empty();
    let key = format!("{mutate:?}|{invocation:?}");
    let out: (Option<i32>, String) = if shared {
        let cell = repo_runs().lock().unwrap().entry(key).or_default().clone();
        cell.get_or_init(|| match run(&invocation) {
            Ok(o) => (
                o.status.code(),
                String::from_utf8_lossy(&o.stdout).to_string(),
            ),
            Err(e) => (None, format!("harness error: {e}")),
        })
        .clone()
    } else {
        let o = run(&invocation)?;
        (
            o.status.code(),
            String::from_utf8_lossy(&o.stdout).to_string(),
        )
    };
    drop(temp);
    drop(mutated_config);
    if mutate.is_some() && out.0 == Some(2) && case.expect_exit != 2 {
        return Err(format!(
            "switching {} off made the config invalid (exit 2); the mutation proves nothing",
            mutate.unwrap_or_default()
        ));
    }

    let stdout = out.1;
    let mut unmet = Vec::new();
    let code = out.0.unwrap_or(-1);
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
        || !case.expect_no_findings.is_empty()
        || !case.expect_rules_ran.is_empty()
        || !case.expect_notices.is_empty()
        || !case.expect_mirrored.is_empty();
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
        let has_finding_matchers = !case.expect_findings.is_empty()
            || !case.expect_no_findings.is_empty()
            || !case.expect_mirrored.is_empty();
        let findings = match j.get("findings").and_then(|f| f.as_array()) {
            Some(f) => f.clone(),
            None => {
                if has_finding_matchers {
                    unmet.push("output has no findings array".to_string());
                }
                Vec::new()
            }
        };
        for m in &case.expect_mirrored {
            let rules_under = |prefix: &str| -> std::collections::BTreeSet<String> {
                findings
                    .iter()
                    .filter(|f| {
                        f.get("file")
                            .and_then(|v| v.as_str())
                            .is_some_and(|p| p.starts_with(prefix))
                    })
                    .filter_map(|f| f.get("rule").and_then(|r| r.as_str()).map(String::from))
                    .collect()
            };
            let (l, r) = (rules_under(&m.left), rules_under(&m.right));
            if l.is_empty() || l != r {
                unmet.push(format!(
                    "findings under {} ({l:?}) and {} ({r:?}) are not the same non-empty set of rules",
                    m.left, m.right
                ));
            }
        }
        let executed = j
            .get("rules_executed")
            .and_then(|r| r.as_array())
            .cloned()
            .unwrap_or_default();
        for rule in &case.expect_rules_ran {
            let entry = executed
                .iter()
                .find(|e| e.get("rule").and_then(|r| r.as_str()) == Some(rule));
            let status = entry.and_then(|e| e.get("status")).and_then(|s| s.as_str());
            let examined = entry
                .and_then(|e| e.pointer("/population/examined"))
                .and_then(|n| n.as_u64())
                .unwrap_or(0);
            if status != Some("ran") || examined == 0 {
                unmet.push(format!(
                    "rule {rule} did not run over a non-empty population (status {status:?}, examined {examined})"
                ));
            }
        }
        let notices = j
            .get("notices")
            .and_then(|n| n.as_array())
            .cloned()
            .unwrap_or_default();
        for n in &case.expect_notices {
            let field = |v: &serde_json::Value, k: &str| {
                v.get(k).and_then(|x| x.as_str()).unwrap_or("").to_string()
            };
            let found = notices.iter().any(|v| {
                field(v, "subject") == n.subject
                    && n.message_contains
                        .as_deref()
                        .is_none_or(|t| field(v, "message").contains(t))
            });
            if !found {
                unmet.push(format!(
                    "no notice with subject {:?} containing {:?}",
                    n.subject, n.message_contains
                ));
            }
        }
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

/// Run the case, then prove each rule it names: with the rule off in the
/// case's own config the case must fail. Returns the outcome and the rules
/// whose proof held.
fn run_case(case: &Case, corpus_dir: &Path) -> CaseResult {
    let unmet = match evaluate(case, corpus_dir, None) {
        Ok(u) => u,
        Err(e) => {
            return (
                Outcome::Fail(vec![format!("harness error: {e}")]),
                Vec::new(),
            )
        }
    };
    let rules = proven_rules(case);
    if let Some(g) = &case.known_gap {
        let outcome = match (
            unmet.is_empty(),
            g.reason.trim().is_empty() || g.reference.trim().is_empty(),
        ) {
            (_, true) => Outcome::Fail(vec!["known_gap needs a ref and a reason".into()]),
            (false, false) => Outcome::KnownGap(format!("{} -- {}", g.reference, unmet.join("; "))),
            (true, false) => Outcome::Fail(vec![format!(
                "known gap {} has closed: the case now passes, remove its known_gap",
                g.reference
            )]),
        };
        return (outcome, Vec::new());
    }
    if !unmet.is_empty() {
        return (Outcome::Fail(unmet), Vec::new());
    }
    let mut problems = Vec::new();
    if let Some(reason) = &case.mutation_exempt {
        // Exempt: the case asserts something no single rule produces, so there
        // is nothing to switch off. It proves no rule and is not counted.
        return if reason.trim().is_empty() {
            (
                Outcome::Fail(vec!["mutation_exempt needs a reason".into()]),
                Vec::new(),
            )
        } else {
            (Outcome::Pass, Vec::new())
        };
    }
    if rules.is_empty() {
        problems.push(
            "the case proves no rule: name one in expect_findings, expect_rules_ran or proves, or give mutation_exempt a reason"
                .to_string(),
        );
    }
    let mut proven = Vec::new();
    // `expect_rules_ran` asserts the same way for every rule it lists, so one
    // sampled rule is switched off to show the assertion can fail; the rest
    // are counted once that held. Findings and `proves` are mutated one by one.
    let sampled = case.expect_rules_ran.first().cloned();
    for rule in &rules {
        if !urzua_core::rules::ALL_RULES.contains(&rule.as_str()) {
            problems.push(format!("{rule} is not a rule this build ships"));
            continue;
        }
        let only_ran = case.expect_rules_ran.contains(rule)
            && !case.proves.contains(rule)
            && !case
                .expect_findings
                .iter()
                .any(|m| m.rule.as_ref() == Some(rule));
        if only_ran && sampled.as_ref() != Some(rule) {
            proven.push(rule.clone());
            continue;
        }
        match evaluate(case, corpus_dir, Some(rule)) {
            Ok(m) if m.is_empty() => problems.push(format!(
                "passes with {rule} switched off, so it does not prove {rule}"
            )),
            Ok(_) => proven.push(rule.clone()),
            Err(e) => problems.push(format!("mutation of {rule}: {e}")),
        }
    }
    if problems.is_empty() {
        (Outcome::Pass, proven)
    } else {
        (Outcome::Fail(problems), proven)
    }
}

/// Corpus directories under a root, or the dir itself if it is one corpus.
fn corpus_dirs(root: &Path) -> Result<Vec<PathBuf>, String> {
    if root.join("expectations").is_dir() {
        return Ok(vec![root.to_path_buf()]);
    }
    let rd = std::fs::read_dir(root).map_err(|e| format!("{}: {e}", root.display()))?;
    let mut dirs = Vec::new();
    for entry in rd {
        let p = entry
            .map_err(|e| format!("{}: {e}", root.display()))?
            .path();
        if p.join("expectations").is_dir() {
            dirs.push(p);
        }
    }
    dirs.sort();
    Ok(dirs)
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
    // (corpus name, corpus dir, case) in discovery order.
    let mut queue: Vec<(String, PathBuf, Case)> = Vec::new();
    for root in &roots {
        let dirs = match corpus_dirs(root) {
            Ok(d) => d,
            Err(e) => {
                println!("FAIL  cannot read corpus root {e}");
                fail += 1;
                continue;
            }
        };
        for dir in dirs {
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
                queue.push((name.clone(), dir.clone(), case));
            }
        }
    }

    // Cases are independent (each owns a throwaway repo), so run them on a few
    // threads; results are printed in discovery order, so output is stable.
    let next = std::sync::atomic::AtomicUsize::new(0);
    let results: std::sync::Mutex<Vec<Option<CaseResult>>> =
        std::sync::Mutex::new((0..queue.len()).map(|_| None).collect());
    let workers = std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(2)
        .clamp(1, 8);
    std::thread::scope(|scope| {
        for _ in 0..workers {
            scope.spawn(|| loop {
                let i = next.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                let Some((_, dir, case)) = queue.get(i) else {
                    break;
                };
                let r = run_case(case, dir);
                results.lock().unwrap()[i] = Some(r);
            });
        }
    });
    let results = results.into_inner().unwrap();

    let mut proven: std::collections::BTreeSet<String> = std::collections::BTreeSet::new();
    for ((name, _, case), result) in queue.iter().zip(results) {
        total += 1;
        let (outcome, rules) = result.expect("every queued case ran");
        proven.extend(rules);
        match outcome {
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
    println!("acceptance: {total} cases, {pass} ok, {gaps} known-gap, {fail} failed");
    // A name filter runs part of the suite, so the coverage of the rest is
    // unknown and is not reported.
    if filter.is_none() {
        report_coverage(&proven);
    }
    // A name filter (`cargo test <name>` passes it to every target) may
    // legitimately match no case; only an unfiltered run with no cases fails.
    let empty_unfiltered = total == 0 && filter.is_none();
    if fail > 0 || empty_unfiltered {
        if empty_unfiltered {
            println!("acceptance: no cases ran");
        }
        std::process::exit(1);
    }
}

/// States which shipped rules have an acceptance case whose mutation proof
/// held and which have none. Printed on every unfiltered run: the gap is
/// reported, not enforced, because a rule another change adds has no case yet.
fn report_coverage(proven: &std::collections::BTreeSet<String>) {
    let all = urzua_core::rules::ALL_RULES;
    let uncovered: Vec<&str> = all
        .iter()
        .copied()
        .filter(|r| !proven.contains(*r))
        .collect();
    println!(
        "acceptance coverage: {} of {} rules have a case that fails with the rule off",
        all.len() - uncovered.len(),
        all.len()
    );
    if uncovered.is_empty() {
        return;
    }
    println!(
        "acceptance coverage: {} rules have NO acceptance case:",
        uncovered.len()
    );
    for r in uncovered {
        println!("  uncovered  {r}");
    }
}
