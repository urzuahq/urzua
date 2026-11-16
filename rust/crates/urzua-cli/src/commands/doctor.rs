//! `urzua doctor` (SPEC-15): reports on the tool's own configuration and
//! invocation health, not on record content -- `check` validates records;
//! whether *itself* is correctly invoked is a different question with a
//! different failure mode. An unrecognized config key is an error here, not
//! a warning: the alternative is a typo silently disabling a rule.

use std::path::PathBuf;
use std::process::ExitCode;

use urzua_core::report::{CouldNotRun, NextAction, Notice, Report};

use super::hooks::{self, invokes_urzua, HookState};
use crate::discovery::{find_repo_root, load_config};
use crate::emit;

#[derive(serde::Serialize)]
struct DoctorCheck {
    check: String,
    status: DoctorStatus,
    message: String,
    /// The git hook this check is about (`hook-wired` only).
    #[serde(skip_serializing_if = "Option::is_none")]
    layer: Option<String>,
    /// The layer's wiring state, machine-readable (`hook-wired` only).
    #[serde(skip_serializing_if = "Option::is_none")]
    state: Option<HookState>,
    /// Where the hooks live: `default`, `hooks-path`, `husky` or
    /// `pre-commit-framework` (`hook-wired` only).
    #[serde(skip_serializing_if = "Option::is_none")]
    hook_layout: Option<&'static str>,
    /// The one command that repairs this check, when there is one.
    #[serde(skip_serializing_if = "Option::is_none")]
    fix: Option<String>,
}

/// What is wrong with the local git hook layers, as plain messages: the
/// session-start check of the harness reads the same wiring `doctor` does.
pub(crate) fn git_hook_problems(repo_root: &std::path::Path) -> Vec<String> {
    let decl = load_config(&crate::discovery::default_config_path(repo_root))
        .map(|c| c.hooks)
        .unwrap_or_default();
    hook_checks(repo_root, &decl)
        .into_iter()
        .filter(|c| c.status != DoctorStatus::Ok)
        .map(|c| c.message)
        .collect()
}

fn hook_checks(
    repo_root: &std::path::Path,
    decl: &urzua_core::config::HooksDecl,
) -> Vec<DoctorCheck> {
    let layout = match hooks::detect(repo_root) {
        Ok(l) => l,
        Err(message) => {
            return hooks::HOOKS
                .iter()
                .map(|layer| DoctorCheck {
                    check: "hook-wired".to_string(),
                    status: DoctorStatus::Error,
                    message: format!("{layer}: {message} -- cannot tell whether the hook is wired"),
                    layer: Some((*layer).to_string()),
                    state: None,
                    hook_layout: None,
                    fix: None,
                })
                .collect();
        }
    };
    hooks::HOOKS
        .iter()
        .zip(hooks::wiring(&layout, decl))
        .map(|(layer, w)| {
            let (status, state, message, fix) = match w {
                Err(message) => (DoctorStatus::Error, None, message, None),
                Ok(w) => (
                    if w.state == HookState::InstalledRunningUrzua {
                        DoctorStatus::Ok
                    } else {
                        DoctorStatus::Warn
                    },
                    Some(w.state),
                    w.message,
                    w.fix,
                ),
            };
            DoctorCheck {
                check: "hook-wired".to_string(),
                status,
                message,
                layer: Some((*layer).to_string()),
                state,
                hook_layout: Some(layout.kind()),
                fix,
            }
        })
        .collect()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, serde::Serialize)]
#[serde(rename_all = "kebab-case")]
enum DoctorStatus {
    Ok,
    Warn,
    Error,
}

#[derive(serde::Serialize)]
struct DoctorReport {
    status: DoctorStatus,
    checks: Vec<DoctorCheck>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    notices: Vec<Notice>,
    /// Not serialized -- `run` has three real exit codes (2 = config
    /// missing, 1 = config parse error or any check `Error`, 0 = otherwise),
    /// but `status` alone can't distinguish the first two (both are
    /// `DoctorStatus::Error`). Set explicitly by whichever branch builds the
    /// report, so `exit_code()` never has to (mis)derive it from `status`.
    #[serde(skip)]
    exit_code: u8,
    /// Emitted as the top-level `next_action`.
    #[serde(skip)]
    next: Option<NextAction>,
}

