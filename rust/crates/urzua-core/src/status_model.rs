//! A record type's declared status vocabulary (MILE-132, BUG-59, BUG-65,
//! BUG-178).
//!
//! Which statuses a type has, which of them finish a record, which fields a
//! status requires or forbids, and which field values mean "not done yet" are
//! repository configuration, not engine policy. A type that declares nothing
//! behaves exactly as before.
//!
//! ```yaml
//! record_types:
//!   bug:
//!     statuses:
//!       values: [Open, Fixed, WontFix]
//!       success: [Fixed]          # terminal success
//!       failure: [WontFix]        # terminal failure
//!       implementable: [Open]     # code may cite it (optional)
//!       claim_retained: []        # may keep an Implemented claim (optional)
//!       pending: [pending, "not yet written", {pattern: "^\\(pending"}]
//!       require: [{when: [Fixed], fields: [Regression-test]}]
//!       forbid:  [{when: [Open], fields: [Fixed-in]}]
//! ```
//!
//! Transitions between statuses are MILE-144, not this.

use crate::config::{ConfigError, RecordTypeConfig};
use serde::Deserialize;
use std::collections::BTreeMap;

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StatusDecl {
    /// Every status the type allows.
    pub values: Vec<String>,
    /// Terminal statuses that mean the work was done.
    #[serde(default)]
    pub success: Vec<String>,
    /// Terminal statuses that mean it ended without being done.
    #[serde(default)]
    pub failure: Vec<String>,
    /// Statuses at which implementation of the record is permitted
    /// (MILE-133). Independent of `success`/`failure`: an in-progress
    /// milestone is implementable and not terminal.
    #[serde(default)]
    pub implementable: Vec<String>,
    /// Statuses reached only after acceptance that may still carry an
    /// Implemented/Verified claim (e.g. Superseded), `embodiment.requires-accepted-status`.
    /// Could later be derived from transitions (MILE-144).
    #[serde(default)]
    pub claim_retained: Vec<String>,
    /// Whole field values that mean "not done yet", replacing the compiled-in
    /// substring match for this type (BUG-65). Omitted keeps that match.
    #[serde(default)]
    pub pending: Option<Vec<PendingEntry>>,
    #[serde(default)]
    pub require: Vec<StatusFieldRule>,
    #[serde(default)]
    pub forbid: Vec<StatusFieldRule>,
    /// How the type's `Status` is held to its delivery evidence (`MILE-183`).
    #[serde(default)]
    pub delivery: Option<DeliveryDecl>,
}

/// `statuses.delivery` (`MILE-183`): which computed evidence tiers mean the work
/// was delivered, which statuses mean it was not, and what a delivered record's
/// `Status` is repaired to. All of it is the type's declaration; the engine
/// knows no status or tier by name beyond the tiers `Embodiment` can compute.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DeliveryDecl {
    /// Computed tiers (from `Realized-by` locators and source back-pointers,
    /// drift ignored) that say the work exists.
    pub delivered_tiers: Vec<String>,
    /// Computed tiers that say nothing was delivered. A success status with one
    /// of these is a status with no evidence.
    pub unevidenced_tiers: Vec<String>,
    /// Statuses that say the work is not done. Delivered evidence under one of
    /// them is a disagreement.
    pub not_done: Vec<String>,
    /// The `success` status `urzua fix` writes when the evidence says delivered.
    pub set_status: String,
}

/// One entry of `statuses.pending` (`MILE-162`): a whole value, or a pattern
/// for a family of placeholders that share a stem and differ in a tail. The
/// pattern is structured config (`{pattern: "^\\(pending"}`), not a string DSL;
/// it is an ordinary `regex` expression searched in the trimmed value, so the
/// author anchors it (`^`, `$`) as wanted.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PendingEntry {
    Value(String),
    Pattern(PendingPattern),
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PendingPattern {
    pub pattern: String,
}

