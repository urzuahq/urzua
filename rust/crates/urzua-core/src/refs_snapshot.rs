//! The committed snapshot of a repository's issue and pull request numbers
//! (`MILE-143`).
//!
//! `citation.issue-exists` reads it and `urzua refs refresh` writes it; nothing
//! else does, and `check` never reaches the network. The shape lives here, with
//! no I/O, so both sides agree on it by construction.

use serde::{Deserialize, Serialize};

/// The snapshot file's one supported `schema_version`.
pub const SNAPSHOT_SCHEMA_VERSION: u32 = 1;

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RefsSnapshot {
    pub schema_version: u32,
    /// `owner/name` of the repository the numbers belong to.
    pub repository: String,
    /// The date (`YYYY-MM-DD`) the refresh ran. A cited number newer than
    /// this date cannot be in the snapshot.
    pub generated_at: String,
    /// Issue numbers (not pull requests), ascending.
    pub issues: Vec<u64>,
    /// Pull request numbers, ascending.
    pub pulls: Vec<u64>,
}

impl RefsSnapshot {
    /// Build a snapshot with both lists sorted and de-duplicated, so the file
    /// a refresh writes does not depend on the order the API returned.
    pub fn new(
        repository: &str,
        generated_at: &str,
        mut issues: Vec<u64>,
        mut pulls: Vec<u64>,
    ) -> Self {
        issues.sort_unstable();
        issues.dedup();
        pulls.sort_unstable();
        pulls.dedup();
        Self {
            schema_version: SNAPSHOT_SCHEMA_VERSION,
            repository: repository.to_string(),
            generated_at: generated_at.to_string(),
            issues,
            pulls,
        }
    }

    /// Whether `number` is an issue or a pull request of the repository. GitHub
    /// shares one number space between the two and redirects either URL to the
    /// other, so a citation is not held to the kind in its link.
    pub fn contains(&self, number: u64) -> bool {
        self.issues.binary_search(&number).is_ok() || self.pulls.binary_search(&number).is_ok()
    }

    /// The highest number recorded, or `None` for an empty snapshot.
    pub fn newest(&self) -> Option<u64> {
        self.issues.last().max(self.pulls.last()).copied()
    }
}

/// `snapshot`, or why it cannot serve a rule declaring `repository`. Case is
/// ignored in the repository comparison: GitHub names are case-insensitive.
pub fn validate_snapshot(snapshot: RefsSnapshot, repository: &str) -> Result<RefsSnapshot, String> {
    if snapshot.schema_version != SNAPSHOT_SCHEMA_VERSION {
        return Err(format!(
            "has schema_version {}, but this build of urzua reads version {SNAPSHOT_SCHEMA_VERSION}",
            snapshot.schema_version
        ));
    }
    if !snapshot.repository.eq_ignore_ascii_case(repository) {
        return Err(format!(
            "describes '{}', but the rule's issue_refs declares '{repository}'",
            snapshot.repository
        ));
    }
    for (name, list) in [("issues", &snapshot.issues), ("pulls", &snapshot.pulls)] {
        if list.windows(2).any(|w| w[0] >= w[1]) {
            return Err(format!(
                "lists '{name}' out of order or with a repeated number; they must be strictly ascending (run `urzua refs refresh` to rewrite it)"
            ));
        }
    }
    Ok(snapshot)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn raw(issues: Vec<u64>) -> RefsSnapshot {
        RefsSnapshot {
            schema_version: SNAPSHOT_SCHEMA_VERSION,
            repository: "o/r".into(),
            generated_at: "2026-01-01".into(),
            issues,
            pulls: Vec::new(),
        }
    }

    #[test]
    fn an_unsorted_or_repeating_list_is_rejected() {
        assert!(validate_snapshot(raw(vec![5, 3]), "o/r").is_err());
        assert!(validate_snapshot(raw(vec![3, 3]), "o/r").is_err());
        assert!(validate_snapshot(raw(vec![3, 5]), "o/r").is_ok());
    }
}
