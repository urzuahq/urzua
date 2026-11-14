//! `urzua migrate filenames` (MILE-134, SPEC-14): rename a corpus's record files to the
//! filename convention each type declares, and rewrite the citations of the renamed
//! paths. The plan is `urzua_core::migrate_filenames`; this module reads the corpus,
//! writes under the lock `fix --apply` takes, and reports.

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use urzua_core::migrate_filenames::{
    plan, Citation, FilenamePlan, PlanInput, Rename, Skipped, StalePin, Unrewritten,
};
use urzua_core::report::{
    census, Failure, Notice, NoticeSeverity, Outcome as Examined, Population, PopulationUnit,
    Report,
};

use crate::emit;
use crate::session::{emit_or_not_run, Session};

/// Files larger than this are not read for citations.
const MAX_SCANNED_BYTES: u64 = 2 * 1024 * 1024;

#[derive(serde::Serialize)]
struct FilenamesReport {
    status: FilenamesStatus,
    apply: bool,
    /// The records the plan read: a run that renames nothing says how many it
    /// looked at, so an empty `renames` is told apart from a run over nothing.
    population: Population,
    renames: Vec<Rename>,
    rewrites: Vec<Citation>,
    unrewritten: Vec<Unrewritten>,
    stale_pins: Vec<StalePin>,
    skipped: Vec<Skipped>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    results: Vec<WriteResult>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    notices: Vec<Notice>,
}

/// `partial-failure` when any write failed, so the report and the exit code agree.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "kebab-case")]
enum FilenamesStatus {
    Ok,
    PartialFailure,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "kebab-case")]
enum Action {
    Rewrite,
    Rename,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "kebab-case")]
enum Outcome {
    Applied,
    Skipped,
    Failed,
}

#[derive(serde::Serialize)]
struct WriteResult {
    path: PathBuf,
    action: Action,
    outcome: Outcome,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<String>,
}

impl Report for FilenamesReport {
    fn notices(&self) -> &[Notice] {
        &self.notices
    }
    fn exit_code(&self) -> ExitCode {
        let failed = self.results.iter().any(|r| r.outcome == Outcome::Failed);
        ExitCode::from(u8::from(failed))
    }
}

pub fn run(config_path: Option<PathBuf>, apply: bool) -> ExitCode {
    emit_or_not_run(run_inner(config_path, apply))
}

fn run_inner(config_path: Option<PathBuf>, apply: bool) -> Result<ExitCode, Failure> {
    let session = Session::open(config_path)?;
    let (rewritable, report_only) = scanned_texts(&session);
    let existing: HashSet<PathBuf> = session
        .discovered
        .paths
        .iter()
        .filter(|p| !session.discovered.staged_deletions.contains(*p))
        .cloned()
        .collect();
    let planned = plan(&PlanInput {
        records: &session.records,
        config: &session.config,
        existing: &existing,
        rewritable: &rewritable,
        report_only: &report_only,
    });
    let population = census(
        PopulationUnit::Record,
        session.records.iter().collect(),
        |_| Examined::Examined,
    );
    let notices = notices_for(&planned, apply);
    let results = if apply && !planned.renames.is_empty() {
        let _lock = urzua_io::FixLock::acquire(&session.repo_root)?;
        write_plan(&session.repo_root, &planned, &rewritable)
    } else {
        Vec::new()
    };
    let failed = results.iter().any(|r| r.outcome == Outcome::Failed);
    let FilenamePlan {
        renames,
        skipped,
        rewrites,
        unrewritten,
        stale_pins,
        ..
    } = planned;
    Ok(emit(&FilenamesReport {
        status: if failed {
            FilenamesStatus::PartialFailure
        } else {
            FilenamesStatus::Ok
        },
        apply,
        population,
        renames,
        rewrites,
        unrewritten,
        stale_pins,
        skipped,
        results,
        notices,
    }))
}

/// The tracked texts to scan for citations: records and other markdown files are
/// rewritten, any other readable text file is only reported.
fn scanned_texts(session: &Session) -> (HashMap<PathBuf, String>, HashMap<PathBuf, String>) {
    let mut rewritable = session.full_text.clone();
    let mut report_only = HashMap::new();
    for path in &session.discovered.paths {
        if rewritable.contains_key(path) || session.discovered.staged_deletions.contains(path) {
            continue;
        }
        let full = session.repo_root.join(path);
        let small = std::fs::symlink_metadata(&full)
            .is_ok_and(|m| m.file_type().is_file() && m.len() <= MAX_SCANNED_BYTES);
        let Some(text) = small
            .then(|| urzua_io::read_to_string(&full).ok())
            .flatten()
        else {
            continue;
        };
        let markdown = path
            .extension()
            .is_some_and(|e| e.eq_ignore_ascii_case("md"));
        if markdown {
            rewritable.insert(path.clone(), text);
        } else {
            report_only.insert(path.clone(), text);
        }
    }
    (rewritable, report_only)
}

fn notices_for(planned: &FilenamePlan, apply: bool) -> Vec<Notice> {
    let mut notices = pin_notice(planned, apply);
    notices.extend(stable_id_notice(planned));
    notices
}

