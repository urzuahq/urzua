//! `urzua audit` (SPEC-11, ADR-79): a deprecated alias of `urzua check`.
//!
//! `audit` once owned a rule set of its own (supersession reciprocity and
//! dangling references), which `ADR-53` made wrong: every rule is a declared,
//! opt-in policy, and a hardcoded subset inside the tool overrides the
//! adopter's declaration (`BUG-42`, `BUG-84`, `BUG-99`). It now owns none. It
//! runs `check`'s engine over the rules the configuration declares, with no
//! selection of its own, so its report is `check`'s and it cannot disagree with
//! `check` or exit 2 on a configuration `check` accepts. The one addition is a
//! `Notice` naming the replacement; a `Notice` never moves the exit code
//! (`ADR-46`). The command is removed the release after.

use std::path::PathBuf;
use std::process::ExitCode;

use urzua_core::report::{Notice, NoticeSeverity};

use crate::commands::check::{self, CheckArgs};

/// Notice subject, so a consumer can find (and drop) the deprecation notice.
pub const NOTICE_SUBJECT: &str = "audit";

pub fn run(config_path: Option<PathBuf>) -> ExitCode {
    check::run(
        config_path,
        CheckArgs {
            extra_notices: vec![Notice {
                severity: NoticeSeverity::Warning,
                subject: NOTICE_SUBJECT.to_string(),
                message: "`urzua audit` is deprecated and now runs exactly `urzua check` over the configured rules; call `urzua check` instead (`urzua check --rules pointer.resolution,relation.supersession-reciprocity` for the narrower report `audit` used to give). It is removed in the next release.".to_string(),
            }],
            ..Default::default()
        },
    )
}