impl Report for DoctorReport {
    fn notices(&self) -> &[Notice] {
        &self.notices
    }
    fn exit_code(&self) -> ExitCode {
        ExitCode::from(self.exit_code)
    }
    fn next_action(&self) -> Option<NextAction> {
        self.next.clone().or_else(|| {
            self.checks
                .iter()
                .filter(|c| c.status != DoctorStatus::Ok)
                .find_map(|c| {
                    c.fix
                        .clone()
                        .map(|command| NextAction::runs_as_printed(command, c.message.clone()))
                })
        })
    }
}

/// The step past a missing config, from what `init` would do here today: a
/// step that sends the user to an `init` that refuses is a loop.
fn missing_config_step(
    repo_root: &std::path::Path,
    config_path: &std::path::Path,
) -> (String, Option<NextAction>) {
    use crate::commands::init::{adopt_outlook, AdoptOutlook, GREENFIELD_COMMAND};
    let missing = format!("{} does not exist", config_path.display());
    match adopt_outlook(repo_root, config_path) {
        Ok(AdoptOutlook::Proposes(n)) => (
            format!("{missing} -- `urzua init` proposes {n} record type(s) for this corpus; run `urzua init --dry-run` to read the proposal, then `urzua init` to write it"),
            Some(NextAction::runs_as_printed(
                "urzua init".to_string(),
                format!("adopt proposes {n} record type(s) for the records found here"),
            )),
        ),
        Ok(AdoptOutlook::NothingToAdopt(why)) => (
            format!("{missing} -- `urzua init` has nothing to adopt here ({why}); run `{GREENFIELD_COMMAND}` to start, or write the config by hand"),
            Some(NextAction::runs_as_printed(
                GREENFIELD_COMMAND.to_string(),
                "no record directory to adopt; greenfield mode writes a config for the types --types names".to_string(),
            )),
        ),
        Ok(AdoptOutlook::Unwritable(why)) => (
            format!("{missing} -- `urzua init` cannot propose a config here: {why}"),
            None,
        ),
        Err(e) => (
            format!("{missing} -- the tracked files could not be listed to see what `urzua init` would propose: {e}"),
            None,
        ),
    }
}

impl DoctorCheck {
    /// A check with no hook layer or state.
    fn new(check: &str, status: DoctorStatus, message: String) -> Self {
        DoctorCheck {
            check: check.to_string(),
            status,
            message,
            layer: None,
            state: None,
            hook_layout: None,
            fix: None,
        }
    }
}

/// Emits one report, JSON, unconditionally (ADR-23) -- BUG-4 was `doctor`
/// printing `[OK]`/`[WARN]`/`[ERROR]` lines instead, the one command that
/// hadn't caught up with every other command's output contract.
pub fn run(config: Option<PathBuf>) -> ExitCode {
    let repo_root = match find_repo_root(&[PathBuf::from(".")]) {
        Ok(root) => root,
        Err(e) => return emit(&CouldNotRun::from(e)),
    };

    let mut checks = Vec::new();
    // The global `--config`, as every other command reads it: reporting on the
    // default file instead would be a silent no-op.
    let config_path = config.unwrap_or_else(|| crate::discovery::default_config_path(&repo_root));

    if !config_path.exists() {
        let (message, next) = missing_config_step(&repo_root, &config_path);
        checks.push(DoctorCheck::new(
            "config-exists",
            DoctorStatus::Error,
            message,
        ));
        return emit(&DoctorReport {
            status: DoctorStatus::Error,
            checks,
            notices: Vec::new(),
            exit_code: 2,
            next,
        });
    }
    checks.push(DoctorCheck::new(
        "config-exists",
        DoctorStatus::Ok,
        format!("{} exists", config_path.display()),
    ));

    let config = match load_config(&config_path) {
        Ok(c) => {
            checks.push(DoctorCheck::new(
                "config-parses",
                DoctorStatus::Ok,
                // The harness table keeps its unrecognized keys as problems
                // rather than refusing to load (`ADR-89`); `harness-config`
                // below names them, so this line must not deny them.
                match c.harness.as_ref().map_or(0, |h| h.problems.len()) {
                    0 => "config parses (no unrecognized keys)".to_string(),
                    n => format!(
                        "config parses ({n} unrecognized harness {}, named under harness-config)",
                        if n == 1 { "entry" } else { "entries" }
                    ),
                },
            ));
            c
        }
        Err(e) => {
            checks.push(DoctorCheck::new(
                "config-parses",
                DoctorStatus::Error,
                format!("config does not parse: {e}"),
            ));
            return emit(&DoctorReport {
                status: DoctorStatus::Error,
                checks,
                notices: Vec::new(),
                exit_code: 1,
                next: None,
            });
        }
    };

    config_shape_checks(&config, &repo_root, &mut checks);
    template_section_checks(&config, &repo_root, &mut checks);
    checks.push(ci_wired_check(&repo_root));

    // Local hook layers (ADR-71, MILE-179). Warn, not error, when absent:
    // a local layer is a convenience and only CI is the guarantee, so a clone
    // without hooks is reported but does not fail doctor. A hook directory
    // that cannot be read is an error: not-knowing is not the same as absent.
    checks.extend(hook_checks(&repo_root, &config.hooks));

    // The agent harness (ADR-89), when the configuration declares one.
    for (level, check, message) in crate::harness::doctor_checks(&repo_root, &config) {
        checks.push(DoctorCheck::new(
            check,
            match level {
                crate::harness::Level::Ok => DoctorStatus::Ok,
                crate::harness::Level::Warn => DoctorStatus::Warn,
            },
            message,
        ));
    }

    emit(&report_from(checks))
}

