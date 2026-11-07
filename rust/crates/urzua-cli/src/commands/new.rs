//! `urzua new` (SPEC-12): creates a record from the configured template with
//! a stable ID assigned (ADR-0003). Never asks the author to pick a number
//! -- scans the type's directory for the highest existing prefix and takes
//! the next one.

use std::io::Write;
use std::path::PathBuf;
use std::process::ExitCode;

use urzua_core::config::NewDecl;
use urzua_core::report::{
    CouldNotRun, NewReport, Notice, NoticeSeverity, PullRefsReport, ReportStatus, SkippedNumber,
    SkippedReason,
};

use super::new_refs::{self, FetchFailure, LockOutcome};

use crate::session::Workspace;
use crate::{emit, NOTICE_IDENTITY};

pub fn run(
    config_path: Option<PathBuf>,
    record_type: String,
    title: Option<String>,
    by: Option<String>,
) -> ExitCode {
    let Workspace {
        repo_root,
        config_path,
        config,
    } = match Workspace::open(&[], config_path) {
        Ok(session) => session,
        Err(e) => return emit(&CouldNotRun::from(e)),
    };

    let Some(type_config) = config.record_types.get(&record_type) else {
        return emit(&CouldNotRun::from(format!(
            "unrecognized record type '{record_type}' -- see [record_types] in {}",
            config_path.display()
        )));
    };

    // `none` declares this type has nowhere for a header, and therefore
    // nowhere to write a `Stable-Id` -- generating a record with no identity
    // would be worse than refusing (`ADR-50`).
    if type_config.header_shape == urzua_core::header::HeaderShape::None {
        return emit(&CouldNotRun::from(format!(
            "record type '{record_type}' declares header_shape: none -- urzua new cannot generate an identity for a type with no header"
        )));
    }

    // The open-pull-request check (`ADR-93`), only when the repository opts in: fetch
    // the pull-request head refs, then take the numbering lock. The fetch comes first
    // so the lock is not held across the network; the lock is held from here to the
    // file write, and the directories are read under it.
    let opts = &config.new;
    let mut notices: Vec<Notice> = Vec::new();
    let mut pull = PullRefsReport {
        fetched: false,
        reason: Some("disabled".to_string()),
        remote: None,
        refs: 0,
        worktrees_scanned: 0,
        skipped_numbers: Vec::new(),
    };
    let mut _lock = None;
    if opts.check_open_prs {
        pull.remote = Some(opts.remote.clone());
        match new_refs::fetch_pull_refs(&repo_root, opts) {
            Ok(()) => {
                pull.fetched = true;
                pull.reason = None;
            }
            Err(failure) => {
                pull.reason = Some(failure.reason().to_string());
                let detail = match &failure {
                    FetchFailure::Offline(why) => format!("offline: {why}"),
                    FetchFailure::Timeout => {
                        format!("timed out after {}s", opts.fetch_timeout_seconds)
                    }
                };
                let message = format!(
                    "could not fetch pull request refs from '{}' ({detail}); numbering from the local refs, so a number claimed on an unfetched pull request will not be seen",
                    opts.remote
                );
                eprintln!("urzua: {message}");
                notices.push(Notice {
                    severity: NoticeSeverity::Warning,
                    subject: NOTICE_PULL_REFS.to_string(),
                    message,
                });
            }
        }
        pull.refs = new_refs::count_pull_refs(&repo_root);
        match new_refs::acquire_lock(&repo_root, opts.lock_timeout_seconds) {
            LockOutcome::Held(lock) => _lock = Some(lock),
            LockOutcome::Unavailable(why) => notices.push(Notice {
                severity: NoticeSeverity::Warning,
                subject: NOTICE_PULL_REFS.to_string(),
                message: format!(
                    "the numbering lock was not taken ({why}); a concurrent `urzua new` could take the same number"
                ),
            }),
            LockOutcome::TimedOut(why) => return emit(&CouldNotRun::from(why)),
        }
        notices.push(Notice {
            severity: NoticeSeverity::Info,
            subject: NOTICE_PULL_REFS.to_string(),
            message: "separate clones are not scanned: a record created and not pushed in another clone of this repository is invisible to numbering, and to the fetch".to_string(),
        });
    }

    // One namespace across every directory the type declares (`MILE-130`):
    // the number is taken past the highest in any of them. The primary `dir`
    // must be readable; an extra directory that is absent holds no ids, so it
    // is skipped with a notice (`type.dir-matches-nothing` reports it).
    let mut filenames: Vec<String> = Vec::new();
    for (index, declared_dir) in type_config.all_dirs().enumerate() {
        let dir = repo_root.join(declared_dir);
        match std::fs::read_dir(&dir) {
            Ok(entries) => filenames.extend(
                entries
                    .filter_map(|e| e.ok())
                    .filter_map(|e| e.file_name().into_string().ok()),
            ),
            Err(e) if index > 0 && e.kind() == std::io::ErrorKind::NotFound => eprintln!(
                "urzua: extra_dirs entry '{declared_dir}' of record type '{record_type}' does not exist; numbering without it"
            ),
            // The first record of a type: its directory does not exist yet and
            // is created when the record is written.
            Err(e) if index == 0 && e.kind() == std::io::ErrorKind::NotFound => eprintln!(
                "urzua: {declared_dir} does not exist yet; creating it for the first {record_type} record"
            ),
            Err(e) => {
                return emit(&CouldNotRun::from(format!(
                    "could not read {}: {e}",
                    dir.display()
                )))
            }
        }
    }
    let dir = repo_root.join(&type_config.dir);
    // The type's declared identity pattern, when it has one, decides which
    // numbers are taken and what a conforming file name is (`MILE-151`): the
    // generator reads the same grammar the checker does.
    let declared = match urzua_core::document_model::declared_model(&record_type, type_config) {
        Ok(model) => model.and_then(|m| m.identity),
        Err(e) => return emit(&CouldNotRun::from(e.to_string())),
    };
    let compiled = match declared
        .as_ref()
        .map(urzua_core::document_model::CompiledIdentity::compile)
        .transpose()
    {
        Ok(c) => c,
        Err(e) => {
            return emit(&CouldNotRun::from(format!(
                "record type '{record_type}': {e}"
            )))
        }
    };
    let number_of = |name: &str| -> Option<u32> {
        match &compiled {
            Some(identity) => urzua_core::new_record::declared_number(identity, name),
            None => urzua_core::new_record::parse_record_filename(name).map(|(_, n)| n),
        }
    };
    let (display_number, skipped) = match assign_display_number(
        &repo_root,
        &type_config.all_dirs().collect::<Vec<_>>(),
        &filenames,
        &number_of,
        opts.check_open_prs.then_some(opts),
        &mut pull,
        &mut notices,
    ) {
        Ok(n) => n,
        Err(e) => {
            return emit(&CouldNotRun::from(format!(
                "record type '{record_type}': {e}"
            )))
        }
    };

    let title = title.unwrap_or_else(|| "Title".to_string());
    // A slug-less filename (`ADR-6-.md`) is not one the numbering scan or
    // the identity index recognise as a record, so the next `new` would
    // reissue the same number. Refuse before anything is written.
    let slug = urzua_core::new_record::slugify(&title);
    if slug.is_empty() {
        return emit(&CouldNotRun::from(format!(
            "title '{title}' has no ASCII letters or digits to build a filename slug from -- give it a title containing at least one"
        )));
    }
    // Type prefix explicit in the filename (ADR-36). The type's configured
    // `prefix` when it declares one (e.g. `MILE` for `milestone`), otherwise the
    // type name itself, upper-cased.
    let type_prefix = type_config
        .prefix
        .clone()
        .unwrap_or_else(|| record_type.to_ascii_uppercase());
    // Never zero-padded by default (ADR-36 amendment): padding doesn't solve
    // lexicographic sort order permanently, only defers the break. A declared
    // pattern that requires padding is the one reason to pad, and the name is
    // fitted to it before anything is written.
    let filename = match (&compiled, &declared) {
        (Some(identity), Some(decl)) => {
            match urzua_core::new_record::declared_file_name(
                identity,
                &decl.pattern,
                &type_prefix,
                display_number,
                &slug,
            ) {
                Ok(name) => name,
                Err(e) => return emit(&CouldNotRun::from(e)),
            }
        }
        _ => format!("{type_prefix}-{display_number}-{slug}.md"),
    };

    // Every record gets a `Stable-Id` (`ADR-21`). A closed field set
    // (`known_fields`) that does not list it is a configuration inconsistency
    // the author must resolve, so it is disclosed below rather than answered by
    // writing a record with no identity (`BUG-182` had omitted it).
    let admits_stable_id = type_config.known_fields.is_none()
        || type_config
            .declared_fields()
            .contains(&urzua_core::values::FieldName::new("Stable-Id"));
    let stable_id = urzua_id::StableId::generate();
    let today = urzua_io::today();
    let identity = match urzua_io::resolve_identity(by.as_deref(), &repo_root) {
        Ok(id) => id,
        Err(e) => {
            return emit(&CouldNotRun::from(format!(
                "could not resolve an identity: {e}"
            )))
        }
    };

    // The entry state of the type's own vocabulary, not a choice: moving a
    // record on from it stays a human decision.
    let status_decl = type_config.status_decl();
    let params = urzua_core::new_record::NewRecordParams {
        display_number,
        // The H1 id is padded exactly as the file name is: the width of the
        // `number` the declared pattern reads back from it (`BUG-188`).
        h1_width: compiled
            .as_ref()
            .and_then(|c| c.identify(&filename).number)
            .map_or(0, |n| n.len()),
        h1_prefix: compiled.as_ref().map(|_| type_prefix.as_str()),
        initial_status: status_decl
            .as_ref()
            .and_then(|d| d.values.first())
            .map(String::as_str),
        status_field: type_config.relation_field(urzua_core::config::RelationRole::Status),
        date_field: type_config.date_field(),
        author_field: type_config.author_field(),
        title_field: type_config.title_field(),
        initial_version: type_config.initial_version(),
        title: &title,
        stable_id: stable_id.as_str(),
        author: &identity.name,
        today: &today,
    };

    // BUG-3: the configured header_shape wins unconditionally -- a template
    // file's own (possibly stale) header is never
    // allowed to override what the config actually declares. A yaml-
    // frontmatter type still gets a template's body scaffolding (`## What`,
    // the revision log) when one exists; it just doesn't get the template's
    // header.
    let template_path = repo_root
        .join(".urzua/templates")
        .join(format!("{record_type}.md"));
    // `none` returned above, so the only shape left is `yaml-frontmatter`
    // (MILE-85 removed the blockquote and bold-list shapes a template's own
    // header could once have been rendered in).
    let content = {
        let template = urzua_io::read_to_string(&template_path).ok();
        // The header fields the template names beyond the required ones stay
        // on the new record, blank, when the type declares them (`BUG-188`);
        // one the type does not declare would only trip
        // `header.field-set-consistency`, so it is left out.
        let declared = type_config.declared_fields();
        let template_fields: Vec<String> = template
            .as_deref()
            .map(urzua_core::new_record::template_header_fields)
            .unwrap_or_default()
            .into_iter()
            .filter(|f| {
                type_config.known_fields.is_none()
                    || declared.contains(&urzua_core::values::FieldName::new(f.as_str()))
            })
            .collect();
        let mut out = urzua_core::new_record::render_synthetic_yaml(
            &params,
            &type_config.required_fields,
            &template_fields,
        );
        if let Some(template) = template {
            if let Some(body) = urzua_core::new_record::template_body(&template) {
                out.push('\n');
                out.push_str(body);
            }
        }
        out
    };

    let file_path = dir.join(&filename);
    // A record directory that is (or sits below) a symlink out of the
    // repository must not receive the new file.
    if let Err(e) = urzua_io::confine_to_root(&repo_root, &file_path) {
        return emit(&CouldNotRun::from(e));
    }
    if let Err(e) = std::fs::create_dir_all(&dir) {
        return emit(&CouldNotRun::from(format!(
            "could not create {}: {e}",
            dir.display()
        )));
    }
    // `create_new` is atomic (matching `FixLock`'s own pattern) -- a
    // separate `exists()` check followed by `write` would leave a race
    // window where a concurrent `new` invocation for the same slug clobbers
    // the other's file instead of refusing.
    match std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&file_path)
        .and_then(|mut f| f.write_all(content.as_bytes()))
    {
        Ok(()) => {}
        Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {
            return emit(&CouldNotRun::from(format!(
                "{} already exists -- refusing to overwrite",
                file_path.display()
            )))
        }
        Err(e) => {
            return emit(&CouldNotRun::from(format!(
                "could not write {}: {e}",
                file_path.display()
            )))
        }
    }

    notices.extend(identity.warning.into_iter().map(|message| Notice {
        severity: NoticeSeverity::Warning,
        subject: NOTICE_IDENTITY.to_string(),
        message,
    }));
    if !admits_stable_id {
        notices.push(Notice {
            severity: NoticeSeverity::Warning,
            subject: "stable-id".to_string(),
            message: format!(
                "record type '{record_type}' declares a closed field set (`known_fields`) that does not list \
                 `Stable-Id`, yet every record type gets one (ADR-21); the new record carries it and \
                 `header.field-set-consistency` will report it until `Stable-Id` is added to the type's `known_fields`"
            ),
        });
    }

    emit(&NewReport {
        status: ReportStatus::Ok,
        path: file_path
            .strip_prefix(&repo_root)
            .unwrap_or(&file_path)
            .to_path_buf(),
        display_number,
        stable_id: Some(stable_id.as_str().to_string()),
        pull_refs: pull,
        skipped,
        notices,
    })
}