impl<'de> Deserialize<'de> for PendingEntry {
    /// Read by hand so a malformed entry is named, instead of serde's generic
    /// "did not match any variant".
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        use serde::de::Error;
        let value = yaml_serde::Value::deserialize(d)?;
        match value {
            yaml_serde::Value::String(s) => Ok(PendingEntry::Value(s)),
            m @ yaml_serde::Value::Mapping(_) => yaml_serde::from_value::<PendingPattern>(m)
                .map(PendingEntry::Pattern)
                .map_err(|e| {
                    D::Error::custom(format!(
                        "a `pending` pattern entry is `{{pattern: \"...\"}}` with no other key: {e}"
                    ))
                }),
            _ => Err(D::Error::custom(
                "a `pending` entry is a whole value (a string) or `{pattern: \"...\"}`",
            )),
        }
    }
}

impl PendingEntry {
    fn matches(&self, value: &str) -> bool {
        match self {
            PendingEntry::Value(v) => v.trim().eq_ignore_ascii_case(value),
            // Validated at load, so a pattern that does not compile here (a
            // hand-built declaration) simply matches nothing.
            PendingEntry::Pattern(p) => regex::Regex::new(&p.pattern)
                .map(|re| re.is_match(value))
                .unwrap_or(false),
        }
    }
}

/// Fields tied to the statuses in `when`.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StatusFieldRule {
    pub when: Vec<String>,
    pub fields: Vec<String>,
}

impl StatusDecl {
    pub fn declares(&self, status: &str) -> bool {
        self.values.iter().any(|v| v == status)
    }

    /// Terminal success or terminal failure.
    pub fn is_terminal(&self, status: &str) -> bool {
        self.success
            .iter()
            .chain(&self.failure)
            .any(|v| v == status)
    }

    /// Whether a field value is a declared pending whole value, or matches a
    /// declared pending pattern (`MILE-162`).
    /// `None` when the type declares no pending vocabulary.
    pub fn is_pending_value(&self, value: &str) -> Option<bool> {
        let pending = self.pending.as_ref()?;
        let value = value.trim();
        Some(pending.iter().any(|p| p.matches(value)))
    }

    pub fn required_at(&self, status: &str) -> Vec<&str> {
        fields_at(&self.require, status)
    }

    pub fn forbidden_at(&self, status: &str) -> Vec<&str> {
        fields_at(&self.forbid, status)
    }
}

fn fields_at<'a>(rules: &'a [StatusFieldRule], status: &str) -> Vec<&'a str> {
    let mut out: Vec<&str> = Vec::new();
    for rule in rules.iter().filter(|r| r.when.iter().any(|w| w == status)) {
        for f in &rule.fields {
            if !out.contains(&f.as_str()) {
                out.push(f);
            }
        }
    }
    out
}

impl RecordTypeConfig {
    /// The type's validated status declaration. The config was validated at
    /// load, so `None` means the type declares none.
    pub fn status_decl(&self) -> Option<StatusDecl> {
        self.statuses
            .as_ref()
            .and_then(|v| yaml_serde::from_value(v.clone()).ok())
    }
}

fn invalid(record_type: &str, reason: String) -> ConfigError {
    ConfigError::InvalidDocumentModel {
        record_type: record_type.to_string(),
        key: "statuses",
        reason,
    }
}

/// A list that must hold at least one non-blank, distinct entry.
fn check_list(what: &str, list: &[String], allow_empty: bool) -> Result<(), String> {
    if list.is_empty() && !allow_empty {
        return Err(format!("`{what}` must list at least one entry"));
    }
    let mut seen = std::collections::BTreeSet::new();
    for entry in list {
        if entry.trim().is_empty() {
            return Err(format!("`{what}` has a blank entry"));
        }
        if !seen.insert(entry.as_str()) {
            return Err(format!("`{what}` lists '{entry}' twice"));
        }
    }
    Ok(())
}

/// `pending` must hold at least one entry; whole values are non-blank and
/// distinct, patterns compile and are distinct (a pattern that cannot compile
/// would otherwise match nothing, silently).
fn check_pending(entries: &[PendingEntry]) -> Result<(), String> {
    if entries.is_empty() {
        return Err("`pending` must list at least one entry".into());
    }
    let mut seen = std::collections::BTreeSet::new();
    for entry in entries {
        match entry {
            PendingEntry::Value(v) => {
                if v.trim().is_empty() {
                    return Err("`pending` has a blank entry".into());
                }
                if !seen.insert(format!("v:{v}")) {
                    return Err(format!("`pending` lists '{v}' twice"));
                }
            }
            PendingEntry::Pattern(p) => {
                if p.pattern.is_empty() {
                    return Err("`pending` has a blank pattern".into());
                }
                regex::Regex::new(&p.pattern).map_err(|e| {
                    format!(
                        "`pending` pattern '{}' is not a valid regular expression: {e}",
                        p.pattern
                    )
                })?;
                if !seen.insert(format!("p:{}", p.pattern)) {
                    return Err(format!("`pending` lists the pattern '{}' twice", p.pattern));
                }
            }
        }
    }
    Ok(())
}

