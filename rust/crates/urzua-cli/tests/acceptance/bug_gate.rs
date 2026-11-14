//! The acceptance bug-coverage gate (SPEC-4, "Bug coverage gate", MILE-101):
//! every Fixed bug record is reproduced by an acceptance case, or is accounted
//! for in a committed file, and the files that excuse a bug can only shrink.
//!
//! A case names the bugs it reproduces in its `bugs:` list. A Fixed bug with no
//! tagged case is in exactly one of
//!
//! - a lane gap file (`bug-gaps-<lane>.yaml`): corpus-shaped, a case is still to
//!   be written. The lane's PR deletes the entry when it tags a case. An entry
//!   can never be added: it must also sit in `bug-gap-ceilings.tsv`, a list that
//!   only shrinks, so a bug fixed after the gate was introduced needs a case.
//! - the ledger (`bug-ledger.yaml`): not corpus-shaped (a release or CI
//!   workflow, a refactor, a comment, a performance change, documentation), with
//!   a one-line reason. No `check`, `new`, `init` or `fix` run over a corpus can
//!   show such a bug, so a case is not the evidence for it.
//!
//! The lists cannot grow. Alongside the checks above, the gate compares the
//! gap files, the ceiling and the ledger with the merge base's copies (`HEAD`
//! against `URZUA_BUG_GATE_BASE`, default `origin/main`) and fails on any entry
//! the change adds: a gap file cannot take a bug it did not hold, nor the ceiling
//! a line, nor the ledger a bug, without an explicit override
//! (`URZUA_BUG_LISTS_MAY_GROW=1`, which CI sets for a pull request carrying the
//! `bug-lists-grow` label, so a reviewer sees the addition named). A base that
//! has none of the files is the change that introduces them, and adds nothing.
//!
//! The gate runs no corpus and no binary: it reads the bug records, the case
//! files and the committed lists. Shared by `acceptance_bug_gate.rs`, which
//! tests it and enforces it over the repository.

use serde::Deserialize;
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;
use urzua_io::git::GitRun;

/// The lanes that own a gap file, in order.
pub const LANES: &[&str] = &["a", "b", "c", "d"];

/// The committed gap files, one per lane, in `LANES` order.
pub const GAP_FILES: &[(&str, &str)] = &[
    ("a", include_str!("bug-gaps-a.yaml")),
    ("b", include_str!("bug-gaps-b.yaml")),
    ("c", include_str!("bug-gaps-c.yaml")),
    ("d", include_str!("bug-gaps-d.yaml")),
];

/// The committed ledger of bugs no case can show.
pub const LEDGER: &str = include_str!("bug-ledger.yaml");

/// The most the gap files may ever hold: `<bug><TAB><lane>` per line.
pub const CEILINGS: &str = include_str!("bug-gap-ceilings.tsv");

/// One ledger entry: a bug no corpus run can show, and why.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Ledgered {
    pub bug: String,
    pub reason: String,
}

/// A bug a case claims to reproduce.
#[derive(Debug, Clone)]
pub struct Tag {
    pub case: String,
    pub bug: String,
}

/// Everything the gate judges.
#[derive(Default)]
pub struct Facts {
    /// Bug id to the `Status` its record carries.
    pub statuses: BTreeMap<String, String>,
    pub tags: Vec<Tag>,
    /// `(lane, bug)` per gap-file entry.
    pub gaps: Vec<(String, String)>,
    pub ledger: Vec<Ledgered>,
    /// `(bug, lane)` per ceiling line.
    pub ceiling: Vec<(String, String)>,
}

/// The slice of a case file the gate reads. Not `deny_unknown_fields`: the
/// runner owns the case format, the gate only wants `id` and `bugs`.
#[derive(Deserialize)]
struct CaseTags {
    id: String,
    #[serde(default)]
    bugs: Vec<String>,
}

pub fn parse_gap_file(lane: &str, text: &str) -> Result<Vec<(String, String)>, String> {
    let ids: Vec<String> =
        yaml_serde::from_str(text).map_err(|e| format!("bug-gaps-{lane}.yaml: {e}"))?;
    Ok(ids.into_iter().map(|b| (lane.to_string(), b)).collect())
}

pub fn parse_ledger(text: &str) -> Result<Vec<Ledgered>, String> {
    yaml_serde::from_str(text).map_err(|e| format!("bug-ledger.yaml: {e}"))
}