/// Numbers `new` in the working tree, then skips past any a local branch or
/// remote-tracking ref already uses (`MILE-89`). With `scan` (`new.check_open_prs`,
/// `ADR-93`) it also reads the refs under `refs/urzua/pr` and the trees of every
/// sibling worktree, untracked files included. Git only. A git failure falls back
/// to the working-tree-only number and says so in a notice. It returns the number
/// and the numbers it skipped with their reasons, which are never printed
/// (`ADR-93`, Amendment of 2026-11-07).
fn assign_display_number(
    repo_root: &std::path::Path,
    dirs: &[&str],
    filenames: &[String],
    number_of: &dyn Fn(&str) -> Option<u32>,
    scan: Option<&NewDecl>,
    pull: &mut PullRefsReport,
    notices: &mut Vec<Notice>,
) -> Result<(u32, Vec<SkippedReason>), String> {
    let mut prefixes = vec!["refs/heads", "refs/remotes"];
    if scan.is_some() {
        prefixes.push(new_refs::PR_NAMESPACE);
    }
    let mut claims = match other_ref_claims(repo_root, dirs, &prefixes) {
        Ok(c) => c,
        Err(reason) => {
            notices.push(Notice {
                severity: NoticeSeverity::Warning,
                subject: NOTICE_NUMBERING.to_string(),
                message: format!(
                    "numbering from the working tree only; could not read other refs ({reason}). \
                     A number claimed on an unmerged branch will not be seen."
                ),
            });
            return urzua_core::new_record::next_display_number_by(filenames, number_of)
                .map(|number| (number, Vec::new()));
        }
    };
    if scan.is_some() {
        match new_refs::sibling_worktrees(repo_root) {
            Ok(siblings) => {
                for path in siblings {
                    if !path.is_dir() {
                        notices.push(Notice {
                            severity: NoticeSeverity::Warning,
                            subject: NOTICE_PULL_REFS.to_string(),
                            message: format!(
                                "worktree {} is listed by git but its directory is missing; skipped",
                                path.display()
                            ),
                        });
                        continue;
                    }
                    let mut names = Vec::new();
                    for dir in dirs {
                        if let Ok(entries) = std::fs::read_dir(path.join(dir)) {
                            names.extend(
                                entries
                                    .filter_map(|e| e.ok())
                                    .filter_map(|e| e.file_name().into_string().ok()),
                            );
                        }
                    }
                    claims.push((format!("worktree {}", path.display()), names));
                    pull.worktrees_scanned += 1;
                }
            }
            Err(why) => notices.push(Notice {
                severity: NoticeSeverity::Warning,
                subject: NOTICE_PULL_REFS.to_string(),
                message: format!(
                    "could not list the worktrees ({why}); a number drafted and untracked in a sibling worktree will not be seen"
                ),
            }),
        }
    }
    let got =
        urzua_core::new_record::next_display_number_with_claims_by(filenames, &claims, number_of)?;
    let skipped = skipped_reasons(&got.skipped);
    pull.skipped_numbers = got
        .skipped
        .iter()
        .map(|(number, held_by)| SkippedNumber {
            number: *number,
            held_by: held_by.clone(),
        })
        .collect();
    Ok((got.number, skipped))
}