fn validate_one(rt: &RecordTypeConfig, d: &StatusDecl) -> Result<(), String> {
    check_list("values", &d.values, false)?;
    check_list("success", &d.success, true)?;
    check_list("failure", &d.failure, true)?;
    check_list("implementable", &d.implementable, true)?;
    check_list("claim_retained", &d.claim_retained, true)?;
    if let Some(p) = &d.pending {
        check_pending(p)?;
    }
    for (what, list) in [
        ("success", &d.success),
        ("failure", &d.failure),
        ("implementable", &d.implementable),
        ("claim_retained", &d.claim_retained),
    ] {
        if let Some(v) = list.iter().find(|v| !d.declares(v)) {
            return Err(format!("`{what}` names '{v}', which is not in `values`"));
        }
    }
    if let Some(v) = d.success.iter().find(|v| d.failure.contains(v)) {
        return Err(format!("'{v}' is in both `success` and `failure`"));
    }
    if let Some(delivery) = &d.delivery {
        validate_delivery(rt, d, delivery)?;
    }
    let declared = rt.known_fields.as_ref().map(|_| rt.declared_fields());
    for (what, rules) in [("require", &d.require), ("forbid", &d.forbid)] {
        for rule in rules {
            check_list(&format!("{what}.when"), &rule.when, false)?;
            check_list(&format!("{what}.fields"), &rule.fields, false)?;
            if let Some(w) = rule.when.iter().find(|w| !d.declares(w)) {
                return Err(format!(
                    "`{what}.when` names '{w}', which is not in `values`"
                ));
            }
            if let Some(declared) = &declared {
                if let Some(f) = rule
                    .fields
                    .iter()
                    .find(|f| !declared.contains(&crate::values::FieldName::new(f.as_str())))
                {
                    return Err(format!(
                        "`{what}.fields` names '{f}', which is in neither `required_fields` nor `known_fields`"
                    ));
                }
            }
        }
    }
    for status in &d.values {
        let required = d.required_at(status);
        if let Some(f) = d
            .forbidden_at(status)
            .into_iter()
            .find(|f| required.contains(f))
        {
            return Err(format!(
                "'{f}' is both required and forbidden at '{status}'"
            ));
        }
    }
    Ok(())
}

/// `delivery` names only tiers `Embodiment` can compute without drift, statuses
/// the type declares, and an evidence field the type carries; a half-reading
/// would otherwise be a rule that silently never fires (`ADR-7`).
fn validate_delivery(
    rt: &RecordTypeConfig,
    d: &StatusDecl,
    delivery: &DeliveryDecl,
) -> Result<(), String> {
    const TIERS: [&str; 4] = ["Verified", "Implemented", "Specified", "Not started"];
    check_list("delivery.delivered_tiers", &delivery.delivered_tiers, false)?;
    check_list(
        "delivery.unevidenced_tiers",
        &delivery.unevidenced_tiers,
        false,
    )?;
    check_list("delivery.not_done", &delivery.not_done, false)?;
    for (what, list) in [
        ("delivery.delivered_tiers", &delivery.delivered_tiers),
        ("delivery.unevidenced_tiers", &delivery.unevidenced_tiers),
    ] {
        if let Some(t) = list.iter().find(|t| !TIERS.contains(&t.as_str())) {
            return Err(format!(
                "`{what}` names '{t}', which is not a tier computed from evidence (one of: {})",
                TIERS.join(", ")
            ));
        }
    }
    if let Some(t) = delivery
        .delivered_tiers
        .iter()
        .find(|t| delivery.unevidenced_tiers.contains(t))
    {
        return Err(format!(
            "'{t}' is in both `delivery.delivered_tiers` and `delivery.unevidenced_tiers`"
        ));
    }
    if let Some(s) = delivery.not_done.iter().find(|s| !d.declares(s)) {
        return Err(format!(
            "`delivery.not_done` names '{s}', which is not in `values`"
        ));
    }
    if let Some(s) = delivery
        .not_done
        .iter()
        .find(|s| d.success.contains(s) || d.failure.contains(s))
    {
        return Err(format!(
            "`delivery.not_done` names '{s}', which is a terminal `success` or `failure` status"
        ));
    }
    if !d.success.contains(&delivery.set_status) {
        return Err(format!(
            "`delivery.set_status` names '{}', which is not one of `success` ({})",
            delivery.set_status,
            d.success.join(", ")
        ));
    }
    let locator = rt.relation_field(crate::config::RelationRole::EmbodimentLocator);
    if let Some(known) = rt.known_fields.as_ref().map(|_| rt.declared_fields()) {
        if !known.contains(&crate::values::FieldName::new(locator)) {
            return Err(format!(
                "`delivery` reads the evidence field '{locator}', which is in neither `required_fields` nor `known_fields`"
            ));
        }
    }
    Ok(())
}