pub fn parse_ceilings(text: &str) -> Result<Vec<(String, String)>, String> {
    let mut out = Vec::new();
    for (i, line) in text.lines().enumerate() {
        let line = line.trim_end();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        match line.split_once('\t') {
            Some((bug, lane)) if !bug.is_empty() && !lane.is_empty() => {
                out.push((bug.to_string(), lane.to_string()))
            }
            _ => {
                return Err(format!(
                    "bug-gap-ceilings.tsv:{}: want <bug><TAB><lane>",
                    i + 1
                ))
            }
        }
    }
    Ok(out)
}

/// The `Status` in a record's frontmatter, if it has one.
pub fn header_status(text: &str) -> Option<String> {
    let mut lines = text.lines();
    if lines.next()?.trim_end() != "---" {
        return None;
    }
    for line in lines {
        if line.trim_end() == "---" {
            break;
        }
        if let Some(v) = line.strip_prefix("Status:") {
            return Some(v.trim().trim_matches('\'').trim_matches('"').to_string());
        }
    }
    None
}

/// Every `BUG-<n>-*.md` under `docs/bugs`, as id to status.
pub fn load_statuses(bugs_dir: &Path) -> Result<BTreeMap<String, String>, String> {
    let mut out = BTreeMap::new();
    let entries =
        std::fs::read_dir(bugs_dir).map_err(|e| format!("{}: {e}", bugs_dir.display()))?;
    for entry in entries {
        let path = entry.map_err(|e| e.to_string())?.path();
        let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
        let Some(rest) = name.strip_prefix("BUG-") else {
            continue;
        };
        let digits: String = rest.chars().take_while(|c| c.is_ascii_digit()).collect();
        if digits.is_empty() || !name.ends_with(".md") {
            continue;
        }
        let text = std::fs::read_to_string(&path).map_err(|e| format!("{name}: {e}"))?;
        let status =
            header_status(&text).ok_or_else(|| format!("{name}: no Status in its header"))?;
        out.insert(format!("BUG-{digits}"), status);
    }
    Ok(out)
}

/// Every `bugs:` tag in every case file under `corpora/*/expectations/`.
pub fn load_tags(corpora: &Path) -> Result<Vec<Tag>, String> {
    let mut out = Vec::new();
    let mut dirs: Vec<_> = std::fs::read_dir(corpora)
        .map_err(|e| format!("{}: {e}", corpora.display()))?
        .filter_map(|e| e.ok().map(|e| e.path().join("expectations")))
        .filter(|p| p.is_dir())
        .collect();
    dirs.sort();
    for dir in dirs {
        let mut files: Vec<_> = std::fs::read_dir(&dir)
            .map_err(|e| e.to_string())?
            .filter_map(|e| e.ok().map(|e| e.path()))
            .filter(|p| p.extension().is_some_and(|x| x == "yaml"))
            .collect();
        files.sort();
        for file in files {
            let text =
                std::fs::read_to_string(&file).map_err(|e| format!("{}: {e}", file.display()))?;
            let case: CaseTags =
                yaml_serde::from_str(&text).map_err(|e| format!("{}: {e}", file.display()))?;
            out.extend(case.bugs.into_iter().map(|bug| Tag {
                case: case.id.clone(),
                bug,
            }));
        }
    }
    Ok(out)
}

/// The committed lists plus what the repository holds at `root`.
pub fn load(root: &Path) -> Result<Facts, String> {
    let mut gaps = Vec::new();
    for (lane, text) in GAP_FILES {
        gaps.extend(parse_gap_file(lane, text)?);
    }
    Ok(Facts {
        statuses: load_statuses(&root.join("docs/bugs"))?,
        tags: load_tags(&root.join("rust/crates/urzua-cli/tests/acceptance/corpora"))?,
        gaps,
        ledger: parse_ledger(LEDGER)?,
        ceiling: parse_ceilings(CEILINGS)?,
    })
}

fn status_problem(facts: &Facts, bug: &str) -> Option<String> {
    match facts.statuses.get(bug) {
        None => Some("is not a bug record".to_string()),
        Some(s) if s != "Fixed" => Some(format!("is {s}, not Fixed")),
        Some(_) => None,
    }
}