/// The report over every check: its status is the worst one, and an error
/// exits 1.
fn report_from(checks: Vec<DoctorCheck>) -> DoctorReport {
    let has_error = checks.iter().any(|c| c.status == DoctorStatus::Error);
    let has_warn = checks.iter().any(|c| c.status == DoctorStatus::Warn);
    let status = if has_error {
        DoctorStatus::Error
    } else if has_warn {
        DoctorStatus::Warn
    } else {
        DoctorStatus::Ok
    };

    DoctorReport {
        status,
        checks,
        notices: Vec::new(),
        exit_code: if has_error { 1 } else { 0 },
        next: None,
    }
}

/// The checks on what the configuration declares: record types, enabled
/// rules, and each type's directories and required fields.
fn config_shape_checks(
    config: &urzua_core::config::Config,
    repo_root: &std::path::Path,
    checks: &mut Vec<DoctorCheck>,
) {
    if config.record_types.is_empty() {
        checks.push(DoctorCheck::new(
            "record-types-declared",
            DoctorStatus::Error,
            "no record_types declared -- check will never examine anything".to_string(),
        ));
    }

    // Every rule is opt-in (ADR-53): a config enabling none makes `check`
    // exit 2 not-run, so "healthy" here would be the silent no-op doctor
    // exists to catch.
    if config
        .rules
        .values()
        .all(|r| r.level == urzua_core::config::RuleLevel::Off)
    {
        checks.push(DoctorCheck::new(
            "rules-enabled",
            DoctorStatus::Error,
            "no rule is enabled under `rules:` -- check will report not-run and examine nothing"
                .to_string(),
        ));
    }

    for name in config.sorted_type_names() {
        let cfg = &config.record_types[name];
        for dir in cfg.all_dirs() {
            if repo_root.join(dir).is_dir() {
                checks.push(DoctorCheck::new(
                    "record-type-dir",
                    DoctorStatus::Ok,
                    format!("record type '{name}' -> {dir} exists"),
                ));
            } else {
                checks.push(DoctorCheck::new(
                    "record-type-dir",
                    DoctorStatus::Error,
                    format!("record type '{name}' declares dir '{dir}', which does not exist"),
                ));
            }
        }
        if cfg.required_fields.is_empty() {
            checks.push(DoctorCheck::new(
                "record-type-required-fields",
                DoctorStatus::Warn,
                format!(
                    "record type '{name}' has no required_fields -- field.quality and field.pending will never fire for it"
                ),
            ));
        }
    }
}