/// Validates every type's `statuses` declaration at load. A key present with
/// no value is refused rather than read as absent, which would turn a
/// half-written declaration into a silent no-op (ADR-7).
pub(crate) fn validate(
    types: &BTreeMap<String, RecordTypeConfig>,
) -> Result<BTreeMap<String, StatusDecl>, ConfigError> {
    let mut out = BTreeMap::new();
    for (name, rt) in types {
        let Some(raw) = &rt.statuses else { continue };
        if raw.is_null() {
            return Err(invalid(
                name,
                "present with no value; give it a mapping or remove the key".into(),
            ));
        }
        let decl: StatusDecl =
            yaml_serde::from_value(raw.clone()).map_err(|e| invalid(name, e.to_string()))?;
        validate_one(rt, &decl).map_err(|reason| invalid(name, reason))?;
        out.insert(name.clone(), decl);
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use crate::config::{parse, ConfigError};

    fn cfg(statuses: &str) -> String {
        format!(
            "schema_version: 2\nrecord_types:\n  bug:\n    dir: docs/bugs\n    required_fields: [Status]\n    known_fields: [Fixed-in]\n    statuses:\n{statuses}"
        )
    }

    fn err(statuses: &str) -> String {
        match parse(&cfg(statuses)) {
            Err(ConfigError::InvalidDocumentModel {
                key: "statuses",
                reason,
                ..
            }) => reason,
            other => panic!("expected an invalid statuses error, got {other:?}"),
        }
    }

    #[test]
    fn a_full_declaration_loads_and_reads_back() {
        let c = parse(&cfg("      values: [Open, Fixed, WontFix]\n      success: [Fixed]\n      failure: [WontFix]\n      pending: [pending]\n      require: [{when: [Fixed], fields: [Fixed-in]}]\n      forbid: [{when: [Open], fields: [Fixed-in]}]\n")).unwrap();
        let d = c.record_types["bug"].status_decl().unwrap();
        assert!(d.declares("Open") && !d.declares("Nope"));
        assert!(d.is_terminal("Fixed") && d.is_terminal("WontFix") && !d.is_terminal("Open"));
        assert_eq!(d.required_at("Fixed"), vec!["Fixed-in"]);
        assert_eq!(d.forbidden_at("Open"), vec!["Fixed-in"]);
        assert_eq!(d.is_pending_value(" Pending "), Some(true));
        assert_eq!(d.is_pending_value("pending review"), Some(false));
    }

    #[test]
    fn a_pending_pattern_matches_a_placeholder_family_beside_whole_values_observed_failing() {
        let c = parse(&cfg("      values: [Open]\n      pending: [\"not yet written\", {pattern: \"^\\\\(pending\"}]\n")).unwrap();
        let d = c.record_types["bug"].status_decl().unwrap();
        assert_eq!(d.is_pending_value("(pending review)"), Some(true));
        assert_eq!(d.is_pending_value("(pending ADR-9)"), Some(true));
        assert_eq!(d.is_pending_value(" Not Yet Written "), Some(true));
        // Matches no whole value and no pattern: a real value.
        assert_eq!(d.is_pending_value("pending review"), Some(false));
        assert_eq!(d.is_pending_value("Ana"), Some(false));
    }

    #[test]
    fn an_invalid_or_malformed_pending_pattern_fails_the_load_observed_failing() {
        assert!(
            err("      values: [Open]\n      pending: [{pattern: \"(\"}]\n")
                .contains("regular expression")
        );
        assert!(err("      values: [Open]\n      pending: [{pattern: \"\"}]\n").contains("blank"));
        assert!(err(
            "      values: [Open]\n      pending: [{pattern: \"^a\"}, {pattern: \"^a\"}]\n"
        )
        .contains("twice"));
        assert!(
            err("      values: [Open]\n      pending: [{regex: \"^a\"}]\n").contains("pending")
        );
        assert!(
            err("      values: [Open]\n      pending: [{pattern: \"a\", flags: i}]\n")
                .contains("pending")
        );
    }

    #[test]
    fn implementable_loads_and_need_not_be_terminal_observed_failing() {
        let c = parse(&cfg("      values: [Planned, InProgress, Done]\n      success: [Done]\n      implementable: [Planned, InProgress]\n")).unwrap();
        let d = c.record_types["bug"].status_decl().unwrap();
        assert_eq!(d.implementable, vec!["Planned", "InProgress"]);
        assert!(!d.is_terminal("InProgress"));
    }

    #[test]
    fn claim_retained_loads_and_is_validated_observed_failing() {
        let c = parse(&cfg("      values: [Proposed, Accepted, Superseded]\n      success: [Accepted]\n      claim_retained: [Superseded]\n")).unwrap();
        let d = c.record_types["bug"].status_decl().unwrap();
        assert_eq!(d.claim_retained, vec!["Superseded"]);
        assert!(
            err("      values: [Open]\n      claim_retained: [Gone]\n").contains("claim_retained")
        );
        assert!(
            err("      values: [Open]\n      claim_retained: [Open, Open]\n").contains("twice")
        );
    }

    #[test]
    fn a_type_without_statuses_loads_unchanged() {
        let c = parse("schema_version: 2\nrecord_types:\n  bug:\n    dir: docs/bugs\n").unwrap();
        assert!(c.record_types["bug"].status_decl().is_none());
    }

    #[test]
    fn invalid_declarations_fail_the_load_observed_failing() {
        assert!(err("      values: []\n").contains("values"));
        assert!(err("      values: [Open, Open]\n").contains("Open"));
        assert!(err("      values: [Open, \" \"]\n").contains("blank"));
        assert!(err("      values: [Open]\n      success: [Fixed]\n").contains("Fixed"));
        assert!(
            err("      values: [Open]\n      implementable: [Ready]\n").contains("implementable")
        );
        assert!(err("      values: [Open]\n      implementable: [Open, Open]\n").contains("twice"));
        assert!(err(
            "      values: [Open, Fixed]\n      success: [Fixed]\n      failure: [Fixed]\n"
        )
        .contains("both"));
        assert!(
            err("      values: [Open]\n      require: [{when: [Gone], fields: [Fixed-in]}]\n")
                .contains("Gone")
        );
        assert!(
            err("      values: [Open]\n      require: [{when: [Open], fields: [Nope]}]\n")
                .contains("Nope")
        );
        assert!(err("      values: [Open]\n      require: [{when: [Open], fields: [Fixed-in]}]\n      forbid: [{when: [Open], fields: [Fixed-in]}]\n").contains("Fixed-in"));
        assert!(err("      values: [Open]\n      pending: []\n").contains("pending"));
        assert!(err("      values: [Open]\n      colour: red\n").contains("colour"));
    }

    #[test]
    fn a_key_with_no_value_is_refused_not_read_as_absent() {
        let e =
            parse("schema_version: 2\nrecord_types:\n  bug:\n    dir: docs/bugs\n    statuses:\n")
                .unwrap_err();
        assert!(
            matches!(
                e,
                ConfigError::InvalidDocumentModel {
                    key: "statuses",
                    ..
                }
            ),
            "{e}"
        );
    }
}