/// Subject of the notices about the pull-request refs, the lock and the worktrees.
const NOTICE_PULL_REFS: &str = "new.pull-refs";

/// Subject of the notice that numbering used the working tree alone.
const NOTICE_NUMBERING: &str = "new.numbering";

/// The reason a holder of a number is named in `skipped`: `open-pr <ref>` for a
/// ref under `refs/urzua/pr`, `worktree <path>` for a sibling worktree's tree,
/// and `local` for any local branch or remote-tracking ref.
fn skip_reason(holder: &str) -> String {
    if holder.starts_with("worktree ") {
        holder.to_string()
    } else if holder
        .strip_prefix(new_refs::PR_NAMESPACE)
        .is_some_and(|rest| rest.starts_with('/'))
    {
        format!("open-pr {holder}")
    } else {
        "local".to_string()
    }
}

/// One entry per skipped number and distinct reason, in a fixed order: by
/// number, then open pull requests, worktrees and `local`, then by text. Many
/// local refs holding one number are one `local` entry; `pull_refs.skipped_numbers`
/// keeps the refs themselves.
fn skipped_reasons(skipped: &[(u32, Vec<String>)]) -> Vec<SkippedReason> {
    let rank = |reason: &str| {
        if reason.starts_with("open-pr ") {
            0
        } else if reason.starts_with("worktree ") {
            1
        } else {
            2
        }
    };
    let mut out: Vec<SkippedReason> = skipped
        .iter()
        .flat_map(|(number, holders)| {
            holders.iter().map(|h| SkippedReason {
                number: *number,
                reason: skip_reason(h),
            })
        })
        .collect();
    out.sort_by(|a, b| {
        (a.number, rank(&a.reason), &a.reason).cmp(&(b.number, rank(&b.reason), &b.reason))
    });
    out.dedup();
    out
}

