//! Typed options, one value per rule that reads list options: the rule's own
//! options as a type, built from its `RuleSetting` in one place so a caller
//! never re-reads a field with `unwrap_or_default()`.
//!
//! A required option (`RULE_OPTIONS`, `optional: false`) is a `Vec` here: config
//! load refuses an enabled rule that omits it or declares it empty, so a value
//! reaching a rule is always declared. An option whose absence means something
//! different from an empty list stays an `Option` (`fields`: absent checks every
//! declared field).

use super::{BoundaryOrigin, ResolvedBoundary};
use crate::config::{BoundaryKind, IndexSpec, LocatorKind, RuleOpts, RuleSetting};

/// `claim.status-agreement`: the paths scanned and the statuses that count as
/// closed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClaimOpts {
    pub claim_paths: Vec<String>,
    pub closed_statuses: Vec<String>,
    /// The frontmatter key a claim file lists the records it closes under
    /// (`ADR-85`); `None` leaves the rule reading closing verbs alone.
    pub closes: Option<String>,
    /// A claim file with frontmatter and without that key is a finding.
    pub strict: bool,
    /// `false` stops closing verbs in prose being read as claims (`ADR-88`);
    /// `true` unless declared.
    pub verb_claims: bool,
}

impl Default for ClaimOpts {
    fn default() -> Self {
        ClaimOpts {
            claim_paths: Vec::new(),
            closed_statuses: Vec::new(),
            closes: None,
            strict: false,
            verb_claims: true,
        }
    }
}

impl RuleOpts for ClaimOpts {
    fn from_setting(s: &RuleSetting) -> Self {
        ClaimOpts {
            claim_paths: s.claim_paths.clone().unwrap_or_default(),
            closed_statuses: s.closed_statuses.clone().unwrap_or_default(),
            closes: s.closes.clone(),
            strict: s.strict.unwrap_or(false),
            verb_claims: s.verb_claims.unwrap_or(true),
        }
    }
}

/// `field.quality`: `template_placeholders` is off unless declared (`BUG-22`).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct FieldQualityOpts {
    pub template_placeholders: bool,
}

impl RuleOpts for FieldQualityOpts {
    fn from_setting(s: &RuleSetting) -> Self {
        FieldQualityOpts {
            template_placeholders: s.template_placeholders.unwrap_or(false),
        }
    }
}

/// `boundary.published-path-reference`: every boundary the repository declares,
/// the entries of `boundaries` first and `published_paths` (one published
/// boundary, severity the rule's level, no reason required) after them, and the
/// declared inline ignore marker.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PublishedPathOpts {
    pub boundaries: Vec<ResolvedBoundary>,
    pub ignore_marker: Option<String>,
}

