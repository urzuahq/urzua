//! `urzua new` (SPEC-12): creates a record from the configured template with
//! a stable ID assigned (ADR-0003). Never asks the author to pick a number
//! -- scans the type's directory for the highest existing prefix and takes
//! the next one.

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

    let config_path = config_path.unwrap_or_else(|| repo_root.join(".urzua/config.toml"));
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
    let display_number = urzua_core::new_record::next_display_number(&filenames);

    let title = title.unwrap_or_else(|| "Title".to_string());
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

    let params = urzua_core::new_record::NewRecordParams {
        display_number,
        title: &title,
        stable_id: stable_id.as_str(),
        author: &identity.name,
        today: &today,
    };

    // BUG-3: the configured header_shape wins unconditionally -- a template
    // file's own (possibly stale, possibly blockquote) header shape is never
    // allowed to override what the config actually declares. A yaml-
    // frontmatter type still gets a template's body scaffolding (`## What`,
    // the revision log) when one exists; it just doesn't get the template's
    // header.
    let template_path = repo_root
        .join(".urzua/templates")
        .join(format!("{record_type}.md"));
    let content = if type_config.header_shape == urzua_core::header::HeaderShape::YamlFrontmatter {
        let mut out =
            urzua_core::new_record::render_synthetic_yaml(&params, &type_config.required_fields);
        if let Ok(template) = urzua_io::read_to_string(&template_path) {
            if let Some(body) = urzua_core::new_record::template_body(&template) {
                out.push('\n');
                out.push_str(body);
            }
        }
        out
    } else {
        match urzua_io::read_to_string(&template_path) {
            Ok(template) => urzua_core::new_record::render_from_template(&template, &params),
            Err(e) => {
                return emit(&CouldNotRun::from(format!(
                    "no template at {} and no synthesizable shape declared: {e}",
                    template_path.display()
                )))
            }
        }
    };

    // Type prefix explicit in the filename (ADR-36) -- matches how a record
    // is referenced in prose everywhere else (e.g. "Implements: ADR-36").
    // Never zero-padded (ADR-36 amendment): padding doesn't solve
    // lexicographic sort order permanently, only defers the break. The
    // prefix is the type's configured `prefix` when it declares one (e.g.
    // `MILE` for `milestone`), otherwise the type name itself, upper-cased.
    let type_prefix = type_config
        .prefix
        .clone()
        .unwrap_or_else(|| record_type.to_ascii_uppercase());
    let filename = format!(
        "{}-{}-{}.md",
        type_prefix,
        display_number,
        urzua_core::new_record::slugify(&title)
    );
    let file_path = dir.join(&filename);
    if file_path.exists() {
        return emit(&CouldNotRun::from(format!(
            "{} already exists -- refusing to overwrite",
            file_path.display()
        )));
    }

    if let Err(e) = std::fs::write(&file_path, content) {
        return emit(&CouldNotRun::from(format!(
            "could not write {}: {e}",
            file_path.display()
        )));
    }

    let notices = identity
        .warning
        .into_iter()
        .map(|message| Notice {
            severity: NoticeSeverity::Warning,
            subject: NOTICE_IDENTITY.to_string(),
            message,
        })
        .collect();

    emit(&NewReport {
        status: ReportStatus::Ok,
        path: file_path
            .strip_prefix(&repo_root)
            .unwrap_or(&file_path)
            .to_path_buf(),
        display_number,
        stable_id: stable_id.as_str().to_string(),
        notices,
    })
}