fn git_output(repo_root: &std::path::Path, args: &[&str]) -> Result<Vec<u8>, String> {
    let out = std::process::Command::new("git")
        .arg("-C")
        .arg(repo_root)
        .args(args)
        .output()
        .map_err(|e| format!("could not run git: {e}"))?;
    if out.status.success() {
        Ok(out.stdout)
    } else {
        Err(format!(
            "git {} failed: {}",
            args[0],
            String::from_utf8_lossy(&out.stderr).trim()
        ))
    }
}

/// `(ref name, filenames in `dir`)` for every local branch and remote-tracking
/// ref. One `for-each-ref`, then one `ls-tree` per *distinct commit* (many
/// refs share a tip), which reads only the one directory's tree object --
/// unlike `git log --all --name-only`, which walks all history and cannot say
/// which ref a name came from.
fn other_ref_claims(
    repo_root: &std::path::Path,
    dirs: &[&str],
    prefixes: &[&str],
) -> Result<Vec<(String, Vec<String>)>, String> {
    let mut args = vec!["for-each-ref", "--format=%(objectname) %(refname)"];
    args.extend_from_slice(prefixes);
    let refs = git_output(repo_root, &args)?;
    let mut by_commit: std::collections::BTreeMap<String, Vec<String>> = Default::default();
    for line in String::from_utf8_lossy(&refs).lines() {
        let Some((sha, full)) = line.split_once(' ') else {
            continue;
        };
        // `origin/HEAD` is a symbolic alias of another remote ref.
        if full.ends_with("/HEAD") {
            continue;
        }
        let short = full
            .strip_prefix("refs/heads/")
            .or_else(|| full.strip_prefix("refs/remotes/"))
            .unwrap_or(full);
        by_commit
            .entry(sha.to_string())
            .or_default()
            .push(short.to_string());
    }
    let commits: Vec<String> = by_commit.keys().cloned().collect();
    let dir_names: Vec<String> = dirs.iter().map(|d| d.to_string()).collect();
    let files_by_commit = urzua_io::files_in_dirs_at_commits(repo_root, &commits, &dir_names)?;
    let mut claims = Vec::new();
    for (sha, names) in by_commit {
        let files: Vec<String> = files_by_commit
            .get(&sha)
            .map(|paths| {
                paths
                    .iter()
                    .map(|p| p.rsplit('/').next().unwrap_or(p).to_string())
                    .collect()
            })
            .unwrap_or_default();
        for name in names {
            claims.push((name, files.clone()));
        }
    }
    Ok(claims)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn held(number: u32, holders: &[&str]) -> (u32, Vec<String>) {
        (number, holders.iter().map(|h| h.to_string()).collect())
    }

    #[test]
    fn a_holder_is_classified_as_an_open_pr_a_worktree_or_local() {
        assert_eq!(
            skip_reason("refs/urzua/pr/origin/7"),
            "open-pr refs/urzua/pr/origin/7"
        );
        assert_eq!(skip_reason("worktree /w/side"), "worktree /w/side");
        assert_eq!(skip_reason("other-pr"), "local");
        assert_eq!(skip_reason("origin/main"), "local");
        // A branch that only starts with the namespace's text is not a pull ref.
        assert_eq!(skip_reason("refs/urzua/prx"), "local");
    }

    #[test]
    fn skipped_reasons_are_one_per_number_and_reason_in_a_fixed_order() {
        let got = skipped_reasons(&[
            held(5, &["b-branch", "worktree /w/side", "a-branch"]),
            held(
                2,
                &["refs/urzua/pr/origin/9", "refs/urzua/pr/origin/3", "x"],
            ),
        ]);
        let text: Vec<String> = got
            .iter()
            .map(|s| format!("{} {}", s.number, s.reason))
            .collect();
        assert_eq!(
            text,
            [
                "2 open-pr refs/urzua/pr/origin/3",
                "2 open-pr refs/urzua/pr/origin/9",
                "2 local",
                "5 worktree /w/side",
                "5 local",
            ]
        );
        assert!(skipped_reasons(&[]).is_empty());
    }
}
