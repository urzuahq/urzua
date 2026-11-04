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
//!       pending: [pending, "not yet written"]
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
    /// Whole field values that mean "not done yet", replacing the compiled-in
    /// substring match for this type (BUG-65). Omitted keeps that match.
    #[serde(default)]
    pub pending: Option<Vec<String>>,
    #[serde(default)]
    pub require: Vec<StatusFieldRule>,
    #[serde(default)]
    pub forbid: Vec<StatusFieldRule>,
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

    /// Whether a whole field value is one of the declared pending values.
    /// `None` when the type declares no pending vocabulary.
    pub fn is_pending_value(&self, value: &str) -> Option<bool> {
        let pending = self.pending.as_ref()?;
        let value = value.trim();
        Some(pending.iter().any(|p| p.trim().eq_ignore_ascii_case(value)))
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

fn validate_one(rt: &RecordTypeConfig, d: &StatusDecl) -> Result<(), String> {
    check_list("values", &d.values, false)?;
    check_list("success", &d.success, true)?;
    check_list("failure", &d.failure, true)?;
    if let Some(p) = &d.pending {
        check_list("pending", p, false)?;
    }
    for (what, list) in [("success", &d.success), ("failure", &d.failure)] {
        if let Some(v) = list.iter().find(|v| !d.declares(v)) {
            return Err(format!("`{what}` names '{v}', which is not in `values`"));
        }
    }
    if let Some(v) = d.success.iter().find(|v| d.failure.contains(v)) {
        return Err(format!("'{v}' is in both `success` and `failure`"));
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
