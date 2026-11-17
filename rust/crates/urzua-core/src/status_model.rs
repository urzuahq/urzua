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
//!       blocked: []               # written statuses that mean "waiting" (optional)
//!       pending: [pending, "not yet written", {pattern: "^\\(pending"}]
//!       require: [{when: [Fixed], fields: [Regression-test]}]
//!       forbid:  [{when: [Open], fields: [Fixed-in]}]
//! ```
//!
//! Transitions between statuses are MILE-144, not this.

use crate::config::{ConfigError, RecordTypeConfig};
use crate::embodiment_model::EmbodimentDecl;
use serde::Deserialize;
use std::collections::BTreeMap;

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StatusDecl {
    /// Every status the type allows. Empty for a type that declares only
    /// `pending` (a suppression type such as a waiver, whose `Reason` is not a
    /// lifecycle vocabulary, `MILE-205`); such a type declares no status.
    #[serde(default)]
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
    /// Non-terminal statuses a person writes to say the record is waiting on a
    /// dependency (`MILE-144`, `RFC-48`). With a sequencing relation declared,
    /// `sequence.status-agreement` holds them to the derived `blocked` value: a
    /// record is blocked when any `Depends-on` target is not in its own type's
    /// `success`. A type declaring none is not examined by that rule.
    #[serde(default)]
    pub blocked: Vec<String>,
    /// Statuses that claim the work has advanced, and so contradict being
    /// derived-blocked (`MILE-144`, `ADR-83`). May include a `success` status
    /// (a record cannot finish before its dependencies do); it never lists a
    /// `failure` or `blocked` status. A status outside the list, `Planned` for
    /// instance, is correct for a blocked record. A type declaring neither this
    /// nor `blocked` is not examined by `sequence.status-agreement`.
    #[serde(default)]
    pub progress: Vec<String>,
    /// Whole field values that mean "not done yet", replacing the compiled-in
    /// substring match for this type (BUG-65). Omitted keeps that match.
    #[serde(default)]
    pub pending: Option<Vec<PendingEntry>>,
    /// Whole field values that are unedited template text, compared without
    /// regard to case, replacing the compiled-in placeholder words for this
    /// type (`MILE-233`). Omitted keeps those words.
    #[serde(default)]
    pub placeholders: Option<Vec<String>>,
    #[serde(default)]
    pub require: Vec<StatusFieldRule>,
    #[serde(default)]
    pub forbid: Vec<StatusFieldRule>,
    /// How the type's `Status` is held to its delivery evidence (`MILE-183`).
    #[serde(default)]
    pub delivery: Option<DeliveryDecl>,
    /// The statuses a new record may start at (`MILE-144`, `ADR-72`). Required
    /// with `transitions`, and the roots `unreachable` walks from.
    #[serde(default)]
    pub initial: Vec<String>,
    /// Which statuses may follow each status (`ADR-72`). A declared status with
    /// no entry is terminal: nothing may follow it. Empty means the type
    /// declares no lifecycle and no transition is judged.
    #[serde(default)]
    pub transitions: BTreeMap<String, Vec<String>>,
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
    /// of these is a status with no evidence. May be empty: the type then never
    /// reports a success status for lack of evidence (`ADR-84`).
    pub unevidenced_tiers: Vec<String>,
    /// Statuses that say the work is not done. Delivered evidence under one of
    /// them is a disagreement.
    pub not_done: Vec<String>,
    /// The `success` status `urzua fix` writes when the evidence says delivered.
    pub set_status: String,
    /// Whether a source or test back-pointer that cites the record counts as
    /// delivery evidence beside the evidence field (`ADR-84`). Omitted keeps
    /// today's behaviour: it counts. `false` holds the type to its declared
    /// evidence field alone.
    #[serde(default = "back_pointer_evidence_default")]
    pub back_pointer_evidence: bool,
    /// Opt-in per-item evidence (`ADR-96`, `MILE-214`): the Done-means section
    /// becomes a list whose items each name their evidence, and the status is
    /// computed from the items. Omitted keeps today's behaviour byte for byte.
    #[serde(default)]
    pub items: Option<crate::done_items::ItemsDecl>,
}

impl DeliveryDecl {
    /// The tiers at which an item's own locators resolve it: the declared
    /// `items.resolving_tiers`, else `delivered_tiers`.
    pub fn resolving_tiers(&self) -> &[String] {
        self.items
            .as_ref()
            .and_then(|i| i.resolving_tiers.as_deref())
            .unwrap_or(&self.delivered_tiers)
    }
}

fn back_pointer_evidence_default() -> bool {
    true
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
    pub(crate) fn matches(&self, value: &str) -> bool {
        match self {
            PendingEntry::Value(v) => v.trim().eq_ignore_ascii_case(value),
            // Validated at load, so a pattern that does not compile here (a
            // hand-built declaration) simply matches nothing.
            PendingEntry::Pattern(p) => compiled(&p.pattern).is_some_and(|re| re.is_match(value)),
        }
    }
}

/// A pending pattern, compiled once per process. A pending value is asked of
/// every field of every record by several rules, so compiling per call was a
/// third of a single-record `check --stdin-path` run (`MILE-191`).
fn compiled(pattern: &str) -> Option<std::sync::Arc<regex::Regex>> {
    use std::collections::HashMap;
    use std::sync::{Arc, Mutex, OnceLock};
    static CACHE: OnceLock<Mutex<HashMap<String, Option<Arc<regex::Regex>>>>> = OnceLock::new();
    let mut cache = CACHE.get_or_init(Default::default).lock().ok()?;
    cache
        .entry(pattern.to_string())
        .or_insert_with(|| regex::Regex::new(pattern).ok().map(Arc::new))
        .clone()
}

/// Fields tied to the statuses in `when` and to the values of other fields in
/// `when_field`. Each condition present must hold; at least one
/// is declared. An empty `when` places no condition on the status.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StatusFieldRule {
    #[serde(default)]
    pub when: Vec<String>,
    #[serde(
        default,
        deserialize_with = "crate::field_values::deserialize_when_field"
    )]
    pub when_field: BTreeMap<String, Vec<String>>,
    pub fields: Vec<String>,
}