impl RuleOpts for PublishedPathOpts {
    fn from_setting(s: &RuleSetting) -> Self {
        let mut boundaries: Vec<ResolvedBoundary> = s
            .boundaries
            .iter()
            .flatten()
            .enumerate()
            .map(|(idx, b)| ResolvedBoundary {
                origin: BoundaryOrigin::Declared(idx),
                kind: b.kind,
                reason: Some(b.reason.clone()),
                severity: b.severity.severity(),
                paths: b.paths.clone(),
                exclude: b.exclude.clone().unwrap_or_default(),
            })
            .collect();
        if let Some(paths) = s.published_paths.as_ref().filter(|p| !p.is_empty()) {
            // A rule that is off runs nothing, so it has no boundary to resolve;
            // every other level gives the boundary its own severity.
            if let Some(severity) = s.level.severity() {
                boundaries.push(ResolvedBoundary {
                    origin: BoundaryOrigin::PublishedPaths,
                    kind: BoundaryKind::Published,
                    reason: None,
                    severity,
                    paths: paths.clone(),
                    exclude: Vec::new(),
                });
            }
        }
        PublishedPathOpts {
            boundaries,
            ignore_marker: s.ignore_marker.clone(),
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

/// The declared inline ignore marker of a back-pointer rule (`ADR-80`
/// amendment): the text that excuses the one line it is on.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct IgnoreMarkerOpts {
    pub ignore_marker: Option<String>,
}

impl RuleOpts for IgnoreMarkerOpts {
    fn from_setting(s: &RuleSetting) -> Self {
        IgnoreMarkerOpts {
            ignore_marker: s.ignore_marker.clone(),
        }
    }
}

/// `embodiment.locator-exists`: the locator kinds accepted and the kinds that
/// may carry a pin (`BUG-176`). Both default to `[file]`, so a repository that
/// declares neither accepts exactly what it always did.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LocatorExistsOpts {
    pub locator_kinds: Vec<LocatorKind>,
    pub pinnable: Vec<LocatorKind>,
}

impl Default for LocatorExistsOpts {
    fn default() -> Self {
        LocatorExistsOpts {
            locator_kinds: vec![LocatorKind::File],
            pinnable: vec![LocatorKind::File],
        }
    }
}

impl RuleOpts for LocatorExistsOpts {
    fn from_setting(s: &RuleSetting) -> Self {
        let default = LocatorExistsOpts::default();
        LocatorExistsOpts {
            locator_kinds: s.locator_kinds.clone().unwrap_or(default.locator_kinds),
            pinnable: s.pinnable.clone().unwrap_or(default.pinnable),
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
    /// The decision gate (`MILE-133`); empty means the gate is not declared.
    pub decision_types: std::collections::BTreeMap<String, Vec<String>>,
    pub fix_types: Vec<String>,
    pub link_fields: Vec<String>,
}

impl RuleOpts for SourceRequiresRecordOpts {
    fn from_setting(s: &RuleSetting) -> Self {
        SourceRequiresRecordOpts {
            source_paths: s.source_paths.clone().unwrap_or_default(),
            exclude_paths: s.exclude_paths.clone().unwrap_or_default(),
            decision_types: s.decision_types.clone().unwrap_or_default(),
            fix_types: s.fix_types.clone().unwrap_or_default(),
            link_fields: s.link_fields.clone().unwrap_or_default(),
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

/// `diff.decider-required`: with `require_evidence` a gated change that arrives
/// with no actors file is an error instead of `info` (`MILE-200`).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DeciderRequiredOpts {
    pub require_evidence: bool,
}

impl RuleOpts for DeciderRequiredOpts {
    fn from_setting(s: &RuleSetting) -> Self {
        DeciderRequiredOpts {
            require_evidence: s.require_evidence.unwrap_or(false),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::config::{Config, RuleOpts};
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
        assert_eq!((o.closes, o.strict), (None, false));
    }

    #[test]
    fn the_claim_field_options_are_read_through_the_typed_accessor_observed_failing() {
        let c = config("  claim.status-agreement:\n    level: error\n    claim_paths: [a]\n    closed_statuses: [Done]\n    closes: closes\n    strict: true\n");
        let o: ClaimOpts = c.enabled_rule_opts(RULE_CLAIM_STATUS_AGREEMENT).unwrap();
        assert_eq!(o.closes.as_deref(), Some("closes"));
        assert!(o.strict);
    }

    #[test]
    fn verb_claims_defaults_to_true_and_reads_false() {
        let on = config("  claim.status-agreement:\n    level: error\n    claim_paths: [a]\n    closed_statuses: [Done]\n    closes: closes\n");
        let o: ClaimOpts = on.enabled_rule_opts(RULE_CLAIM_STATUS_AGREEMENT).unwrap();
        assert!(o.verb_claims);
        let off = config("  claim.status-agreement:\n    level: error\n    claim_paths: [a]\n    closed_statuses: [Done]\n    closes: closes\n    verb_claims: false\n");
        let o: ClaimOpts = off.enabled_rule_opts(RULE_CLAIM_STATUS_AGREEMENT).unwrap();
        assert!(!o.verb_claims);
        assert!(ClaimOpts::default().verb_claims);
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
    fn locator_kinds_and_pinnable_default_to_file_and_are_checked_at_load() {
        use crate::config::LocatorKind::{Directory, File, Glob};
        let rules = |body: &str| {
            crate::config::parse(&format!(
                "schema_version: 2\nrecord_types:\n  adr:\n    dir: docs/adr\nrules:\n  embodiment.locator-exists:\n    level: error\n{body}"
            ))
        };
        let o: LocatorExistsOpts = rules("").unwrap().rule_opts(RULE_EMBODIMENT_LOCATOR_EXISTS);
        assert_eq!((o.locator_kinds, o.pinnable), (vec![File], vec![File]));
        let o: LocatorExistsOpts =
            rules("    locator_kinds: [file, directory, glob]\n    pinnable: [glob]\n")
                .unwrap()
                .rule_opts(RULE_EMBODIMENT_LOCATOR_EXISTS);
        assert_eq!(o.locator_kinds, [File, Directory, Glob]);
        assert_eq!(o.pinnable, [Glob]);
        // A kind that can be pinned but not named is a rule that always reports.
        let err = rules("    pinnable: [directory]\n")
            .unwrap_err()
            .to_string();
        assert!(
            err.contains("pinnable") && err.contains("locator_kinds"),
            "{err}"
        );
        assert!(rules("    locator_kinds: []\n").is_err());
        assert!(rules("    locator_kinds: [tree]\n").is_err());
    }

    #[test]
    fn an_absent_optional_option_is_not_an_empty_one() {
        let setting = |y: &str| -> crate::config::RuleSetting { yaml_serde::from_str(y).unwrap() };
        let o = PointerTargetOpts::from_setting(&setting("level: error\nnot_in: [Superseded]\n"));
        assert_eq!(o.fields, None);
        let o = PointerTargetOpts::from_setting(&setting(
            "level: error\nnot_in: [Superseded]\nfields: [Implements]\n",
        ));
        assert_eq!(o.fields, Some(vec!["Implements".to_string()]));
    }
}

/// `embodiment.locator-on-back-pointer-path`: the scanned paths and, optionally,
/// the test-class subset (`MILE-189`).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct LocatorMechanismOpts {
    pub back_pointer_paths: Vec<String>,
    pub test_paths: Vec<String>,
}

impl RuleOpts for LocatorMechanismOpts {
    fn from_setting(s: &RuleSetting) -> Self {
        LocatorMechanismOpts {
            back_pointer_paths: s.back_pointer_paths.clone().unwrap_or_default(),
            test_paths: s.test_paths.clone().unwrap_or_default(),
        }
    }
}

/// `embodiment.claim-expired` (`ADR-97`): the periods, the anchor field, the
/// categories that must be pinned before a renewal and the batch cap, with the
/// rule's own level, which is the ceiling for every entry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClaimExpiredOpts {
    pub level: crate::config::RuleLevel,
    pub anchor_field: Option<String>,
    pub require_pin: Vec<String>,
    /// What drift does to a renewal, per category (`ADR-97`, amendment of
    /// 2026-11-09); a category not named is `block`.
    pub drift: std::collections::BTreeMap<String, crate::config::DriftPolicy>,
    pub max_batch: usize,
    pub periods: Vec<crate::config::ClaimPeriod>,
}

impl ClaimExpiredOpts {
    /// The drift policy of `category`: `block` unless declared.
    pub fn drift_policy(&self, category: &str) -> crate::config::DriftPolicy {
        self.drift.get(category).copied().unwrap_or_default()
    }

    /// Whether `category` has a declared drift policy.
    pub fn drift_declared(&self, category: &str) -> bool {
        self.drift.contains_key(category)
    }
}

impl Default for ClaimExpiredOpts {
    fn default() -> Self {
        ClaimExpiredOpts {
            level: crate::config::RuleLevel::Off,
            anchor_field: None,
            require_pin: Vec::new(),
            drift: std::collections::BTreeMap::new(),
            max_batch: 1,
            periods: Vec::new(),
        }
    }
}

impl RuleOpts for ClaimExpiredOpts {
    fn from_setting(s: &RuleSetting) -> Self {
        ClaimExpiredOpts {
            level: s.level,
            anchor_field: s.anchor_field.as_ref().map(|a| a.trim().to_string()),
            require_pin: s
                .require_pin
                .iter()
                .flatten()
                .map(|c| c.trim().to_string())
                .collect(),
            drift: s
                .drift
                .iter()
                .flatten()
                .map(|(c, p)| (c.trim().to_string(), *p))
                .collect(),
            max_batch: s.max_batch.map_or(1, |n| n as usize),
            periods: s.periods.clone().unwrap_or_default(),
        }
    }
}