fn tag_violations(facts: &Facts) -> Vec<String> {
    facts
        .tags
        .iter()
        .filter_map(|t| {
            status_problem(facts, &t.bug).map(|p| {
                format!(
                    "case {} is tagged with {}, which {p}: tag only Fixed bugs",
                    t.case, t.bug
                )
            })
        })
        .collect()
}

fn duplicates<'a>(names: impl Iterator<Item = &'a str>, what: &str) -> Vec<String> {
    let mut seen = BTreeSet::new();
    names
        .filter(|n| !seen.insert(*n))
        .map(|n| format!("{what} names {n} twice"))
        .collect()
}

fn gap_violations(facts: &Facts, tagged: &BTreeSet<&str>) -> Vec<String> {
    let mut out = duplicates(facts.gaps.iter().map(|(_, b)| b.as_str()), "the gap files");
    let ceiling: BTreeMap<&str, &str> = facts
        .ceiling
        .iter()
        .map(|(b, l)| (b.as_str(), l.as_str()))
        .collect();
    for (lane, bug) in &facts.gaps {
        if !LANES.contains(&lane.as_str()) {
            out.push(format!("gap file for unknown lane {lane}"));
        }
        if let Some(p) = status_problem(facts, bug) {
            out.push(format!(
                "gap entry {bug} (lane {lane}) {p}: remove the entry"
            ));
        }
        if tagged.contains(bug.as_str()) {
            out.push(format!("gap entry {bug} (lane {lane}) is stale: a case tags it now, remove the entry and its ceiling line"));
        }
        match ceiling.get(bug.as_str()) {
            None => out.push(format!(
                "gap entry {bug} (lane {lane}) is new: the lists only shrink, so write a case that tags {bug} instead"
            )),
            Some(l) if l != lane => {
                out.push(format!("gap entry {bug} is in lane {lane} but the ceiling gives it lane {l}"))
            }
            Some(_) => {}
        }
    }
    out
}

fn ceiling_violations(facts: &Facts) -> Vec<String> {
    let mut out = duplicates(
        facts.ceiling.iter().map(|(b, _)| b.as_str()),
        "bug-gap-ceilings.tsv",
    );
    let gapped: BTreeSet<&str> = facts.gaps.iter().map(|(_, b)| b.as_str()).collect();
    for (bug, _) in &facts.ceiling {
        if !gapped.contains(bug.as_str()) {
            out.push(format!(
                "bug-gap-ceilings.tsv lists {bug}, which is in no gap file: delete the line, the list only shrinks"
            ));
        }
    }
    out
}

fn ledger_violations(facts: &Facts, tagged: &BTreeSet<&str>) -> Vec<String> {
    let mut out = duplicates(facts.ledger.iter().map(|l| l.bug.as_str()), "the ledger");
    let gapped: BTreeSet<&str> = facts.gaps.iter().map(|(_, b)| b.as_str()).collect();
    for l in &facts.ledger {
        if let Some(p) = status_problem(facts, &l.bug) {
            out.push(format!("ledger entry {} {p}: remove the entry", l.bug));
        }
        if l.reason.trim().is_empty() || l.reason.trim().contains('\n') {
            out.push(format!("ledger entry {} needs a one-line reason", l.bug));
        }
        if tagged.contains(l.bug.as_str()) {
            out.push(format!(
                "ledger entry {} is stale: a case tags it, remove the entry",
                l.bug
            ));
        }
        if gapped.contains(l.bug.as_str()) {
            out.push(format!(
                "{} is in the ledger and a gap file: it is one or the other",
                l.bug
            ));
        }
    }
    out
}

fn uncovered_violations(facts: &Facts, tagged: &BTreeSet<&str>) -> Vec<String> {
    let accounted: BTreeSet<&str> = facts
        .gaps
        .iter()
        .map(|(_, b)| b.as_str())
        .chain(facts.ledger.iter().map(|l| l.bug.as_str()))
        .collect();
    facts
        .statuses
        .iter()
        .filter(|(b, s)| s.as_str() == "Fixed" && !tagged.contains(b.as_str()) && !accounted.contains(b.as_str()))
        .map(|(b, _)| {
            format!("{b} is Fixed and no case tags it: add `bugs: [{b}]` to a case that reproduces it, or, when no corpus run can show it, add it to bug-ledger.yaml with a reason")
        })
        .collect()
}

