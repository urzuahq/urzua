//! `urzua new` (SPEC-12): creates a record from the configured template with
//! a stable ID assigned (ADR-0003). Never asks the author to pick a number
//! -- scans the type's directory for the highest existing prefix and takes
//! the next one.

use std::io::Write;
use std::path::PathBuf;
use std::process::ExitCode;

use urzua_core::report::{CouldNotRun, NewReport, Notice, NoticeSeverity, ReportStatus};

use crate::discovery::{find_repo_root, load_config};
use crate::{emit, NOTICE_IDENTITY};

pub fn run(
    config_path: Option<PathBuf>,
    record_type: String,
    title: Option<String>,
    by: Option<String>,
) -> ExitCode {
    let repo_root = match find_repo_root(&[PathBuf::from(".")]) {
        Ok(root) => root,
        Err(e) => return emit(&CouldNotRun::from(e)),
    };

    let config_path =
        config_path.unwrap_or_else(|| crate::discovery::default_config_path(&repo_root));
    let config = match load_config(&config_path) {
        Ok(c) => c,
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

    let dir = repo_root.join(&type_config.dir);
    let filenames: Vec<String> = match std::fs::read_dir(&dir) {
        Ok(entries) => entries
            .filter_map(|e| e.ok())
            .filter_map(|e| e.file_name().into_string().ok())
            .collect(),
        Err(e) => {
            return emit(&CouldNotRun::from(format!(
                "could not read {}: {e}",
                dir.display()
            )))
        }
    };
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
    let display_number =
        assign_display_number(&repo_root, &type_config.dir, &filenames, &number_of);

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

    // A closed field set (`known_fields` declared) rejects a field it does not
    // list, so `Stable-Id` is written only when the type admits it (`BUG-182`).
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
        h1_prefix: compiled.as_ref().map(|_| type_prefix.as_str()),
        initial_status: status_decl
            .as_ref()
            .and_then(|d| d.values.first())
            .map(String::as_str),
        status_field: type_config.relation_field(urzua_core::config::RelationRole::Status),
        title: &title,
        stable_id: admits_stable_id.then(|| stable_id.as_str()),
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
        let mut out =
            urzua_core::new_record::render_synthetic_yaml(&params, &type_config.required_fields);
        if let Ok(template) = urzua_io::read_to_string(&template_path) {
            if let Some(body) = urzua_core::new_record::template_body(&template) {
                out.push('\n');
                out.push_str(body);
            }
        }
        out
    };

    let file_path = dir.join(&filename);
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

    let mut notices: Vec<Notice> = identity
        .warning
        .into_iter()
        .map(|message| Notice {
            severity: NoticeSeverity::Warning,
            subject: NOTICE_IDENTITY.to_string(),
            message,
        })
        .collect();
    if !admits_stable_id {
        notices.push(Notice {
            severity: NoticeSeverity::Warning,
            subject: "stable-id".to_string(),
            message: format!(
                "record type '{record_type}' declares a closed field set (`known_fields`) that does not list \
                 `Stable-Id`, so none was written; add `Stable-Id` to its `known_fields` to give new records one"
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
        stable_id: admits_stable_id.then(|| stable_id.as_str().to_string()),
        notices,
    })
}

/// Numbers `new` in the working tree, then skips past any a local branch or
/// remote-tracking ref already uses (`MILE-89`). Git only, no network: refs
/// never fetched into this clone are invisible. A git failure falls back to
/// the working-tree-only number and says so on stderr.
fn assign_display_number(
    repo_root: &std::path::Path,
    dir: &str,
    filenames: &[String],
    number_of: &dyn Fn(&str) -> Option<u32>,
) -> u32 {
    let claims = match other_ref_claims(repo_root, dir) {
        Ok(c) => c,
        Err(reason) => {
            eprintln!(
                "urzua: numbering from the working tree only; could not read other refs ({reason}). \
                 A number claimed on an unmerged branch will not be seen."
            );
            return urzua_core::new_record::next_display_number_by(filenames, number_of);
        }
    };
    let got =
        urzua_core::new_record::next_display_number_with_claims_by(filenames, &claims, number_of);
    for (number, refs) in &got.skipped {
        eprintln!(
            "urzua: skipped {number}: already used on {} (local git refs only; unfetched remotes are not checked)",
            refs.join(", ")
        );
    }
    got.number
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
    dir: &str,
) -> Result<Vec<(String, Vec<String>)>, String> {
    let refs = git_output(
        repo_root,
        &[
            "for-each-ref",
            "--format=%(objectname) %(refname)",
            "refs/heads",
            "refs/remotes",
        ],
    )?;
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
    let pathspec = format!("{}/", dir.trim_end_matches('/'));
    let mut claims = Vec::new();
    for (sha, names) in by_commit {
        let listing = git_output(
            repo_root,
            &["ls-tree", "-z", "--name-only", &sha, "--", &pathspec],
        )?;
        let files: Vec<String> = listing
            .split(|b| *b == 0)
            .filter(|p| !p.is_empty())
            .map(|p| {
                let p = String::from_utf8_lossy(p);
                p.rsplit('/').next().unwrap_or(&p).to_string()
            })
            .collect();
        for name in names {
            claims.push((name, files.clone()));
        }
    }
    Ok(claims)
}
