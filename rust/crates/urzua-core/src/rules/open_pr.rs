//! `identity.open-pr-collision` (`ADR-93` clause 7, `MILE-89`): a record a `--base`
//! diff adds that shares its type and number with a file on a fetched
//! pull-request ref, under a different `Stable-Id`.
//!
//! The refs are read by the caller from `refs/urzua/pr` and handed in as data; the
//! rule never fetches and the core stays pure. Plain git cannot say whether a
//! pull request is open, so a head that is closed and unmerged can still hold a
//! number a record legitimately took: the rule is a warning, and the path out is a
//! waiver naming that pull request.

use super::*;
use crate::report::{census, Outcome};
use crate::values::RecordId;

/// One file on a fetched pull-request ref whose id is one a record added by the
/// diff also has.
#[derive(Debug, Clone)]
pub struct PullRefHolder {
    pub id: RecordId,
    /// The ref holding it, for example `refs/urzua/pr/origin/9`.
    pub reference: String,
    /// The file's path on that ref.
    pub path: String,
    /// The `Stable-Id` the file carries there, when it has one.
    pub stable_id: Option<String>,
}

/// What the fetched pull-request refs hold that matters to this diff.
#[derive(Debug, Clone, Default)]
pub struct PullRefClaims {
    /// Every ref under `refs/urzua/pr`: zero means nothing was fetched, and the
    /// rule says so instead of reporting a clean run over nothing (`ADR-55`).
    pub refs: Vec<String>,
    pub holders: Vec<PullRefHolder>,
}

/// The notice for a configured rule over no refs (`ADR-93` clause 8): how to fetch.
pub fn open_pr_missing_refs_notice(remote: &str, pattern: &str) -> Notice {
    Notice {
        severity: NoticeSeverity::Warning,
        subject: RULE_IDENTITY_OPEN_PR_COLLISION.to_string(),
        message: format!(
            "no pull request refs are fetched under refs/urzua/pr, so the rule compared against nothing; `check` never fetches -- run `git fetch --no-tags {remote} '+{pattern}:refs/urzua/pr/{remote}/*'` (or `urzua new` with new.check_open_prs) first"
        ),
        detail: None,
    }
}

pub fn open_pr_collision(
    records: &[Record],
    added: Option<&[std::path::PathBuf]>,
    claims: &PullRefClaims,
) -> (RuleExecution, Vec<Finding>) {
    const RULE_ID: &str = RULE_IDENTITY_OPEN_PR_COLLISION;
    let Some(added) = added else {
        // A diff rule: without `--base` it judges nothing and says so.
        return not_applicable(RULE_ID);
    };
    let added: std::collections::HashSet<&std::path::PathBuf> = added.iter().collect();
    let mut candidates: Vec<&Record> = records.iter().filter(|r| added.contains(&r.path)).collect();
    candidates.sort_by(|a, b| a.path.cmp(&b.path));
    let mut findings = Vec::new();
    for record in &candidates {
        let Some(id) = record_id(record) else {
            continue;
        };
        let id = RecordId::new(&id);
        let own = record.header.get("Stable-Id").map(|v| v.trim().to_string());
        for holder in claims.holders.iter().filter(|h| h.id == id) {
            // The record's own copy on a pull ref is the same record, not a
            // collision: a pull request is not flagged against itself.
            if matches!((&own, &holder.stable_id), (Some(a), Some(b)) if a == b) {
                continue;
            }
            findings.push(Finding::new(
                RULE_ID,
                FindingSeverity::Warning,
                record.path.clone(),
                ActionSource::new("open-pr-collision"),
                format!(
                    "{} is also used by {} on {} (Stable-Id {}, this record's {}) -- a different record. Rename the file and its H1 to the next free number (the one `urzua new` would take); if that pull request is closed and unmerged, waive this finding with a waiver naming it",
                    id.as_str(),
                    holder.path,
                    holder.reference,
                    holder.stable_id.as_deref().unwrap_or("absent"),
                    own.as_deref().unwrap_or("absent"),
                ),
            ));
        }
    }
    let examined: Vec<std::path::PathBuf> = if claims.refs.is_empty() {
        Vec::new()
    } else {
        candidates.iter().map(|r| r.path.clone()).collect()
    };
    let population = census(PopulationUnit::PullRef, claims.refs.clone(), |_| {
        Outcome::Examined
    });
    (RuleExecution::ran(RULE_ID, population, examined), findings)
}