/// Rewriting a pinned file's text changes its hash. The tool does not re-pin: a re-pin
/// carries a reason a person attests, so it names the command instead.
fn pin_notice(planned: &FilenamePlan, apply: bool) -> Vec<Notice> {
    if planned.stale_pins.is_empty() {
        return Vec::new();
    }
    let verb = if apply { "are stale" } else { "will be stale" };
    vec![Notice {
        severity: NoticeSeverity::Warning,
        subject: "stale-pins".to_string(),
        message: format!(
            "{} evidence pin(s) {verb}: they name a file whose text this run rewrites (see `stale_pins`); run `urzua fix --rebaseline` to list them and re-pin each with a reason you attest",
            planned.stale_pins.len()
        ),
        detail: None,
    }]
}

fn stable_id_notice(planned: &FilenamePlan) -> Vec<Notice> {
    let unpaired: Vec<String> = planned
        .renames
        .iter()
        .filter(|r| !r.has_stable_id)
        .map(|r| r.from.display().to_string())
        .collect();
    if unpaired.is_empty() {
        return Vec::new();
    }
    vec![Notice {
        severity: NoticeSeverity::Warning,
        subject: "stable-id-missing".to_string(),
        message: format!(
            "{} record(s) to rename have no Stable-Id, so a diff cannot pair the rename with the old file; run `urzua migrate ids --apply` first: {}",
            unpaired.len(),
            unpaired.join(", ")
        ),
        detail: None,
    }]
}

/// Rewrites citations first, then renames, so a failure never leaves a renamed file
/// with its citations still pointing at the old name: a failed rewrite stops the
/// renames. A rename that fails after the rewrites leaves citations naming the new
/// path of a file still at its old one; each is reported `failed`, and a rerun renames
/// it (its citations already read the new name), which heals the corpus.
fn write_plan(
    root: &Path,
    planned: &FilenamePlan,
    originals: &HashMap<PathBuf, String>,
) -> Vec<WriteResult> {
    let mut results = Vec::new();
    for (path, text) in &planned.new_texts {
        results.push(rewrite_one(root, path, text, originals.get(path)));
    }
    let blocked = results.iter().any(|r| r.outcome == Outcome::Failed);
    for rename in &planned.renames {
        results.push(if blocked {
            WriteResult {
                path: rename.from.clone(),
                action: Action::Rename,
                outcome: Outcome::Skipped,
                error: Some("not renamed: a citation rewrite failed; run again".to_string()),
            }
        } else {
            rename_one(root, rename)
        });
    }
    results
}

fn result(path: &Path, action: Action, outcome: Result<(), String>) -> WriteResult {
    WriteResult {
        path: path.to_path_buf(),
        action,
        outcome: if outcome.is_ok() {
            Outcome::Applied
        } else {
            Outcome::Failed
        },
        error: outcome.err(),
    }
}

/// Writes the new text only if the file is still what was read, as `fix` does.
fn rewrite_one(root: &Path, path: &Path, text: &str, original: Option<&String>) -> WriteResult {
    let full = root.join(path);
    let outcome = (|| {
        match (urzua_io::read_to_string(&full), original) {
            (Ok(on_disk), Some(read)) if on_disk == *read => {}
            (Ok(_), _) => {
                return Err(
                    "the file changed since it was read; nothing was written -- run again"
                        .to_string(),
                )
            }
            (Err(e), _) => return Err(format!("could not re-read: {e}")),
        }
        urzua_io::write_atomic_in(root, &full, text).map_err(|e| e.to_string())
    })();
    result(path, Action::Rewrite, outcome)
}

fn rename_one(root: &Path, rename: &Rename) -> WriteResult {
    let (from, to) = (root.join(&rename.from), root.join(&rename.to));
    let outcome = (|| {
        urzua_io::confine_to_root(root, &from)?;
        urzua_io::confine_to_root(root, &to)?;
        if std::fs::symlink_metadata(&to).is_ok() {
            return Err(format!(
                "{} appeared; nothing is overwritten",
                rename.to.display()
            ));
        }
        std::fs::rename(&from, &to).map_err(|e| e.to_string())
    })();
    result(&rename.from, Action::Rename, outcome)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_rename_that_fails_after_the_rewrites_is_reported_and_leaves_the_rewrites() {
        let root = std::env::temp_dir().join(format!("urzua-mf-unit-{}", std::process::id()));
        std::fs::create_dir_all(root.join("docs")).unwrap();
        std::fs::write(root.join("docs/cites.md"), "see 0001-a.md\n").unwrap();
        let mut planned = FilenamePlan::default();
        planned.new_texts.insert(
            PathBuf::from("docs/cites.md"),
            "see ADR-0001-a.md\n".to_string(),
        );
        // The file to rename is not there, so the rename itself fails.
        planned.renames.push(Rename {
            record_type: "adr".to_string(),
            from: PathBuf::from("docs/0001-a.md"),
            to: PathBuf::from("docs/ADR-0001-a.md"),
            has_stable_id: true,
            id: ("ADR".to_string(), 1),
        });
        let originals = HashMap::from([(
            PathBuf::from("docs/cites.md"),
            "see 0001-a.md\n".to_string(),
        )]);
        let results = write_plan(&root, &planned, &originals);
        let text = std::fs::read_to_string(root.join("docs/cites.md")).unwrap();
        let _ = std::fs::remove_dir_all(&root);
        assert_eq!(results.len(), 2);
        assert_eq!(results[0].outcome, Outcome::Applied);
        assert_eq!(results[1].action, Action::Rename);
        assert_eq!(results[1].outcome, Outcome::Failed);
        assert!(results[1].error.is_some());
        assert_eq!(
            text, "see ADR-0001-a.md\n",
            "the rewrite stays; a rerun renames"
        );
    }
}