/// A field an entry ties to a record, whether the entry keys on the status,
/// and the field conditions that held, in field-name order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Triggered<'a> {
    pub field: &'a str,
    pub on_status: bool,
    pub triggers: Vec<(&'a str, String)>,
}

impl StatusFieldRule {
    /// Whether the entry applies to a record at `status` whose header reads
    /// `value`, and the element of each conditioned field that met it.
    fn applies<'a>(
        &'a self,
        status: &str,
        value: &dyn Fn(&str) -> Option<String>,
    ) -> Option<Vec<(&'a str, String)>> {
        if !self.when.is_empty() && !self.when.iter().any(|w| w == status) {
            return None;
        }
        let mut hits = Vec::new();
        for (field, values) in &self.when_field {
            let held = value(field)?;
            let hit =
                crate::field_values::elements(&held).find(|e| values.iter().any(|v| v == e))?;
            hits.push((field.as_str(), hit.to_string()));
        }
        Some(hits)
    }

    /// Whether some record could meet both entries: their statuses overlap and
    /// every field both condition on shares a value.
    fn overlaps(&self, other: &StatusFieldRule) -> bool {
        let statuses = self.when.is_empty()
            || other.when.is_empty()
            || self.when.iter().any(|w| other.when.contains(w));
        statuses
            && self.when_field.iter().all(|(f, vs)| {
                other
                    .when_field
                    .get(f)
                    .is_none_or(|ws| vs.iter().any(|v| ws.contains(v)))
            })
    }
}

impl StatusDecl {
    pub fn declares(&self, status: &str) -> bool {
        self.values.iter().any(|v| v == status)
    }

    /// The declared vocabulary for a message: the values, or a plain statement
    /// that the type declares none (never an empty list).
    pub fn declared_summary(&self) -> String {
        if self.values.is_empty() {
            "the type declares no values".to_string()
        } else {
            format!("declared: {}", self.values.join(", "))
        }
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

    /// Whether a written status says the record is waiting (`blocked`).
    pub fn is_blocked_status(&self, status: &str) -> bool {
        self.blocked.iter().any(|v| v == status)
    }

    /// Whether a written status claims the work has advanced (`progress`).
    pub fn is_progress_status(&self, status: &str) -> bool {
        self.progress.iter().any(|v| v == status)
    }

    /// Whether `status` finishes the work: the type's `success` list.
    pub fn is_success(&self, status: &str) -> bool {
        self.success.iter().any(|v| v == status)
    }

    /// Whether the type declares a lifecycle graph (`ADR-72`).
    pub fn has_transitions(&self) -> bool {
        !self.transitions.is_empty()
    }

    /// Whether `to` may follow `from`. Only meaningful when
    /// [`has_transitions`](Self::has_transitions).
    pub fn allows(&self, from: &str, to: &str) -> bool {
        self.transitions
            .get(from)
            .is_some_and(|next| next.iter().any(|n| n == to))
    }

    /// Whether a new record may start at `status`.
    pub fn is_initial(&self, status: &str) -> bool {
        self.initial.iter().any(|s| s == status)
    }

    /// Declared statuses no path of `transitions` reaches from `initial`, in
    /// `values` order. Empty for a type declaring no graph.
    pub fn unreachable(&self) -> Vec<&str> {
        if !self.has_transitions() {
            return Vec::new();
        }
        let mut reached: Vec<&str> = self.initial.iter().map(String::as_str).collect();
        let mut i = 0;
        while i < reached.len() {
            for next in self.transitions.get(reached[i]).into_iter().flatten() {
                if !reached.contains(&next.as_str()) {
                    reached.push(next);
                }
            }
            i += 1;
        }
        self.values
            .iter()
            .map(String::as_str)
            .filter(|v| !reached.contains(v))
            .collect()
    }

    /// The fields required of a record at `status` whose header reads `value`.
    pub fn required_for<'a>(
        &'a self,
        status: &str,
        value: &dyn Fn(&str) -> Option<String>,
    ) -> Vec<Triggered<'a>> {
        fields_for(&self.require, status, value)
    }

    /// The fields forbidden to a record at `status` whose header reads `value`.
    pub fn forbidden_for<'a>(
        &'a self,
        status: &str,
        value: &dyn Fn(&str) -> Option<String>,
    ) -> Vec<Triggered<'a>> {
        fields_for(&self.forbid, status, value)
    }

    /// Whether any entry conditions on a field rather than only the status.
    pub fn has_field_conditions(&self) -> bool {
        self.require
            .iter()
            .chain(&self.forbid)
            .any(|r| !r.when_field.is_empty())
    }
}

