//! The revision log an evidence write (`fix --rebaseline`, `fix --pin`) lands
//! in: the record's own, or one created in the type's shape when the record
//! has none and no committed copy of it had one.

use std::cell::{OnceCell, RefCell};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

use urzua_core::config::{Config, RevisionLogId, RevisionLogShape};
use urzua_core::fix::new_log::{self, LogPlan};
use urzua_core::report::{NextAction, Notice, NoticeSeverity};

/// The notice a write that creates a log raises when no base copy was read.
pub(crate) const NOTICE_LOG_BASE: &str = "fix.revision-log-base";

/// How many commits that changed the log identity's text are read for a copy
/// of the record that had a log, when no base ref resolves.
const LOG_HISTORY_DEPTH: usize = 20;

/// The ref a base copy is read from when `--base` is not given.
const DEFAULT_BASE: &str = "origin/HEAD";

/// What an evidence write does about a record's revision log.
pub(crate) enum EvidenceLog {
    /// The record has one.
    Present,
    /// The record's text with an empty log placed in it.
    Create(String),
    /// Refused, with the command that clears it when there is one.
    Refused(String, Option<NextAction>),
}

/// Where committed copies of a record are read to tell a record that never had
/// a log from one whose log was deleted. Each record is looked up once per
/// run, so the listing and the write share the answer.
pub(crate) struct LogHistory<'a> {
    repo_root: &'a Path,
    /// The commit whose copy of a record is read: the merge base of the base
    /// ref and `HEAD`, or the base ref itself when no merge base is visible.
    base: Option<String>,
    /// Why no base copy, or only the base ref's own, is read.
    caveat: Option<String>,
    renames: OnceCell<HashMap<PathBuf, PathBuf>>,
    seen: RefCell<HashMap<PathBuf, Option<String>>>,
}

impl<'a> LogHistory<'a> {
    /// Resolves `base` (or `origin/HEAD` when not given); a `--base` that names
    /// no commit is refused.
    pub(crate) fn resolve(repo_root: &'a Path, base: Option<&str>) -> Result<Self, String> {
        let name = base.unwrap_or(DEFAULT_BASE);
        let (base, caveat) = match urzua_io::resolve_commit(repo_root, name).ok() {
            None if base.is_some() => {
                return Err(format!("--base '{name}' names no commit"));
            }
            None => (
                None,
                Some(format!(
                    "no --base given and {DEFAULT_BASE} does not resolve, so a revision log deleted before HEAD is looked for only in the last {LOG_HISTORY_DEPTH} commits that changed its identity line; pass --base <ref>"
                )),
            ),
            Some(tip) => match urzua_io::merge_base_with_head(repo_root, name) {
                Some(mb) => (Some(mb), None),
                None => (
                    Some(tip),
                    Some(format!(
                        "no merge base of {name} and HEAD is visible (a shallow clone?), so {name}'s own copy of a record is read instead"
                    )),
                ),
            },
        };
        Ok(LogHistory {
            repo_root,
            base,
            caveat,
            renames: OnceCell::new(),
            seen: RefCell::new(HashMap::new()),
        })
    }

    pub(crate) fn repo_root(&self) -> &Path {
        self.repo_root
    }

    /// The notice for a run that created a log while reading no merge-base copy.
    pub(crate) fn notice(&self, created: bool) -> Option<Notice> {
        let message = self.caveat.clone().filter(|_| created)?;
        Some(Notice {
            severity: NoticeSeverity::Warning,
            subject: NOTICE_LOG_BASE.to_string(),
            message,
            detail: None,
        })
    }

    /// `rev:path` of a committed copy of `record` that has a revision log:
    /// `HEAD`'s, then the base copy at the record's path, its path before a
    /// rename git detects, or the path whose `Stable-Id` it carries; with no
    /// base, the record's recent history.
    fn committed_log(
        &self,
        record: &Path,
        stable_id: Option<&str>,
        id: &RevisionLogId,
        shape: &RevisionLogShape,
    ) -> Option<String> {
        if let Some(hit) = self.seen.borrow().get(record) {
            return hit.clone();
        }
        let hit = self.find(record, stable_id, id, shape);
        self.seen
            .borrow_mut()
            .insert(record.to_path_buf(), hit.clone());
        hit
    }

    fn find(
        &self,
        record: &Path,
        stable_id: Option<&str>,
        id: &RevisionLogId,
        shape: &RevisionLogShape,
    ) -> Option<String> {
        let copy = |rev: &str, path: &Path| -> Option<String> {
            urzua_io::read_at_commit_if_present(self.repo_root, rev, path)
                .ok()
                .flatten()
        };
        let has_log = |text: &str| new_log::plan(text, id, shape) == LogPlan::Present;
        let spec = |rev: &str, path: &Path| format!("{rev}:{}", path.display());
        if copy("HEAD", record).is_some_and(|t| has_log(&t)) {
            return Some(spec("HEAD", record));
        }
        let Some(base) = &self.base else {
            return self.history(record, id, &copy, &has_log);
        };
        let mut paths = vec![record.to_path_buf()];
        paths.extend(self.renames(base).get(record).cloned());
        if let Some(sid) = stable_id.map(str::trim).filter(|s| !s.is_empty()) {
            paths.extend(
                urzua_io::markdown_files_containing_at(self.repo_root, base, sid)
                    .into_iter()
                    .filter(|p| {
                        copy(base, p).is_some_and(|t| {
                            urzua_core::header::parse(&t)
                                .get_reserved("Stable-Id")
                                .is_some_and(|v| v.trim() == sid)
                        })
                    }),
            );
        }
        paths
            .iter()
            .find(|p| copy(base, p).is_some_and(|t| has_log(&t)))
            .map(|p| spec(base, p))
    }

