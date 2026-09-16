//! `urzua doctor` (SPEC-15): reports on the tool's own configuration and
//! invocation health, not on record content -- `check` validates records;
//! whether *itself* is correctly invoked is a different question with a
//! different failure mode. An unrecognized config key is an error here, not
//! a warning: the alternative is a typo silently disabling a rule.

use std::path::PathBuf;
use std::process::ExitCode;

use urzua_core::report::{CouldNotRun, Notice, Report};

use crate::discovery::{find_repo_root, load_config};
use crate::emit;

#[derive(serde::Serialize)]
struct DoctorCheck {
    check: String,
    status: DoctorStatus,
    message: String,
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
}

impl Report for DoctorReport {
    fn notices(&self) -> &[Notice] {
        &self.notices
    }
    fn exit_code(&self) -> ExitCode {
        ExitCode::from(self.exit_code)
    }
}

/// Emits one report, JSON, unconditionally (ADR-23) -- BUG-4 was `doctor`
/// printing `[OK]`/`[WARN]`/`[ERROR]` lines instead, the one command that
/// hadn't caught up with every other command's output contract.
pub fn run() -> ExitCode {
    let repo_root = match find_repo_root(&[PathBuf::from(".")]) {
        Ok(root) => root,
        Err(e) => return emit(&CouldNotRun::from(e)),
    };

    let mut checks = Vec::new();
    let config_path = repo_root.join(".urzua/config.toml");

    if !config_path.exists() {
        checks.push(DoctorCheck {
            check: "config-exists".to_string(),
            status: DoctorStatus::Error,
            message: format!(
                "{} does not exist -- run `urzua init` to adopt this corpus",
                config_path.display()
            ),
        });
        return emit(&DoctorReport {
            status: DoctorStatus::Error,
            checks,
            notices: Vec::new(),
            exit_code: 2,
        });
    }
    checks.push(DoctorCheck {
        check: "config-exists".to_string(),
        status: DoctorStatus::Ok,
        message: format!("{} exists", config_path.display()),
    });

    let config = match load_config(&config_path) {
        Ok(c) => {
            checks.push(DoctorCheck {
                check: "config-parses".to_string(),
                status: DoctorStatus::Ok,
                message: "config parses (no unrecognized keys)".to_string(),
            });
            c
        }
        Err(e) => {
            checks.push(DoctorCheck {
                check: "config-parses".to_string(),
                status: DoctorStatus::Error,
                message: format!("config does not parse: {e}"),
            });
            return emit(&DoctorReport {
                status: DoctorStatus::Error,
                checks,
                notices: Vec::new(),
                exit_code: 1,
            });
        }
    };

    if config.record_types.is_empty() {
        checks.push(DoctorCheck {
            check: "record-types-declared".to_string(),
            status: DoctorStatus::Error,
            message: "no record_types declared -- check will never examine anything".to_string(),
        });
    }

    for (name, cfg) in &config.record_types {
        let dir_path = repo_root.join(&cfg.dir);
        if dir_path.is_dir() {
            checks.push(DoctorCheck {
                check: "record-type-dir".to_string(),
                status: DoctorStatus::Ok,
                message: format!("record type '{name}' -> {} exists", cfg.dir),
            });
        } else {
            checks.push(DoctorCheck {
                check: "record-type-dir".to_string(),
                status: DoctorStatus::Error,
                message: format!(
                    "record type '{name}' declares dir '{}', which does not exist",
                    cfg.dir
                ),
            });
        }
        if cfg.required_fields.is_empty() {
            checks.push(DoctorCheck {
                check: "record-type-required-fields".to_string(),
                status: DoctorStatus::Warn,
                message: format!(
                    "record type '{name}' has no required_fields -- field-quality/header rules will never fire for it"
                ),
            });
        }
    }

    let ci_path = repo_root.join(".github/workflows/ci.yml");
    let ci_wired = std::fs::read_to_string(&ci_path)
        .map(|content| content.contains("urzua check") || content.contains("make records"))
        .unwrap_or(false);
    if ci_wired {
        checks.push(DoctorCheck {
            check: "ci-wired".to_string(),
            status: DoctorStatus::Ok,
            message: "CI workflow invokes `urzua check` or `make records`".to_string(),
        });
    } else {
        checks.push(DoctorCheck {
            check: "ci-wired".to_string(),
            status: DoctorStatus::Warn,
            message: format!(
                "{} does not invoke `urzua check` or `make records` -- a check that exists but is never run reports nothing to anyone",
                ci_path.display()
            ),
        });
    }

    let has_error = checks.iter().any(|c| c.status == DoctorStatus::Error);
    let has_warn = checks.iter().any(|c| c.status == DoctorStatus::Warn);
    let status = if has_error {
        DoctorStatus::Error
    } else if has_warn {
        DoctorStatus::Warn
    } else {
        DoctorStatus::Ok
    };

    emit(&DoctorReport {
        status,
        checks,
        notices: Vec::new(),
        exit_code: if has_error { 1 } else { 0 },
    })
}