fn fields_for<'a>(
    rules: &'a [StatusFieldRule],
    status: &str,
    value: &dyn Fn(&str) -> Option<String>,
) -> Vec<Triggered<'a>> {
    let mut out: Vec<Triggered> = Vec::new();
    for rule in rules {
        let Some(triggers) = rule.applies(status, value) else {
            continue;
        };
        for f in &rule.fields {
            if !out.iter().any(|t| t.field == f) {
                out.push(Triggered {
                    field: f,
                    on_status: !rule.when.is_empty(),
                    triggers: triggers.clone(),
                });
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

/// The smallest declaration of a checklist that loads: every key it needs, no
/// optional one. Names are the repository's to replace; the load error shows
/// it, and a test loads it.
pub const SMALLEST_CHECKLIST: &str = "\
statuses:
  values: [Open, InProgress, Done]
  success: [Done]
  delivery:
    delivered_tiers: [Implemented]
    unevidenced_tiers: []
    not_done: [Open, InProgress]
    set_status: Done
    items:
      section: Acceptance
      clause_separator: \" — \"
      evidence_marker: \"evidence:\"
      waiver_marker: \"waived:\"
      partial_status: InProgress
sections: {from: headings, depth: 2, required: [{name: Acceptance}]}";

const DELIVERY_KEYS: [&str; 4] = [
    "delivered_tiers",
    "unevidenced_tiers",
    "not_done",
    "set_status",
];
const ITEMS_KEYS: [&str; 5] = [
    "section",
    "clause_separator",
    "evidence_marker",
    "waiver_marker",
    "partial_status",
];

/// An incomplete checklist declaration says which key is missing, where it
/// belongs, and shows the smallest declaration that loads.
fn checklist_hint(raw: &yaml_serde::Value, reason: String) -> String {
    let Some(delivery) = raw.get("delivery") else {
        return reason;
    };
    let declares_items = delivery.get("items").is_some();
    let missing = reason
        .strip_prefix("missing field `")
        .and_then(|r| r.split('`').next());
    let location = match missing {
        Some(k) if ITEMS_KEYS.contains(&k) && declares_items => {
            Some(format!("statuses.delivery.items.{k}"))
        }
        Some(k) if DELIVERY_KEYS.contains(&k) && declares_items => {
            Some(format!("statuses.delivery.{k}"))
        }
        _ => None,
    };
    let section_not_required = reason.starts_with("`delivery.items.section` names");
    let reason = match location {
        Some(at) => format!("`{at}` is missing"),
        None if section_not_required => reason,
        None => return reason,
    };
    format!(
        "{reason}. The smallest checklist declaration that loads, as the record type's keys (replace the names with its own):\n{SMALLEST_CHECKLIST}"
    )
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

/// `placeholders` replaces the compiled-in words, so an empty list would turn
/// placeholder detection off without saying so (`SPEC-1`): it is refused, as
/// are a blank word and one listed twice ignoring case.
fn check_placeholders(words: &[String]) -> Result<(), String> {
    if words.is_empty() {
        return Err("`placeholders` is empty, which would read no value as a placeholder; list at least one word or omit the key to keep the built-in words".into());
    }
    for (i, w) in words.iter().enumerate() {
        if w.trim().is_empty() || w.trim() != w {
            return Err(format!(
                "`placeholders` lists '{w}', which is blank or has surrounding whitespace"
            ));
        }
        if words[..i]
            .iter()
            .any(|v| v.to_lowercase() == w.to_lowercase())
        {
            return Err(format!("`placeholders` lists '{w}' twice"));
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

fn validate_one(
    rt: &RecordTypeConfig,
    d: &StatusDecl,
    embodiment: &EmbodimentDecl,
) -> Result<(), String> {
    // A type may declare `pending` and no `values` (`MILE-205`); a declaration
    // of neither says nothing and would be a silent no-op, so it is refused.
    if d.values.is_empty() && d.pending.is_none() {
        return Err(
            "`values` must list at least one entry, or the type must declare `pending`".into(),
        );
    }
    check_list("values", &d.values, true)?;
    check_list("success", &d.success, true)?;
    check_list("failure", &d.failure, true)?;
    check_list("implementable", &d.implementable, true)?;
    check_list("claim_retained", &d.claim_retained, true)?;
    check_list("blocked", &d.blocked, true)?;
    check_list("progress", &d.progress, true)?;
    if let Some(p) = &d.pending {
        check_pending(p)?;
    }
    if let Some(p) = &d.placeholders {
        check_placeholders(p)?;
    }
    for (what, list) in [
        ("success", &d.success),
        ("failure", &d.failure),
        ("implementable", &d.implementable),
        ("claim_retained", &d.claim_retained),
        ("blocked", &d.blocked),
        ("progress", &d.progress),
    ] {
        if let Some(v) = list.iter().find(|v| !d.declares(v)) {
            return Err(format!("`{what}` names '{v}', which is not in `values`"));
        }
    }
    if let Some(v) = d.success.iter().find(|v| d.failure.contains(v)) {
        return Err(format!("'{v}' is in both `success` and `failure`"));
    }
    // A blocked status is a waiting state: a terminal one is finished, and
    // `sequence.status-agreement` would contradict itself over it.
    if let Some(v) = d.blocked.iter().find(|v| d.is_terminal(v)) {
        return Err(format!(
            "'{v}' is in `blocked` and is also terminal (`success` or `failure`); a blocked status is a waiting state"
        ));
    }
    // A progress status says the work advanced; a failed record has not, and a
    // waiting status cannot also be one (`ADR-83`).
    if let Some(v) = d.progress.iter().find(|v| d.failure.contains(v)) {
        return Err(format!("'{v}' is in both `progress` and `failure`"));
    }
    if let Some(v) = d.progress.iter().find(|v| d.blocked.contains(v)) {
        return Err(format!(
            "'{v}' is in both `progress` and `blocked`; a record cannot both claim progress and be waiting"
        ));
    }
    validate_transitions(d)?;
    if let Some(delivery) = &d.delivery {
        validate_delivery(rt, d, delivery, embodiment)?;
    }
    validate_field_rules(rt, d)
}

fn validate_field_rules(rt: &RecordTypeConfig, d: &StatusDecl) -> Result<(), String> {
    let declared = rt.known_fields.as_ref().map(|_| rt.declared_fields());
    for (what, rules) in [("require", &d.require), ("forbid", &d.forbid)] {
        for rule in rules {
            if rule.when.is_empty() && rule.when_field.is_empty() {
                return Err(format!(
                    "a `{what}` entry declares neither `when` nor `when_field`, so it would never apply"
                ));
            }
            check_list(&format!("{what}.when"), &rule.when, true)?;
            check_list(&format!("{what}.fields"), &rule.fields, false)?;
            if let Some(w) = rule.when.iter().find(|w| !d.declares(w)) {
                return Err(format!(
                    "`{what}.when` names '{w}', which is not in `values`"
                ));
            }
            check_when_field(rt, what, rule)?;
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
        let required = status_only_fields(&d.require, status);
        if let Some(f) = status_only_fields(&d.forbid, status)
            .into_iter()
            .find(|f| required.contains(f))
        {
            return Err(format!(
                "'{f}' is both required and forbidden at '{status}'"
            ));
        }
    }
    for r in &d.require {
        for f in d.forbid.iter().filter(|f| r.overlaps(f)) {
            if let Some(field) = r.fields.iter().find(|x| f.fields.contains(x)) {
                return Err(format!(
                    "'{field}' is both required and forbidden for a record that meets both entries ({})",
                    describe(r)
                ));
            }
        }
    }
    Ok(())
}

fn status_only_fields<'a>(rules: &'a [StatusFieldRule], status: &str) -> Vec<&'a String> {
    rules
        .iter()
        .filter(|r| r.when_field.is_empty() && r.when.iter().any(|w| w == status))
        .flat_map(|r| &r.fields)
        .collect()
}

fn describe(rule: &StatusFieldRule) -> String {
    let mut parts = Vec::new();
    if !rule.when.is_empty() {
        parts.push(format!("when: {}", rule.when.join(", ")));
    }
    for (f, vs) in &rule.when_field {
        parts.push(format!("{f}: {}", vs.join(", ")));
    }
    parts.join("; ")
}

/// A field condition names a field the type declares, other than the status
/// (which `when` covers), and values from its vocabulary when it has one.
fn check_when_field(
    rt: &RecordTypeConfig,
    what: &str,
    rule: &StatusFieldRule,
) -> Result<(), String> {
    let declared = rt.declared_fields();
    let status_field = rt.relation_field(crate::config::RelationRole::Status);
    for (field, values) in &rule.when_field {
        let key = format!("{what}.when_field.{field}");
        if field == status_field {
            return Err(format!(
                "`{key}` conditions on the status field; name its values under `{what}.when`"
            ));
        }
        if !declared.contains(&crate::values::FieldName::new(field.as_str())) {
            return Err(format!(
                "`{key}` names '{field}', which is in neither `required_fields` nor `known_fields`"
            ));
        }
        check_list(&key, values, false)?;
        if let Some(vocab) = rt.field_vocabulary(field) {
            if let Some(v) = values.iter().find(|v| !vocab.contains(v)) {
                return Err(format!(
                    "`{key}` names '{v}', which is not in `field_values.{field}` ({})",
                    vocab.join(", ")
                ));
            }
        }
    }
    Ok(())
}

/// A lifecycle graph names only declared statuses, starts somewhere, and never
/// lists a status as following itself (`ADR-72`). Half a graph is refused: with
/// no `initial` nothing is reachable, and `initial` with no graph judges
/// nothing, which would be a silent no-op (`ADR-7`).
fn validate_transitions(d: &StatusDecl) -> Result<(), String> {
    if d.transitions.is_empty() && d.initial.is_empty() {
        return Ok(());
    }
    if d.transitions.is_empty() {
        return Err(
            "`initial` is declared without `transitions`; declare the graph or remove it".into(),
        );
    }
    if d.initial.is_empty() {
        return Err(
            "`transitions` is declared without `initial`; name where a new record may start".into(),
        );
    }
    check_list("initial", &d.initial, false)?;
    if let Some(v) = d.initial.iter().find(|v| !d.declares(v)) {
        return Err(format!("`initial` names '{v}', which is not in `values`"));
    }
    for (from, to) in &d.transitions {
        if !d.declares(from) {
            return Err(format!(
                "`transitions` has '{from}', which is not in `values`"
            ));
        }
        check_list(&format!("transitions.{from}"), to, true)?;
        if let Some(v) = to.iter().find(|v| !d.declares(v)) {
            return Err(format!(
                "`transitions.{from}` names '{v}', which is not in `values`"
            ));
        }
        if to.contains(from) {
            return Err(format!(
                "`transitions.{from}` lists '{from}' itself; a status that stays is not a transition"
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
    embodiment: &EmbodimentDecl,
) -> Result<(), String> {
    let tiers: Vec<&str> = embodiment.evidence_states().collect();
    check_list("delivery.delivered_tiers", &delivery.delivered_tiers, false)?;
    // Empty declares the type never reports a success status for lack of
    // evidence (`ADR-84`): a type whose evidence is legitimately absent for
    // some done records (a documentation or performance change) opts out of
    // that direction and keeps the other.
    check_list(
        "delivery.unevidenced_tiers",
        &delivery.unevidenced_tiers,
        true,
    )?;
    check_list("delivery.not_done", &delivery.not_done, false)?;
    for (what, list) in [
        ("delivery.delivered_tiers", &delivery.delivered_tiers),
        ("delivery.unevidenced_tiers", &delivery.unevidenced_tiers),
    ] {
        if let Some(t) = list.iter().find(|t| !tiers.contains(&t.as_str())) {
            return Err(format!(
                "`{what}` names '{t}', which is not a tier computed from evidence (one of: {})",
                tiers.join(", ")
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
    // A type judged from checklist items reads each item's own evidence
    // clause; the header field only adds to it, so it need not be declared.
    if delivery.items.is_none() {
        if let Some(known) = rt.known_fields.as_ref().map(|_| rt.declared_fields()) {
            if !known.contains(&crate::values::FieldName::new(locator)) {
                return Err(format!(
                "`delivery` reads the evidence field '{locator}', which is in neither `required_fields` nor `known_fields`"
            ));
            }
        }
    }
    if let Some(items) = &delivery.items {
        validate_items(rt, d, delivery, items, embodiment)?;
        if let Some(pending) = &items.pending {
            check_pending(pending).map_err(|e| format!("`delivery.items.pending`: {e}"))?;
        }
        check_delivered_tiers_resolve(delivery, "delivery.items")?;
    }
    Ok(())
}

/// `statuses.delivery.items` is refused at load when it would be inert or
/// ambiguous (`ADR-96` section 4, `BUG-44`): every error names the key.
/// An item resolves only at a `resolving_tiers` tier, so a delivered tier
/// outside that list is a tier no item could ever reach.
fn check_delivered_tiers_resolve(delivery: &DeliveryDecl, key: &str) -> Result<(), String> {
    match delivery
        .delivered_tiers
        .iter()
        .find(|t| !delivery.resolving_tiers().contains(t))
    {
        Some(t) => Err(format!(
            "type `delivery.delivered_tiers` names '{t}', which is not in `{key}.resolving_tiers` ({}): no item could resolve at it",
            delivery.resolving_tiers().join(", ")
        )),
        None => Ok(()),
    }
}

fn validate_items(
    rt: &RecordTypeConfig,
    d: &StatusDecl,
    delivery: &DeliveryDecl,
    items: &crate::done_items::ItemsDecl,
    embodiment: &EmbodimentDecl,
) -> Result<(), String> {
    let key = "delivery.items";
    if items.section.trim().is_empty() {
        return Err(format!("`{key}.section` is blank"));
    }
    let required: Vec<String> = crate::document_model::declared_model("", rt)
        .ok()
        .flatten()
        .and_then(|m| m.sections)
        .map(|s| s.required.into_iter().map(|r| r.name).collect())
        .unwrap_or_default();
    if !required.contains(&items.section) {
        return Err(format!(
            "`{key}.section` names '{}', which the type does not require: declare it under `sections.required` (required: {})",
            items.section,
            if required.is_empty() { "none".to_string() } else { required.join(", ") }
        ));
    }
    validate_item_markers(items)?;
    if !d.declares(&items.partial_status) {
        return Err(format!(
            "`{key}.partial_status` names '{}', which is not in `values`",
            items.partial_status
        ));
    }
    if !delivery.not_done.contains(&items.partial_status) {
        return Err(format!(
            "`{key}.partial_status` names '{}', which is not in `delivery.not_done`",
            items.partial_status
        ));
    }
    if d.has_transitions() {
        if let Some(from) = delivery
            .not_done
            .iter()
            .find(|s| **s != items.partial_status && !d.allows(s, &items.partial_status))
        {
            return Err(format!(
                "`{key}.partial_status` names '{}', which `transitions` does not allow from '{from}': `fix` could not repair a record at '{from}' to it",
                items.partial_status
            ));
        }
    }
    if let Some(tiers) = &items.resolving_tiers {
        check_list(&format!("{key}.resolving_tiers"), tiers, false)?;
        let known: Vec<&str> = embodiment.tiers.iter().map(|t| t.state.as_str()).collect();
        if let Some(t) = tiers.iter().find(|t| !known.contains(&t.as_str())) {
            return Err(format!(
                "`{key}.resolving_tiers` names '{t}', which is not a declared embodiment tier (one of: {})",
                known.join(", ")
            ));
        }
    }
    if let Some(kind) = &items.kind {
        validate_kind(rt, delivery, kind, embodiment)?;
    }
    validate_waiver_categories(items)
}

/// The clause markers: none blank, and none a prefix of the other, so a clause
/// can be told apart.
fn validate_item_markers(items: &crate::done_items::ItemsDecl) -> Result<(), String> {
    let key = "delivery.items";
    for (what, value) in [
        ("clause_separator", &items.clause_separator),
        ("evidence_marker", &items.evidence_marker),
        ("waiver_marker", &items.waiver_marker),
    ] {
        if value.trim().is_empty() {
            return Err(format!("`{key}.{what}` is blank"));
        }
    }
    if items.evidence_marker == items.waiver_marker {
        return Err(format!(
            "`{key}.evidence_marker` and `{key}.waiver_marker` are both '{}'",
            items.evidence_marker
        ));
    }
    if items.evidence_marker.starts_with(&items.waiver_marker)
        || items.waiver_marker.starts_with(&items.evidence_marker)
    {
        return Err(format!(
            "`{key}`: one of the markers '{}' and '{}' is a prefix of the other, so a clause could not be told apart",
            items.evidence_marker, items.waiver_marker
        ));
    }
    Ok(())
}

/// The waiver categories: named plainly, listed once, and a category that
/// expires also cites its target.
fn validate_waiver_categories(items: &crate::done_items::ItemsDecl) -> Result<(), String> {
    let key = "delivery.items";
    let mut seen = std::collections::BTreeSet::new();
    for c in &items.waiver_categories {
        if c.name.trim().is_empty() {
            return Err(format!("`{key}.waiver_categories` has a blank name"));
        }
        if c.name
            .contains(|ch: char| ch.is_whitespace() || "(),".contains(ch))
        {
            return Err(format!(
                "`{key}.waiver_categories` name '{}' holds whitespace, a parenthesis or a comma, which the waiver clause cannot carry",
                c.name
            ));
        }
        if !seen.insert(c.name.as_str()) {
            return Err(format!(
                "`{key}.waiver_categories` lists '{}' twice",
                c.name
            ));
        }
        if c.expires_required() && !c.cite_required() {
            return Err(format!(
                "`{key}.waiver_categories` '{}' requires an expiry but no cite: a deferral needs a target (`cite: required`)",
                c.name
            ));
        }
    }
    Ok(())
}

/// `statuses.delivery.items.kind` is refused at load when it would be inert or
/// contradictory (`ADR-96`, amendment of 2026-11-07, `BUG-44`): a field the type
/// does not carry, a kind no record can be told from another, and a delivered
/// tier no item can reach each read as a kind that silently never applies.
fn validate_kind(
    rt: &RecordTypeConfig,
    delivery: &DeliveryDecl,
    kind: &crate::done_items::KindDecl,
    embodiment: &EmbodimentDecl,
) -> Result<(), String> {
    let key = "delivery.items.kind";
    if kind.field.trim().is_empty() {
        return Err(format!("`{key}.field` is blank"));
    }
    if let Some(known) = rt.known_fields.as_ref().map(|_| rt.declared_fields()) {
        if !known.contains(&crate::values::FieldName::new(&kind.field)) {
            return Err(format!(
                "`{key}.field` names '{}', which the type does not declare: add it to `required_fields` or `known_fields`",
                kind.field
            ));
        }
    }
    if kind.kinds.is_empty() {
        return Err(format!("`{key}.kinds` is empty; name at least one kind"));
    }
    let known: Vec<&str> = embodiment.tiers.iter().map(|t| t.state.as_str()).collect();
    let mut seen = std::collections::BTreeSet::new();
    for entry in &kind.kinds {
        if entry.name.trim().is_empty() {
            return Err(format!("`{key}.kinds` has a blank name"));
        }
        if !seen.insert(entry.name.as_str()) {
            return Err(format!("`{key}.kinds` lists '{}' twice", entry.name));
        }
        let Some(tiers) = &entry.delivered_tiers else {
            continue;
        };
        check_list(
            &format!("{key}.kinds.{}.delivered_tiers", entry.name),
            tiers,
            false,
        )?;
        for t in tiers {
            if !known.contains(&t.as_str()) {
                return Err(format!(
                    "`{key}.kinds.{}.delivered_tiers` names '{t}', which is not a declared embodiment tier (one of: {})",
                    entry.name,
                    known.join(", ")
                ));
            }
            if !delivery.resolving_tiers().contains(t) {
                return Err(format!(
                    "`{key}.kinds.{}.delivered_tiers` names '{t}', which is not in `resolving_tiers`: no item could resolve at it",
                    entry.name
                ));
            }
            if delivery.unevidenced_tiers.contains(t) {
                return Err(format!(
                    "`{key}.kinds.{}.delivered_tiers` names '{t}', which is in `delivery.unevidenced_tiers`",
                    entry.name
                ));
            }
        }
    }
    Ok(())
}

/// Validates every type's `statuses` declaration at load. A key present with
/// no value is refused rather than read as absent, which would turn a
/// half-written declaration into a silent no-op (ADR-7).
pub(crate) fn validate(
    types: &BTreeMap<String, RecordTypeConfig>,
    embodiment: &EmbodimentDecl,
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
        let decl: StatusDecl = yaml_serde::from_value(raw.clone())
            .map_err(|e| invalid(name, checklist_hint(raw, e.to_string())))?;
        validate_one(rt, &decl, embodiment)
            .map_err(|reason| invalid(name, checklist_hint(raw, reason)))?;
        out.insert(name.clone(), decl);
    }
    Ok(out)
}

/// A type that declares `delivery.items` is held to it by
/// `status.delivery-agreement`, which is on by declaration; a configuration
/// that turns it off would compute a status nothing holds the record to
/// (`ADR-96` section 4, `BUG-44`), so that one is refused.
pub(crate) fn validate_items_rule(
    types: &BTreeMap<String, RecordTypeConfig>,
    decls: &BTreeMap<String, StatusDecl>,
    rules: &BTreeMap<String, crate::config::RuleSetting>,
) -> Result<(), ConfigError> {
    let rule = crate::rules::RULE_STATUS_DELIVERY_AGREEMENT;
    let off = rules
        .get(rule)
        .is_some_and(|s| s.level == crate::config::RuleLevel::Off);
    for (name, decl) in decls {
        let declares = decl.delivery.as_ref().is_some_and(|d| d.items.is_some());
        if declares && off && types.contains_key(name) {
            return Err(invalid(
                name,
                format!(
                    "`delivery.items` is declared, but `{rule}` is turned off under `rules:`: its `DoneWithUnresolvedItems` finding is what holds a status to the items, so remove the entry (the rule is on wherever a checklist is declared) or give it a level"
                ),
            ));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{Triggered, SMALLEST_CHECKLIST};
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
        let none = |_: &str| None;
        fn fields(t: Vec<Triggered<'_>>) -> Vec<&str> {
            t.into_iter().map(|t| t.field).collect()
        }
        assert_eq!(fields(d.required_for("Fixed", &none)), vec!["Fixed-in"]);
        assert_eq!(fields(d.forbidden_for("Open", &none)), vec!["Fixed-in"]);
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
    fn blocked_loads_and_is_validated() {
        let c = parse(&cfg("      values: [Planned, Blocked, Done]\n      success: [Done]\n      blocked: [Blocked]\n")).unwrap();
        let d = c.record_types["bug"].status_decl().unwrap();
        assert_eq!(d.blocked, vec!["Blocked"]);
        assert!(d.is_blocked_status("Blocked") && !d.is_blocked_status("Planned"));
        assert!(d.is_success("Done") && !d.is_success("Blocked"));
        assert!(err("      values: [Open]\n      blocked: [Gone]\n").contains("blocked"));
        assert!(
            err("      values: [Open, Blocked]\n      blocked: [Blocked, Blocked]\n")
                .contains("twice")
        );
        assert!(
            err("      values: [Open, Done]\n      success: [Done]\n      blocked: [Done]\n")
                .contains("waiting state")
        );
        assert!(err(
            "      values: [Open, Dropped]\n      failure: [Dropped]\n      blocked: [Dropped]\n"
        )
        .contains("waiting state"));
    }

    #[test]
    fn progress_loads_and_is_validated() {
        let c = parse(&cfg("      values: [Planned, InProgress, Done, Dropped]\n      success: [Done]\n      failure: [Dropped]\n      progress: [InProgress, Done]\n")).unwrap();
        let d = c.record_types["bug"].status_decl().unwrap();
        assert!(d.is_progress_status("InProgress") && d.is_progress_status("Done"));
        assert!(!d.is_progress_status("Planned"));
        assert!(err("      values: [Open]\n      progress: [Gone]\n").contains("progress"));
        assert!(
            err("      values: [Open, Doing]\n      progress: [Doing, Doing]\n").contains("twice")
        );
        assert!(err(
            "      values: [Open, Dropped]\n      failure: [Dropped]\n      progress: [Dropped]\n"
        )
        .contains("both `progress` and `failure`"));
        // A written waiting status cannot also claim progress.
        assert!(err(
            "      values: [Open, Blocked]\n      blocked: [Blocked]\n      progress: [Blocked]\n"
        )
        .contains("both `progress` and `blocked`"));
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

    const PENDING_ONLY: &str = "      pending: [pending]\n";

    #[test]
    fn a_type_declaring_pending_and_no_values_loads_observed_failing() {
        let c = parse(&cfg(PENDING_ONLY)).unwrap();
        let d = c.record_types["bug"].status_decl().unwrap();
        assert!(d.values.is_empty());
        assert_eq!(d.is_pending_value("pending"), Some(true));
        assert_eq!(d.is_pending_value("Reason"), Some(false));
    }

    #[test]
    fn an_explicit_empty_values_list_loads_beside_pending() {
        let c = parse(&cfg("      values: []\n      pending: [pending]\n")).unwrap();
        assert!(c.record_types["bug"]
            .status_decl()
            .unwrap()
            .values
            .is_empty());
    }

    #[test]
    fn a_declaration_of_neither_values_nor_pending_is_refused() {
        assert!(err("      values: []\n").contains("values"));
        assert!(err("      {}\n").contains("values"));
    }

    /// Reader: `StatusDecl::declares`. Nothing is declared by an empty vocabulary.
    #[test]
    fn declares_is_false_for_every_status_when_no_values_are_declared() {
        let d = decl(PENDING_ONLY);
        assert!(!d.declares("Open") && !d.declares(""));
        // Unchanged when values are declared.
        assert!(decl("      values: [Open, Fixed]\n").declares("Open"));
    }

    /// Reader: `StatusDecl::unreachable` and `has_transitions`.
    #[test]
    fn a_type_with_no_values_has_no_graph_and_nothing_unreachable() {
        let d = decl(PENDING_ONLY);
        assert!(!d.has_transitions());
        assert!(d.unreachable().is_empty());
        // A graph needs declared statuses to name: refused, not silently ignored.
        assert!(err("      pending: [pending]\n      initial: [Open]\n      transitions:\n        Open: [Fixed]\n")
            .contains("values"));
    }

    /// Reader: the `require`/`forbid` checks over `values` (`validate_one`).
    #[test]
    fn require_and_forbid_on_a_type_with_no_values_name_a_status_it_does_not_declare() {
        assert!(err(
            "      pending: [pending]\n      require: [{when: [Open], fields: [Fixed-in]}]\n"
        )
        .contains("not in `values`"));
        assert!(err(
            "      pending: [pending]\n      forbid: [{when: [Open], fields: [Fixed-in]}]\n"
        )
        .contains("not in `values`"));
    }

    /// Readers: `claim_retained` (claim expiry) and `has_transitions` (diff scope).
    #[test]
    fn a_type_with_no_values_retains_no_claim() {
        let d = decl(PENDING_ONLY);
        assert!(d.claim_retained.is_empty() && !d.has_transitions());
    }

    /// Reader: `first_non_progress_status`.
    #[test]
    fn no_first_non_progress_status_exists_when_no_values_are_declared() {
        assert_eq!(
            crate::rules::first_non_progress_status(&decl(PENDING_ONLY)),
            None
        );
        assert_eq!(
            crate::rules::first_non_progress_status(&decl(
                "      values: [Open, Fixed]\n      success: [Fixed]\n"
            )),
            Some("Open")
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

    fn decl(statuses: &str) -> super::StatusDecl {
        let c = parse(&cfg(statuses)).unwrap();
        c.record_types["bug"].status_decl().unwrap()
    }

    const GRAPH: &str = "      values: [Open, Fixed, WontFix, Lost]\n      success: [Fixed]\n      failure: [WontFix]\n      initial: [Open]\n      transitions:\n        Open: [Fixed, WontFix]\n        Fixed: [Open]\n";

    #[test]
    fn a_declared_graph_answers_what_may_follow_and_what_is_unreachable() {
        let d = decl(GRAPH);
        assert!(d.has_transitions());
        assert!(d.allows("Open", "Fixed"));
        assert!(d.allows("Fixed", "Open"));
        assert!(!d.allows("Fixed", "WontFix"));
        // A status with no entry is terminal: nothing follows it.
        assert!(!d.allows("WontFix", "Open"));
        assert!(d.is_initial("Open") && !d.is_initial("Fixed"));
        assert_eq!(d.unreachable(), ["Lost"]);
    }

    #[test]
    fn a_type_declaring_no_graph_has_no_transitions() {
        let d = decl("      values: [Open, Fixed]\n");
        assert!(!d.has_transitions());
        assert!(d.unreachable().is_empty());
    }

    #[test]
    fn an_invalid_graph_fails_the_load() {
        let base = "      values: [Open, Fixed]\n";
        assert!(err(&format!(
            "{base}      transitions:\n        Open: [Fixed]\n"
        ))
        .contains("initial"));
        assert!(err(&format!("{base}      initial: [Open]\n")).contains("transitions"));
        assert!(err(&format!(
            "{base}      initial: [Gone]\n      transitions:\n        Open: [Fixed]\n"
        ))
        .contains("Gone"));
        assert!(err(&format!(
            "{base}      initial: [Open]\n      transitions:\n        Gone: [Fixed]\n"
        ))
        .contains("Gone"));
        assert!(err(&format!(
            "{base}      initial: [Open]\n      transitions:\n        Open: [Gone]\n"
        ))
        .contains("Gone"));
        assert!(err(&format!(
            "{base}      initial: [Open]\n      transitions:\n        Open: [Open]\n"
        ))
        .contains("itself"));
        assert!(err(&format!(
            "{base}      initial: [Open]\n      transitions:\n        Open: [Fixed, Fixed]\n"
        ))
        .contains("twice"));
    }

    fn kinds(statuses: &str) -> Result<crate::config::Config, ConfigError> {
        parse(&format!(
            "schema_version: 2\nrecord_types:\n  work:\n    dir: docs/work\n    required_fields: [Status]\n    known_fields: [Kind, Parent, Cause]\n    pointer_fields: [Parent]\n    narrative_fields: []\n    field_values: {{Kind: [feature, bug, chore]}}\n    statuses:\n      values: [Open, Done]\n{statuses}"
        ))
    }

    #[test]
    fn a_field_condition_triggers_with_or_without_a_status() {
        let c = kinds("      require: [{when_field: {Kind: [bug]}, fields: [Cause]}]\n      forbid: [{when: [Done], when_field: {Kind: [bug]}, fields: [Parent]}]\n").unwrap();
        let d = c.record_types["work"].status_decl().unwrap();
        let bug = |f: &str| (f == "Kind").then(|| "bug".to_string());
        let chore = |f: &str| (f == "Kind").then(|| "chore".to_string());
        let req = d.required_for("Open", &bug);
        assert_eq!(req[0].field, "Cause");
        assert_eq!(req[0].triggers, vec![("Kind", "bug".to_string())]);
        assert!(!req[0].on_status);
        assert!(d.required_for("Open", &chore).is_empty());
        assert!(d.forbidden_for("Open", &bug).is_empty());
        assert_eq!(d.forbidden_for("Done", &bug)[0].field, "Parent");
    }

    #[test]
    fn a_status_only_conflict_keeps_its_wording() {
        let e = kinds("      require: [{when: [Open], fields: [Cause]}]\n      forbid: [{when: [Open], fields: [Cause]}]\n").unwrap_err().to_string();
        assert!(
            e.ends_with("'Cause' is both required and forbidden at 'Open'"),
            "{e}"
        );
    }

    #[test]
    fn a_bad_field_condition_is_refused_at_load() {
        let e = |s: &str| kinds(s).unwrap_err().to_string();
        assert!(
            e("      require: [{when_field: {Nope: [x]}, fields: [Cause]}]\n").contains("neither")
        );
        assert!(
            e("      require: [{when_field: {Kind: [bgu]}, fields: [Cause]}]\n")
                .contains("field_values.Kind")
        );
        assert!(
            e("      require: [{when_field: {Status: [Open]}, fields: [Cause]}]\n")
                .contains("status field")
        );
        assert!(
            e("      require: [{fields: [Cause]}]\n").contains("neither `when` nor `when_field`")
        );
        assert!(e("      require: [{when_field: {Kind: [bug]}, fields: [Cause]}]\n      forbid: [{when: [Open], fields: [Cause]}]\n").contains("both required and forbidden"));
        assert!(kinds("      require: [{when_field: {Kind: [bug]}, fields: [Cause]}]\n      forbid: [{when_field: {Kind: [chore]}, fields: [Cause]}]\n").is_ok());
    }

    fn type_with(block: &str, known: &str) -> String {
        let indented: String = block.lines().map(|l| format!("    {l}\n")).collect();
        format!(
            "schema_version: 2\nrecord_types:\n  task:\n    dir: docs/task\n    required_fields: [Status]\n    known_fields: {known}\n    header_shape: yaml-frontmatter\n{indented}"
        )
    }

    #[test]
    fn the_smallest_checklist_loads_with_no_evidence_field_declared() {
        let c = parse(&type_with(SMALLEST_CHECKLIST, "[]")).expect("the shown block loads");
        let d = c.record_types["task"].status_decl().unwrap();
        assert!(d.delivery.unwrap().items.is_some());
    }

    #[test]
    fn a_delivery_without_items_still_needs_its_evidence_field() {
        let no_items = SMALLEST_CHECKLIST
            .split("    items:")
            .next()
            .unwrap()
            .to_string();
        let e = parse(&type_with(&no_items, "[]")).unwrap_err().to_string();
        assert!(e.contains("evidence field 'Realized-by'"), "{e}");
        assert!(parse(&type_with(&no_items, "[Realized-by]")).is_ok());
    }

    #[test]
    fn a_missing_checklist_key_is_named_with_where_it_belongs_and_the_smallest_block() {
        for (key, at) in [
            ("partial_status", "statuses.delivery.items.partial_status"),
            (
                "clause_separator",
                "statuses.delivery.items.clause_separator",
            ),
            ("not_done", "statuses.delivery.not_done"),
        ] {
            let broken: String = SMALLEST_CHECKLIST
                .lines()
                .filter(|l| !l.trim_start().starts_with(&format!("{key}:")))
                .map(|l| format!("{l}\n"))
                .collect();
            let e = parse(&type_with(&broken, "[]")).unwrap_err().to_string();
            assert!(e.contains(&format!("`{at}` is missing")), "{e}");
            assert!(e.contains(SMALLEST_CHECKLIST), "{e}");
        }
    }

    #[test]
    fn the_checklist_rules_apply_by_declaration_and_a_named_level_stands() {
        use crate::config::RuleLevel;
        use crate::rules::{
            effective_level, RULE_STATUS_DELIVERY_AGREEMENT, RULE_STATUS_DONE_ITEMS_SHAPE,
            RULE_STATUS_WAIVED_ITEM_STALE,
        };
        let declared = parse(&type_with(SMALLEST_CHECKLIST, "[]")).unwrap();
        assert_eq!(
            effective_level(&declared, RULE_STATUS_DELIVERY_AGREEMENT),
            RuleLevel::Error
        );
        assert_eq!(
            effective_level(&declared, RULE_STATUS_DONE_ITEMS_SHAPE),
            RuleLevel::Error
        );
        assert_eq!(
            effective_level(&declared, RULE_STATUS_WAIVED_ITEM_STALE),
            RuleLevel::Warn
        );
        let named = parse(&format!(
            "{}rules:\n  status.done-items-shape: info\n",
            type_with(SMALLEST_CHECKLIST, "[]")
        ))
        .unwrap();
        assert_eq!(
            effective_level(&named, RULE_STATUS_DONE_ITEMS_SHAPE),
            RuleLevel::Info
        );
        let none = parse(&cfg("      values: [Open]\n")).unwrap();
        assert_eq!(
            effective_level(&none, RULE_STATUS_DELIVERY_AGREEMENT),
            RuleLevel::Off
        );
    }

    #[test]
    fn the_requirement_rules_apply_where_an_entry_is_declared() {
        use crate::config::RuleLevel;
        use crate::rules::{
            effective_level, RULE_STATUS_FORBIDDEN_FIELD_PRESENT,
            RULE_STATUS_REQUIRED_FIELD_MISSING,
        };
        let c = parse(&cfg(
            "      values: [Open, Fixed]\n      require: [{when: [Fixed], fields: [Fixed-in]}]\n",
        ))
        .unwrap();
        assert_eq!(
            effective_level(&c, RULE_STATUS_REQUIRED_FIELD_MISSING),
            RuleLevel::Error
        );
        assert_eq!(
            effective_level(&c, RULE_STATUS_FORBIDDEN_FIELD_PRESENT),
            RuleLevel::Off
        );
        let off = parse(&format!(
            "{}rules:\n  status.required-field-missing: warn\n",
            cfg("      values: [Open, Fixed]\n      require: [{when: [Fixed], fields: [Fixed-in]}]\n")
        ))
        .unwrap();
        assert_eq!(
            effective_level(&off, RULE_STATUS_REQUIRED_FIELD_MISSING),
            RuleLevel::Warn
        );
    }
}
