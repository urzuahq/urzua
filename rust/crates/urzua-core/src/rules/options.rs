//! Typed options, one value per rule that reads list options: the rule's own
//! options as a type, built from its `RuleSetting` in one place so a caller
//! never re-reads a field with `unwrap_or_default()`.
//!
//! A required option (`RULE_OPTIONS`, `optional: false`) is a `Vec` here: config
//! load refuses an enabled rule that omits it or declares it empty, so a value
//! reaching a rule is always declared. An option whose absence means something
//! different from an empty list stays an `Option` (`fields`: absent checks every
//! declared field).

use crate::config::{IndexSpec, RuleOpts, RuleSetting};

/// `claim.status-agreement`: the paths scanned and the statuses that count as
/// closed.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ClaimOpts {
    pub claim_paths: Vec<String>,
    pub closed_statuses: Vec<String>,
}

impl RuleOpts for ClaimOpts {
    fn from_setting(s: &RuleSetting) -> Self {
        ClaimOpts {
            claim_paths: s.claim_paths.clone().unwrap_or_default(),
            closed_statuses: s.closed_statuses.clone().unwrap_or_default(),
        }
    }
}

/// `boundary.published-path-reference`.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PublishedPathOpts {
    pub published_paths: Vec<String>,
}

impl RuleOpts for PublishedPathOpts {
    fn from_setting(s: &RuleSetting) -> Self {
        PublishedPathOpts {
            published_paths: s.published_paths.clone().unwrap_or_default(),
        }
    }
}

/// `index.freshness`.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct IndexOpts {
    pub indexes: Vec<IndexSpec>,
}

impl RuleOpts for IndexOpts {
    fn from_setting(s: &RuleSetting) -> Self {
        IndexOpts {
            indexes: s.indexes.clone().unwrap_or_default(),
        }
    }
}

/// `pointer.target-status`: `fields` is absent (every declared field) or a
/// declared, non-empty restriction.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PointerTargetOpts {
    pub not_in: Vec<String>,
    pub fields: Option<Vec<String>>,
}

impl RuleOpts for PointerTargetOpts {
    fn from_setting(s: &RuleSetting) -> Self {
        PointerTargetOpts {
            not_in: s.not_in.clone().unwrap_or_default(),
            fields: s.fields.clone(),
        }
    }
}

/// `embodiment.back-pointer-decision-accepted`: the fallback statuses for a
/// type that declares no success statuses. Optional; absent and empty both
/// mean no fallback, which is what the rule does with either.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DecisionAcceptedOpts {
    pub accepted_statuses: Vec<String>,
}

impl RuleOpts for DecisionAcceptedOpts {
    fn from_setting(s: &RuleSetting) -> Self {
        DecisionAcceptedOpts {
            accepted_statuses: s.accepted_statuses.clone().unwrap_or_default(),
        }
    }
}

/// `narrative-field.stale`.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct NarrativeStaleOpts {
    pub terminal_statuses: Vec<String>,
}

impl RuleOpts for NarrativeStaleOpts {
    fn from_setting(s: &RuleSetting) -> Self {
        NarrativeStaleOpts {
            terminal_statuses: s.terminal_statuses.clone().unwrap_or_default(),
        }
    }
}

/// `diff.source-requires-record`: `exclude_paths` is optional and an absent one
/// excludes nothing, the same as an empty one.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SourceRequiresRecordOpts {
    pub source_paths: Vec<String>,
    pub exclude_paths: Vec<String>,
}

impl RuleOpts for SourceRequiresRecordOpts {
    fn from_setting(s: &RuleSetting) -> Self {
        SourceRequiresRecordOpts {
            source_paths: s.source_paths.clone().unwrap_or_default(),
            exclude_paths: s.exclude_paths.clone().unwrap_or_default(),
        }
    }
}

/// `diff.source-and-records-mixed`.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SourceAndRecordsMixedOpts {
    pub application_paths: Vec<String>,
    pub exclude_paths: Vec<String>,
}

impl RuleOpts for SourceAndRecordsMixedOpts {
    fn from_setting(s: &RuleSetting) -> Self {
        SourceAndRecordsMixedOpts {
            application_paths: s.application_paths.clone().unwrap_or_default(),
            exclude_paths: s.exclude_paths.clone().unwrap_or_default(),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::config::Config;
    use crate::rules::*;

    fn config(rules: &str) -> Config {
        crate::config::parse(&format!(
            "schema_version: 2\nrecord_types:\n  adr:\n    dir: docs/adr\nrules:\n{rules}"
        ))
        .unwrap()
    }

    #[test]
    fn an_enabled_rule_yields_its_declared_options() {
        let c = config("  claim.status-agreement:\n    level: error\n    claim_paths: [a, b]\n    closed_statuses: [Done]\n");
        let o: ClaimOpts = c.enabled_rule_opts(RULE_CLAIM_STATUS_AGREEMENT).unwrap();
        assert_eq!(o.claim_paths, ["a", "b"]);
        assert_eq!(o.closed_statuses, ["Done"]);
    }

    #[test]
    fn an_off_or_undeclared_rule_has_no_enabled_options() {
        let c = config("  claim.status-agreement:\n    level: off\n");
        assert!(c
            .enabled_rule_opts::<ClaimOpts>(RULE_CLAIM_STATUS_AGREEMENT)
            .is_none());
        assert!(c
            .enabled_rule_opts::<IndexOpts>(RULE_INDEX_FRESHNESS)
            .is_none());
    }

    #[test]
    fn an_absent_optional_option_is_not_an_empty_one() {
        let absent =
            config("  pointer.target-status:\n    level: error\n    not_in: [Superseded]\n");
        let o: PointerTargetOpts = absent.rule_opts(RULE_POINTER_TARGET_STATUS);
        assert_eq!(o.fields, None);
        let set = config("  pointer.target-status:\n    level: error\n    not_in: [Superseded]\n    fields: [Implements]\n");
        let o: PointerTargetOpts = set.rule_opts(RULE_POINTER_TARGET_STATUS);
        assert_eq!(o.fields, Some(vec!["Implements".to_string()]));
    }
}