/// One check per record type that declares required sections (`MILE-6`): the
/// type's template, `.urzua/templates/<type>.md`, has a heading for each of
/// them. A template that lacks one scaffolds records that `section.required`
/// reports the moment they are written, so the disagreement is a `warn` here,
/// naming each missing section. A type with no template has nothing to compare
/// (`urzua new` scaffolds it from the declaration), and a template that cannot
/// be read is an `error`: not knowing is not agreeing. A type that declares no
/// required section is not listed.
fn template_section_checks(
    config: &urzua_core::config::Config,
    repo_root: &std::path::Path,
    checks: &mut Vec<DoctorCheck>,
) {
    const CHECK: &str = "template-sections";
    for name in config.sorted_type_names() {
        let type_config = &config.record_types[name];
        let Ok(Some(model)) = urzua_core::document_model::declared_model(name, type_config) else {
            continue;
        };
        let Some(decl) = model.sections.filter(|d| !d.required.is_empty()) else {
            continue;
        };
        let rel = format!(".urzua/templates/{name}.md");
        let (status, message) = match urzua_io::read_to_string(&repo_root.join(&rel)) {
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => (
                DoctorStatus::Ok,
                format!("record type '{name}' has no template at {rel}; there is nothing to compare its required sections with"),
            ),
            Err(e) => (
                DoctorStatus::Error,
                format!("could not read {rel}: {e} -- cannot tell whether the template and the configuration agree"),
            ),
            Ok(text) => {
                let missing =
                    urzua_core::rules::template_missing_sections(name, type_config, &text);
                if missing.is_empty() {
                    (
                        DoctorStatus::Ok,
                        format!(
                            "{rel} has a heading for each of the {} required section(s) of '{name}'",
                            decl.required.len()
                        ),
                    )
                } else {
                    let named: Vec<String> = missing
                        .iter()
                        .map(|r| format!("`{} {}`", "#".repeat(usize::from(r.level)), r.name))
                        .collect();
                    (
                        DoctorStatus::Warn,
                        format!(
                            "{rel} has no heading for the required section(s) {} of '{name}', so a record made from it fails section.required; add the heading(s) to the template or correct the declaration",
                            named.join(", ")
                        ),
                    )
                }
            }
        };
        checks.push(DoctorCheck::new(CHECK, status, message));
    }
}

/// Whether any workflow under `.github/workflows` invokes the checker.
///
/// Every workflow, not one by name. Reading `ci.yml` alone reported that a
/// repository was unwired when its invocation lived in a differently named
/// workflow (BUG-91).
///
/// No error swallowed. "No workflow invokes the checker" and "the workflows
/// could not be read" are different answers, and collapsing them reports
/// confidently about a directory never opened -- the defect this check was
/// just fixed for, one level down. Only an absent directory is a warning.
fn workflows_invoke_checker(workflows_dir: &std::path::Path) -> Result<bool, String> {
    match std::fs::read_dir(workflows_dir) {
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(false),
        Err(e) => Err(format!("could not read {}: {e}", workflows_dir.display())),
        Ok(entries) => {
            let mut found = false;
            let mut failure: Option<String> = None;
            for entry in entries {
                let path = match entry {
                    Ok(entry) => entry.path(),
                    Err(e) => {
                        failure = Some(format!(
                            "could not read an entry of {}: {e}",
                            workflows_dir.display()
                        ));
                        break;
                    }
                };
                if !path.extension().is_some_and(|e| e == "yml" || e == "yaml") {
                    continue;
                }
                match std::fs::read_to_string(&path) {
                    Ok(content) => {
                        // `urzua-gate` is urzua's own reusable action
                        // (MILE-180), which runs `urzua check` for the caller.
                        if invokes_urzua(&content, Some("check"), true) {
                            found = true;
                            break;
                        }
                    }
                    Err(e) => {
                        failure = Some(format!("could not read {}: {e}", path.display()));
                        break;
                    }
                }
            }
            match failure {
                Some(message) => Err(message),
                None => Ok(found),
            }
        }
    }
}

fn ci_wired_check(repo_root: &std::path::Path) -> DoctorCheck {
    let workflows_dir = repo_root.join(".github/workflows");
    match workflows_invoke_checker(&workflows_dir) {
        Err(message) => DoctorCheck::new(
            "ci-wired",
            DoctorStatus::Error,
            format!(
                "{message} -- cannot tell whether the checker is wired in, which is not the same as it not being"
            ),
        ),
        Ok(true) => DoctorCheck::new(
            "ci-wired",
            DoctorStatus::Ok,
            "a workflow invokes `urzua check` or the urzua-gate action".to_string(),
        ),
        Ok(false) => DoctorCheck::new(
            "ci-wired",
            DoctorStatus::Warn,
            format!(
                "no workflow in {} invokes `urzua check` or the urzua-gate action -- a check that exists but is never run reports nothing to anyone",
                workflows_dir.display()
            ),
        ),
    }
}