/// Every reason the gate fails, empty when it passes.
pub fn violations(facts: &Facts) -> Vec<String> {
    let tagged: BTreeSet<&str> = facts.tags.iter().map(|t| t.bug.as_str()).collect();
    let mut out = tag_violations(facts);
    out.extend(gap_violations(facts, &tagged));
    out.extend(ceiling_violations(facts));
    out.extend(ledger_violations(facts, &tagged));
    out.extend(uncovered_violations(facts, &tagged));
    out
}

/// The bugs each committed list names, as of one revision.
#[derive(Default, Debug, PartialEq, Eq)]
pub struct Lists {
    pub gaps: BTreeSet<String>,
    pub ceiling: BTreeSet<String>,
    pub ledger: BTreeSet<String>,
}

impl Lists {
    /// The lists the working copy of this build was compiled with.
    pub fn current() -> Result<Lists, String> {
        let mut gaps = BTreeSet::new();
        for (lane, text) in GAP_FILES {
            gaps.extend(parse_gap_file(lane, text)?.into_iter().map(|(_, b)| b));
        }
        Ok(Lists {
            gaps,
            ceiling: parse_ceilings(CEILINGS)?
                .into_iter()
                .map(|(b, _)| b)
                .collect(),
            ledger: parse_ledger(LEDGER)?.into_iter().map(|l| l.bug).collect(),
        })
    }
}

/// Every entry `now` has that `base` does not, as one message each. Entries
/// that left a list are not growth.
pub fn growth(base: &Lists, now: &Lists) -> Vec<String> {
    let mut out = Vec::new();
    for (name, was, is) in [
        ("a bug-gaps-<lane>.yaml file", &base.gaps, &now.gaps),
        ("bug-gap-ceilings.tsv", &base.ceiling, &now.ceiling),
        ("bug-ledger.yaml", &base.ledger, &now.ledger),
    ] {
        for bug in is.difference(was) {
            out.push(format!(
                "{bug} was added to {name}: these lists only shrink, so tag a case with `bugs: [{bug}]` instead (a bug no corpus run can show goes in the ledger with the `bug-lists-grow` label on the pull request)"
            ));
        }
    }
    out
}

const LIST_PATHS: &[(&str, &str)] = &[
    ("a", "bug-gaps-a.yaml"),
    ("b", "bug-gaps-b.yaml"),
    ("c", "bug-gaps-c.yaml"),
    ("d", "bug-gaps-d.yaml"),
];
const ACCEPTANCE_DIR: &str = "rust/crates/urzua-cli/tests/acceptance";

fn git_text(root: &Path, args: &[&str]) -> Result<Option<String>, String> {
    let out = GitRun::new()
        .dash_c(root)
        .args(args)
        .discard_stderr()
        .output()
        .map_err(|e| format!("git {args:?}: {e:?}"))?;
    Ok(out
        .status
        .success()
        .then(|| String::from_utf8_lossy(&out.stdout).trim_end().to_string()))
}

/// The lists as the merge base of `HEAD` and `base_ref` held them. `Ok(None)`
/// when the base has none of them (this change introduces the gate); `Err` when
/// the base cannot be resolved at all.
pub fn base_lists(root: &Path, base_ref: &str) -> Result<Option<Lists>, String> {
    let mb = git_text(root, &["merge-base", "HEAD", base_ref])?
        .ok_or_else(|| format!("no merge base between HEAD and {base_ref}"))?;
    let show = |file: &str| git_text(root, &["show", &format!("{mb}:{ACCEPTANCE_DIR}/{file}")]);
    let Some(ceilings) = show("bug-gap-ceilings.tsv")? else {
        return Ok(None);
    };
    let mut lists = Lists {
        ceiling: parse_ceilings(&ceilings)?
            .into_iter()
            .map(|(b, _)| b)
            .collect(),
        ..Lists::default()
    };
    for (lane, file) in LIST_PATHS {
        let text = show(file)?.ok_or_else(|| format!("{file} is missing at {mb}"))?;
        lists
            .gaps
            .extend(parse_gap_file(lane, &text)?.into_iter().map(|(_, b)| b));
    }
    let ledger =
        show("bug-ledger.yaml")?.ok_or_else(|| format!("bug-ledger.yaml is missing at {mb}"))?;
    lists.ledger = parse_ledger(&ledger)?.into_iter().map(|l| l.bug).collect();
    Ok(Some(lists))
}