    fn renames(&self, base: &str) -> &HashMap<PathBuf, PathBuf> {
        self.renames
            .get_or_init(|| urzua_io::renamed_paths_since(self.repo_root, base).unwrap_or_default())
    }

    /// Only without a base ref: at most `LOG_HISTORY_DEPTH` commits, once per
    /// record that lacks a log.
    fn history(
        &self,
        record: &Path,
        id: &RevisionLogId,
        copy: &dyn Fn(&str, &Path) -> Option<String>,
        has_log: &dyn Fn(&str) -> bool,
    ) -> Option<String> {
        urzua_io::commits_changing_text(self.repo_root, record, id.name(), LOG_HISTORY_DEPTH)
            .unwrap_or_default()
            .into_iter()
            .flat_map(|(c, path)| [(format!("{c}^"), path.clone()), (c, path)])
            .find(|(rev, path)| copy(rev, path).is_some_and(|t| has_log(&t)))
            .map(|(rev, path)| format!("{rev}:{}", path.display()))
    }
}

/// What an evidence write does about the revision log of `record` (of
/// `record_type`, text `content`). A missing log is created in the type's
/// template's shape, else the declared one, unless a committed copy of the
/// record had one: a log deleted by hand must be restored, never replaced by a
/// tool-written one that would hide what it held.
pub(crate) fn evidence_log(
    history: &LogHistory<'_>,
    config: &Config,
    record_type: Option<&str>,
    record: &Path,
    content: &str,
    verb: &str,
) -> EvidenceLog {
    let id = record_type
        .map(|t| config.revision_log_id(t))
        .unwrap_or_default();
    let shape = record_type
        .map(|t| config.revision_log_shape(t))
        .unwrap_or_default();
    let plan = new_log::plan(content, &id, &shape);
    if let Some(refusal) = new_log::refusal(&plan, &id, verb) {
        let action = (plan == LogPlan::NoBody).then(|| add_heading_action(record));
        return EvidenceLog::Refused(refusal, action);
    }
    if plan == LogPlan::Present {
        return EvidenceLog::Present;
    }
    let stable_id = urzua_core::header::parse(content)
        .get_reserved("Stable-Id")
        .map(|s| s.to_string());
    if let Some(spec) = history.committed_log(record, stable_id.as_deref(), &id, &shape) {
        return EvidenceLog::Refused(
            format!(
                "the committed copy {spec} has a Revision log ({}) that the record no longer has -- refusing to {verb} into a new log; restore that section first",
                id.describe()
            ),
            Some(NextAction::runs_as_printed(
                urzua_core::report::shell_command(&["git", "show", spec.as_str()]),
                format!(
                    "shows the copy of {} whose Revision log to restore",
                    record.display()
                ),
            )),
        );
    }
    let template = record_type.and_then(|t| {
        urzua_io::read_to_string(
            &history
                .repo_root
                .join(".urzua/templates")
                .join(format!("{t}.md")),
        )
        .ok()
    });
    let section = new_log::section(&id, &shape, template.as_deref());
    EvidenceLog::Create(new_log::insert(content, &section, &id, template.as_deref()))
}

/// `content` with an empty revision log placed in it when it has none, and
/// whether one was; refused as `evidence_log` refuses. Only for writes whose
/// rows are evidence rows.
pub(crate) fn with_evidence_log(
    history: &LogHistory<'_>,
    config: &Config,
    record_type: Option<&str>,
    record: &Path,
    content: &str,
    verb: &str,
) -> Result<(String, bool), String> {
    match evidence_log(history, config, record_type, record, content, verb) {
        EvidenceLog::Present => Ok((content.to_string(), false)),
        EvidenceLog::Create(text) => Ok((text, true)),
        EvidenceLog::Refused(error, _) => Err(error),
    }
}

/// The command that gives `record` a heading, its file stem, for a new
/// revision log to follow.
fn add_heading_action(record: &Path) -> NextAction {
    let stem = record
        .file_stem()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_default();
    let quoted = urzua_core::report::shell_command(&[stem]);
    let path = urzua_core::report::shell_command(&[record.display().to_string()]);
    NextAction::runs_as_printed(
        format!("printf '\\n# %s\\n' {quoted} >> {path}"),
        format!(
            "{} has no body section for a new revision log to follow; this appends a heading named after the file (rename it as you like), then re-run the write",
            record.display()
        ),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_heading_action_quotes_a_file_stem_with_a_single_quote() {
        let dir = std::env::temp_dir().join(format!("urzua-heading-quote-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let record = Path::new("it's a record.md");
        std::fs::write(dir.join(record), "---\n---\n").unwrap();
        let action = add_heading_action(record);
        assert!(action.inputs.is_empty());
        let ran = std::process::Command::new("sh")
            .args(["-c", &action.command])
            .current_dir(&dir)
            .status()
            .unwrap();
        let text = std::fs::read_to_string(dir.join(record)).unwrap();
        let _ = std::fs::remove_dir_all(&dir);
        assert!(ran.success());
        assert_eq!(text, "---\n---\n\n# it's a record\n");
    }
}
