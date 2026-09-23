//! The two seed rules (SPEC-0002 Phase A), chosen because both fire on this
//! project's own corpus today -- a validator whose first run is green has
//! told you nothing about itself. Plus the Phase 6 rules, each with its own
//! planted-violation test.

use crate::config::Config;
use crate::field_state::classify;
use crate::header::HeaderLayout;
use crate::record::Record;
#[cfg(test)]
use crate::report::Population;
use crate::report::{
    census, census_records, Finding, FindingSeverity, Outcome, PopulationUnit, RuleExecution,
    RuleStatus,
};
use crate::FieldState;
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;

/// Rule 1: header format consistency within a record type, per a
/// config-declared required-field list. A majority-rule inference would
/// silently ratify whatever drifted in, so the shape is declared, not voted.
/// Every rule this build ships, by id. One canonical name per rule: the
/// config is validated against this list, so a typo is a load-time error
/// naming the valid set rather than a check that silently never runs.
pub const RULE_HEADER_REQUIRED_FIELDS: &str = "header.required-fields";
pub const RULE_HEADER_LAYOUT_CONSISTENCY: &str = "header.layout-consistency";
pub const RULE_HEADER_FIELD_SET_CONSISTENCY: &str = "header.field-set-consistency";
pub const RULE_TYPE_NO_DECLARED_SPEC: &str = "type.no-declared-spec";
pub const RULE_TYPE_DIR_MATCHES_NOTHING: &str = "type.dir-matches-nothing";
pub const RULE_TYPE_RECORD_OUTSIDE_DECLARED_DIR: &str = "type.record-outside-declared-dir";
pub const RULE_IDENTITY_COLLISION: &str = "identity.collision";
pub const RULE_HEADER_DEPRECATED_SHAPE: &str = "header.deprecated-shape";
pub const RULE_CONFIG_POINTER_DECLARATION_MISSING: &str = "config.pointer-declaration-missing";
pub const RULE_CONFIG_POINTER_FIELD_NOT_KNOWN: &str = "config.pointer-field-not-known";
pub const RULE_CONFIG_POINTER_NARRATIVE_OVERLAP: &str = "config.pointer-narrative-overlap";
pub const RULE_CONFIG_HEADER_NONE_HAS_NO_REQUIRED_FIELDS: &str =
    "config.header-none-has-no-required-fields";
pub const RULE_POINTER_RESOLUTION: &str = "pointer.resolution";
pub const RULE_POINTER_TARGET_STATUS: &str = "pointer.target-status";
pub const RULE_HEADER_POINTER_FIELD_CLEAN: &str = "header.pointer-field-clean";
pub const RULE_NARRATIVE_FIELD_STALE: &str = "narrative-field.stale";
pub const RULE_FIELD_QUALITY: &str = "field.quality";
pub const RULE_FIELD_PENDING: &str = "field.pending";
pub const RULE_CLAIM_STATUS_AGREEMENT: &str = "claim.status-agreement";
pub const RULE_FILENAME_TITLE_CONSISTENCY: &str = "filename.title-consistency";
pub const RULE_REVISION_LOG_CHANGE_CLASS_REQUIRED: &str = "revision-log.change-class-required";
pub const RULE_EMBODIMENT_CONSISTENCY: &str = "embodiment.consistency";
pub const RULE_EMBODIMENT_LOCATOR_EXISTS: &str = "embodiment.locator-exists";
pub const RULE_EMBODIMENT_LOCATOR_PROMOTION_CANDIDATE: &str =
    "embodiment.locator-promotion-candidate";
pub const RULE_RELATION_SUPERSESSION_RECIPROCITY: &str = "relation.supersession-reciprocity";
pub const RULE_RELATION_TARGET_STATUS_UNDECLARED: &str = "relation.target-status-undeclared";
pub const RULE_CONFIG_SCOPE_MATCHES_NOTHING: &str = "config.scope-matches-nothing";
pub const RULE_FIELD_UNTRIMMED_VALUE: &str = "field.untrimmed-value";
pub const RULE_HEADER_FIELD_CASE_MISMATCH: &str = "header.field-case-mismatch";
pub const RULE_CONFIG_RELATION_FIELD_NOT_KNOWN: &str = "config.relation-field-not-known";
pub const RULE_CONFIG_KNOWN_FIELDS_DECLARATION_MISSING: &str =
    "config.known-fields-declaration-missing";

pub const ALL_RULES: &[&str] = &[
    RULE_HEADER_REQUIRED_FIELDS,
    RULE_HEADER_LAYOUT_CONSISTENCY,
    RULE_HEADER_FIELD_SET_CONSISTENCY,
    RULE_TYPE_NO_DECLARED_SPEC,
    RULE_TYPE_DIR_MATCHES_NOTHING,
    RULE_TYPE_RECORD_OUTSIDE_DECLARED_DIR,
    RULE_IDENTITY_COLLISION,
    RULE_HEADER_DEPRECATED_SHAPE,
    RULE_CONFIG_POINTER_DECLARATION_MISSING,
    RULE_CONFIG_POINTER_FIELD_NOT_KNOWN,
    RULE_CONFIG_POINTER_NARRATIVE_OVERLAP,
    RULE_POINTER_RESOLUTION,
    RULE_POINTER_TARGET_STATUS,
    RULE_HEADER_POINTER_FIELD_CLEAN,
    RULE_NARRATIVE_FIELD_STALE,
    RULE_FIELD_QUALITY,
    RULE_FIELD_PENDING,
    RULE_CLAIM_STATUS_AGREEMENT,
    RULE_FILENAME_TITLE_CONSISTENCY,
    RULE_REVISION_LOG_CHANGE_CLASS_REQUIRED,
    RULE_EMBODIMENT_CONSISTENCY,
    RULE_EMBODIMENT_LOCATOR_EXISTS,
    RULE_EMBODIMENT_LOCATOR_PROMOTION_CANDIDATE,
    RULE_RELATION_SUPERSESSION_RECIPROCITY,
    RULE_CONFIG_SCOPE_MATCHES_NOTHING,
    RULE_FIELD_UNTRIMMED_VALUE,
    RULE_HEADER_FIELD_CASE_MISMATCH,
    RULE_CONFIG_HEADER_NONE_HAS_NO_REQUIRED_FIELDS,
    RULE_CONFIG_RELATION_FIELD_NOT_KNOWN,
    RULE_CONFIG_KNOWN_FIELDS_DECLARATION_MISSING,
    RULE_RELATION_TARGET_STATUS_UNDECLARED,
];

/// Rules that derive a record's identity from its filename's type prefix
/// (`ADR-36` declined the bare `NNNN-slug` shape), so a corpus with no prefix
/// gives them nothing to examine. Single source of this fact: `init`'s
/// adopt-mode proposal (`BUG-61`) is the only consumer today, but the fact
/// belongs to the rules themselves, not to one command's module.
/// Rules whose findings name a file outside the corpus a `check <path>`
/// invocation scopes against -- `claim.status-agreement` reads a claim file
/// under `claim_paths`, which a record-type `dir` never covers. `BUG-67`'s
/// path-prefix scoping is correct for every rule reporting on a record; a
/// rule like this one needs the same unconditional exemption `check.rs`
/// already gives a finding about the config file itself, or `BUG-86`'s
/// "scoped invocation silently drops a real finding" recurs for any narrower
/// scope than the one that repository's own Makefile was changed to use.
pub const RULES_REPORTING_OUTSIDE_THE_CORPUS: &[&str] = &[RULE_CLAIM_STATUS_AGREEMENT];

pub const IDENTITY_DEPENDENT_RULES: &[&str] = &[
    RULE_POINTER_RESOLUTION,
    RULE_POINTER_TARGET_STATUS,
    RULE_FILENAME_TITLE_CONSISTENCY,
    RULE_RELATION_SUPERSESSION_RECIPROCITY,
    RULE_NARRATIVE_FIELD_STALE,
];

/// One `(record, field)` candidate per field a record's type declares in
/// `fields_by_type` (`BUG-105`): the same construction four rules duplicated,
/// against `HashMap`s of either `Vec<String>` or `HashSet<String>`.
fn field_slots<'a, T, C>(
    records: &'a [Record],
    fields_by_type: &'a HashMap<String, C>,
) -> Vec<(&'a Record, &'a T)>
where
    &'a C: IntoIterator<Item = &'a T>,
{
    records
        .iter()
        .filter_map(|record| {
            fields_by_type
                .get(&record.record_type)
                .map(|fields| (record, fields))
        })
        .flat_map(|(record, fields)| fields.into_iter().map(move |field| (record, field)))
        .collect()
}

/// One `(record, field)` candidate per pointer *or* narrative field a
/// record's type declares (`BUG-105`): `pointer_target_status` duplicated
/// `pointer_resolution`'s construction statement-for-statement.
fn pointer_and_narrative_slots<'a>(
    records: &'a [Record],
    pointer_fields_by_type: &'a HashMap<String, Vec<String>>,
    narrative_fields_by_type: &'a HashMap<String, Vec<String>>,
) -> Vec<(&'a Record, &'a String)> {
    records
        .iter()
        .flat_map(|record| {
            pointer_fields_by_type
                .get(&record.record_type)
                .into_iter()
                .flatten()
                .chain(
                    narrative_fields_by_type
                        .get(&record.record_type)
                        .into_iter()
                        .flatten(),
                )
                .map(move |field| (record, field))
        })
        .collect()
}

/// One type's projection, by type name, wherever `f` returns `Some` (`ADR-59`):
/// the shape every `*_by_type` projector below shares, so a seventh one is a
/// one-line call instead of another hand-rolled `filter_map`.
fn project_by_type<T>(
    config: &Config,
    f: impl Fn(&crate::config::RecordTypeConfig) -> Option<T>,
) -> HashMap<String, T> {
    config
        .record_types
        .iter()
        .filter_map(|(name, cfg)| f(cfg).map(|v| (name.clone(), v)))
        .collect()
}

/// A type's `required_fields`, by type name (`ADR-59`): projected inside the
/// rule that needs it rather than built once and handed in as a bare
/// collection indistinguishable at the call site from a different projection
/// of the same shape.
fn required_fields_by_type(config: &Config) -> HashMap<String, Vec<String>> {
    project_by_type(config, |cfg| Some(cfg.required_fields.clone()))
}

/// A type's `header_layout`, by type name, when declared (`ADR-59`).
fn header_layout_by_type(config: &Config) -> HashMap<String, HeaderLayout> {
    project_by_type(config, |cfg| cfg.header_layout)
}

/// A type's declared field set, by type name, only for a type that declares
/// `known_fields` (`ADR-59`): `header.field-set-consistency` skips a type
/// declaring only `required_fields` by design, and a map with an entry for
/// every type would silently stop skipping it.
fn known_fields_by_type(config: &Config) -> HashMap<String, HashSet<crate::values::FieldName>> {
    project_by_type(config, |cfg| {
        cfg.known_fields.as_ref().map(|_| cfg.declared_fields())
    })
}

/// Every field a type declares (`required_fields` ∪ `known_fields`), by type
/// name (`ADR-59`, `BUG-106`).
fn declared_fields_by_type(config: &Config) -> HashMap<String, HashSet<crate::values::FieldName>> {
    project_by_type(config, |cfg| Some(cfg.declared_fields()))
}

/// A type's `pointer_fields`, by type name, when declared (`ADR-59`).
fn pointer_fields_by_type(config: &Config) -> HashMap<String, Vec<String>> {
    project_by_type(config, |cfg| cfg.pointer_fields.clone())
}

/// A type's `narrative_fields`, by type name, when declared (`ADR-59`).
fn narrative_fields_by_type(config: &Config) -> HashMap<String, Vec<String>> {
    project_by_type(config, |cfg| cfg.narrative_fields.clone())
}

pub fn header_required_fields(
    records: &[Record],
    config: &Config,
) -> (RuleExecution, Vec<Finding>) {
    let required_by_type = required_fields_by_type(config);
    let required_by_type = &required_by_type;
    const RULE_ID: &str = RULE_HEADER_REQUIRED_FIELDS;
    let mut findings = Vec::new();

    // Record-scoped findings -- an unparsed header, a duplicated key -- are
    // about the record, not about any one slot, so they are emitted once per
    // record. Decision 1's rule against reporting on a candidate you did not
    // examine governs *per-slot* findings; a record-scoped finding explaining
    // why its slots are unreadable is the opposite of a contradiction.
    for record in records {
        let Some(required) = required_by_type.get(&record.record_type) else {
            continue;
        };
        // A `none`-shaped type has nowhere for a header to be (`ADR-50`): a
        // missing region here is the declared truth, not a defect to report.
        if config
            .record_types
            .get(&record.record_type)
            .is_some_and(|t| t.header_shape == crate::header::HeaderShape::None)
        {
            continue;
        }

        if record.header.region.is_none() {
            let detail = match &record.header.parse_error {
                Some(e) => format!(" -- YAML parse error: {e}"),
                None => String::new(),
            };
            findings.push(Finding {
                rule: RULE_ID.to_string(),
                severity: FindingSeverity::Error,
                file: record.path.clone(),
                line: None,
 waived: None,
                message: format!(
                    "no header-shaped region found -- required fields {required:?} cannot be checked{detail}"
                ),
            });
            continue;
        }

        for dup in record.header.duplicate_keys() {
            findings.push(Finding {
                rule: RULE_ID.to_string(),
                severity: FindingSeverity::Error,
                file: record.path.clone(),
                line: None,
 waived: None,
                message: format!("header key '{dup}' appears more than once -- ambiguous which value is operative"),
            });
        }
    }

    // The population is the declared slot: one `(record, required field)` pair
    // per field the record's type declares.
    let slots = field_slots(records, required_by_type);

    let (population, examined_records) = census_records(
        PopulationUnit::Field,
        slots,
        |(record, _)| record.path.clone(),
        |(record, field)| {
            // An unparsed header leaves every slot of that record unreadable: the
            // rule was handed them and could not judge them. The record-scoped
            // finding above says why.
            if record.header.region.is_none() {
                return Outcome::Unreadable;
            }
            if record.header.get(field.as_str()).is_none() {
                findings.push(Finding {
                    rule: RULE_ID.to_string(),
                    severity: FindingSeverity::Error,
                    file: record.path.clone(),
                    line: None,
                    waived: None,
                    message: format!(
                        "missing required header field '{field}' for record type '{}'",
                        record.record_type
                    ),
                });
            }
            Outcome::Examined
        },
    );

    (
        RuleExecution {
            rule: RULE_ID.to_string(),
            population: Some(population),
            status: RuleStatus::Ran,
            examined_records,
        },
        findings,
    )
}

/// Rule (MILE-0075): a record's header line layout matches its type's
/// declared `header_layout`, when one is declared. `HeaderShape::Blockquote`
/// deliberately tolerates one-per-line and pipe-delimited interchangeably for
/// parsing (RFC-0010), which is correct for reading a corpus that hasn't
/// picked one -- but it means drift within a type that HAS settled on one
/// (found live: SPEC-2 through SPEC-6 all pipe-delimited, SPEC-1 alone
/// one-per-line) previously went unflagged. Declared, not voted, same
/// principle as Rule 1: a type with no declared `header_layout` is skipped
/// entirely, examined stays 0 for it -- this is additive, never a forced
/// migration.
pub fn header_layout_consistency(
    records: &[Record],
    config: &Config,
) -> (RuleExecution, Vec<Finding>) {
    let declared_by_type = header_layout_by_type(config);
    let declared_by_type = &declared_by_type;
    const RULE_ID: &str = RULE_HEADER_LAYOUT_CONSISTENCY;
    let mut findings = Vec::new();

    // Eligible: records of a type that declares a layout. A type declaring
    // none puts its records outside this rule's population -- the rule does not
    // apply. A record whose header yields no detectable layout IS in the
    // population and simply was not judged, which `eligible > examined` says
    // and a bare count could not.
    let candidates: Vec<&Record> = records
        .iter()
        .filter(|r| declared_by_type.contains_key(&r.record_type))
        .collect();

    let (population, examined_records) = census_records(
        PopulationUnit::Record,
        candidates,
        |record| record.path.clone(),
        |record| {
            let Some(&declared) = declared_by_type.get(&record.record_type) else {
                return Outcome::Absent;
            };
            let Some(actual) = record.header.layout() else {
                return Outcome::Absent;
            };

            if actual != declared {
                let (declared_label, actual_label) = (layout_label(declared), layout_label(actual));
                findings.push(Finding {
                rule: RULE_ID.to_string(),
                severity: FindingSeverity::Warning,
                file: record.path.clone(),
                line: None,
                waived: None,
                message: format!(
                    "header layout '{actual_label}' disagrees with '{declared_label}', declared for record type '{}'",
                    record.record_type
                ),
            });
            }
            Outcome::Examined
        },
    );

    (
        RuleExecution {
            rule: RULE_ID.to_string(),
            population: Some(population),
            status: RuleStatus::Ran,
            examined_records,
        },
        findings,
    )
}

fn layout_label(layout: HeaderLayout) -> &'static str {
    match layout {
        HeaderLayout::OnePerLine => "one-per-line",
        HeaderLayout::PipeDelimited => "pipe-delimited",
    }
}

/// Rule (MILE-0081): every header field on a record belongs to its type's
/// declared `required_fields` or `known_fields`. `header.required-fields`
/// only catches a *missing* required field; nothing catches the opposite --
/// a field present on some records of a type but not others (found live:
/// SPEC-1 carries `Embodiment`/`Author`/`Derives-from` that SPEC-2 through
/// SPEC-6 don't). Declared, not voted, same principle as `header_layout`: a
/// type with no declared `known_fields` is skipped entirely, examined stays
/// 0 for it -- required so an existing corpus's legitimate optional fields
/// (`Stable-Id`, `Realized-by`, `Derives-from` on `adr`, none of which are
/// *required*) don't all become false positives the moment this rule ships.
/// Exact (`ADR-57`): matching case-insensitively would accept a name nobody
/// declared -- `Maße` and `Masse` are different words.
pub fn field_is_declared(
    key: &crate::values::FieldName,
    allowed: &HashSet<crate::values::FieldName>,
) -> bool {
    allowed.contains(key)
}

/// A hint on a finding, never a verdict, so an approximate answer is safe here.
///
/// `min` rather than `find`: several declared names can differ only in case, and
/// a message that changes between runs on one corpus is `BUG-79`'s shape.
pub fn near_miss<'a>(
    key: &crate::values::FieldName,
    allowed: &'a HashSet<crate::values::FieldName>,
) -> Option<&'a crate::values::FieldName> {
    let key = key.as_str().to_lowercase();
    allowed
        .iter()
        .filter(|d| d.as_str().to_lowercase() == key)
        .min()
}

pub fn header_field_set_consistency(
    records: &[Record],
    config: &Config,
) -> (RuleExecution, Vec<Finding>) {
    let allowed_by_type = known_fields_by_type(config);
    let allowed_by_type = &allowed_by_type;
    const RULE_ID: &str = RULE_HEADER_FIELD_SET_CONSISTENCY;
    let mut findings = Vec::new();

    // Eligible: records of a type declaring `known_fields`. An unparsed header
    // stays eligible and unexamined (BUG-78) rather than vanishing.
    let candidates: Vec<&Record> = records
        .iter()
        .filter(|r| allowed_by_type.contains_key(&r.record_type))
        .collect();

    let (population, examined_records) = census_records(
        PopulationUnit::Record,
        candidates,
        |record| record.path.clone(),
        |record| {
            let Some(allowed) = allowed_by_type.get(&record.record_type) else {
                return Outcome::Absent;
            };
            // A record whose header region never parsed has an empty field list,
            // which is indistinguishable here from a record whose fields are all
            // allowed. Counting it as examined inflated the one signal ADR-55 makes
            // load-bearing; `header.required-fields` reports the unparsed header
            // (BUG-78).
            if record.header.region.is_none() || record.header.parse_error.is_some() {
                return Outcome::Unreadable;
            }

            for field in &record.header.fields {
                if !field_is_declared(&field.key, allowed) {
                    findings.push(Finding {
                    rule: RULE_ID.to_string(),
                    severity: FindingSeverity::Warning,
                    file: record.path.clone(),
                    line: Some(field.line),
                    waived: None,
                    message: match near_miss(&field.key, allowed) {
                        Some(declared) => format!(
                            "field '{}' is not declared for record type '{}' -- the type declares '{declared}', which differs only in case",
                            field.key, record.record_type
                        ),
                        None => format!(
                            "field '{}' is not declared (required_fields or known_fields) for record type '{}'",
                            field.key, record.record_type
                        ),
                    },
                });
                }
            }
            Outcome::Examined
        },
    );

    (
        RuleExecution {
            rule: RULE_ID.to_string(),
            population: Some(population),
            status: RuleStatus::Ran,
            examined_records,
        },
        findings,
    )
}

/// A `dir` selects that directory and not its subtree (RFC-35). A
/// record-shaped file one level below a declared `dir` is therefore owned by no
/// type and drops out of the corpus entirely -- `check` reports a smaller
/// `files_examined` and says nothing, so a configuration written before RFC-35
/// loses coverage on upgrade with no diagnostic (BUG-62).
///
/// `candidates` is every tracked path the caller considered; `claimed` is the
/// subset some type took.
pub fn type_record_outside_declared_dir(
    config: &Config,
    candidates: &[std::path::PathBuf],
) -> (RuleExecution, Vec<Finding>) {
    const RULE_ID: &str = RULE_TYPE_RECORD_OUTSIDE_DECLARED_DIR;
    let mut findings = Vec::new();

    let dirs: Vec<std::path::PathBuf> = config
        .record_types
        .values()
        .map(|t| std::path::PathBuf::from(&t.dir))
        .collect();

    // Eligible: every tracked, record-shaped path below a declared dir. A
    // markdown file elsewhere in the repo is a document, not an ungoverned
    // record, and saying otherwise would flag every README.
    let eligible_paths: Vec<&std::path::PathBuf> = candidates
        .iter()
        .filter(|path| {
            let Some(file_name) = path.file_name().and_then(|n| n.to_str()) else {
                return false;
            };
            if !crate::record::is_governed_record_filename(file_name) {
                return false;
            }
            dirs.iter().any(|d| path.starts_with(d))
        })
        .collect();

    let mut unowned: Vec<&std::path::PathBuf> = Vec::new();
    let population = census(PopulationUnit::Path, eligible_paths, |path| {
        // Ownership is decided by the path, which is what RFC-35 defines. It
        // was decided by the loaded record set, which also excludes a file the
        // loader could not read -- so a staged deletion directly inside a
        // declared dir was reported as sitting outside it (BUG-71).
        if !dirs.iter().any(|d| path.parent() == Some(d.as_path())) {
            unowned.push(path);
        }
        Outcome::Examined
    });

    unowned.sort();
    for path in unowned {
        findings.push(Finding {
            rule: RULE_ID.to_string(),
            severity: FindingSeverity::Warning,
            file: path.clone(),
            line: None,
            message: "sits below a declared record type's dir but not directly in it, so no type \
                      owns it and no rule examines it"
                .to_string(),
            waived: None,
        });
    }

    (
        RuleExecution {
            rule: RULE_ID.to_string(),
            population: Some(population),
            status: RuleStatus::Ran,
            examined_records: Vec::new(),
        },
        findings,
    )
}

/// Rule (MILE-0077 follow-up, ADR-0043): unlike every other rule, this one
/// examines `Config` directly rather than `&[Record]` -- it's an inventory
/// of the *schema* itself, not a per-record check. "Does this type need a
/// spec" stays editorial (ADR-0041 rejected a mechanical formula for that);
/// this only makes visible which configured types currently have no `spec`
/// pointer declared at all, the same "declared, not voted" shape as
/// `header_layout`/`known_fields`. A type with none declared is a real,
/// permanently valid state under ADR-0041 -- this is a signal to review,
/// never a mandate to write one.
/// A declared record type whose `dir` matches no record.
///
/// `RFC-35` defines `dir` as *that directory* rather than that subtree, which
/// is unambiguous but unforgiving: a `dir` one level off matches nothing at
/// all. Alone that degrades to `not-run`, which is visible; mixed with any
/// working type it is silent -- `check` reports `ok` and exit 0 over a corpus
/// it never examined. `claim_paths` has a load-time guard for the same hazard
/// (`BUG-56`); `dir` has none.
///
/// `matched` is supplied by the caller, which is where discovery happens.
pub fn type_dir_matches_nothing(
    config: &Config,
    config_path: &std::path::Path,
    matched: &HashMap<String, usize>,
    dir_exists: &dyn Fn(&str) -> bool,
) -> (RuleExecution, Vec<Finding>) {
    const RULE_ID: &str = RULE_TYPE_DIR_MATCHES_NOTHING;
    let mut findings = Vec::new();

    let mut names: Vec<&String> = config.record_types.keys().collect();
    names.sort();

    let population = census(PopulationUnit::RecordType, names, |name| {
        if matched.get(*name).copied().unwrap_or(0) == 0 {
            let dir = &config.record_types[*name].dir;
            // A directory that exists and is empty is a type declared and not
            // yet used, which is a legitimate state -- some types are expected
            // to hold nothing most of the time. An *absent* directory is the
            // misdeclaration this rule exists for. Git does not track empty
            // directories, so declaring a type means committing a placeholder
            // alongside it.
            if !dir_exists(dir) {
                findings.push(Finding {
                    rule: RULE_ID.to_string(),
                    severity: FindingSeverity::Warning,
                    file: config_path.to_path_buf(),
                    line: None,
                    waived: None,
                    message: format!(
                        "record type '{name}' declares dir '{dir}', which does not exist -- \
                         every rule for this type examines nothing"
                    ),
                });
            }
        }
        Outcome::Examined
    });

    (
        RuleExecution {
            rule: RULE_ID.to_string(),
            population: Some(population),
            status: RuleStatus::Ran,
            examined_records: Vec::new(),
        },
        findings,
    )
}

pub fn type_no_declared_spec(
    config: &Config,
    config_path: &std::path::Path,
) -> (RuleExecution, Vec<Finding>) {
    const RULE_ID: &str = RULE_TYPE_NO_DECLARED_SPEC;
    let mut findings = Vec::new();

    let mut type_names: Vec<&String> = config.record_types.keys().collect();
    type_names.sort();

    let population = census(PopulationUnit::RecordType, type_names, |type_name| {
        let type_config = &config.record_types[*type_name];
        if type_config.spec.is_none() {
            findings.push(Finding {
                rule: RULE_ID.to_string(),
                severity: FindingSeverity::Warning,
                file: config_path.to_path_buf(),
                line: None,
                waived: None,
                message: format!(
                    "record type '{type_name}' has no declared spec -- add `spec = \"SPEC-N\"` once one exists, or leave undeclared if ADR-41's editorial judgment says one isn't warranted"
                ),
            });
        }
        Outcome::Examined
    });

    (
        RuleExecution {
            rule: RULE_ID.to_string(),
            population: Some(population),
            status: RuleStatus::Ran,
            examined_records: Vec::new(),
        },
        findings,
    )
}

/// ADR-0033: `blockquote`/`bold-list` are deprecated, `yaml-frontmatter` is
/// the one shape this project actively grows. Same config-level shape as
/// `type_no_declared_spec` -- a schema inventory check, not a per-record
/// one. Parsing support for the deprecated shapes stays (an external
/// adopter's un-migrated corpus, or a first-time evaluator's existing docs,
/// per ADR-0033's own stated reason for deprecating rather than removing);
/// this rule only surfaces which configured types still declare one.
pub fn header_deprecated_shape(
    config: &Config,
    config_path: &std::path::Path,
) -> (RuleExecution, Vec<Finding>) {
    const RULE_ID: &str = RULE_HEADER_DEPRECATED_SHAPE;
    let mut findings = Vec::new();

    let mut type_names: Vec<&String> = config.record_types.keys().collect();
    type_names.sort();

    let population = census(PopulationUnit::RecordType, type_names, |type_name| {
        let type_config = &config.record_types[*type_name];
        // Explicit enumeration, not `!= YamlFrontmatter` (`ADR-50`): a
        // negative test reports every future shape as deprecated on sight,
        // and `none` (no header at all) is a different axis, not a
        // deprecated one. A `match`, not `matches!`, so a fifth `HeaderShape`
        // variant forces a decision here instead of silently inheriting a
        // default.
        let deprecated = match type_config.header_shape {
            crate::header::HeaderShape::Blockquote | crate::header::HeaderShape::BoldList => true,
            crate::header::HeaderShape::YamlFrontmatter | crate::header::HeaderShape::None => false,
        };
        if deprecated {
            findings.push(Finding {
                rule: RULE_ID.to_string(),
                severity: FindingSeverity::Warning,
                file: config_path.to_path_buf(),
                line: None,
                waived: None,
                message: format!(
                    "record type '{type_name}' declares a deprecated header shape -- migrate to `header_shape = \"yaml-frontmatter\"` (ADR-33)"
                ),
            });
        }
        Outcome::Examined
    });

    (
        RuleExecution {
            rule: RULE_ID.to_string(),
            population: Some(population),
            status: RuleStatus::Ran,
            examined_records: Vec::new(),
        },
        findings,
    )
}

/// Which of the two kinds a relationship field belongs to (MILE-0090/
/// ADR-0044) -- serialized on every `urzua graph` edge. `Supersedes /
/// Superseded-by` edges also report `Pointer`: its values are always clean
/// comma-separated references or `—`, never prose, so it's pointer-*shaped*
/// even though a different rule (`supersession_reciprocity`, which also
/// checks reciprocity) resolves it -- this enum describes value shape, not
/// which rule produced the edge.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RelationKind {
    Pointer,
    Narrative,
}

impl RelationKind {
    fn as_str(self) -> &'static str {
        match self {
            RelationKind::Pointer => "pointer",
            RelationKind::Narrative => "narrative",
        }
    }
}

impl serde::Serialize for RelationKind {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(self.as_str())
    }
}

/// What a category of relationship field means for validation -- data, not
/// per-kind functions (SPEC-0002: "rules are data where possible, not
/// code"). A real third kind, if one is ever decided, is one new `const`
/// here, never a new hardcoded function.
pub(crate) struct FieldKindSpec {
    pub(crate) kind: RelationKind,
    /// Does `header_pointer_field_clean` apply to fields of this kind?
    enforce_clean_format: bool,
    /// Does `narrative_field_stale` apply to fields of this kind?
    check_target_staleness: bool,
}

pub(crate) const POINTER: FieldKindSpec = FieldKindSpec {
    kind: RelationKind::Pointer,
    enforce_clean_format: true,
    check_target_staleness: false,
};
pub(crate) const NARRATIVE: FieldKindSpec = FieldKindSpec {
    kind: RelationKind::Narrative,
    enforce_clean_format: false,
    check_target_staleness: true,
};

/// Every field, across every configured type, whose `FieldKindSpec`
/// satisfies `capability` -- e.g. `fields_with_capability(config, |k|
/// k.enforce_clean_format)` for `header_pointer_field_clean`. Existence
/// resolution (`pointer_resolution`, `graph`'s dangling check) isn't a
/// capability -- it applies to every declared field regardless of kind, so
/// those two read `pointer_fields`/`narrative_fields` directly instead of
/// going through this helper.
fn fields_with_capability(
    config: &Config,
    capability: impl Fn(&FieldKindSpec) -> bool,
) -> HashMap<String, Vec<String>> {
    let mut result = HashMap::new();
    for (type_name, type_config) in &config.record_types {
        let mut fields = Vec::new();
        if capability(&POINTER) {
            if let Some(pointer_fields) = &type_config.pointer_fields {
                fields.extend(pointer_fields.iter().cloned());
            }
        }
        if capability(&NARRATIVE) {
            if let Some(narrative_fields) = &type_config.narrative_fields {
                fields.extend(narrative_fields.iter().cloned());
            }
        }
        if !fields.is_empty() {
            result.insert(type_name.clone(), fields);
        }
    }
    result
}

/// Rule (MILE-0090/ADR-0044): a type declaring either `pointer_fields` or
/// `narrative_fields` must declare both explicitly, even as an empty array --
/// omitting one is not the same as deliberately declaring zero fields of that
/// kind. Same config-level, schema-inventory shape as `type_no_declared_spec`.
pub fn config_pointer_declaration_missing(
    config: &Config,
    config_path: &std::path::Path,
) -> (RuleExecution, Vec<Finding>) {
    const RULE_ID: &str = RULE_CONFIG_POINTER_DECLARATION_MISSING;
    let mut findings = Vec::new();

    let mut type_names: Vec<&String> = config.record_types.keys().collect();
    type_names.sort();

    let population = census(PopulationUnit::RecordType, type_names, |type_name| {
        let type_config = &config.record_types[*type_name];
        let declares_either =
            type_config.pointer_fields.is_some() || type_config.narrative_fields.is_some();
        if declares_either
            && (type_config.pointer_fields.is_none() || type_config.narrative_fields.is_none())
        {
            findings.push(Finding {
                rule: RULE_ID.to_string(),
                severity: FindingSeverity::Error,
                file: config_path.to_path_buf(),
                line: None,
                waived: None,
                message: format!(
                    "record type '{type_name}' declares only one of pointer_fields/narrative_fields -- declare both explicitly, even as `[]`, since omitting one is not the same as declaring zero fields of that kind"
                ),
            });
        }
        Outcome::Examined
    });

    (
        RuleExecution {
            rule: RULE_ID.to_string(),
            population: Some(population),
            status: RuleStatus::Ran,
            examined_records: Vec::new(),
        },
        findings,
    )
}

/// Rule (`RFC-43`/`ADR-62`): a type declaring no `known_fields` at all gets no
/// field-set governance from `header.field-set-consistency` (`ADR-53`'s
/// "declared, not voted" default) -- opt-in, so a repository can require the
/// choice be made explicit rather than leaving it ambiguous by omission, the
/// same shape `config.pointer-declaration-missing` already is for a
/// different pair of fields.
pub fn config_known_fields_declaration_missing(
    config: &Config,
    config_path: &std::path::Path,
) -> (RuleExecution, Vec<Finding>) {
    const RULE_ID: &str = RULE_CONFIG_KNOWN_FIELDS_DECLARATION_MISSING;
    let mut findings = Vec::new();

    let mut type_names: Vec<&String> = config.record_types.keys().collect();
    type_names.sort();

    let population = census(PopulationUnit::RecordType, type_names, |type_name| {
        let type_config = &config.record_types[*type_name];
        // A `none`-shaped type has nowhere for a header field to be (`ADR-50`):
        // `known_fields` governs which fields are permitted beyond
        // `required_fields`, which is meaningless when no field can exist at
        // all, the same reasoning `config.header-none-has-no-required-fields`
        // already applies to `required_fields` itself.
        if type_config.header_shape != crate::header::HeaderShape::None
            && type_config.known_fields.is_none()
        {
            findings.push(Finding {
                rule: RULE_ID.to_string(),
                severity: FindingSeverity::Error,
                file: config_path.to_path_buf(),
                line: None,
                waived: None,
                message: format!(
                    "record type '{type_name}' does not declare known_fields -- declare it explicitly, even as `[]`, to make this type's field-set governance a conscious choice rather than an unchecked default"
                ),
            });
        }
        Outcome::Examined
    });

    (
        RuleExecution {
            rule: RULE_ID.to_string(),
            population: Some(population),
            status: RuleStatus::Ran,
            examined_records: Vec::new(),
        },
        findings,
    )
}

/// A declared alias not present in a type's own `required_fields`/
/// `known_fields`, shared by every "config declares a field name nothing
/// else knows about" rule (`config.pointer-field-not-known`,
/// `config.relation-field-not-known`).
fn undeclared_alias_finding(
    rule_id: &str,
    config_path: &std::path::Path,
    message: String,
) -> Finding {
    Finding {
        rule: rule_id.to_string(),
        severity: FindingSeverity::Error,
        file: config_path.to_path_buf(),
        line: None,
        waived: None,
        message,
    }
}

/// Rule (MILE-0090/ADR-0044): every field named in a type's `pointer_fields`
/// or `narrative_fields` must also appear in that type's own
/// `required_fields`/`known_fields` -- a relationship field the schema
/// inventory itself doesn't know about would silently never trip
/// `header.field-set-consistency`. Compared exactly (`ADR-57`).
pub fn config_pointer_field_not_known(
    config: &Config,
    config_path: &std::path::Path,
) -> (RuleExecution, Vec<Finding>) {
    const RULE_ID: &str = RULE_CONFIG_POINTER_FIELD_NOT_KNOWN;
    let mut findings = Vec::new();

    let mut type_names: Vec<&String> = config.record_types.keys().collect();
    type_names.sort();

    let population = census(PopulationUnit::RecordType, type_names, |type_name| {
        let type_config = &config.record_types[*type_name];
        let declared = type_config.declared_fields();

        let mut relation_fields: Vec<&String> = Vec::new();
        if let Some(pointer_fields) = &type_config.pointer_fields {
            relation_fields.extend(pointer_fields);
        }
        if let Some(narrative_fields) = &type_config.narrative_fields {
            relation_fields.extend(narrative_fields);
        }

        for field in relation_fields {
            if !declared.contains(field.as_str()) {
                findings.push(undeclared_alias_finding(
                    RULE_ID,
                    config_path,
                    format!(
                        "record type '{type_name}' declares '{field}' as a pointer/narrative field, but it isn't in required_fields or known_fields"
                    ),
                ));
            }
        }
        Outcome::Examined
    });

    (
        RuleExecution {
            rule: RULE_ID.to_string(),
            population: Some(population),
            status: RuleStatus::Ran,
            examined_records: Vec::new(),
        },
        findings,
    )
}

/// Rule (`RFC-42`/`ADR-61`): a declared `relation_fields` override must also
/// appear in that type's own `required_fields`/`known_fields` -- otherwise
/// `ADR-60`'s declaration gate silently never examines it, the same trap
/// `BUG-109` closed for the unqualified `Status` case.
pub fn config_relation_field_not_known(
    config: &Config,
    config_path: &std::path::Path,
) -> (RuleExecution, Vec<Finding>) {
    const RULE_ID: &str = RULE_CONFIG_RELATION_FIELD_NOT_KNOWN;
    let mut findings = Vec::new();

    let mut type_names: Vec<&String> = config.record_types.keys().collect();
    type_names.sort();

    let population = census(PopulationUnit::RecordType, type_names, |type_name| {
        let type_config = &config.record_types[*type_name];
        let Some(relation_fields) = &type_config.relation_fields else {
            return Outcome::Examined;
        };
        let declared = type_config.declared_fields();
        for role in crate::config::RelationRole::ALL {
            let Some(field) = relation_fields.get(role) else {
                continue;
            };
            if !declared.contains(field) {
                findings.push(undeclared_alias_finding(
                    RULE_ID,
                    config_path,
                    format!(
                        "record type '{type_name}' declares '{field}' for relation_fields.{}, but it isn't in required_fields or known_fields",
                        role.config_key()
                    ),
                ));
            }
        }
        Outcome::Examined
    });

    (
        RuleExecution {
            rule: RULE_ID.to_string(),
            population: Some(population),
            status: RuleStatus::Ran,
            examined_records: Vec::new(),
        },
        findings,
    )
}

/// Rule (MILE-0090/ADR-0044): a field must be exactly one kind -- named in
/// both `pointer_fields` and `narrative_fields` for the same type is a
/// contradiction (clean-format enforcement and staleness-checking would both
/// apply, but the two kinds' rules disagree about whether prose is allowed).
pub fn config_pointer_narrative_overlap(
    config: &Config,
    config_path: &std::path::Path,
) -> (RuleExecution, Vec<Finding>) {
    const RULE_ID: &str = RULE_CONFIG_POINTER_NARRATIVE_OVERLAP;
    let mut findings = Vec::new();

    let mut type_names: Vec<&String> = config.record_types.keys().collect();
    type_names.sort();

    let population = census(PopulationUnit::RecordType, type_names, |type_name| {
        let type_config = &config.record_types[*type_name];
        let (Some(pointer_fields), Some(narrative_fields)) =
            (&type_config.pointer_fields, &type_config.narrative_fields)
        else {
            return Outcome::Examined;
        };
        let declared_pointers: HashSet<&String> = pointer_fields.iter().collect();
        for field in narrative_fields {
            if declared_pointers.contains(field) {
                findings.push(Finding {
                    rule: RULE_ID.to_string(),
                    severity: FindingSeverity::Error,
                    file: config_path.to_path_buf(),
                    line: None,
                    waived: None,
                    message: format!(
                        "record type '{type_name}' declares '{field}' in both pointer_fields and narrative_fields -- a field must be exactly one kind"
                    ),
                });
            }
        }
        Outcome::Examined
    });

    (
        RuleExecution {
            rule: RULE_ID.to_string(),
            population: Some(population),
            status: RuleStatus::Ran,
            examined_records: Vec::new(),
        },
        findings,
    )
}

/// Rule (`ADR-50`): a type declaring `header_shape: none` has nowhere for a
/// field to be, so a non-empty `required_fields` is a self-contradiction --
/// the same shape as `config.pointer-narrative-overlap`.
pub fn config_header_none_has_no_required_fields(
    config: &Config,
    config_path: &std::path::Path,
) -> (RuleExecution, Vec<Finding>) {
    const RULE_ID: &str = RULE_CONFIG_HEADER_NONE_HAS_NO_REQUIRED_FIELDS;
    let mut findings = Vec::new();

    let mut type_names: Vec<&String> = config.record_types.keys().collect();
    type_names.sort();

    let population = census(PopulationUnit::RecordType, type_names, |type_name| {
        let type_config = &config.record_types[*type_name];
        if type_config.header_shape == crate::header::HeaderShape::None
            && !type_config.required_fields.is_empty()
        {
            findings.push(Finding {
                rule: RULE_ID.to_string(),
                severity: FindingSeverity::Error,
                file: config_path.to_path_buf(),
                line: None,
                waived: None,
                message: format!(
                    "record type '{type_name}' declares header_shape: none but required_fields {:?} -- a type with no header has nowhere for a required field to be",
                    type_config.required_fields
                ),
            });
        }
        Outcome::Examined
    });

    (
        RuleExecution {
            rule: RULE_ID.to_string(),
            population: Some(population),
            status: RuleStatus::Ran,
            examined_records: Vec::new(),
        },
        findings,
    )
}

/// Every record indexed by its normalized identifier (BUG-0002: keyed by
/// numeric value, not the raw string, so a reference's padding never has to
/// match the filename's exactly). Shared by `pointer_resolution`,
/// `narrative_field_stale`, and `graph()` -- previously three near-identical
/// copies of this same loop, one of which (`graph()`'s) never normalized at
/// all (BUG-0011).
pub fn build_normalized_index(records: &[Record]) -> HashMap<crate::values::RecordId, &Record> {
    build_index_reporting_collisions(records).0
}

/// Two records resolving to one identifier is a corpus error the index cannot
/// represent: it keeps one and the other stops existing for every rule that
/// resolves a reference. Reported here rather than papered over, because the
/// tool knows both records are there (BUG-79).
pub fn identity_collision(
    records: &[Record],
    collisions: Vec<IdentifierCollision<'_>>,
) -> (RuleExecution, Vec<Finding>) {
    const RULE_ID: &str = RULE_IDENTITY_COLLISION;

    // A record whose filename yields no identifier is eligible and unexamined:
    // it was handed to the rule, and the rule has no identity to collide.
    let (population, examined_records) = census_records(
        PopulationUnit::Record,
        records.iter().collect(),
        |record| record.path.clone(),
        |record| {
            if record_id(record).is_some() {
                Outcome::Examined
            } else {
                Outcome::OutOfScope
            }
        },
    );

    let mut findings = Vec::new();
    for (id, claimants) in collisions {
        let mut paths: Vec<String> = claimants
            .iter()
            .map(|r| r.path.display().to_string())
            .collect();
        paths.sort();
        for record in &claimants {
            findings.push(Finding {
                rule: RULE_ID.to_string(),
                severity: FindingSeverity::Error,
                file: record.path.clone(),
                line: None,
                waived: None,
                message: format!(
                    "identifier {id} is claimed by {} records ({}) -- a reference to it resolves to only one of them",
                    claimants.len(),
                    paths.join(", ")
                ),
            });
        }
    }

    (
        RuleExecution {
            rule: RULE_ID.to_string(),
            population: Some(population),
            status: RuleStatus::Ran,
            examined_records,
        },
        findings,
    )
}

/// Records by normalized identifier.
pub type RecordIndex<'a> = HashMap<crate::values::RecordId, &'a Record>;

/// One identifier and every record claiming it.
pub type IdentifierCollision<'a> = (crate::values::RecordId, Vec<&'a Record>);

/// The index plus the identifiers more than one record claims. Collecting
/// straight into a map kept whichever record sorted last and made the loser
/// invisible to every rule that resolves a reference -- so the verdict on a
/// corpus depended on filename order (BUG-79).
pub fn build_index_reporting_collisions(
    records: &[Record],
) -> (RecordIndex<'_>, Vec<IdentifierCollision<'_>>) {
    use crate::values::RecordId;
    let mut index: HashMap<RecordId, &Record> = HashMap::new();
    let mut claimants: HashMap<RecordId, Vec<&Record>> = HashMap::new();
    for record in records {
        let Some(id) = record_id(record) else {
            continue;
        };
        let key = RecordId::new(&id);
        claimants.entry(key.clone()).or_default().push(record);
        index.entry(key).or_insert(record);
    }
    let mut collisions: Vec<IdentifierCollision<'_>> = claimants
        .into_iter()
        .filter(|(_, rs)| rs.len() > 1)
        .collect();
    collisions.sort_by(|a, b| a.0.cmp(&b.0));
    (index, collisions)
}

/// Rule 2 (config-driven per type since MILE-0090/ADR-0044): a
/// `pointer_fields`/`narrative_fields` entry resolves to a real record when
/// it names one. The target's status is surfaced in the message, never
/// judged -- whether a `Draft` target is acceptable is a policy decision
/// (RFC-0012), not this checker's call. Both kinds resolve identically here;
/// existence-checking isn't a `FieldKindSpec` capability; only the clean-
/// format check and the staleness check differ per kind.
/// Split out of `pointer.resolution` (MILE-80), which emitted a finding for
/// *every* reference that resolved -- 172 of 213 on this repo's own corpus, all
/// saying a reference worked. One rule id cannot carry two severities, so the
/// dangling-reference error and this policy could never be levelled apart.
///
/// Fires only on the statuses a repository declares unacceptable. With none
/// declared it examines nothing and reports nothing, rather than falling back
/// to a built-in list: a table keyed on record type names makes the rule
/// silently stop applying to every corpus that names its types differently.
pub fn pointer_target_status(
    records: &[Record],
    config: &Config,
    not_in: &[String],
    index: &RecordIndex,
) -> (RuleExecution, Vec<Finding>) {
    let pointer_fields_by_type = pointer_fields_by_type(config);
    let pointer_fields_by_type = &pointer_fields_by_type;
    let narrative_fields_by_type = narrative_fields_by_type(config);
    let narrative_fields_by_type = &narrative_fields_by_type;
    const RULE_ID: &str = RULE_POINTER_TARGET_STATUS;
    let mut findings = Vec::new();

    // Declared slots: every pointer or narrative field the record's type
    // declares, whether or not the record wrote it. A slot left unwritten is
    // eligible and unexamined -- handed to the rule and not judged.
    let slots =
        pointer_and_narrative_slots(records, pointer_fields_by_type, narrative_fields_by_type);

    let (population, examined_records) = census_records(
        PopulationUnit::Field,
        slots,
        |(record, _)| record.path.clone(),
        |(record, field_name)| {
            if record.header.region.is_none() {
                return Outcome::Unreadable;
            }
            let Some(value) = record.header.get(field_name.as_str()) else {
                return Outcome::Absent;
            };
            let references = extract_references(value);
            if references.is_empty() {
                return Outcome::Absent;
            }

            for reference in references {
                let Some(target) = index.get(&crate::values::RecordId::new(&reference)) else {
                    continue;
                };
                // A lookup miss is unjudged, not a status match (BUG-100).
                let target_status_field = relation_field_name(
                    config,
                    &target.record_type,
                    crate::config::RelationRole::Status,
                );
                let Ok(status) = declared_cross_record_value(config, target, target_status_field)
                else {
                    continue;
                };
                if not_in.iter().any(|s| s == status) {
                    findings.push(Finding {
                        rule: RULE_ID.to_string(),
                        severity: FindingSeverity::Warning,
                        file: record.path.clone(),
                        line: None,
                        waived: None,
                        message: format!(
                            "{field_name}: {reference} resolves, but its {target_status_field} is {status}"
                        ),
                    });
                }
            }
            Outcome::Examined
        },
    );

    (
        RuleExecution {
            rule: RULE_ID.to_string(),
            population: Some(population),
            status: RuleStatus::Ran,
            examined_records,
        },
        findings,
    )
}

pub fn pointer_resolution(
    records: &[Record],
    config: &Config,
    index: &RecordIndex,
) -> (RuleExecution, Vec<Finding>) {
    let pointer_fields_by_type = pointer_fields_by_type(config);
    let pointer_fields_by_type = &pointer_fields_by_type;
    let narrative_fields_by_type = narrative_fields_by_type(config);
    let narrative_fields_by_type = &narrative_fields_by_type;
    const RULE_ID: &str = RULE_POINTER_RESOLUTION;
    let mut findings = Vec::new();

    // Declared slots: every pointer or narrative field the record's type
    // declares, whether or not the record wrote it. A slot left unwritten is
    // eligible and unexamined -- handed to the rule and not judged.
    let slots =
        pointer_and_narrative_slots(records, pointer_fields_by_type, narrative_fields_by_type);

    let (population, examined_records) = census_records(
        PopulationUnit::Field,
        slots,
        |(record, _)| record.path.clone(),
        |(record, field_name)| {
            if record.header.region.is_none() {
                return Outcome::Unreadable;
            }
            let Some(value) = record.header.get(field_name.as_str()) else {
                return Outcome::Absent;
            };
            let references = extract_references(value);
            if references.is_empty() {
                return Outcome::Absent;
            }

            for reference in references {
                if index.contains_key(&crate::values::RecordId::new(&reference)) {
                    continue;
                }
                findings.push(Finding {
                    rule: RULE_ID.to_string(),
                    severity: FindingSeverity::Error,
                    file: record.path.clone(),
                    line: None,
                    waived: None,
                    message: format!(
                        "{field_name}: {reference} does not resolve to any discovered record"
                    ),
                });
            }
            Outcome::Examined
        },
    );

    (
        RuleExecution {
            rule: RULE_ID.to_string(),
            population: Some(population),
            status: RuleStatus::Ran,
            examined_records,
        },
        findings,
    )
}

/// BUG-0007: a pointer field's value silently tolerating trailing prose after
/// the reference token (`extract_references` only reads the leading token,
/// discarding the rest) turned out to be a corpus-wide habit, not two
/// isolated mistakes -- `Derives-from: RFC-1 (Accepted)`'s status annotation
/// alone appears in 33 files. That annotation is redundant with
/// `pointer.resolution`'s own live status report and a staleness risk
/// (nothing re-verifies it once written), and `Parent`'s freeform prose in
/// `SPEC-2`/`SPEC-4` is worse -- neither was ever a deliberate schema
/// decision, `extract_references` just never rejected it.
///
/// This rule is a check on the raw field value, independent of
/// `pointer_resolution`: a field is clean only if every comma-separated
/// entry, once trimmed, is *exactly* a reference token (or the `—` no-value
/// placeholder) -- nothing before or after it. Applies only to fields whose
/// `FieldKindSpec` declares `enforce_clean_format` (i.e. `pointer_fields`,
/// config-driven since MILE-0090) -- `narrative_fields` are deliberately
/// exempt, the same exemption `Blocked-on` always had.
pub fn header_pointer_field_clean(
    records: &[Record],
    config: &Config,
) -> (RuleExecution, Vec<Finding>) {
    const RULE_ID: &str = RULE_HEADER_POINTER_FIELD_CLEAN;
    let mut findings = Vec::new();

    let clean_fields_by_type = fields_with_capability(config, |k| k.enforce_clean_format);

    // Declared slots: a field the type says carries clean references. A slot
    // the record did not write is eligible and unexamined -- handed to the rule
    // and not judged -- rather than outside its population.
    let slots = field_slots(records, &clean_fields_by_type);

    let (population, examined_records) = census_records(
        PopulationUnit::Field,
        slots,
        |(record, _)| record.path.clone(),
        |(record, field_name)| {
            let Some(value) = record.header.get(field_name.as_str()) else {
                return Outcome::Absent;
            };
            if value.trim() == "—" {
                return Outcome::Examined;
            }

            for entry in value.split(',') {
                let entry = entry.trim();
                let is_clean_reference = is_record_reference(entry);
                if !is_clean_reference {
                    findings.push(Finding {
                        rule: RULE_ID.to_string(),
                        severity: FindingSeverity::Warning,
                        file: record.path.clone(),
                        line: None,
                        waived: None,
                        message: format!(
                            "{field_name} entry {entry:?} isn't a clean reference -- pointer fields should hold only comma-separated reference IDs (e.g. \"RFC-1\", not \"RFC-1 (Accepted)\"); pointer.resolution already reports a resolved target's live Status"
                        ),
                    });
                }
            }
            Outcome::Examined
        },
    );

    (
        RuleExecution {
            rule: RULE_ID.to_string(),
            population: Some(population),
            status: RuleStatus::Ran,
            examined_records,
        },
        findings,
    )
}

/// Rule (MILE-0083, generalized to any `narrative_fields` entry by
/// MILE-0090/ADR-0044 -- renamed from `blocked_on_stale`): a narrative-field
/// pointer whose target has reached a terminal status is a staleness
/// signal, distinct from `pointer_resolution`'s generic "resolves; target
/// Status = X" noise -- a blocker (or any other narrative-tolerant
/// relationship) that has actually cleared deserves its own actionable
/// finding, not one more line among many routine ones. Terminal-status sets
/// are a small, hardcoded MVP per record type (ADR-18's own "ship the MVP,
/// extend once a real case demands more" precedent), not a config surface --
/// nothing has asked for one yet. A dangling reference is deliberately not
/// this rule's job; that's `pointer_resolution`'s error case, reused rather
/// than duplicated here.
pub fn narrative_field_stale(
    records: &[Record],
    config: &Config,
    terminal_statuses: &[String],
    index: &RecordIndex,
) -> (RuleExecution, Vec<Finding>) {
    const RULE_ID: &str = RULE_NARRATIVE_FIELD_STALE;
    let mut findings = Vec::new();

    let narrative_fields_by_type = fields_with_capability(config, |k| k.check_target_staleness);

    // Declared narrative slots. A slot the record did not write, or wrote as
    // prose yielding no reference, is eligible and unexamined: BUG-39 was the
    // extractor silently returning nothing on prose, and its fix was verified
    // by watching this count move from 7 to 8. Counting a slot as examined
    // merely because it is present would pin the number and retire the
    // instrument that measured that fix.
    let slots = field_slots(records, &narrative_fields_by_type);

    let (population, examined_records) = census_records(
        PopulationUnit::Field,
        slots,
        |(record, _)| record.path.clone(),
        |(record, field_name)| {
            if record.header.region.is_none() {
                return Outcome::Unreadable;
            }
            let Some(value) = record.header.get(field_name.as_str()) else {
                return Outcome::Absent;
            };
            let references = extract_references(value);
            if references.is_empty() {
                return Outcome::Absent;
            }

            for reference in references {
                let Some(target) = index.get(&crate::values::RecordId::new(&reference)) else {
                    continue; // pointer_resolution already reports a dangling reference
                };
                let target_status_field = relation_field_name(
                    config,
                    &target.record_type,
                    crate::config::RelationRole::Status,
                );
                let Ok(status) = declared_cross_record_value(config, target, target_status_field)
                else {
                    continue;
                };
                if terminal_statuses.iter().any(|t| t == status) {
                    let source_status_field = relation_field_name(
                        config,
                        &record.record_type,
                        crate::config::RelationRole::Status,
                    );
                    findings.push(Finding {
                        rule: RULE_ID.to_string(),
                        severity: FindingSeverity::Warning,
                        file: record.path.clone(),
                        line: None,
                        waived: None,
                        message: format!(
                            "{field_name}: {reference} has reached a terminal status ({status}) -- re-examine whether this record's {source_status_field}/{field_name} should update"
                        ),
                    });
                }
            }
            Outcome::Examined
        },
    );

    (
        RuleExecution {
            rule: RULE_ID.to_string(),
            population: Some(population),
            status: RuleStatus::Ran,
            examined_records,
        },
        findings,
    )
}

/// A record's own identifier, derived from its filename: `TYPE-NNNN-slug.md`,
/// type prefix explicit in the filename, matching how a record is referenced
/// in prose everywhere else (e.g. `Implements: ADR-36`). Filename-derived,
/// not header-derived, so a record can be referenced before its own header
/// claims anything about itself.
///
/// BUG-2: no fixed digit-count is required (any non-empty numeric prefix
/// resolves) -- a hardcoded 4-digit check here would silently stop matching
/// past 9999 records of one type. Matching itself is done by numeric value
/// (`normalize_id`), not by this string, so `ADR-0034` and a hand-typed
/// `ADR-34` reference resolve identically regardless of padding.
///
/// ADR-36's amendment removed acceptance of the legacy, pre-type-prefix
/// `NNNN-slug.md` shape (BUG-9): it was never a real shape any adopter's own
/// corpus would independently use, only this repo's own pre-ADR-36 history,
/// and that history no longer exists in the corpus either.
pub(crate) fn record_id(record: &Record) -> Option<String> {
    let stem = record.path.file_stem()?.to_str()?;
    let type_prefix = record.type_prefix.clone();

    let rest = stem.strip_prefix(&format!("{type_prefix}-"))?;
    let number = rest.split_once('-').map_or(rest, |(n, _)| n);
    if number.is_empty() || !number.chars().all(|c| c.is_ascii_digit()) {
        return None;
    }
    Some(format!("{type_prefix}-{number}"))
}

/// Extract reference tokens like `RFC-0001` from a field value that may list
/// several, comma-separated, with trailing annotation in parentheses (e.g.
/// `RFC-0001 (Draft)`). A claim is the reference that begins an entry;
/// everything after it up to the next comma is annotation.
/// Find reference tokens anywhere in a line of prose.
///
/// [`extract_references`] reads the first whitespace token of each
/// comma-separated entry, which is correct for a header value and silently
/// empty for a sentence.
/// `RFC-9's` is a reference to `RFC-9`; `RFC-9a` is not (BUG-39). Only a
/// possessive is stripped, because it is the one suffix that attaches to a
/// reference without changing which record is meant -- anything else is a
/// different identifier.
fn strip_possessive(token: &str) -> &str {
    token
        .strip_suffix("'s")
        .or_else(|| token.strip_suffix("\u{2019}s"))
        .unwrap_or(token)
}

/// A record reference like `RFC-9` or `DOC-ADR-2` (a hyphenated type prefix,
/// `parse_record_filename`'s own supported shape) -- every segment before
/// the last is a non-empty, uppercase prefix component, and the last segment
/// is the number. `split_once('-')` here would take only the first hyphen,
/// which rejects a hyphenated prefix outright (`BUG-111`).
fn is_record_reference(token: &str) -> bool {
    let segments: Vec<&str> = token.split('-').collect();
    let Some((num, prefix_segments)) = segments.split_last() else {
        return false;
    };
    !num.is_empty()
        && num.chars().all(|c| c.is_ascii_digit())
        && !prefix_segments.is_empty()
        && prefix_segments
            .iter()
            .all(|s| !s.is_empty() && s.chars().all(|c| c.is_ascii_uppercase()))
}

pub(crate) fn scan_references(line: &str) -> Vec<String> {
    let mut out = Vec::new();
    for token in line.split(|c: char| c.is_whitespace() || c == '(' || c == '[') {
        let token = strip_possessive(token.trim_matches(|c: char| {
            !c.is_ascii_alphanumeric() && c != '-' && c != '\'' && c != '\u{2019}'
        }));
        // A quoted reference in prose ('RFC-9') keeps its opening mark through
        // the trim above, which allows quotes so the possessive survives.
        let token = token.trim_matches(['\'', '\u{2019}']);
        if is_record_reference(token) {
            out.push(token.to_string());
        }
    }
    out
}

pub(crate) fn extract_references(value: &str) -> Vec<String> {
    value
        .split(',')
        .filter_map(|entry| {
            let entry = entry.trim();
            let token = entry.split_whitespace().next()?;
            let token = strip_possessive(token.trim_end_matches(['.', ':']));
            is_record_reference(token).then(|| token.to_string())
        })
        .collect()
}

/// Rule 3 (Phase 6): a required field's *quality*, not just its presence.
/// `header.required-fields` only asks "is the key there"; a field holding
/// unedited template text or an explicit pending marker passes that check
/// and still isn't a real value. Blank is reported here too (redundantly
/// with Rule 1) so this rule's own report is self-contained.
/// Split out of `field.quality` (BUG-38). A required field marked `Pending`
/// says someone decided the work is unfinished; a `Blank` one says someone
/// forgot. A repository that does not declare this rule is not told about its
/// own deliberate markers, which is the right default -- it wrote them.
pub fn field_pending(records: &[Record], config: &Config) -> (RuleExecution, Vec<Finding>) {
    let required_by_type = required_fields_by_type(config);
    let required_by_type = &required_by_type;
    const RULE_ID: &str = RULE_FIELD_PENDING;
    let mut findings = Vec::new();

    // Same population as `field.quality`: one declared slot per required field.
    let slots = field_slots(records, required_by_type);

    let (population, examined_records) = census_records(
        PopulationUnit::Field,
        slots,
        |(record, _)| record.path.clone(),
        |(record, field)| {
            // Unreadable, not absent -- the same reason as `field.quality`.
            if record.header.region.is_none() || record.header.parse_error.is_some() {
                return Outcome::Unreadable;
            }
            if classify(record.header.get(field.as_str())) == FieldState::Pending {
                findings.push(Finding {
                    rule: RULE_ID.to_string(),
                    severity: FindingSeverity::Warning,
                    file: record.path.clone(),
                    line: None,
                    waived: None,
                    message: format!(
                        "field '{field}' is marked pending -- work declared unfinished"
                    ),
                });
            }
            Outcome::Examined
        },
    );

    (
        RuleExecution {
            rule: RULE_ID.to_string(),
            population: Some(population),
            status: RuleStatus::Ran,
            examined_records,
        },
        findings,
    )
}

/// The references a line actually claims to close: the run immediately
/// following a closing verb, ending at the first token that is neither a
/// reference nor a separator.
///
/// Deliberately narrow. A missed claim is recoverable; a false error on a
/// correct sentence teaches people to stop reading the output.
fn claimed_closed(line: &str) -> Vec<String> {
    const VERBS: [&str; 3] = ["closes", "fixes", "resolves"];
    let mut out = Vec::new();
    let mut tokens = line.split_whitespace().peekable();

    while let Some(token) = tokens.next() {
        let word = token
            .trim_matches(|c: char| !c.is_ascii_alphanumeric())
            .to_lowercase();
        if !VERBS.contains(&word.as_str()) {
            continue;
        }
        let mut expecting = true;
        let mut seen_one = false;
        while let Some(next) = tokens.peek() {
            // A conjunction bridges two references in one claim ("X and Y",
            // "X, and Y") but never starts one -- "Fixes and BUG-40" claims
            // nothing. Bridgeable from either state once a reference has been
            // seen, because an Oxford comma leaves the run open.
            if seen_one && next.eq_ignore_ascii_case("and") {
                tokens.next();
                expecting = true;
                continue;
            }
            if !expecting {
                break;
            }
            let refs = scan_references(next);
            if refs.is_empty() {
                break;
            }
            expecting = next.ends_with(',');
            seen_one = true;
            out.extend(refs);
            tokens.next();
        }
    }
    out
}

/// A file outside the corpus claiming a record is closed, while the record
/// itself says otherwise.
///
/// Written after a changeset in this repository announced that it closed
/// `BUG-36`. It did not -- it fixed a hazard recorded *beside* that bug -- and
/// nothing noticed, because the claim and the record it contradicted live in
/// different files and only one of them was ever read.
///
/// `closed_statuses` is declared, never inferred. A built-in list would make
/// this rule stop applying the moment a repository used a status the list did
/// not know, which is the failure direction this project treats as the worse
/// one.
///
/// Matches on verb and proximity, not meaning, so it can be wrong in both
/// directions: a present-tense sentence discussing someone else's claim will
/// match, and a claim phrased without one of the verbs will not. A waiver
/// (`ADR-11`) is the intended escape for the former -- deliberately a record,
/// so an exception is visible rather than a silent pattern tweak.
pub fn claim_status_agreement(
    claims: &[(String, String)],
    closed_statuses: &[String],
    config: &Config,
    index: &RecordIndex,
) -> (RuleExecution, Vec<Finding>) {
    const RULE_ID: &str = RULE_CLAIM_STATUS_AGREEMENT;
    let mut findings = Vec::new();

    // Present tense only: a claim is written in the present, while prose
    // *about* a past claim is written in the past. A narrowing, not a fix --
    // a present-tense sentence discussing a claim still matches.
    let mut occurrences: Vec<(&String, usize, String, String)> = Vec::new();
    for (path, content) in claims {
        for (idx, line) in content.lines().enumerate() {
            for reference in claimed_closed(line) {
                let normalized = crate::values::RecordId::new(&reference).to_string();
                occurrences.push((path, idx + 1, reference, normalized));
            }
        }
    }

    // One candidate per *distinct* claimed reference, deduped the same way the
    // verdict is: two changesets closing one record are one thing to judge.
    // Counting occurrences instead made the number exceed the corpus while
    // saying nothing about records (BUG-90).
    let mut seen: std::collections::HashSet<&str> = std::collections::HashSet::new();
    let candidates: Vec<&str> = occurrences
        .iter()
        .map(|(_, _, _, normalized)| normalized.as_str())
        .filter(|normalized| seen.insert(normalized))
        .collect();

    // A claimed reference resolving to no record is eligible and unexamined --
    // the claim was made and the engine could not judge it. It was a bare
    // `continue` that moved no number.
    let population = census(PopulationUnit::Claim, candidates, |normalized| {
        if index.contains_key(*normalized) {
            Outcome::Examined
        } else {
            Outcome::Absent
        }
    });

    // Findings are per occurrence, not per candidate: each place the claim is
    // written is its own thing to correct.
    for (path, line, reference, normalized) in occurrences {
        let Some(target) = index.get(normalized.as_str()) else {
            continue;
        };
        // A lookup miss is unjudged, not a status match (BUG-100).
        let status_field = relation_field_name(
            config,
            &target.record_type,
            crate::config::RelationRole::Status,
        );
        let Ok(status) = declared_cross_record_value(config, target, status_field) else {
            continue;
        };
        if closed_statuses.iter().any(|s| s == status) {
            continue;
        }
        findings.push(Finding {
            rule: RULE_ID.to_string(),
            severity: FindingSeverity::Error,
            file: PathBuf::from(path),
            line: Some(line),
            waived: None,
            message: format!(
                "claims to close {reference}, but {reference} has {status_field} {status}"
            ),
        });
    }

    (
        RuleExecution {
            rule: RULE_ID.to_string(),
            population: Some(population),
            status: RuleStatus::Ran,
            examined_records: Vec::new(),
        },
        findings,
    )
}

/// A declared field whose value carries leading or trailing whitespace.
///
/// Every other rule compares a value exactly (`ADR-57`), so `"Superseded   "`
/// is not `Superseded` and a status rule configured to report it stays silent.
/// Trimming inside those rules would paper over it; this reports it instead --
/// the split `yamllint` makes between formatting and meaning.
///
/// Only reachable through `yaml-frontmatter`: the blockquote parser trims at
/// parse time and YAML trims an unquoted scalar, so it takes a quoted value to
/// carry the space this far.
pub fn field_untrimmed_value(records: &[Record], config: &Config) -> (RuleExecution, Vec<Finding>) {
    let declared_by_type = declared_fields_by_type(config);
    let declared_by_type = &declared_by_type;
    const RULE_ID: &str = RULE_FIELD_UNTRIMMED_VALUE;
    let mut findings = Vec::new();

    let slots = field_slots(records, declared_by_type);

    let (population, examined_records) = census_records(
        PopulationUnit::Field,
        slots,
        |(record, _)| record.path.clone(),
        |(record, field)| {
            if record.header.region.is_none() {
                return Outcome::Unreadable;
            }
            let Some(value) = record.header.get(field.as_str()) else {
                return Outcome::Absent;
            };
            if value != value.trim() {
                findings.push(Finding {
                    rule: RULE_ID.to_string(),
                    severity: FindingSeverity::Warning,
                    file: record.path.clone(),
                    line: None,
                    waived: None,
                    message: format!(
                        "field '{field}' is {value:?} -- the surrounding whitespace is part of the value, so any exact comparison against '{}' fails",
                        value.trim()
                    ),
                });
            }
            Outcome::Examined
        },
    );

    (
        RuleExecution {
            rule: RULE_ID.to_string(),
            population: Some(population),
            status: RuleStatus::Ran,
            examined_records,
        },
        findings,
    )
}

/// A declared field written under a different case than its declaration
/// (`RFC-40`, `ADR-58`): `ADR-57` makes that a different name, so `read_declared`
/// treats it as absent everywhere else, and this is the one place that says
/// which absences are really this instead of non-adoption.
pub fn header_field_case_mismatch(
    records: &[Record],
    config: &Config,
) -> (RuleExecution, Vec<Finding>) {
    let declared_by_type = declared_fields_by_type(config);
    let declared_by_type = &declared_by_type;
    const RULE_ID: &str = RULE_HEADER_FIELD_CASE_MISMATCH;
    let mut findings = Vec::new();

    let slots = field_slots(records, declared_by_type);

    let (population, examined_records) = census_records(
        PopulationUnit::Field,
        slots,
        |(record, _)| record.path.clone(),
        |(record, field)| {
            if record.header.region.is_none() {
                return Outcome::Unreadable;
            }
            if record.header.get(field.as_str()).is_some() {
                return Outcome::Examined;
            }
            // `.min_by`, not `.find`: several header keys can differ from the
            // declared name only in case, and a message that changes between
            // runs on one corpus is `near_miss`'s own shape (`BUG-79`).
            let Some(found) = record
                .header
                .fields
                .iter()
                .filter(|f| {
                    f.key.as_str().eq_ignore_ascii_case(field.as_str())
                        && f.key.as_str() != field.as_str()
                })
                .min_by(|a, b| a.key.cmp(&b.key))
            else {
                return Outcome::Absent;
            };
            findings.push(Finding {
                rule: RULE_ID.to_string(),
                severity: FindingSeverity::Error,
                file: record.path.clone(),
                line: Some(found.line),
                waived: None,
                message: format!(
                    "declared field '{field}' found as '{}' -- field names compare exactly (ADR-57); rename it",
                    found.key
                ),
            });
            Outcome::Examined
        },
    );

    (
        RuleExecution {
            rule: RULE_ID.to_string(),
            population: Some(population),
            status: RuleStatus::Ran,
            examined_records,
        },
        findings,
    )
}

pub fn field_quality(records: &[Record], config: &Config) -> (RuleExecution, Vec<Finding>) {
    let required_by_type = required_fields_by_type(config);
    let required_by_type = &required_by_type;
    const RULE_ID: &str = RULE_FIELD_QUALITY;
    let mut findings = Vec::new();

    // The population is the declared slot: one `(record, required field)` pair
    // per field the record's *type* declares. Built here, before the rule runs,
    // so `eligible` is this list's length and cannot disagree with what the
    // body does. It is far larger than the record count and always was
    // (`BUG-40`); the unit is what makes that legible rather than alarming.
    let slots = field_slots(records, required_by_type);

    let (population, examined_records) = census_records(
        PopulationUnit::Field,
        slots,
        |(record, _)| record.path.clone(),
        |(record, field)| {
            // Unreadable, not absent: the header did not parse, so this slot
            // has no value to classify. Calling it Blank reports something
            // untrue about a file that may well set the field, and a rule whose
            // `examined` can never fall short of `eligible` can never be caught
            // not looking (ADR-55). `header.required-fields` reports the parse
            // error itself, once per record rather than once per slot.
            if record.header.region.is_none() || record.header.parse_error.is_some() {
                return Outcome::Unreadable;
            }
            let state = classify(record.header.get(field.as_str()));
            // `Pending` is `field.pending`'s subject, not this rule's: it means
            // someone declared the work unfinished, where Blank and Placeholder
            // mean someone forgot. One rule carries one declared level, so
            // keeping both here left no setting that was correct (BUG-38).
            if matches!(state, FieldState::Blank | FieldState::Placeholder) {
                findings.push(Finding {
                    rule: RULE_ID.to_string(),
                    severity: FindingSeverity::Error,
                    file: record.path.clone(),
                    line: None,
                    waived: None,
                    message: format!("field '{field}' is {state:?} -- not a real, present value"),
                });
            }
            Outcome::Examined
        },
    );

    (
        RuleExecution {
            rule: RULE_ID.to_string(),
            population: Some(population),
            status: RuleStatus::Ran,
            examined_records,
        },
        findings,
    )
}

/// Rule 4 (Phase 6): a filename's own claimed number must match its
/// document's H1 title. A rename that updates one and not the other is
/// silent otherwise -- nothing else in this corpus cross-checks the two.
pub fn filename_title_consistency(
    records: &[Record],
    full_text: &HashMap<std::path::PathBuf, String>,
) -> (RuleExecution, Vec<Finding>) {
    const RULE_ID: &str = RULE_FILENAME_TITLE_CONSISTENCY;
    let mut findings = Vec::new();

    let (population, examined_records) = census_records(
        PopulationUnit::Record,
        records.iter().collect(),
        |record| record.path.clone(),
        |record| {
            let Some(filename_number) = filename_number(record) else {
                return Outcome::OutOfScope;
            };
            let Some(content) = full_text.get(&record.path) else {
                return Outcome::Absent;
            };

            let mut push = |line: Option<usize>, message: String| {
                findings.push(Finding {
                    rule: RULE_ID.to_string(),
                    severity: FindingSeverity::Error,
                    file: record.path.clone(),
                    line,
                    waived: None,
                    message,
                });
            };

            // Skipping instead would drop a record the rule was asked about.
            let Ok(filename_number) = filename_number.parse::<u64>() else {
                push(
                    None,
                    format!("filename number {filename_number} is too large to compare"),
                );
                return Outcome::Examined;
            };

            match first_h1(content, record.header.region) {
            None => push(
                None,
                "no H1 title found to check against the filename's number".to_string(),
            ),
            Some(h1) => match h1.number {
                None => push(
                    Some(h1.line),
                    format!(
                        "H1 title '{}' carries no number to check against the filename's number {filename_number}",
                        elided(h1.text)
                    ),
                ),
                Some(n) if n != filename_number => push(
                    Some(h1.line),
                    format!(
                        "filename claims number {filename_number}, but the H1 title '{}' claims {n}",
                        elided(h1.text)
                    ),
                ),
                Some(_) => {}
            },
        }
            Outcome::Examined
        },
    );

    (
        RuleExecution {
            rule: RULE_ID.to_string(),
            population: Some(population),
            status: RuleStatus::Ran,
            examined_records,
        },
        findings,
    )
}

/// Same acceptance as `record_id` (ADR-36/BUG-2): `TYPE-NNNN-slug.md`, any
/// non-empty numeric prefix, no fixed digit-count.
fn filename_number(record: &Record) -> Option<String> {
    let stem = record.path.file_stem()?.to_str()?;
    let type_prefix = record.type_prefix.clone();

    let rest = stem.strip_prefix(&format!("{type_prefix}-"))?;
    let number = rest.split_once('-').map_or(rest, |(n, _)| n);
    (!number.is_empty() && number.chars().all(|c| c.is_ascii_digit())).then(|| number.to_string())
}

/// Whether an H1 exists and whether it carries a number are independent
/// questions, so they stay independent `Option`s rather than collapsing into
/// one absent value.
struct H1<'a> {
    /// 1-indexed, matching [`crate::header::Header::region`].
    line: usize,
    text: &'a str,
    /// Compared by numeric value (BUG-0002), so padding never mismatches.
    number: Option<u64>,
}

/// Frontmatter is skipped because a YAML comment line inside it starts with
/// `# ` and would otherwise read as the title. Only a region opening at line
/// 1 is frontmatter -- blockquote and bold-list headers sit *after* the H1,
/// where skipping past them would skip the title itself.
fn first_h1(content: &str, header_region: Option<(usize, usize)>) -> Option<H1<'_>> {
    let skip_through = header_region
        .filter(|&(start, _)| start == 1)
        .map(|(_, end)| end);
    let mut fence: Option<(u8, usize)> = None;

    for (idx, line) in content.lines().enumerate() {
        let lineno = idx + 1;
        if skip_through.is_some_and(|end| lineno <= end) {
            continue;
        }

        let trimmed = line.trim_start();
        if let Some(marker @ (b'`' | b'~')) = trimmed.bytes().next() {
            let run = trimmed.bytes().take_while(|&b| b == marker).count();
            if run >= 3 {
                match fence {
                    // A fence closes only on its own marker, at its own
                    // length or longer, with nothing following. A shorter
                    // run or the other family is content -- closing on
                    // either reopens the body mid-block.
                    Some((open, len))
                        if marker == open && run >= len && trimmed[run..].trim().is_empty() =>
                    {
                        fence = None;
                    }
                    None => fence = Some((marker, run)),
                    Some(_) => {}
                }
                continue;
            }
        }
        // A fenced `# ` is sample text; matching it fabricates a mismatch
        // against a number that is not a record number.
        if fence.is_some() || !line.starts_with("# ") {
            continue;
        }

        let text = line[2..].trim();
        // Skip, never abandon: a bare `#` is not a title, but a real one may
        // follow it, and reporting "no H1 found" over it hides the mismatch
        // this rule exists to find (BUG-64).
        if text.is_empty() {
            continue;
        }
        let digits: String = text
            .split_whitespace()
            .next()
            .unwrap_or_default()
            .chars()
            .filter(char::is_ascii_digit)
            .collect();

        return Some(H1 {
            line: lineno,
            text,
            number: digits.parse().ok(),
        });
    }

    None
}

/// The text reaches the JSON contract, and a foreign corpus's H1 has no
/// length this repo controls.
fn elided(text: &str) -> String {
    const MAX: usize = 60;
    if text.chars().count() <= MAX {
        return text.to_string();
    }
    text.chars().take(MAX).chain(['…']).collect()
}

/// Rule 6 (ADR-0014): every existing revision-log entry names a real
/// `change_class`. A `—` or blank Class column is exactly the failure
/// ADR-0014 exists to prevent -- a substantive change folded silently into
/// "no class recorded," undetectable by any per-field presence check because
/// the field itself is present, just not a real value.
///
/// This rule does not require a revision log to exist at all -- that is
/// SPEC-0002's separate "required sections per profile" rule. It only checks
/// entries in a section that is already there.
pub fn revision_log_change_class(
    records: &[Record],
    full_text: &HashMap<std::path::PathBuf, String>,
) -> (RuleExecution, Vec<Finding>) {
    const RULE_ID: &str = RULE_REVISION_LOG_CHANGE_CLASS_REQUIRED;
    let mut findings = Vec::new();

    // Every record is eligible. A record with no `**Revision log**` marker is
    // *absent* from this rule's judgement, not outside its population -- and
    // absence is exactly what BUG-50 reports as indistinguishable from
    // compliance: SPEC-1 lost 18 revision rows by losing one line and the rule
    // stayed green. `eligible` minus `examined` is exactly that subtraction, expressed
    // where `records_examined` alone could not carry it.
    let candidates: Vec<&Record> = records.iter().collect();

    let (population, examined_records) = census_records(
        PopulationUnit::Record,
        candidates,
        |record| record.path.clone(),
        |record| {
            let Some(content) = full_text.get(&record.path) else {
                return Outcome::Absent;
            };
            let Some(entries) = find_revision_log_entries(content) else {
                return Outcome::Absent;
            };

            for entry in entries {
                let class = entry.change_class.trim_matches('*').trim();
                if !matches!(class, "substantive" | "structural") {
                    findings.push(Finding {
                    rule: RULE_ID.to_string(),
                    severity: FindingSeverity::Error,
                    file: record.path.clone(),
                    line: Some(entry.line),
                    waived: None,
                    message: format!(
                        "revision log entry dated {} has no real change_class (found {:?}) -- expected \"substantive\" or \"structural\"",
                        entry.date, entry.change_class
                    ),
                });
                }
            }
            Outcome::Examined
        },
    );

    (
        RuleExecution {
            rule: RULE_ID.to_string(),
            population: Some(population),
            status: RuleStatus::Ran,
            examined_records,
        },
        findings,
    )
}

pub(crate) struct RevisionLogEntry {
    #[allow(dead_code)]
    pub(crate) date: String,
    #[allow(dead_code)]
    pub(crate) change_class: String,
    pub(crate) line: usize,
}

/// Finds a `**Revision log**` marker and parses the pipe-delimited table
/// that follows it. `None` means no such section exists in this record at
/// all -- distinct from `Some(vec![])`, an existing but empty table, though
/// both currently produce no findings from this rule.
pub(crate) fn find_revision_log_entries(content: &str) -> Option<Vec<RevisionLogEntry>> {
    let lines: Vec<&str> = content.lines().collect();
    let marker_idx = lines.iter().position(|l| l.contains("**Revision log**"))?;

    let mut entries = Vec::new();
    let mut row_index = 0; // 0 before the header row, 1 = header seen, 2+ = data rows
    for (idx, raw_line) in lines.iter().enumerate().skip(marker_idx + 1) {
        let stripped = raw_line.trim_start().trim_start_matches('>').trim();
        if stripped.is_empty() {
            if row_index == 0 {
                continue; // blank blockquote continuation before the table
            }
            break; // the table has ended
        }
        if !stripped.starts_with('|') {
            break;
        }

        let cells: Vec<&str> = stripped
            .trim_matches('|')
            .split('|')
            .map(|c| c.trim())
            .collect();
        row_index += 1;
        if row_index == 1 {
            continue; // header row: "Date | Change | Class"
        }
        if cells
            .iter()
            .all(|c| !c.is_empty() && c.chars().all(|ch| ch == '-'))
        {
            continue; // separator row: |---|---|---|
        }

        let (Some(date), Some(class)) = (cells.first(), cells.last()) else {
            continue;
        };
        entries.push(RevisionLogEntry {
            date: date.to_string(),
            change_class: class.to_string(),
            line: idx + 1,
        });
    }

    Some(entries)
}

/// A record's `Realized-by` field, categorized (ADR-0018's MVP scope --
/// three flat lists, not RFC-0005's full claim graph). `spec:`/`code:`/
/// `test:` prefixes; an unrecognized prefix is dropped rather than reported,
/// since this field's closure isn't this rule's concern.
#[derive(Debug, Default, PartialEq, Eq)]
pub(crate) struct RealizedBy {
    pub(crate) spec: Vec<String>,
    pub(crate) code: Vec<String>,
    pub(crate) test: Vec<String>,
}

pub(crate) fn parse_realized_by(value: &str) -> RealizedBy {
    let mut result = RealizedBy::default();
    for segment in value.split(',') {
        let segment = segment.trim();
        let Some((prefix, locator)) = segment.split_once(':') else {
            continue;
        };
        let locator = locator.trim().to_string();
        match prefix.trim() {
            "spec" => result.spec.push(locator),
            "code" => result.code.push(locator),
            "test" => result.test.push(locator),
            _ => {}
        }
    }
    result
}

/// Every locator path across all three categories, flattened -- what a
/// caller needs to walk each one's git history for drift detection
/// (ADR-0032). Order isn't meaningful; only membership is.
pub fn realized_by_locator_paths(value: &str) -> Vec<String> {
    let parsed = parse_realized_by(value);
    parsed
        .spec
        .into_iter()
        .chain(parsed.code)
        .chain(parsed.test)
        .collect()
}

/// ADR-0018's tier computation: the highest tier with a non-empty category.
/// `test` outranks `code` outranks `spec` -- a claim verified by a test is
/// stronger evidence than a claim only pointed at by a spec. `drifted`
/// (ADR-0032) overrides every tier unconditionally -- `DriftDetected` is an
/// alarm state reachable from any other state, per the schema's own
/// description, not just another tier to rank against the others.
pub(crate) fn compute_embodiment(realized_by: &RealizedBy, drifted: bool) -> &'static str {
    if drifted {
        "Drift detected"
    } else if !realized_by.test.is_empty() {
        "Verified"
    } else if !realized_by.code.is_empty() {
        "Implemented"
    } else if !realized_by.spec.is_empty() {
        "Specified"
    } else {
        "Not started"
    }
}

/// The records whose type declares *every* named role's field (resolved via
/// `RecordTypeConfig::relation_field`, `RFC-42`/`ADR-61`), as the rule's
/// candidate slots. A rule needing two fields takes the pair as one
/// candidate, so a record carrying only one of them is eligible and
/// unexamined rather than outside the population.
///
/// Declared means `required_fields` **or** `known_fields`: the latter is
/// documented as the fields a type carries *beyond* the former, so reading it
/// alone silently drops every type that requires the field instead of merely
/// permitting it. `config.pointer-field-not-known` already unions the two.
fn declared_slots_for_roles<'a>(
    records: &'a [Record],
    config: &Config,
    roles: &[crate::config::RelationRole],
) -> Vec<&'a Record> {
    records
        .iter()
        .filter(|record| {
            config
                .record_types
                .get(&record.record_type)
                .is_some_and(|t| {
                    roles
                        .iter()
                        .all(|role| t.declared_fields().contains(t.relation_field(*role)))
                })
        })
        .collect()
}

/// A declared field's value, or the `Outcome` its absence means (`RFC-40`):
/// `Unreadable` if the header itself didn't parse, `Absent` otherwise.
fn declared_value<'a>(record: &'a Record, key: &str) -> Result<&'a str, Outcome> {
    match record.header.read_declared(key) {
        crate::header::FieldRead::Present(value) => Ok(value),
        crate::header::FieldRead::Missing => Err(Outcome::Absent),
        crate::header::FieldRead::Unreadable => Err(Outcome::Unreadable),
    }
}

/// The field name `record`'s own type uses for `role` (`RFC-42`/`ADR-61`),
/// or the role's pre-`RFC-42` default if the type isn't configured at all.
pub fn relation_field_name<'a>(
    config: &'a Config,
    record_type: &str,
    role: crate::config::RelationRole,
) -> &'a str {
    config
        .record_types
        .get(record_type)
        .map(|t| t.relation_field(role))
        .unwrap_or_else(|| role.default_field_name())
}

/// `declared_value`, additionally gated on `record`'s own type declaring
/// `key` at all (`ADR-60`): a type that never declares `Status` is not this
/// rule's business to judge, the same as any other undeclared field --
/// `Status` is adopter vocabulary like every other field `ADR-53` governs,
/// not an engine-reserved one.
fn declared_cross_record_value<'a>(
    config: &Config,
    record: &'a Record,
    key: &str,
) -> Result<&'a str, Outcome> {
    let declared = config
        .record_types
        .get(&record.record_type)
        .is_some_and(|t| {
            t.declared_fields()
                .contains(&crate::values::FieldName::from(key))
        });
    if !declared {
        return Err(Outcome::Absent);
    }
    declared_value(record, key)
}

/// A `Realized-by` locator naming a path that is not in the working tree.
///
/// `Embodiment` is computed from these paths, so a locator naming nothing lets
/// a record claim verified work against a file that is not there -- and the
/// claim reads as stronger than `Not started`, not weaker.
///
/// `present` is supplied by the caller: this crate does not touch the
/// filesystem.
pub fn embodiment_locator_exists(
    records: &[Record],
    config: &Config,
    present: &dyn Fn(&str) -> bool,
) -> (RuleExecution, Vec<Finding>) {
    const RULE_ID: &str = RULE_EMBODIMENT_LOCATOR_EXISTS;
    let mut findings = Vec::new();

    // One candidate per declared slot, not per locator: the population is the
    // rule's input, not its work count.
    let slots = declared_slots_for_roles(
        records,
        config,
        &[crate::config::RelationRole::EmbodimentLocator],
    );

    let (population, examined_records) = census_records(
        PopulationUnit::Field,
        slots,
        |record| record.path.clone(),
        |record| {
            let field = relation_field_name(
                config,
                &record.record_type,
                crate::config::RelationRole::EmbodimentLocator,
            );
            let value = match declared_value(record, field) {
                Ok(v) => v,
                Err(outcome) => return outcome,
            };
            let realized = parse_realized_by(value);
            for locator in realized
                .spec
                .iter()
                .chain(&realized.code)
                .chain(&realized.test)
            {
                // An empty locator joins to the repository root, which exists --
                // so it would pass while still computing to `Implemented`.
                if locator.trim().is_empty() {
                    findings.push(Finding {
                        rule: RULE_ID.to_string(),
                        severity: FindingSeverity::Error,
                        file: record.path.clone(),
                        line: None,
                        waived: None,
                        message: format!("{field} has an empty locator"),
                    });
                    continue;
                }
                if present(locator) {
                    continue;
                }
                findings.push(Finding {
                    rule: RULE_ID.to_string(),
                    severity: FindingSeverity::Error,
                    file: record.path.clone(),
                    line: None,
                    waived: None,
                    message: format!("{field} names '{locator}', which is not in the working tree"),
                });
            }
            Outcome::Examined
        },
    );

    (
        RuleExecution {
            rule: RULE_ID.to_string(),
            population: Some(population),
            status: RuleStatus::Ran,
            examined_records,
        },
        findings,
    )
}

/// Rule (ADR-0018/ADR-0032): a record's stated `Embodiment` must agree with
/// what its own `Realized-by` locators compute to, including drift -- a
/// locator that changed, per git history, since the `Realized-by` line was
/// last touched. Both `Embodiment` and `Realized-by` have to be present --
/// a record with no `Realized-by` at all is eligible and unexamined on a
/// corpus that hasn't adopted the field, not a defect in the rule. `drifted` is
/// precomputed by the caller (git history is I/O, this function isn't --
/// same shape as `full_text` elsewhere in this module).
pub fn embodiment_consistency(
    records: &[Record],
    config: &Config,
    drifted: &HashSet<PathBuf>,
) -> (RuleExecution, Vec<Finding>) {
    const RULE_ID: &str = RULE_EMBODIMENT_CONSISTENCY;
    let mut findings = Vec::new();

    // The candidate is the *pair*: the rule needs both fields, so a record
    // carrying only one of them is handed to the rule and reaches no verdict.
    // Treating each field as its own slot would make that state unreportable.
    let slots = declared_slots_for_roles(
        records,
        config,
        &[
            crate::config::RelationRole::EmbodimentState,
            crate::config::RelationRole::EmbodimentLocator,
        ],
    );

    let (population, examined_records) = census_records(
        PopulationUnit::Field,
        slots,
        |record| record.path.clone(),
        |record| {
            let state_field = relation_field_name(
                config,
                &record.record_type,
                crate::config::RelationRole::EmbodimentState,
            );
            let locator_field = relation_field_name(
                config,
                &record.record_type,
                crate::config::RelationRole::EmbodimentLocator,
            );
            let stated = match declared_value(record, state_field) {
                Ok(v) => v,
                Err(outcome) => return outcome,
            };
            let realized_by_value = match declared_value(record, locator_field) {
                Ok(v) => v,
                Err(outcome) => return outcome,
            };

            let computed = compute_embodiment(
                &parse_realized_by(realized_by_value),
                drifted.contains(&record.path),
            );
            if stated.trim() != computed {
                findings.push(Finding {
                    rule: RULE_ID.to_string(),
                    severity: FindingSeverity::Warning,
                    file: record.path.clone(),
                    line: None,
                    waived: None,
                    message: format!(
                    "stated {state_field} '{}' disagrees with '{computed}', computed from {locator_field}",
                    stated.trim()
                ),
                });
            }
            Outcome::Examined
        },
    );

    (
        RuleExecution {
            rule: RULE_ID.to_string(),
            population: Some(population),
            status: RuleStatus::Ran,
            examined_records,
        },
        findings,
    )
}

/// Rule (ADR-0018): the same locator cited by more than one record's
/// `Realized-by` is a promotion candidate -- each record would otherwise
/// drift independently on what is really one piece of shared evidence (the
/// ADR-0072/0073/0074 shape RFC-0005 names). Reports only; a human runs the
/// actual promotion into a `claim` record, never this rule.
pub fn embodiment_locator_promotion_candidate(
    records: &[Record],
    config: &Config,
) -> (RuleExecution, Vec<Finding>) {
    const RULE_ID: &str = RULE_EMBODIMENT_LOCATOR_PROMOTION_CANDIDATE;
    let mut findings = Vec::new();

    let mut citers: std::collections::BTreeMap<
        String,
        std::collections::BTreeSet<std::path::PathBuf>,
    > = std::collections::BTreeMap::new();

    let slots = declared_slots_for_roles(
        records,
        config,
        &[crate::config::RelationRole::EmbodimentLocator],
    );

    // Findings are emitted after the census, not inside it: this rule judges
    // locators across records, so no single candidate is at fault.
    let (population, examined_records) = census_records(
        PopulationUnit::Field,
        slots,
        |record| record.path.clone(),
        |record| {
            let field = relation_field_name(
                config,
                &record.record_type,
                crate::config::RelationRole::EmbodimentLocator,
            );
            let realized_by_value = match declared_value(record, field) {
                Ok(v) => v,
                Err(outcome) => return outcome,
            };

            let parsed = parse_realized_by(realized_by_value);
            for locator in parsed
                .spec
                .iter()
                .chain(parsed.code.iter())
                .chain(parsed.test.iter())
            {
                // A BTreeSet, not a Vec: the same record citing one locator
                // under both `code:` and `test:` is still one record, not two
                // independent citers -- cross-record duplication is what needs
                // promotion, not cross-category duplication within one record.
                citers
                    .entry(locator.clone())
                    .or_default()
                    .insert(record.path.clone());
            }
            Outcome::Examined
        },
    );

    for (locator, paths) in citers {
        if paths.len() < 2 {
            continue;
        }
        let names: Vec<String> = paths.iter().map(|p| p.display().to_string()).collect();
        let first_path = paths.iter().next().expect("checked len >= 2 above").clone();
        findings.push(Finding {
            rule: RULE_ID.to_string(),
            severity: FindingSeverity::Warning,
            file: first_path,
            line: None,
            waived: None,
            message: format!(
                "locator '{locator}' is cited by {} records ({}) -- consider promoting to a shared claim record",
                paths.len(),
                names.join(", ")
            ),
        });
    }

    (
        RuleExecution {
            rule: RULE_ID.to_string(),
            population: Some(population),
            status: RuleStatus::Ran,
            examined_records,
        },
        findings,
    )
}

/// A declared rule whose candidates the configuration could not reach
/// (`MILE-106`).
///
/// Fires on `out_of_scope > 0`, never on a merely empty result. Those are
/// different problems with opposite fixes: a corpus that has not written a
/// revision log yet is doing nothing wrong and time resolves it, while a
/// `prefix` that matches no filename will examine zero records forever
/// (`BUG-61`). `type.dir-matches-nothing` draws the same line between an
/// absent directory and an empty one, and is the precedent this follows.
///
/// Deliberately silent on `Unreadable`: `header.required-fields` already
/// reports the parse error, per record, with the parser's own message.
///
/// Judges the run rather than the corpus, so it reads the executions the other
/// rules produced and never sees a record. It excludes itself -- judging its
/// own execution while producing it is not a verdict anyone can act on.
pub fn config_scope_matches_nothing(
    executed: &[RuleExecution],
    config_path: &std::path::Path,
) -> (RuleExecution, Vec<Finding>) {
    const RULE_ID: &str = RULE_CONFIG_SCOPE_MATCHES_NOTHING;
    let mut findings = Vec::new();

    let candidates: Vec<&RuleExecution> = executed
        .iter()
        .filter(|e| e.rule != RULE_ID && e.status == RuleStatus::Ran)
        .collect();

    let population = census(PopulationUnit::Rule, candidates, |exec| {
        let Some(population) = &exec.population else {
            return Outcome::Absent;
        };
        if population.out_of_scope() == 0 {
            return Outcome::Examined;
        }
        findings.push(Finding {
            rule: RULE_ID.to_string(),
            severity: FindingSeverity::Warning,
            file: config_path.to_path_buf(),
            line: None,
            waived: None,
            message: format!(
                "'{}' was handed {} candidate(s) the configuration does not reach -- \
                 {} of {} are outside what this repository declared, so the rule cannot \
                 apply to them however the records are edited",
                exec.rule,
                population.out_of_scope(),
                population.out_of_scope(),
                population.eligible()
            ),
        });
        Outcome::Examined
    });

    (
        RuleExecution {
            rule: RULE_ID.to_string(),
            population: Some(population),
            status: RuleStatus::Ran,
            examined_records: Vec::new(),
        },
        findings,
    )
}

/// Rule 5 (Phase 6): `Supersedes`/`Superseded-by` reciprocity. Checking only
/// the forward claim leaves the reverse unguarded -- a record can claim to
/// supersede something that doesn't reciprocally point back, sending a
/// reader of the *target* to a record that denies the relation.
pub fn supersession_reciprocity(
    records: &[Record],
    config: &Config,
    index: &RecordIndex,
) -> (RuleExecution, Vec<Finding>) {
    const RULE_ID: &str = RULE_RELATION_SUPERSESSION_RECIPROCITY;
    let mut findings = Vec::new();

    // A record whose filename yields no id is eligible but unexaminable: the
    // reciprocity test is "does the target name *me* back", which needs an id.
    let slots = declared_slots_for_roles(
        records,
        config,
        &[crate::config::RelationRole::Supersession],
    );

    let (population, examined_records) = census_records(
        PopulationUnit::Field,
        slots,
        |record| record.path.clone(),
        |record| {
            let field = relation_field_name(
                config,
                &record.record_type,
                crate::config::RelationRole::Supersession,
            );
            let Some(id) = record_id(record) else {
                return Outcome::OutOfScope;
            };
            let normalized_id = crate::values::RecordId::new(&id);
            let Some(value) = record.header.get(field) else {
                return Outcome::Absent;
            };
            // `—` is this corpus's written "nothing supersedes this", so the slot
            // was answered. Skipping before counting left the reciprocating half of
            // a correct pair uncounted.
            if value.trim() == "—" {
                return Outcome::Examined;
            }

            for reference in extract_references(value) {
                let Some(target) = index.get(&crate::values::RecordId::new(&reference)) else {
                    findings.push(Finding {
                        rule: RULE_ID.to_string(),
                        severity: FindingSeverity::Error,
                        file: record.path.clone(),
                        line: None,
                        waived: None,
                        message: format!(
                            "{field}: {reference} does not resolve to any discovered record"
                        ),
                    });
                    continue;
                };
                let target_field = relation_field_name(
                    config,
                    &target.record_type,
                    crate::config::RelationRole::Supersession,
                );
                let target_value = target.header.get(target_field).unwrap_or("");
                let target_names_back = extract_references(target_value)
                    .iter()
                    .any(|r| crate::values::RecordId::new(r) == normalized_id);
                if !target_names_back {
                    findings.push(Finding {
                    rule: RULE_ID.to_string(),
                    severity: FindingSeverity::Error,
                    file: record.path.clone(),
                    line: None,
 waived: None,
                    message: format!(
                        "{field} claims a relation with {reference}, but {reference}'s {target_field} does not reciprocally name {id}"
                    ),
                });
                }
            }
            Outcome::Examined
        },
    );

    (
        RuleExecution {
            rule: RULE_ID.to_string(),
            population: Some(population),
            status: RuleStatus::Ran,
            examined_records,
        },
        findings,
    )
}

/// Rule (`RFC-45`/`ADR-63`): a resolved pointer/narrative reference's target
/// may not declare `Status` at all (`ADR-60`'s gate then silently excludes
/// it from every rule that reads it -- `pointer.target-status`,
/// `narrative-field.stale`). The one place this is diagnosed, rather than
/// each consuming rule inventing its own check, the same shape
/// `header.field-case-mismatch` already is for a declared-field case
/// mismatch (`RFC-40`/`ADR-58`).
pub fn relation_target_status_undeclared(
    records: &[Record],
    config: &Config,
    index: &RecordIndex,
) -> (RuleExecution, Vec<Finding>) {
    const RULE_ID: &str = RULE_RELATION_TARGET_STATUS_UNDECLARED;
    let mut findings = Vec::new();

    let pointer_fields_by_type = pointer_fields_by_type(config);
    let pointer_fields_by_type = &pointer_fields_by_type;
    let narrative_fields_by_type = narrative_fields_by_type(config);
    let narrative_fields_by_type = &narrative_fields_by_type;

    let slots =
        pointer_and_narrative_slots(records, pointer_fields_by_type, narrative_fields_by_type);

    let (population, examined_records) = census_records(
        PopulationUnit::Field,
        slots,
        |(record, _)| record.path.clone(),
        |(record, field_name)| {
            if record.header.region.is_none() {
                return Outcome::Unreadable;
            }
            let Some(value) = record.header.get(field_name.as_str()) else {
                return Outcome::Absent;
            };
            let references = extract_references(value);
            if references.is_empty() {
                return Outcome::Absent;
            }

            for reference in references {
                let Some(target) = index.get(&crate::values::RecordId::new(&reference)) else {
                    continue; // pointer_resolution already reports a dangling reference
                };
                let target_status_field = relation_field_name(
                    config,
                    &target.record_type,
                    crate::config::RelationRole::Status,
                );
                let declared = config
                    .record_types
                    .get(&target.record_type)
                    .is_some_and(|t| t.declared_fields().contains(target_status_field));
                if !declared {
                    findings.push(Finding {
                        rule: RULE_ID.to_string(),
                        severity: FindingSeverity::Warning,
                        file: record.path.clone(),
                        line: None,
                        waived: None,
                        message: format!(
                            "{field_name}: {reference} resolves, but its type '{}' does not declare {target_status_field} -- its status can never be checked",
                            target.record_type
                        ),
                    });
                }
            }
            Outcome::Examined
        },
    );

    (
        RuleExecution {
            rule: RULE_ID.to_string(),
            population: Some(population),
            status: RuleStatus::Ran,
            examined_records,
        },
        findings,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::record::Record;
    use std::path::PathBuf;

    fn record(path: &str, record_type: &str, content: &str) -> Record {
        Record::parse(PathBuf::from(path), record_type.to_string(), content)
    }

    fn record_with_prefix(path: &str, record_type: &str, prefix: &str, content: &str) -> Record {
        Record::parse_with_shape_and_prefix(
            PathBuf::from(path),
            record_type.to_string(),
            content,
            crate::header::HeaderShape::default(),
            prefix.to_string(),
        )
    }

    /// A minimal `Config` from `(type, RecordTypeConfig)` pairs -- for the
    /// capability-driven rules (`header_pointer_field_clean`,
    /// `narrative_field_stale`) that read `pointer_fields`/`narrative_fields`
    /// via `fields_with_capability` rather than a precomputed map (MILE-90).
    fn config_with_types(entries: Vec<(&str, crate::config::RecordTypeConfig)>) -> Config {
        Config {
            schema_version: 2,
            rules: HashMap::new(),
            record_types: entries
                .into_iter()
                .map(|(type_name, type_config)| (type_name.to_string(), type_config))
                .collect(),
        }
    }

    #[test]
    fn narrative_field_stale_judges_a_type_the_engine_has_never_heard_of() {
        // The compiled-in table knew five type names and returned "not
        // terminal" for every other, so the rule was inert for any corpus that
        // named its types differently -- which is every adopter (BUG-59).
        let blocked = record("docs/decisions/DEC-1-x.md", "dec", "> Blocked-on: DEC-2\n");
        let target = record("docs/decisions/DEC-2-y.md", "dec", "> Status: Ratified\n");
        let config = config_with_types(vec![(
            "dec",
            type_config_pointer(&["Status"], None, None, Some(&["Blocked-on"])),
        )]);

        let records = [blocked, target];
        let index = build_normalized_index(&records);
        let (exec, findings) =
            narrative_field_stale(&records, &config, &["Ratified".to_string()], &index);
        let population = exec
            .population
            .expect("the rule must state what it was handed");
        assert_eq!(population.unit(), PopulationUnit::Field);
        assert_eq!(population.eligible(), 2, "both records declare the slot");
        assert_eq!(
            population.examined(),
            1,
            "only DEC-1 wrote the slot, so only DEC-1 reaches a verdict"
        );
        assert_eq!(findings.len(), 1, "DEC-2 is terminal by declaration");
        assert!(findings[0].message.contains("DEC-2"));
    }

    #[test]
    fn header_field_set_consistency_does_not_count_an_unparsed_header() {
        // An unparsed header leaves an empty field list, which is
        // indistinguishable here from a record whose fields are all allowed.
        // Counting it inflates the signal ADR-55 makes load-bearing (BUG-78).
        let (r, _) = yaml_record(
            "docs/adr/ADR-1-x.md",
            "---\nStatus: [unclosed\n---\n# 1 — X\n",
        );
        let config = config_for_known(vec![("adr", vec!["status"])]);
        let (exec, _) = header_field_set_consistency(&[r], &config);
        let population = exec.population.expect("the rule carries a population");
        assert_eq!(
            (population.eligible(), population.examined()),
            (1, 0),
            "the record is in the population and the rule never saw its fields"
        );
    }

    fn terminal_for_tests() -> Vec<String> {
        [
            "Fixed",
            "WontFix",
            "Accepted",
            "Rejected",
            "Superseded",
            "Done",
            "WontDo",
        ]
        .iter()
        .map(|s| s.to_string())
        .collect()
    }

    #[test]
    fn missing_required_field_is_a_finding() {
        let r = record("docs/adr/0001-x.md", "adr", "> Status: Accepted\n");
        let config = config_for_required(vec![("adr", vec!["Status", "Deciders"])]);

        let (exec, findings) = header_required_fields(&[r], &config);
        assert_eq!(examined(&exec), 2, "both declared slots were read");
        assert_eq!(findings.len(), 1);
        assert!(findings[0].message.contains("Deciders"));
    }

    /// `BUG-98`/`ADR-50`: a type declaring `header_shape: none` has no
    /// header at all, by declaration -- a missing region there is the
    /// declared truth, not a defect to report.
    #[test]
    fn a_type_declaring_no_header_is_not_reported_as_missing_one_observed_failing() {
        let r = Record::parse_with_shape(
            PathBuf::from("docs/decisions/0001-x.md"),
            "dec".to_string(),
            "Just prose. No header block anywhere in this file.\n",
            crate::header::HeaderShape::None,
        );
        let config = config_with_types(vec![(
            "dec",
            crate::config::RecordTypeConfig {
                header_shape: crate::header::HeaderShape::None,
                ..type_config(None)
            },
        )]);

        let (exec, findings) = header_required_fields(&[r], &config);
        assert_eq!(
            examined(&exec),
            0,
            "a type with no header has nothing for this rule to examine"
        );
        assert!(
            findings.is_empty(),
            "a declared absence of a header must not be reported as a missing one: {findings:?}"
        );
    }

    #[test]
    fn a_broken_yaml_header_surfaces_the_real_parse_error_observed_failing() {
        // BUG-0012: before this fix, a header that failed to parse as YAML
        // produced only "no header-shaped region found" -- the real
        // yaml_serde error (line, reason) was discarded, forcing manual
        // byte-level diagnosis every time this was actually hit.
        let r = Record::parse_with_shape(
            PathBuf::from("docs/adr/ADR-1-x.md"),
            "adr".to_string(),
            "---\nSubject: `oops\nStatus: Draft\n---\n# Title\n",
            crate::header::HeaderShape::YamlFrontmatter,
        );
        let config = config_for_required(vec![("adr", vec!["Status"])]);

        let (exec, findings) = header_required_fields(&[r], &config);
        let population = exec.population.expect("the rule carries a population");
        assert_eq!(
            (population.eligible(), population.examined()),
            (1, 0),
            "the declared slot survives a header that did not parse, unjudged"
        );
        assert_eq!(findings.len(), 1);
        assert!(
            findings[0].message.contains("YAML parse error"),
            "expected the real parse error surfaced, got: {}",
            findings[0].message
        );
    }

    #[test]
    fn a_present_required_field_produces_no_finding() {
        let r = record(
            "docs/adr/0001-x.md",
            "adr",
            "> Status: Accepted\n> Deciders: someone\n",
        );
        let config = config_for_required(vec![("adr", vec!["Status", "Deciders"])]);

        let (exec, findings) = header_required_fields(&[r], &config);
        assert_eq!(examined(&exec), 2, "both declared slots were read");
        assert!(findings.is_empty(), "unexpected findings: {findings:?}");
    }

    #[test]
    fn a_planted_violation_is_observed_failing() {
        // The exact corpus defect this rule was chosen to catch: SPEC-0001's
        // shape lacks a field SPEC-0002-0005 share.
        let r = record(
            "docs/specs/0001-x.md",
            "spec",
            "> Status: Draft\n> Embodiment: Not started\n",
        );
        let config = config_for_required(vec![("spec", vec!["Version"])]);

        let (_, findings) = header_required_fields(&[r], &config);
        assert_eq!(findings.len(), 1);
        assert!(findings[0].message.contains("Version"));
    }

    #[test]
    fn a_declared_pipe_delimited_type_flags_a_one_per_line_record() {
        // The exact corpus defect this rule was chosen to catch: SPEC-0001
        // rendered one-field-per-line while SPEC-0002 through SPEC-0006
        // settled on pipe-delimited, and nothing said so.
        let r = record(
            "docs/specs/0001-x.md",
            "spec",
            "> Version: 0.1\n> Status: Draft\n",
        );
        let config = config_for_layout(vec![("spec", HeaderLayout::PipeDelimited)]);

        let (exec, findings) = header_layout_consistency(&[r], &config);
        assert_eq!(examined(&exec), 1);
        assert_eq!(findings.len(), 1);
        assert!(findings[0].message.contains("one-per-line"));
        assert!(findings[0].message.contains("pipe-delimited"));
    }

    #[test]
    fn a_matching_layout_produces_no_finding() {
        let r = record(
            "docs/specs/0002-x.md",
            "spec",
            "> Version: 0.1 | Status: Draft\n",
        );
        let config = config_for_layout(vec![("spec", HeaderLayout::PipeDelimited)]);

        let (exec, findings) = header_layout_consistency(&[r], &config);
        assert_eq!(examined(&exec), 1);
        assert!(findings.is_empty());
    }

    #[test]
    fn a_type_with_no_declared_layout_is_skipped_entirely() {
        let r = record("docs/adr/0001-x.md", "adr", "> Status: Accepted\n");
        let config = config_with_types(vec![]);

        let (exec, findings) = header_layout_consistency(&[r], &config);
        assert_eq!(examined(&exec), 0);
        assert!(findings.is_empty());
    }

    #[test]
    fn an_undeclared_field_is_a_finding() {
        // The exact corpus defect this rule was chosen to catch: SPEC-1
        // carries Embodiment/Author/Derives-from that SPEC-2 through SPEC-6
        // don't -- modeled here on the smaller adr case for a focused test.
        let r = record(
            "docs/adr/0001-x.md",
            "adr",
            "> Status: Accepted\n> Date: 2026-09-07\n> Sponsor: someone\n",
        );
        let config = config_for_known(vec![("adr", vec!["Status", "Date"])]);

        let (exec, findings) = header_field_set_consistency(&[r], &config);
        assert_eq!(examined(&exec), 1);
        assert_eq!(findings.len(), 1);
        assert!(findings[0].message.contains("Sponsor"));
    }

    #[test]
    fn a_known_field_produces_no_finding() {
        let r = record(
            "docs/adr/0001-x.md",
            "adr",
            "> Status: Accepted\n> Realized-by: code:x.rs\n",
        );
        let config = config_for_known(vec![("adr", vec!["Status", "Realized-by"])]);

        let (exec, findings) = header_field_set_consistency(&[r], &config);
        assert_eq!(examined(&exec), 1);
        assert!(findings.is_empty());
    }

    #[test]
    fn a_type_with_no_declared_known_fields_is_skipped_entirely() {
        let r = record("docs/specs/0001-x.md", "spec", "> Version: 0.1\n");
        let config = config_with_types(vec![]);

        let (exec, findings) = header_field_set_consistency(&[r], &config);
        assert_eq!(examined(&exec), 0);
        assert!(findings.is_empty());
    }

    fn type_config(spec: Option<&str>) -> crate::config::RecordTypeConfig {
        crate::config::RecordTypeConfig {
            dir: "docs/x".to_string(),
            required_fields: Vec::new(),
            header_shape: crate::header::HeaderShape::default(),
            prefix: None,
            header_layout: None,
            known_fields: None,
            pointer_fields: None,
            narrative_fields: None,
            spec: spec.map(|s| s.to_string()),
            relation_fields: None,
        }
    }

    /// A `Config` declaring `required_fields` per type (`ADR-59`), for the
    /// rules that used to take a bare `required_by_type`/`declared_by_type`
    /// map directly.
    fn config_for_required(entries: Vec<(&str, Vec<&str>)>) -> Config {
        config_with_types(
            entries
                .into_iter()
                .map(|(name, fields)| {
                    (
                        name,
                        crate::config::RecordTypeConfig {
                            required_fields: fields.into_iter().map(String::from).collect(),
                            ..type_config(None)
                        },
                    )
                })
                .collect(),
        )
    }

    /// A `Config` declaring `known_fields` per type (`ADR-59`), for
    /// `header_field_set_consistency`'s `allowed_by_type`.
    fn config_for_known(entries: Vec<(&str, Vec<&str>)>) -> Config {
        config_with_types(
            entries
                .into_iter()
                .map(|(name, fields)| {
                    (
                        name,
                        crate::config::RecordTypeConfig {
                            known_fields: Some(fields.into_iter().map(String::from).collect()),
                            ..type_config(None)
                        },
                    )
                })
                .collect(),
        )
    }

    /// A `Config` declaring `header_layout` per type (`ADR-59`).
    fn config_for_layout(entries: Vec<(&str, crate::header::HeaderLayout)>) -> Config {
        config_with_types(
            entries
                .into_iter()
                .map(|(name, layout)| {
                    (
                        name,
                        crate::config::RecordTypeConfig {
                            header_layout: Some(layout),
                            ..type_config(None)
                        },
                    )
                })
                .collect(),
        )
    }

    fn type_config_with_shape(
        shape: crate::header::HeaderShape,
    ) -> crate::config::RecordTypeConfig {
        crate::config::RecordTypeConfig {
            dir: "docs/x".to_string(),
            required_fields: Vec::new(),
            header_shape: shape,
            prefix: None,
            header_layout: None,
            known_fields: None,
            pointer_fields: None,
            narrative_fields: None,
            spec: None,
            relation_fields: None,
        }
    }

    /// Full builder for the config-level pointer/narrative-field validation
    /// tests (MILE-90) -- the two helpers above default both new fields to
    /// `None`, which isn't useful for testing them directly.
    #[test]
    fn a_slot_behind_an_unparsed_header_is_not_judged_and_not_reported_on() {
        // The record plainly sets `Status`, and the header does not parse, so
        // the rule cannot read either slot. Calling both Blank reports
        // something untrue about the file, and pinning `examined` to `eligible`
        // makes the rule invisible to the one check ADR-55 asks for: a rule
        // whose population can never show a gap can never be caught not
        // looking. `header.required-fields` reports the parse error itself.
        let r = record(
            "docs/adr/ADR-1-x.md",
            "adr",
            "---\nStatus: Accepted\n  Date  :: nope\n---\n",
        );
        let config = config_for_required(vec![("adr", vec!["Status", "Date"])]);

        let (exec, findings) = field_quality(&[r.clone()], &config);
        let population = exec.population.expect("the rule carries a population");
        assert_eq!(
            (population.eligible(), population.examined()),
            (2, 0),
            "both declared slots are unreadable, not judged"
        );
        assert!(
            findings.is_empty(),
            "no finding about a slot the rule could not read: {findings:?}"
        );

        let (exec, findings) = field_pending(&[r], &config);
        let population = exec.population.expect("the rule carries a population");
        assert_eq!((population.eligible(), population.examined()), (2, 0));
        assert!(findings.is_empty(), "{findings:?}");
    }

    /// What the rule judged, per its own census. A planted test asserts the
    /// rule *looked* -- a finding's absence alone is the ADR-55 defect.
    fn examined(exec: &RuleExecution) -> usize {
        exec.population
            .as_ref()
            .expect("every rule carries a population")
            .examined()
    }

    fn unreadable(exec: &RuleExecution) -> usize {
        exec.population
            .as_ref()
            .expect("every rule carries a population")
            .unreadable()
    }

    #[test]
    fn a_slot_declared_only_as_required_is_still_a_declared_slot() {
        // `known_fields` is documented as the fields a type carries *beyond*
        // `required_fields`, so a type that requires `Realized-by` declares it
        // just as much as one that lists it as known. Selecting candidates from
        // `known_fields` alone handed these rules nothing, and a rule handed
        // nothing reports a clean `eligible: 0` -- the ADR-55 shape.
        let r = record(
            "docs/adr/ADR-1-x.md",
            "adr",
            "> Embodiment: Verified\n> Realized-by: code:src/lib.rs\n",
        );
        let config = config_with_types(vec![(
            "adr",
            type_config_pointer(&["Embodiment", "Realized-by"], None, None, None),
        )]);

        let (exec, findings) = embodiment_consistency(&[r], &config, &HashSet::new());
        let population = exec
            .population
            .expect("the rule must state what it was handed");
        assert_eq!(
            (population.eligible(), population.examined()),
            (1, 1),
            "the pair is declared, so the record is a candidate the rule judged"
        );
        assert_eq!(findings.len(), 1, "Verified disagrees with Implemented");
    }

    #[test]
    fn a_supersession_slot_declared_only_as_required_is_still_declared() {
        let old = record(
            "docs/adr/ADR-1-x.md",
            "adr",
            "> Supersedes / Superseded-by: —\n",
        );
        let new = record(
            "docs/adr/ADR-2-y.md",
            "adr",
            "> Supersedes / Superseded-by: ADR-0001\n",
        );
        let config = config_with_types(vec![(
            "adr",
            type_config_pointer(&["Supersedes / Superseded-by"], None, None, None),
        )]);

        let records = [old, new];
        let index = build_normalized_index(&records);
        let (exec, findings) = supersession_reciprocity(&records, &config, &index);
        let population = exec
            .population
            .expect("the rule must state what it was handed");
        assert_eq!(population.eligible(), 2, "both records declare the slot");
        assert_eq!(findings.len(), 1, "the claim is one-directional");
    }

    fn embodiment_config() -> Config {
        config_with_types(vec![(
            "adr",
            type_config_pointer(&[], Some(&["Embodiment", "Realized-by"]), None, None),
        )])
    }

    fn type_config_pointer(
        required_fields: &[&str],
        known_fields: Option<&[&str]>,
        pointer_fields: Option<&[&str]>,
        narrative_fields: Option<&[&str]>,
    ) -> crate::config::RecordTypeConfig {
        crate::config::RecordTypeConfig {
            dir: "docs/x".to_string(),
            required_fields: required_fields.iter().map(|s| s.to_string()).collect(),
            header_shape: crate::header::HeaderShape::default(),
            prefix: None,
            header_layout: None,
            known_fields: known_fields.map(|f| f.iter().map(|s| s.to_string()).collect()),
            pointer_fields: pointer_fields.map(|f| f.iter().map(|s| s.to_string()).collect()),
            narrative_fields: narrative_fields.map(|f| f.iter().map(|s| s.to_string()).collect()),
            spec: None,
            relation_fields: None,
        }
    }

    #[test]
    fn a_type_with_no_declared_spec_is_a_finding_observed_failing() {
        // The exact live case this rule was built to catch: before this
        // session declared any `spec` pointer, every configured type
        // (including milestone/bug/waiver, which already had SPEC-6/9/10)
        // had none wired into config at all.
        let mut record_types = HashMap::new();
        record_types.insert("milestone".to_string(), type_config(None));
        let config = Config {
            schema_version: 2,
            rules: HashMap::new(),
            record_types,
        };

        let (exec, findings) =
            type_no_declared_spec(&config, std::path::Path::new(".urzua/config.yaml"));
        assert_eq!(examined(&exec), 1);
        assert_eq!(findings.len(), 1);
        assert!(findings[0].message.contains("milestone"));
    }

    #[test]
    fn a_declared_spec_pointer_produces_no_finding() {
        let mut record_types = HashMap::new();
        record_types.insert("milestone".to_string(), type_config(Some("SPEC-6")));
        let config = Config {
            schema_version: 2,
            rules: HashMap::new(),
            record_types,
        };

        let (exec, findings) =
            type_no_declared_spec(&config, std::path::Path::new(".urzua/config.yaml"));
        assert_eq!(examined(&exec), 1);
        assert!(findings.is_empty());
    }

    #[test]
    fn a_type_still_declaring_blockquote_is_a_finding() {
        let mut record_types = HashMap::new();
        record_types.insert(
            "adr".to_string(),
            type_config_with_shape(crate::header::HeaderShape::Blockquote),
        );
        let config = Config {
            schema_version: 2,
            rules: HashMap::new(),
            record_types,
        };

        let (exec, findings) =
            header_deprecated_shape(&config, std::path::Path::new(".urzua/config.yaml"));
        assert_eq!(examined(&exec), 1);
        assert_eq!(findings.len(), 1);
        assert!(findings[0].message.contains("adr"));
    }

    #[test]
    fn a_type_declaring_yaml_frontmatter_produces_no_finding() {
        let mut record_types = HashMap::new();
        record_types.insert(
            "adr".to_string(),
            type_config_with_shape(crate::header::HeaderShape::YamlFrontmatter),
        );
        let config = Config {
            schema_version: 2,
            rules: HashMap::new(),
            record_types,
        };

        let (exec, findings) =
            header_deprecated_shape(&config, std::path::Path::new(".urzua/config.yaml"));
        assert_eq!(examined(&exec), 1);
        assert!(findings.is_empty());
    }

    #[test]
    fn a_type_declaring_only_pointer_fields_is_a_missing_declaration_observed_failing() {
        let config = config_with_types(vec![(
            "spec",
            type_config_pointer(&["Implements"], None, Some(&["Implements"]), None),
        )]);
        let (exec, findings) =
            config_pointer_declaration_missing(&config, std::path::Path::new(".urzua/config.yaml"));
        assert_eq!(examined(&exec), 1);
        assert_eq!(findings.len(), 1);
        assert!(findings[0].message.contains("spec"));
    }

    #[test]
    fn a_type_declaring_both_lists_including_an_explicit_empty_one_is_not_missing() {
        let config = config_with_types(vec![(
            "waiver",
            type_config_pointer(&[], None, Some(&[]), Some(&[])),
        )]);
        let (exec, findings) =
            config_pointer_declaration_missing(&config, std::path::Path::new(".urzua/config.yaml"));
        assert_eq!(examined(&exec), 1);
        assert!(findings.is_empty());
    }

    #[test]
    fn a_type_with_no_known_fields_declared_is_a_missing_declaration_observed_failing() {
        let config = config_with_types(vec![(
            "spec",
            type_config_pointer(&["Status"], None, None, None),
        )]);
        let (exec, findings) = config_known_fields_declaration_missing(
            &config,
            std::path::Path::new(".urzua/config.yaml"),
        );
        assert_eq!(examined(&exec), 1);
        assert_eq!(findings.len(), 1, "{findings:?}");
        assert!(findings[0].message.contains("spec"));
    }

    #[test]
    fn a_header_none_type_with_no_known_fields_is_not_a_missing_declaration_observed_failing() {
        let config = config_with_types(vec![(
            "waiver",
            type_config_with_shape(crate::header::HeaderShape::None),
        )]);
        let (exec, findings) = config_known_fields_declaration_missing(
            &config,
            std::path::Path::new(".urzua/config.yaml"),
        );
        assert_eq!(examined(&exec), 1);
        assert!(
            findings.is_empty(),
            "a type with no header has nowhere for known_fields to govern: {findings:?}"
        );
    }

    #[test]
    fn a_type_declaring_known_fields_as_an_explicit_empty_list_is_not_missing() {
        let config = config_with_types(vec![(
            "waiver",
            type_config_pointer(&[], Some(&[]), None, None),
        )]);
        let (exec, findings) = config_known_fields_declaration_missing(
            &config,
            std::path::Path::new(".urzua/config.yaml"),
        );
        assert_eq!(examined(&exec), 1);
        assert!(findings.is_empty(), "{findings:?}");
    }

    fn exec_with(
        rule: &str,
        eligible: usize,
        examined: usize,
        out_of_scope: usize,
    ) -> RuleExecution {
        RuleExecution {
            rule: rule.to_string(),
            population: Some(Population::detailed(
                PopulationUnit::Record,
                eligible,
                examined,
                out_of_scope,
                0,
            )),
            status: RuleStatus::Ran,
            examined_records: Vec::new(),
        }
    }

    #[test]
    fn a_scope_reaching_nothing_is_reported_observed_failing() {
        // BUG-61's shape: init wrote a prefix matching no filename, so the rule
        // examines zero records forever and no edit to a record changes that.
        let executed = [exec_with("identity.collision", 2, 0, 2)];
        let (exec, findings) =
            config_scope_matches_nothing(&executed, std::path::Path::new(".urzua/config.yaml"));
        assert_eq!(examined(&exec), 1, "the rule must judge the execution");
        assert_eq!(findings.len(), 1, "{findings:?}");
        assert!(findings[0].message.contains("identity.collision"));
    }

    #[test]
    fn the_finding_names_the_configuration_the_caller_selected() {
        // check's scoped filter keeps a config finding only when its path equals
        // the selected config, so a hardcoded path makes this finding vanish
        // under `--config` -- a finding that disappears rather than one that is
        // wrong.
        let executed = [exec_with("identity.collision", 2, 0, 2)];
        let chosen = std::path::Path::new("custom/urzua.yaml");
        let (_, findings) = config_scope_matches_nothing(&executed, chosen);
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].file, chosen);
    }

    #[test]
    fn a_rule_with_nothing_written_yet_is_not_reported() {
        // Identical eligible/examined to the case above. The corpus simply has
        // no revision logs, which time resolves and config cannot.
        let executed = [exec_with("revision-log.change-class-required", 2, 0, 0)];
        let (exec, findings) =
            config_scope_matches_nothing(&executed, std::path::Path::new(".urzua/config.yaml"));
        assert_eq!(examined(&exec), 1);
        assert!(
            findings.is_empty(),
            "an empty corpus is not a misconfiguration: {findings:?}"
        );
    }

    #[test]
    fn it_does_not_judge_its_own_execution() {
        let executed = [exec_with(RULE_CONFIG_SCOPE_MATCHES_NOTHING, 3, 0, 3)];
        let (exec, findings) =
            config_scope_matches_nothing(&executed, std::path::Path::new(".urzua/config.yaml"));
        assert_eq!(
            exec.population.expect("carries a population").eligible(),
            0,
            "its own execution is not a candidate"
        );
        assert!(findings.is_empty(), "{findings:?}");
    }

    #[test]
    fn a_rule_that_did_not_run_is_not_a_candidate() {
        let executed = [RuleExecution {
            rule: "field.quality".to_string(),
            population: None,
            status: RuleStatus::NotEnabled,
            examined_records: Vec::new(),
        }];
        let (exec, findings) =
            config_scope_matches_nothing(&executed, std::path::Path::new(".urzua/config.yaml"));
        assert_eq!(exec.population.expect("carries a population").eligible(), 0);
        assert!(findings.is_empty());
    }

    /// `ADR-50`: `none` is a different axis, not a deprecated shape.
    #[test]
    fn a_type_declaring_no_header_is_not_reported_as_deprecated() {
        let config = config_with_types(vec![(
            "dec",
            crate::config::RecordTypeConfig {
                header_shape: crate::header::HeaderShape::None,
                ..type_config(None)
            },
        )]);
        let (exec, findings) =
            header_deprecated_shape(&config, std::path::Path::new(".urzua/config.yaml"));
        assert_eq!(examined(&exec), 1);
        assert!(findings.is_empty(), "{findings:?}");
    }

    #[test]
    fn a_blockquote_type_is_still_reported_as_deprecated_observed_failing() {
        let config = config_with_types(vec![("adr", type_config(None))]);
        let (_, findings) =
            header_deprecated_shape(&config, std::path::Path::new(".urzua/config.yaml"));
        assert_eq!(findings.len(), 1, "{findings:?}");
    }

    /// `ADR-50`: a type with no header has nowhere for a required field to be.
    #[test]
    fn a_header_none_type_with_required_fields_is_a_contradiction_observed_failing() {
        let config = config_with_types(vec![(
            "dec",
            crate::config::RecordTypeConfig {
                header_shape: crate::header::HeaderShape::None,
                required_fields: vec!["Status".to_string()],
                ..type_config(None)
            },
        )]);
        let (exec, findings) = config_header_none_has_no_required_fields(
            &config,
            std::path::Path::new(".urzua/config.yaml"),
        );
        assert_eq!(examined(&exec), 1);
        assert_eq!(findings.len(), 1, "{findings:?}");
        assert!(findings[0].message.contains("Status"));
    }

    #[test]
    fn a_header_none_type_with_no_required_fields_is_not_a_contradiction() {
        let config = config_with_types(vec![(
            "dec",
            crate::config::RecordTypeConfig {
                header_shape: crate::header::HeaderShape::None,
                ..type_config(None)
            },
        )]);
        let (_, findings) = config_header_none_has_no_required_fields(
            &config,
            std::path::Path::new(".urzua/config.yaml"),
        );
        assert!(findings.is_empty(), "{findings:?}");
    }

    #[test]
    fn a_type_declaring_neither_list_is_not_missing() {
        let config = config_with_types(vec![("adr", type_config_pointer(&[], None, None, None))]);
        let (exec, findings) =
            config_pointer_declaration_missing(&config, std::path::Path::new(".urzua/config.yaml"));
        assert_eq!(examined(&exec), 1);
        assert!(findings.is_empty());
    }

    #[test]
    fn the_case_only_hint_names_the_same_declaration_every_run() {
        // A fresh `HashSet` each time: iteration order is randomised per
        // instance but fixed within one, so reusing a single set would pass
        // against an order-dependent implementation.
        for _ in 0..40 {
            let allowed: HashSet<crate::values::FieldName> =
                ["Blocked-on", "Blocked-On", "BLOCKED-on"]
                    .iter()
                    .map(|s| crate::values::FieldName::from(*s))
                    .collect();
            assert_eq!(
                near_miss(&crate::values::FieldName::from("blocked-on"), &allowed)
                    .map(|f| f.as_str()),
                Some("BLOCKED-on"),
                "the hint must not depend on which spelling the set yields first"
            );
        }
    }

    #[test]
    fn a_pointer_field_differing_only_in_case_is_absent_from_known_fields() {
        // Accepted here, the field is unreadable everywhere else: the record
        // writes `Derives-from` and `header.pointer-field-clean` looks up
        // `Derives-From`, which ADR-57 makes a different name.
        let config = config_with_types(vec![(
            "adr",
            type_config_pointer(
                &["Status"],
                Some(&["Derives-from"]),
                Some(&["Derives-From"]),
                Some(&[]),
            ),
        )]);
        let (exec, findings) =
            config_pointer_field_not_known(&config, std::path::Path::new(".urzua/config.yaml"));
        assert_eq!(examined(&exec), 1);
        assert_eq!(findings.len(), 1, "{findings:?}");
        assert!(findings[0].message.contains("Derives-From"));
    }

    #[test]
    fn a_relation_field_absent_from_known_fields_is_a_finding_observed_failing() {
        let config = config_with_types(vec![(
            "rfc",
            crate::config::RecordTypeConfig {
                relation_fields: Some(crate::config::RelationFields {
                    status: Some("State".to_string()),
                    ..Default::default()
                }),
                ..type_config_pointer(&["Date"], None, None, None)
            },
        )]);
        let (exec, findings) =
            config_relation_field_not_known(&config, std::path::Path::new(".urzua/config.yaml"));
        assert_eq!(examined(&exec), 1);
        assert_eq!(findings.len(), 1, "{findings:?}");
        assert!(findings[0].message.contains("State"));
    }

    #[test]
    fn a_relation_field_present_in_known_fields_is_not_a_finding() {
        let config = config_with_types(vec![(
            "rfc",
            crate::config::RecordTypeConfig {
                relation_fields: Some(crate::config::RelationFields {
                    status: Some("State".to_string()),
                    ..Default::default()
                }),
                ..type_config_pointer(&["State"], None, None, None)
            },
        )]);
        let (exec, findings) =
            config_relation_field_not_known(&config, std::path::Path::new(".urzua/config.yaml"));
        assert_eq!(examined(&exec), 1);
        assert!(findings.is_empty(), "{findings:?}");
    }

    #[test]
    fn a_type_with_no_declared_relation_fields_is_not_a_finding() {
        let config = config_with_types(vec![(
            "rfc",
            type_config_pointer(&["Status"], None, None, None),
        )]);
        let (exec, findings) =
            config_relation_field_not_known(&config, std::path::Path::new(".urzua/config.yaml"));
        assert_eq!(examined(&exec), 1);
        assert!(findings.is_empty());
    }

    #[test]
    fn pointer_and_narrative_names_differing_only_in_case_do_not_overlap() {
        let config = config_with_types(vec![(
            "adr",
            type_config_pointer(
                &[],
                Some(&["Blocked-on", "Blocked-On"]),
                Some(&["Blocked-on"]),
                Some(&["Blocked-On"]),
            ),
        )]);
        let (exec, findings) =
            config_pointer_narrative_overlap(&config, std::path::Path::new(".urzua/config.yaml"));
        assert_eq!(examined(&exec), 1);
        assert!(
            findings.is_empty(),
            "two different names are two fields (ADR-57): {findings:?}"
        );
    }

    #[test]
    fn a_pointer_field_absent_from_known_fields_is_a_finding_observed_failing() {
        let config = config_with_types(vec![(
            "spec",
            type_config_pointer(&["Status"], None, Some(&["Feeds-into"]), Some(&[])),
        )]);
        let (exec, findings) =
            config_pointer_field_not_known(&config, std::path::Path::new(".urzua/config.yaml"));
        assert_eq!(examined(&exec), 1);
        assert_eq!(findings.len(), 1);
        assert!(findings[0].message.contains("Feeds-into"));
    }

    #[test]
    fn a_pointer_field_present_in_known_fields_is_not_a_finding() {
        let config = config_with_types(vec![(
            "spec",
            type_config_pointer(
                &["Status"],
                Some(&["Feeds-into"]),
                Some(&["Feeds-into"]),
                Some(&[]),
            ),
        )]);
        let (exec, findings) =
            config_pointer_field_not_known(&config, std::path::Path::new(".urzua/config.yaml"));
        assert_eq!(examined(&exec), 1);
        assert!(findings.is_empty());
    }

    #[test]
    fn a_field_present_in_both_pointer_and_narrative_lists_is_an_overlap_observed_failing() {
        let config = config_with_types(vec![(
            "milestone",
            type_config_pointer(
                &["Blocked-on"],
                None,
                Some(&["Blocked-on"]),
                Some(&["Blocked-on"]),
            ),
        )]);
        let (exec, findings) =
            config_pointer_narrative_overlap(&config, std::path::Path::new(".urzua/config.yaml"));
        assert_eq!(examined(&exec), 1);
        assert_eq!(findings.len(), 1);
        assert!(findings[0].message.contains("Blocked-on"));
    }

    #[test]
    fn disjoint_pointer_and_narrative_lists_are_not_an_overlap() {
        let config = config_with_types(vec![(
            "milestone",
            type_config_pointer(
                &["Implements", "Blocked-on"],
                None,
                Some(&["Implements"]),
                Some(&["Blocked-on"]),
            ),
        )]);
        let (exec, findings) =
            config_pointer_narrative_overlap(&config, std::path::Path::new(".urzua/config.yaml"));
        assert_eq!(examined(&exec), 1);
        assert!(findings.is_empty());
    }

    #[test]
    fn target_status_judges_only_the_statuses_a_repository_declared() {
        // Replaces a_resolving_pointer_surfaces_target_status_without_judging_it,
        // whose name was the defect: surfacing a status without judging it
        // produced 172 of 213 findings on this repo's own corpus.
        let target = record("docs/rfc/RFC-1-x.md", "rfc", "> Status: Draft\n");
        let source = record("docs/specs/SPEC-1-x.md", "spec", "> Implements: RFC-0001\n");
        let config = config_with_types(vec![
            (
                "spec",
                type_config_pointer(&[], None, Some(&["Implements"]), Some(&[])),
            ),
            ("rfc", type_config_pointer(&["Status"], None, None, None)),
        ]);
        let records = [target, source];
        let index = build_normalized_index(&records);

        let (exec, findings) =
            pointer_target_status(&records, &config, &["Draft".to_string()], &index);
        assert_eq!(examined(&exec), 1);
        assert_eq!(findings.len(), 1);
        assert!(findings[0].message.contains("Draft"));

        // Nothing declared unacceptable means nothing reported -- never a
        // built-in fallback list, which is how a rule silently stops applying.
        let (_, none_declared) = pointer_target_status(&records, &config, &[], &index);
        assert!(none_declared.is_empty(), "{none_declared:?}");

        // A status outside the declared set is not this rule's business.
        let (_, other) =
            pointer_target_status(&records, &config, &["Superseded".to_string()], &index);
        assert!(other.is_empty(), "{other:?}");
    }

    #[test]
    fn a_type_declaring_a_custom_status_field_name_is_read_by_that_name_observed_failing() {
        let target = record("docs/rfc/RFC-1-x.md", "rfc", "> State: Draft\n");
        let source = record("docs/specs/SPEC-1-x.md", "spec", "> Implements: RFC-0001\n");
        let config = config_with_types(vec![
            (
                "spec",
                type_config_pointer(&[], None, Some(&["Implements"]), Some(&[])),
            ),
            (
                "rfc",
                crate::config::RecordTypeConfig {
                    relation_fields: Some(crate::config::RelationFields {
                        status: Some("State".to_string()),
                        ..Default::default()
                    }),
                    ..type_config_pointer(&["State"], None, None, None)
                },
            ),
        ]);
        let records = [target, source];
        let index = build_normalized_index(&records);

        let (exec, findings) =
            pointer_target_status(&records, &config, &["Draft".to_string()], &index);
        assert_eq!(examined(&exec), 1);
        assert_eq!(findings.len(), 1, "{findings:?}");
        assert!(findings[0].message.contains("Draft"));
    }

    #[test]
    fn a_non_resolving_pointer_is_an_error() {
        let source = record("docs/specs/SPEC-1-x.md", "spec", "> Implements: RFC-9999\n");
        let config = config_with_types(vec![(
            "spec",
            type_config_pointer(&[], None, Some(&["Implements"]), Some(&[])),
        )]);
        let records = [source];
        let index = build_normalized_index(&records);
        let (_, findings) = pointer_resolution(&records, &config, &index);
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].severity, FindingSeverity::Error);
    }

    #[test]
    fn a_parent_pointer_resolves_the_same_as_implements() {
        // Every spec's `Parent: SPEC-N` pointer was completely unchecked
        // before Parent was added to this rule's scanned fields -- a typo'd
        // or dangling Parent would never have been caught.
        let parent = record("docs/specs/SPEC-1-cli.md", "spec", "> Status: Draft\n");
        let child = record(
            "docs/specs/SPEC-2-urzua-check.md",
            "spec",
            "> Parent: SPEC-1 (v0 CLI). Extra trailing prose that isn't a reference.\n",
        );
        let config = config_with_types(vec![(
            "spec",
            type_config_pointer(&[], None, Some(&["Parent"]), Some(&[])),
        )]);
        let records = [parent, child];
        let index = build_normalized_index(&records);
        let (_, findings) = pointer_resolution(&records, &config, &index);
        assert!(findings.is_empty(), "{findings:?}");
    }

    #[test]
    fn a_dangling_parent_pointer_is_an_error() {
        let child = record("docs/specs/SPEC-2-x.md", "spec", "> Parent: SPEC-9999\n");
        let config = config_with_types(vec![(
            "spec",
            type_config_pointer(&[], None, Some(&["Parent"]), Some(&[])),
        )]);
        let records = [child];
        let index = build_normalized_index(&records);
        let (_, findings) = pointer_resolution(&records, &config, &index);
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].severity, FindingSeverity::Error);
        assert!(findings[0].message.contains("Parent: SPEC-9999"));
    }

    #[test]
    fn a_derives_from_status_annotation_is_flagged_observed_failing() {
        // The exact live pattern found across 33 files: a hand-typed status
        // snapshot baked into the field value itself, redundant with
        // pointer.resolution's own live report and never re-verified.
        let r = record(
            "docs/adr/ADR-38-x.md",
            "adr",
            "> Status: Accepted\n> Derives-from: RFC-1 (Accepted)\n",
        );
        let config = config_with_types(vec![(
            "adr",
            type_config_pointer(&[], None, Some(&["Derives-from"]), None),
        )]);
        let (exec, findings) = header_pointer_field_clean(&[r], &config);
        assert_eq!(examined(&exec), 1);
        assert_eq!(findings.len(), 1);
        assert!(findings[0].message.contains("RFC-1 (Accepted)"));
        assert_eq!(findings[0].severity, FindingSeverity::Warning);
    }

    #[test]
    fn a_parent_field_with_freeform_trailing_prose_is_flagged() {
        let r = record(
            "docs/specs/SPEC-2-x.md",
            "spec",
            "> Status: Draft\n> Parent: SPEC-1 (v0 CLI). Cross-cutting rules stated there.\n",
        );
        let config = config_with_types(vec![(
            "spec",
            type_config_pointer(&[], None, Some(&["Parent"]), None),
        )]);
        let (_, findings) = header_pointer_field_clean(&[r], &config);
        assert_eq!(findings.len(), 1);
    }

    #[test]
    fn a_value_with_trailing_whitespace_is_reported_observed_failing() {
        let r = Record::parse_with_shape_and_prefix(
            PathBuf::from("docs/adr/ADR-1-x.md"),
            "adr".to_string(),
            "---\nStatus: \"Superseded   \"\n---\n# 1 — X\n",
            crate::header::HeaderShape::YamlFrontmatter,
            "ADR".to_string(),
        );
        let config = config_for_required(vec![("adr", vec!["Status"])]);

        let (exec, findings) = field_untrimmed_value(&[r], &config);
        assert_eq!(examined(&exec), 1, "the slot is written, so it is judged");
        assert_eq!(findings.len(), 1, "{findings:?}");
        assert!(findings[0].message.contains("Superseded"));
    }

    #[test]
    fn a_clean_value_is_not_reported_as_untrimmed() {
        let r = record("docs/adr/ADR-1-x.md", "adr", "> Status: Superseded\n");
        let config = config_for_required(vec![("adr", vec!["Status"])]);
        let (exec, findings) = field_untrimmed_value(&[r], &config);
        assert_eq!(examined(&exec), 1);
        assert!(findings.is_empty(), "{findings:?}");
    }

    /// BUG-101: a declared field written under a different case used to be
    /// silently absent everywhere. This rule is the one place that names it.
    #[test]
    fn a_declared_field_written_under_a_different_case_is_reported_observed_failing() {
        let r = record("docs/adr/ADR-1-x.md", "adr", "> status: Accepted\n");
        let config = config_for_required(vec![("adr", vec!["Status"])]);
        let (exec, findings) = header_field_case_mismatch(&[r], &config);
        assert_eq!(examined(&exec), 1);
        assert_eq!(findings.len(), 1, "{findings:?}");
        assert!(findings[0].message.contains("'status'"), "{findings:?}");
        assert_eq!(findings[0].severity, FindingSeverity::Error);
    }

    #[test]
    fn a_declared_field_written_correctly_cased_is_not_reported() {
        let r = record("docs/adr/ADR-1-x.md", "adr", "> Status: Accepted\n");
        let config = config_for_required(vec![("adr", vec!["Status"])]);
        let (exec, findings) = header_field_case_mismatch(&[r], &config);
        assert_eq!(examined(&exec), 1);
        assert!(findings.is_empty(), "{findings:?}");
    }

    #[test]
    fn a_declared_field_never_written_at_all_is_absent_not_reported() {
        let r = record("docs/adr/ADR-1-x.md", "adr", "> Author: x\n");
        let config = config_for_required(vec![("adr", vec!["Status"])]);
        let (exec, findings) = header_field_case_mismatch(&[r], &config);
        assert_eq!(
            examined(&exec),
            0,
            "genuine non-adoption is Absent, not Examined"
        );
        assert!(findings.is_empty(), "{findings:?}");
    }

    #[test]
    fn a_status_carrying_whitespace_does_not_silently_match_and_is_reported() {
        // ADR-57 compares exactly, so "Superseded   " is not Superseded and the
        // status rule is right to stay silent. Trimming inside it would paper
        // over a value the author did not write; field.untrimmed-value reports
        // the whitespace instead, which is where the adopter can act on it.
        let yaml = |path: &str, content: &str| {
            Record::parse_with_shape_and_prefix(
                PathBuf::from(path),
                "adr".to_string(),
                content,
                crate::header::HeaderShape::YamlFrontmatter,
                "ADR".to_string(),
            )
        };
        let target = yaml(
            "docs/adr/ADR-1-x.md",
            "---\nStatus: \"Superseded   \"\n---\n# 1 — X\n",
        );
        let source = yaml(
            "docs/adr/ADR-2-y.md",
            "---\nDerives-from: ADR-1\n---\n# 2 — Y\n",
        );
        let config = config_with_types(vec![(
            "adr",
            type_config_pointer(&["Status"], None, Some(&["Derives-from"]), Some(&[])),
        )]);

        let target_status_records = [target.clone(), source];
        let (_, status_findings) = pointer_target_status(
            &target_status_records,
            &config,
            &["Superseded".to_string()],
            &build_normalized_index(&target_status_records),
        );
        assert!(
            status_findings.is_empty(),
            "the value is not that status: {status_findings:?}"
        );

        let (_, whitespace) = field_untrimmed_value(&[target], &config);
        assert_eq!(
            whitespace.len(),
            1,
            "and the whitespace must be reported, or nothing tells the author"
        );
    }

    #[test]
    fn a_clean_bare_reference_is_not_flagged() {
        let r = record(
            "docs/adr/ADR-10-x.md",
            "adr",
            "> Status: Accepted\n> Derives-from: RFC-1\n> Parent: —\n",
        );
        let config = config_with_types(vec![(
            "adr",
            type_config_pointer(&[], None, Some(&["Derives-from", "Parent"]), None),
        )]);
        let (exec, findings) = header_pointer_field_clean(&[r], &config);
        // One record; two declared clean-format slots, both written.
        let population = exec.population.expect("the rule carries a population");
        assert_eq!(population.unit(), PopulationUnit::Field);
        assert_eq!(population.eligible(), 2);
        assert_eq!(population.examined(), 2);
        assert!(findings.is_empty());
    }

    #[test]
    fn multiple_clean_references_are_not_flagged() {
        let r = record(
            "docs/adr/ADR-8-x.md",
            "adr",
            "> Status: Accepted\n> Derives-from: RFC-8, ADR-15, ADR-19\n",
        );
        let config = config_with_types(vec![(
            "adr",
            type_config_pointer(&[], None, Some(&["Derives-from"]), None),
        )]);
        let (_, findings) = header_pointer_field_clean(&[r], &config);
        assert!(findings.is_empty());
    }

    #[test]
    fn an_empty_entry_between_commas_is_flagged_not_silently_skipped() {
        // A trailing/double comma (`RFC-1,,ADR-2`) produces an empty
        // split segment -- that must be flagged, not treated as a second
        // "no value" placeholder alongside the real one, "—".
        let r = record(
            "docs/adr/ADR-11-x.md",
            "adr",
            "> Status: Accepted\n> Derives-from: RFC-1,,ADR-2\n",
        );
        let config = config_with_types(vec![(
            "adr",
            type_config_pointer(&[], None, Some(&["Derives-from"]), None),
        )]);
        let (_, findings) = header_pointer_field_clean(&[r], &config);
        assert_eq!(findings.len(), 1);
    }

    #[test]
    fn a_bare_placeholder_field_is_still_not_flagged() {
        let r = record(
            "docs/adr/ADR-12-x.md",
            "adr",
            "> Status: Accepted\n> Parent: —\n",
        );
        let config = config_with_types(vec![(
            "adr",
            type_config_pointer(&[], None, Some(&["Parent"]), None),
        )]);
        let (_, findings) = header_pointer_field_clean(&[r], &config);
        assert!(findings.is_empty());
    }

    #[test]
    fn narrative_fields_are_excluded_even_with_trailing_prose() {
        // Blocked-on deliberately mixes free text with an optional embedded
        // reference (SPEC-6) -- must never be flagged by this rule, since
        // it's declared narrative, not pointer (MILE-90).
        let r = record(
            "docs/milestones/MILE-38-x.md",
            "milestone",
            "> Status: Planned\n> Blocked-on: MILE-38 (staleness detection) -- sequenced first.\n",
        );
        let config = config_with_types(vec![(
            "milestone",
            type_config_pointer(&[], None, None, Some(&["Blocked-on"])),
        )]);
        let (exec, findings) = header_pointer_field_clean(&[r], &config);
        assert_eq!(examined(&exec), 0);
        assert!(findings.is_empty());
    }

    #[test]
    fn a_blocked_on_pointer_to_a_terminal_bug_is_a_stale_finding() {
        // The exact live case this rule was built for: MILE-2/3's Blocked-on
        // named BUG-3, which had already shipped (Status: Fixed) with nothing
        // catching it, since Blocked-on used to be unchecked free-text prose.
        let bug = record("docs/bugs/BUG-3-x.md", "bug", "> Status: Fixed\n");
        let milestone = record(
            "docs/milestones/MILE-2-x.md",
            "milestone",
            "> Status: Planned\n> Blocked-on: BUG-3\n",
        );
        let config = config_with_types(vec![
            (
                "milestone",
                type_config_pointer(&[], None, None, Some(&["Blocked-on"])),
            ),
            ("bug", type_config_pointer(&["Status"], None, None, None)),
        ]);
        let records = [bug, milestone];
        let index = build_normalized_index(&records);
        let (exec, findings) =
            narrative_field_stale(&records, &config, &terminal_for_tests(), &index);
        assert_eq!(examined(&exec), 1);
        assert_eq!(findings.len(), 1);
        assert!(findings[0].message.contains("BUG-3"));
        assert!(findings[0].message.contains("Fixed"));
        assert_eq!(findings[0].severity, FindingSeverity::Warning);
    }

    #[test]
    fn a_blocked_on_pointer_to_a_non_terminal_record_is_not_stale() {
        let bug = record("docs/bugs/BUG-4-x.md", "bug", "> Status: Open\n");
        let milestone = record(
            "docs/milestones/MILE-9-x.md",
            "milestone",
            "> Status: Planned\n> Blocked-on: BUG-4\n",
        );
        let config = config_with_types(vec![(
            "milestone",
            type_config_pointer(&[], None, None, Some(&["Blocked-on"])),
        )]);
        let records = [bug, milestone];
        let index = build_normalized_index(&records);
        let (exec, findings) =
            narrative_field_stale(&records, &config, &terminal_for_tests(), &index);
        assert_eq!(examined(&exec), 1);
        assert!(findings.is_empty());
    }

    #[test]
    fn blocked_on_free_text_with_no_reference_is_skipped() {
        let milestone = record(
            "docs/milestones/MILE-5-x.md",
            "milestone",
            "> Status: Planned\n> Blocked-on: a decision that hasn't been made\n",
        );
        let config = config_with_types(vec![(
            "milestone",
            type_config_pointer(&[], None, None, Some(&["Blocked-on"])),
        )]);
        let records = [milestone];
        let index = build_normalized_index(&records);
        let (exec, findings) =
            narrative_field_stale(&records, &config, &terminal_for_tests(), &index);
        let population = exec
            .population
            .expect("the rule must state what it was handed");
        assert_eq!(population.eligible(), 1, "the slot is declared and written");
        assert_eq!(
            population.examined(),
            0,
            "prose yielding no reference is handed but not judged -- the BUG-39 instrument"
        );
        assert!(findings.is_empty());
    }

    #[test]
    fn a_dangling_blocked_on_reference_is_not_this_rule_s_job() {
        let milestone = record(
            "docs/milestones/MILE-6-x.md",
            "milestone",
            "> Status: Planned\n> Blocked-on: BUG-9999\n",
        );
        let config = config_with_types(vec![(
            "milestone",
            type_config_pointer(&[], None, None, Some(&["Blocked-on"])),
        )]);
        let records = [milestone];
        let index = build_normalized_index(&records);
        let (exec, findings) =
            narrative_field_stale(&records, &config, &terminal_for_tests(), &index);
        assert_eq!(examined(&exec), 1);
        assert!(
            findings.is_empty(),
            "a dangling reference is pointer_resolution's error case, not this rule's"
        );
    }

    #[test]
    fn narrative_field_stale_generalizes_beyond_blocked_on() {
        // MILE-90: the rule now reads whatever the type's own
        // narrative_fields declares, not a hardcoded "Blocked-on" literal --
        // proven with a differently-named field.
        let bug = record("docs/bugs/BUG-7-x.md", "bug", "> Status: Fixed\n");
        let rfc = record(
            "docs/rfc/RFC-1-x.md",
            "rfc",
            "> Status: Draft\n> Motivated-by: BUG-7\n",
        );
        let config = config_with_types(vec![
            (
                "rfc",
                type_config_pointer(&[], None, None, Some(&["Motivated-by"])),
            ),
            ("bug", type_config_pointer(&["Status"], None, None, None)),
        ]);
        let records = [bug, rfc];
        let index = build_normalized_index(&records);
        let (exec, findings) =
            narrative_field_stale(&records, &config, &terminal_for_tests(), &index);
        assert_eq!(examined(&exec), 1);
        assert_eq!(findings.len(), 1);
        assert!(findings[0].message.starts_with("Motivated-by:"));
    }

    #[test]
    fn narrative_field_stale_reads_a_custom_status_field_name_observed_failing() {
        let bug = record("docs/bugs/BUG-8-x.md", "bug", "> State: Fixed\n");
        let rfc = record(
            "docs/rfc/RFC-2-x.md",
            "rfc",
            "> Status: Draft\n> Motivated-by: BUG-8\n",
        );
        let config = config_with_types(vec![
            (
                "rfc",
                type_config_pointer(&[], None, None, Some(&["Motivated-by"])),
            ),
            (
                "bug",
                crate::config::RecordTypeConfig {
                    relation_fields: Some(crate::config::RelationFields {
                        status: Some("State".to_string()),
                        ..Default::default()
                    }),
                    ..type_config_pointer(&["State"], None, None, None)
                },
            ),
        ]);
        let records = [bug, rfc];
        let index = build_normalized_index(&records);
        let (exec, findings) =
            narrative_field_stale(&records, &config, &terminal_for_tests(), &index);
        assert_eq!(examined(&exec), 1);
        assert_eq!(findings.len(), 1, "{findings:?}");
        assert!(findings[0].message.contains("Fixed"), "{findings:?}");
    }

    #[test]
    fn a_five_digit_filename_still_resolves_bug_0002_observed_failing() {
        // Before the fix, record_id() rejected any numeric prefix that
        // wasn't exactly 4 digits -- past 9999 records of one type,
        // references would have silently stopped resolving.
        let target = record("docs/rfc/RFC-10000-x.md", "rfc", "> Status: Draft\n");
        let source = record(
            "docs/specs/SPEC-1-x.md",
            "spec",
            "> Implements: RFC-10000\n",
        );
        let config = config_with_types(vec![(
            "spec",
            type_config_pointer(&[], None, Some(&["Implements"]), Some(&[])),
        )]);
        let records = [target, source];
        let index = build_normalized_index(&records);
        let (_, findings) = pointer_resolution(&records, &config, &index);
        // Resolution succeeding reports nothing (MILE-80). a_dangling_parent_pointer_is_an_error
        // is the control that keeps this from passing on a rule that never fires.
        assert!(findings.is_empty(), "{findings:?}");
    }

    #[test]
    fn a_reference_resolves_regardless_of_zero_padding() {
        // ADR-0034 (the filename's own padding) and a hand-typed ADR-34
        // reference must resolve to the same record (BUG-0002).
        let target = record("docs/adr/ADR-0034-x.md", "adr", "> Status: Accepted\n");
        let source = record("docs/specs/SPEC-1-y.md", "spec", "> Implements: ADR-34\n");
        let config = config_with_types(vec![(
            "spec",
            type_config_pointer(&[], None, Some(&["Implements"]), Some(&[])),
        )]);
        let records = [target, source];
        let index = build_normalized_index(&records);
        let (_, findings) = pointer_resolution(&records, &config, &index);
        // Resolution succeeding reports nothing (MILE-80). a_dangling_parent_pointer_is_an_error
        // is the control that keeps this from passing on a rule that never fires.
        assert!(findings.is_empty(), "{findings:?}");
    }

    #[test]
    fn a_legacy_pre_type_prefix_filename_no_longer_resolves() {
        // ADR-36's amendment (BUG-9): the pre-type-prefix `NNNN-slug.md`
        // shape was never a real external-adopter case, only this repo's own
        // pre-ADR-36 history, and that history no longer exists in the
        // corpus. A reference to a filename in that shape is now correctly
        // dangling, not silently resolved.
        let legacy_style = record("docs/adr/0037-y.md", "adr", "> Status: Accepted\n");
        let source = record("docs/specs/SPEC-1-z.md", "spec", "> Implements: ADR-37\n");
        let config = config_with_types(vec![(
            "spec",
            type_config_pointer(&[], None, Some(&["Implements"]), Some(&[])),
        )]);
        let records = [legacy_style, source];
        let index = build_normalized_index(&records);
        let (_, findings) = pointer_resolution(&records, &config, &index);
        assert_eq!(findings.len(), 1, "unexpected findings: {findings:?}");
        assert_eq!(findings[0].severity, FindingSeverity::Error);
    }

    #[test]
    fn a_configured_type_prefix_resolves_independently_of_the_type_name() {
        // A type's filename/ID prefix can be configured shorter than its own
        // name (e.g. `milestone` type configured with `prefix = "MILE"`) --
        // record_id must key off the configured prefix, not a derived
        // upper-cased type name, so `Implements: MILE-1` resolves.
        let milestone = record_with_prefix(
            "docs/milestones/MILE-1-x.md",
            "milestone",
            "MILE",
            "> Status: Planned\n",
        );
        let source = record("docs/adr/ADR-1-y.md", "adr", "> Implements: MILE-1\n");
        let config = config_with_types(vec![(
            "adr",
            type_config_pointer(&[], None, Some(&["Implements"]), Some(&[])),
        )]);
        let records = [milestone, source];
        let index = build_normalized_index(&records);
        let (_, findings) = pointer_resolution(&records, &config, &index);
        assert!(findings.is_empty(), "{findings:?}");
    }

    #[test]
    fn filename_title_consistency_skips_a_legacy_shaped_filename() {
        // No type prefix means no derivable filename_number -- the rule has
        // nothing to compare a title against, so it skips rather than
        // silently accepting or falsely flagging it (BUG-9/ADR-36 amendment).
        let new_style = record(
            "docs/adr/ADR-36-x.md",
            "adr",
            "# 36 — X\n\n> Status: Accepted\n",
        );
        let legacy_style = record(
            "docs/adr/0037-y.md",
            "adr",
            "# 37 — Y\n\n> Status: Accepted\n",
        );
        let mut full_text = HashMap::new();
        full_text.insert(
            new_style.path.clone(),
            "# 36 — X\n\n> Status: Accepted\n".to_string(),
        );
        full_text.insert(
            legacy_style.path.clone(),
            "# 37 — Y\n\n> Status: Accepted\n".to_string(),
        );
        let (exec, findings) = filename_title_consistency(&[new_style, legacy_style], &full_text);
        assert_eq!(examined(&exec), 1);
        assert!(findings.is_empty(), "unexpected findings: {findings:?}");
    }

    #[test]
    fn filename_title_consistency_still_catches_a_real_mismatch_on_the_new_shape() {
        let r = record(
            "docs/adr/ADR-0036-x.md",
            "adr",
            "# 0099 — Wrong Number\n\n> Status: Accepted\n",
        );
        let mut full_text = HashMap::new();
        full_text.insert(
            r.path.clone(),
            "# 0099 — Wrong Number\n\n> Status: Accepted\n".to_string(),
        );
        let (_, findings) = filename_title_consistency(&[r], &full_text);
        assert_eq!(findings.len(), 1);
        assert!(findings[0].message.contains("36") && findings[0].message.contains("99"));
    }

    /// A record with frontmatter, for the cases where the H1's own line
    /// number is what's under test.
    fn yaml_record(path: &str, content: &str) -> (Record, HashMap<PathBuf, String>) {
        let r = Record::parse_with_shape(
            PathBuf::from(path),
            "adr".to_string(),
            content,
            crate::header::HeaderShape::YamlFrontmatter,
        );
        let mut full_text = HashMap::new();
        full_text.insert(r.path.clone(), content.to_string());
        (r, full_text)
    }

    #[test]
    fn filename_title_consistency_distinguishes_an_unnumbered_h1_observed_failing() {
        // An H1 that exists but carries no number is a different defect from
        // no H1 at all -- "add a title" vs. "this corpus numbers its records
        // somewhere other than the H1" -- so it cannot share their message.
        let (r, full_text) = yaml_record(
            "docs/adr/ADR-7-x.md",
            "---\nStatus: Accepted\n---\n# Add Status Field\n",
        );
        let (_, findings) = filename_title_consistency(&[r], &full_text);
        assert_eq!(findings.len(), 1);
        assert!(
            !findings[0].message.contains("no H1 title found"),
            "an H1 that is present must not be reported as absent, got: {}",
            findings[0].message
        );
        assert!(
            findings[0].message.contains("Add Status Field"),
            "expected the title quoted back, got: {}",
            findings[0].message
        );
    }

    #[test]
    fn filename_title_consistency_reports_the_real_h1_line_observed_failing() {
        // Every record in this corpus carries frontmatter, so an H1 is never
        // on line 1 and a hardcoded line points the reader at the wrong place.
        let (r, full_text) = yaml_record(
            "docs/adr/ADR-36-x.md",
            "---\nStatus: Accepted\n---\n# 99 — Wrong Number\n",
        );
        let (_, findings) = filename_title_consistency(&[r], &full_text);
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].line, Some(4));
    }

    #[test]
    fn filename_title_consistency_ignores_an_h1_inside_a_fenced_block_observed_failing() {
        // Without fence tracking this reports a mismatch against `1` from a
        // shell snippet -- a fabricated finding about a number that is not a
        // record number, which is worse than saying nothing.
        let (r, full_text) = yaml_record(
            "docs/adr/ADR-8-x.md",
            "---\nStatus: Accepted\n---\nProse, no heading.\n\n```sh\n# 1. install\n```\n",
        );
        let (_, findings) = filename_title_consistency(&[r], &full_text);
        assert_eq!(findings.len(), 1);
        assert!(
            findings[0].message.contains("no H1 title found"),
            "a fenced heading is not a title, got: {}",
            findings[0].message
        );
    }

    #[test]
    fn filename_title_consistency_closes_a_fence_only_on_its_own_marker_observed_failing() {
        // A fence closes on its own marker at its own length or longer. A
        // shorter run, or the other marker family, is content -- treating
        // either as a close reopens the body and the sample heading inside
        // it becomes the title.
        for body in [
            "````md\n```\n# 1. sample\n```\n````\n",
            "```md\n~~~\n# 1. sample\n~~~\n```\n",
        ] {
            let (r, full_text) = yaml_record(
                "docs/adr/ADR-8-x.md",
                &format!("---\nStatus: Accepted\n---\nProse.\n\n{body}"),
            );
            let (_, findings) = filename_title_consistency(&[r], &full_text);
            assert_eq!(findings.len(), 1);
            assert!(
                findings[0].message.contains("no H1 title found"),
                "nested fence {body:?} leaked a heading: {}",
                findings[0].message
            );
        }
    }

    #[test]
    fn filename_title_consistency_keeps_the_absent_h1_message() {
        let (r, full_text) = yaml_record(
            "docs/adr/ADR-9-x.md",
            "---\nStatus: Accepted\n---\nNo heading anywhere.\n",
        );
        let (_, findings) = filename_title_consistency(&[r], &full_text);
        assert_eq!(findings.len(), 1);
        assert_eq!(
            findings[0].message,
            "no H1 title found to check against the filename's number"
        );
    }

    #[test]
    fn filename_title_consistency_treats_an_empty_h1_as_absent() {
        let (r, full_text) = yaml_record("docs/adr/ADR-9-x.md", "---\nStatus: Accepted\n---\n# \n");
        let (_, findings) = filename_title_consistency(&[r], &full_text);
        assert_eq!(findings.len(), 1);
        assert!(findings[0].message.contains("no H1 title found"));
    }

    #[test]
    fn an_empty_h1_skips_that_line_and_keeps_looking() {
        let (r, full_text) = yaml_record(
            "docs/adr/ADR-7-x.md",
            "---\nStatus: Accepted\n---\n# \n\n# ADR-9 — Wrong number\n",
        );
        let (_, findings) = filename_title_consistency(&[r], &full_text);
        assert_eq!(findings.len(), 1);
        assert!(
            !findings[0].message.contains("no H1 title found"),
            "a real H1 follows the empty one: {}",
            findings[0].message
        );
    }

    #[test]
    fn field_quality_flags_placeholder_not_just_absence() {
        // A field present per Rule 1 (the key exists) but never actually
        // filled in -- Rule 1 alone would pass this record.
        let r = record("docs/adr/0001-x.md", "adr", "> Author: name\n");
        let config = config_for_required(vec![("adr", vec!["Author"])]);

        let (exec, findings) = field_quality(&[r], &config);
        assert_eq!(examined(&exec), 1);
        assert_eq!(findings.len(), 1);
        assert!(findings[0].message.contains("Placeholder"));
    }

    /// `BUG-125`: a rule's silence about an unreadable header used to depend
    /// on `header.required-fields` being separately enabled to disclose it --
    /// a pairing `ADR-53` never guarantees. `Population` now discloses
    /// `unreadable` on its own, regardless of what else is enabled.
    #[test]
    fn field_quality_discloses_unreadable_on_its_own_observed_failing() {
        let r = record("docs/adr/0001-x.md", "adr", "this is not a header at all");
        let config = config_for_required(vec![("adr", vec!["Status"])]);

        let (exec, findings) = field_quality(&[r], &config);
        assert_eq!(examined(&exec), 0, "{exec:?}");
        assert_eq!(unreadable(&exec), 1, "{exec:?}");
        assert!(findings.is_empty(), "{findings:?}");
    }

    /// Observed failing against the real defect: a changeset in this repository
    /// announced it closed `BUG-36` while `BUG-36` said `Open`, and shipped.
    #[test]
    fn a_claim_to_close_an_open_record_is_an_error_observed_failing() {
        let bug = record("docs/bugs/BUG-36-x.md", "bug", "> Status: Open\n");
        let claims = vec![(
            ".changeset/x.md".to_string(),
            "rendered through a real serializer -- which also closes BUG-36, where a directory\nname escaped its value.\n".to_string(),
        )];
        let closed = vec!["Fixed".to_string()];

        let records = [bug];
        let index = build_normalized_index(&records);
        let config = config_for_required(vec![("bug", vec!["Status"])]);
        let (exec, findings) = claim_status_agreement(&claims, &closed, &config, &index);
        assert_eq!(examined(&exec), 1);
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].severity, FindingSeverity::Error);
        assert_eq!(findings[0].line, Some(1));
        assert!(findings[0].message.contains("Status Open"), "{findings:?}");
    }

    /// BUG-100: a target record whose `Status` key is written under a
    /// different case is not judged, not judged-and-wrong. Before the fix
    /// this fabricated the sentinel `"(no Status field)"`, which matched no
    /// configured closed status and pushed a blocking `Error` on every claim.
    #[test]
    fn a_claim_against_a_case_differently_written_status_key_is_not_judged_observed_failing() {
        let bug = record("docs/bugs/BUG-36-x.md", "bug", "> status: Open\n");
        let claims = vec![(".changeset/x.md".to_string(), "closes BUG-36.".to_string())];
        let closed = vec!["Fixed".to_string()];

        let records = [bug];
        let index = build_normalized_index(&records);
        let config = config_for_required(vec![("bug", vec!["Status"])]);
        let (exec, findings) = claim_status_agreement(&claims, &closed, &config, &index);
        assert_eq!(examined(&exec), 1);
        assert!(
            findings.is_empty(),
            "a lookup miss must not be reported as a status mismatch: {findings:?}"
        );
    }

    #[test]
    fn claim_status_agreement_reads_a_custom_status_field_name_observed_failing() {
        let bug = record("docs/bugs/BUG-36-x.md", "bug", "> State: Open\n");
        let claims = vec![(
            ".changeset/x.md".to_string(),
            "which also closes BUG-36.".to_string(),
        )];
        let closed = vec!["Fixed".to_string()];

        let records = [bug];
        let index = build_normalized_index(&records);
        let config = config_with_types(vec![(
            "bug",
            crate::config::RecordTypeConfig {
                relation_fields: Some(crate::config::RelationFields {
                    status: Some("State".to_string()),
                    ..Default::default()
                }),
                ..type_config_pointer(&["State"], None, None, None)
            },
        )]);
        let (exec, findings) = claim_status_agreement(&claims, &closed, &config, &index);
        assert_eq!(examined(&exec), 1);
        assert_eq!(findings.len(), 1, "{findings:?}");
        assert!(findings[0].message.contains("State Open"), "{findings:?}");
    }

    /// `BUG-109`/`ADR-60`: a target record's `Status` is only judged when its
    /// own type declares `Status` -- the same rule `header.field-case-mismatch`
    /// already applies. A type that never declared it is not this rule's
    /// business, the same as any other undeclared field.
    #[test]
    fn a_claim_against_a_target_whose_type_never_declares_status_is_not_judged_observed_failing() {
        let bug = record("docs/bugs/BUG-36-x.md", "bug", "> Status: Open\n");
        let claims = vec![(".changeset/x.md".to_string(), "closes BUG-36.".to_string())];
        let closed = vec!["Fixed".to_string()];

        let records = [bug];
        let index = build_normalized_index(&records);
        // "bug" declares no fields at all -- Status included.
        let config = config_for_required(vec![("bug", vec![])]);
        let (exec, findings) = claim_status_agreement(&claims, &closed, &config, &index);
        assert_eq!(examined(&exec), 1);
        assert!(
            findings.is_empty(),
            "a type that never declared Status must not have it judged: {findings:?}"
        );
    }

    /// BUG-45, both shapes, each of which produced a blocking error on a
    /// correct sentence.
    #[test]
    fn a_verb_inside_another_word_is_not_a_claim_observed_failing() {
        assert!(claimed_closed("The path prefixes changed; see RFC-9 for why.").is_empty());
        assert!(claimed_closed("This discloses RFC-9's reasoning.").is_empty());
        assert!(claimed_closed("Suffixes and RFC-9 are unrelated.").is_empty());
    }

    /// `BUG-111`: `parse_record_filename` supports a hyphenated type prefix
    /// (`DOC-ADR-2-x.md`), but the reference recognizer used to split on only
    /// the first hyphen, rejecting `DOC-ADR-2` as not-a-reference everywhere
    /// it's cited.
    #[test]
    fn a_hyphenated_type_prefix_is_a_recognized_reference_observed_failing() {
        assert!(is_record_reference("DOC-ADR-2"));
        assert!(is_record_reference("RFC-9"));
        assert!(
            !is_record_reference("DOC-adr-2"),
            "a lowercase segment is not a prefix"
        );
        assert!(
            !is_record_reference("DOC-ADR-"),
            "nothing after the last hyphen is not a number"
        );
        assert_eq!(
            scan_references("see DOC-ADR-2 for context"),
            vec!["DOC-ADR-2".to_string()]
        );
        assert_eq!(
            extract_references("DOC-ADR-2, RFC-9"),
            vec!["DOC-ADR-2".to_string(), "RFC-9".to_string()]
        );
    }

    #[test]
    fn only_the_reference_the_verb_governs_is_claimed_observed_failing() {
        // "Fixes BUG-39, which RFC-9 predicted" claimed RFC-9 too.
        assert_eq!(
            claimed_closed("Fixes BUG-39, which RFC-9 predicted."),
            vec!["BUG-39".to_string()]
        );
        // A comma-separated run after the verb is all claimed.
        // An Oxford comma leaves the run open, so `and` must be bridgeable
        // from either state -- but never directly after the verb.
        assert_eq!(
            claimed_closed("Fixes BUG-39, and BUG-40."),
            vec!["BUG-39".to_string(), "BUG-40".to_string()]
        );
        assert!(claimed_closed("Fixes and BUG-40 are unrelated.").is_empty());
        assert_eq!(
            claimed_closed("Closes BUG-39, BUG-40 and BUG-41 in one change."),
            vec![
                "BUG-39".to_string(),
                "BUG-40".to_string(),
                "BUG-41".to_string()
            ]
        );
        // The ordinary single case still works.
        assert_eq!(
            claimed_closed("which also closes BUG-36, where a directory name"),
            vec!["BUG-36".to_string()]
        );
    }

    #[test]
    fn a_claim_to_close_an_already_closed_record_is_silent() {
        let bug = record("docs/bugs/BUG-36-x.md", "bug", "> Status: Fixed\n");
        let claims = vec![(".changeset/x.md".to_string(), "closes BUG-36\n".to_string())];
        let records = [bug];
        let index = build_normalized_index(&records);
        let config = config_for_required(vec![("bug", vec!["Status"])]);
        let (_, findings) =
            claim_status_agreement(&claims, &["Fixed".to_string()], &config, &index);
        assert!(findings.is_empty(), "{findings:?}");
    }

    /// Mentioning a record is not claiming to close it.
    #[test]
    fn a_reference_without_a_closing_verb_is_not_a_claim() {
        let bug = record("docs/bugs/BUG-36-x.md", "bug", "> Status: Open\n");
        let claims = vec![(
            ".changeset/x.md".to_string(),
            "see BUG-36 for the four hardcoded paths\n".to_string(),
        )];
        let records = [bug];
        let index = build_normalized_index(&records);
        let config = config_for_required(vec![("bug", vec!["Status"])]);
        let (_, findings) =
            claim_status_agreement(&claims, &["Fixed".to_string()], &config, &index);
        assert!(findings.is_empty(), "{findings:?}");
    }

    /// `extract_references` reads the first token of each comma-separated entry,
    /// which is correct for a header value and silently empty on a sentence.
    /// BUG-39, live in this corpus: `MILE-13` declares
    /// `Blocked-on: RFC-9's own Q2`, and neither extractor could see `RFC-9`.
    #[test]
    fn a_possessive_does_not_hide_a_reference_observed_failing() {
        let value = "RFC-9's own Q2";
        assert_eq!(extract_references(value), vec!["RFC-9".to_string()]);
        assert_eq!(scan_references(value), vec!["RFC-9".to_string()]);

        // Narrow deliberately: a suffixed identifier is a different record, not
        // a possessive, and must stay unrecognised.
        assert!(scan_references("RFC-9a is unrelated").is_empty());
        assert!(scan_references("RFC-9s covers nine").is_empty());

        // Ordinary trailing punctuation was already handled; keep it asserted
        // so a narrower possessive rule cannot regress it.
        assert_eq!(
            scan_references("see RFC-9, then stop"),
            vec!["RFC-9".to_string()]
        );
        assert_eq!(scan_references("see RFC-9."), vec!["RFC-9".to_string()]);
    }

    #[test]
    fn scanning_prose_finds_references_that_extract_references_misses() {
        let line = "concatenation -- which also closes BUG-36, where a directory name escaped";
        assert!(extract_references(line).is_empty());
        assert_eq!(scan_references(line), vec!["BUG-36".to_string()]);
    }

    /// BUG-49, found live: `ADR-42` named `.urzua/config.toml` for two days
    /// after `ADR-52` deleted it, and nothing reported it.
    /// A declared type with no records may be exactly right, so the finding is
    /// an *absent* directory rather than an empty one.
    #[test]
    fn an_empty_declared_directory_is_not_a_finding() {
        let yaml = "schema_version: 2\nrecord_types:\n  present:\n    dir: \"docs/present\"\n  absent:\n    dir: \"docs/absent\"\n";
        let config = crate::config::parse(yaml).unwrap();
        let path = PathBuf::from(".urzua/config.yaml");

        // docs/present exists and is empty; docs/absent does not exist.
        let exists = |d: &str| d == "docs/present";
        let (exec, findings) = type_dir_matches_nothing(&config, &path, &HashMap::new(), &exists);

        assert_eq!(examined(&exec), 2, "both types are examined");
        assert_eq!(findings.len(), 1);
        assert!(findings[0].message.contains("docs/absent"), "{findings:?}");
        assert!(!findings[0].message.contains("present"), "{findings:?}");
    }

    #[test]
    fn a_locator_naming_a_missing_path_is_an_error_observed_failing() {
        let r = record(
            "docs/adr/ADR-1-x.md",
            "adr",
            "> Realized-by: code:src/real.rs, code:src/gone.rs\n",
        );
        let present = |p: &str| p == "src/real.rs";

        let (exec, findings) =
            embodiment_locator_exists(std::slice::from_ref(&r), &embodiment_config(), &present);
        // One candidate per declared slot, whatever its locator count: the
        // population is the rule's input, not its work count.
        assert_eq!(examined(&exec), 1);
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].severity, FindingSeverity::Error);
        assert!(findings[0].message.contains("src/gone.rs"), "{findings:?}");

        // Every locator present means silence, not a rule that cannot fire.
        let all_there = |_: &str| true;
        let (_, none) =
            embodiment_locator_exists(std::slice::from_ref(&r), &embodiment_config(), &all_there);
        assert!(none.is_empty(), "{none:?}");

        // An empty locator names nothing, and the repo root exists -- so it
        // passed a raw existence check while still computing to `Implemented`.
        let empty = record("docs/adr/ADR-2-y.md", "adr", "> Realized-by: code:\n");
        let (_, findings) = embodiment_locator_exists(&[empty], &embodiment_config(), &all_there);
        assert_eq!(findings.len(), 1, "{findings:?}");
        assert!(
            findings[0].message.contains("empty locator"),
            "{findings:?}"
        );
    }

    #[test]
    fn a_type_declaring_a_custom_locator_field_name_is_read_by_that_name_observed_failing() {
        let config = config_with_types(vec![(
            "adr",
            crate::config::RecordTypeConfig {
                relation_fields: Some(crate::config::RelationFields {
                    embodiment_locator: Some("Evidence".to_string()),
                    ..Default::default()
                }),
                ..type_config_pointer(&[], Some(&["Evidence"]), None, None)
            },
        )]);
        let r = record(
            "docs/adr/ADR-1-x.md",
            "adr",
            "> Evidence: code:src/gone.rs\n",
        );
        let present = |_: &str| false;

        let (exec, findings) =
            embodiment_locator_exists(std::slice::from_ref(&r), &config, &present);
        assert_eq!(examined(&exec), 1);
        assert_eq!(findings.len(), 1, "{findings:?}");
        assert!(findings[0].message.contains("src/gone.rs"), "{findings:?}");
    }

    #[test]
    fn a_pending_field_belongs_to_field_pending_not_field_quality() {
        // BUG-38, observed on this repo's own corpus: `field.quality` held both
        // states, so a repository declaring it `error` had a deliberate
        // `Pending` marker block CI, and declaring it `warn` stopped a genuinely
        // blank required field from blocking. No setting was correct.
        let r = record("docs/adr/0001-x.md", "adr", "> Deciders: Pending\n");
        let config = config_for_required(vec![("adr", vec!["Deciders"])]);

        let (_, quality) = field_quality(std::slice::from_ref(&r), &config);
        assert!(quality.is_empty(), "{quality:?}");

        let (_, pending) = field_pending(std::slice::from_ref(&r), &config);
        assert_eq!(pending.len(), 1);
        assert!(pending[0].message.contains("pending"));

        // The forgotten case stays with field.quality, and field.pending
        // must not claim it.
        let blank = record("docs/adr/0002-y.md", "adr", "> Deciders:\n");
        let (_, q2) = field_quality(std::slice::from_ref(&blank), &config);
        assert_eq!(q2.len(), 1);
        assert_eq!(q2[0].severity, FindingSeverity::Error);
        let (_, p2) = field_pending(&[blank], &config);
        assert!(p2.is_empty(), "{p2:?}");
    }

    #[test]
    fn field_quality_passes_a_real_value() {
        let r = record("docs/adr/0001-x.md", "adr", "> Author: someone\n");
        let config = config_for_required(vec![("adr", vec!["Author"])]);

        let (_, findings) = field_quality(&[r], &config);
        assert!(findings.is_empty(), "unexpected findings: {findings:?}");
    }

    #[test]
    fn field_quality_flags_a_retired_placeholder_as_an_error() {
        // BUG-5: (project lead) used to be this project's own placeholder
        // convention and classified as Present -- field.quality never had a
        // chance to catch a regression back to it (the ADR-38 incident).
        let r = record("docs/adr/0001-x.md", "adr", "> Author: (project lead)\n");
        let config = config_for_required(vec![("adr", vec!["Author"])]);

        let (_, findings) = field_quality(&[r], &config);
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].severity, FindingSeverity::Error);
    }

    #[test]
    fn filename_title_mismatch_is_a_planted_violation_observed_failing() {
        let r = record("docs/adr/ADR-1-x.md", "adr", "# 0002 — Wrong Number\n");
        let mut full_text = HashMap::new();
        full_text.insert(r.path.clone(), "# 0002 — Wrong Number\n".to_string());

        let (exec, findings) = filename_title_consistency(&[r], &full_text);
        assert_eq!(examined(&exec), 1);
        assert_eq!(findings.len(), 1);
        assert!(findings[0].message.contains('1'));
        assert!(findings[0].message.contains('2'));
    }

    #[test]
    fn matching_filename_and_title_produce_no_finding() {
        let r = record("docs/adr/ADR-1-x.md", "adr", "# 0001 — Correct\n");
        let mut full_text = HashMap::new();
        full_text.insert(r.path.clone(), "# 0001 — Correct\n".to_string());

        let (exec, findings) = filename_title_consistency(&[r], &full_text);
        assert_eq!(examined(&exec), 1);
        assert!(findings.is_empty(), "unexpected findings: {findings:?}");
    }

    #[test]
    fn a_revision_log_entry_with_no_real_change_class_is_a_planted_violation_observed_failing() {
        let content = "# 0001 — X\n\n> **Revision log**\n>\n> | Date | Change | Class |\n> |---|---|---|\n> | 2026-08-20 | Split out. | — |\n";
        let r = record("docs/specs/0001-x.md", "spec", content);
        let mut full_text = HashMap::new();
        full_text.insert(r.path.clone(), content.to_string());

        let (exec, findings) = revision_log_change_class(&[r], &full_text);
        assert_eq!(examined(&exec), 1);
        assert_eq!(findings.len(), 1);
        assert!(findings[0].message.contains("2026-08-20"));
    }

    #[test]
    fn a_revision_log_entry_with_a_real_change_class_produces_no_finding() {
        let content = "# 0001 — X\n\n> **Revision log**\n>\n> | Date | Change | Class |\n> |---|---|---|\n> | 2026-08-20 | Renamed a field. | **structural** |\n";
        let r = record("docs/specs/0001-x.md", "spec", content);
        let mut full_text = HashMap::new();
        full_text.insert(r.path.clone(), content.to_string());

        let (_, findings) = revision_log_change_class(&[r], &full_text);
        assert!(findings.is_empty(), "unexpected findings: {findings:?}");
    }

    #[test]
    fn a_record_with_no_revision_log_section_is_not_examined() {
        let content = "# 0001 — X\n\n> Status: Draft\n";
        let r = record("docs/adr/0001-x.md", "adr", content);
        let mut full_text = HashMap::new();
        full_text.insert(r.path.clone(), content.to_string());

        let (exec, findings) = revision_log_change_class(&[r], &full_text);
        assert_eq!(examined(&exec), 0);
        assert!(findings.is_empty());
    }

    #[test]
    fn embodiment_disagreeing_with_realized_by_is_a_planted_violation_observed_failing() {
        let r = record(
            "docs/adr/0001-x.md",
            "adr",
            "> Embodiment: Verified\n> Realized-by: code:src/lib.rs\n",
        );
        let (exec, findings) = embodiment_consistency(&[r], &embodiment_config(), &HashSet::new());
        assert_eq!(examined(&exec), 1);
        assert_eq!(findings.len(), 1);
        assert!(findings[0].message.contains("Verified"));
        assert!(findings[0].message.contains("Implemented"));
    }

    #[test]
    fn embodiment_agreeing_with_realized_by_produces_no_finding() {
        let r = record(
            "docs/adr/0001-x.md",
            "adr",
            "> Embodiment: Implemented\n> Realized-by: code:src/lib.rs\n",
        );
        let (_, findings) = embodiment_consistency(&[r], &embodiment_config(), &HashSet::new());
        assert!(findings.is_empty(), "unexpected findings: {findings:?}");
    }

    #[test]
    fn a_type_declaring_custom_embodiment_field_names_is_read_by_those_names_observed_failing() {
        let config = config_with_types(vec![(
            "adr",
            crate::config::RecordTypeConfig {
                relation_fields: Some(crate::config::RelationFields {
                    embodiment_state: Some("Phase".to_string()),
                    embodiment_locator: Some("Evidence".to_string()),
                    ..Default::default()
                }),
                ..type_config_pointer(&[], Some(&["Phase", "Evidence"]), None, None)
            },
        )]);
        let r = record(
            "docs/adr/0001-x.md",
            "adr",
            "> Phase: Verified\n> Evidence: code:src/lib.rs\n",
        );
        let (exec, findings) = embodiment_consistency(&[r], &config, &HashSet::new());
        assert_eq!(examined(&exec), 1);
        assert_eq!(findings.len(), 1, "{findings:?}");
        assert!(findings[0].message.contains("Phase"));
        assert!(findings[0].message.contains("Evidence"));
    }

    #[test]
    fn a_test_locator_outranks_a_code_locator() {
        let realized = parse_realized_by("code:src/lib.rs, test:tests/it.rs");
        assert_eq!(compute_embodiment(&realized, false), "Verified");
    }

    #[test]
    fn a_drifted_record_computes_to_drift_detected_regardless_of_tier() {
        let realized = parse_realized_by("code:src/lib.rs, test:tests/it.rs");
        assert_eq!(compute_embodiment(&realized, true), "Drift detected");
    }

    #[test]
    fn a_record_marked_drifted_produces_a_finding_even_when_verified_matches_its_tier() {
        let r = record(
            "docs/adr/0001-x.md",
            "adr",
            "> Embodiment: Verified\n> Realized-by: code:src/lib.rs, test:tests/it.rs\n",
        );
        let mut drifted = HashSet::new();
        drifted.insert(r.path.clone());
        let (_, findings) = embodiment_consistency(&[r], &embodiment_config(), &drifted);
        assert_eq!(findings.len(), 1);
        assert!(findings[0].message.contains("Drift detected"));
    }

    #[test]
    fn a_record_missing_half_the_pair_is_handed_to_the_rule_and_not_judged() {
        let r = record("docs/adr/0001-x.md", "adr", "> Embodiment: Not started\n");
        let (exec, findings) = embodiment_consistency(&[r], &embodiment_config(), &HashSet::new());
        let population = exec
            .population
            .expect("the rule must state what it was handed");
        assert_eq!(
            (population.eligible(), population.examined()),
            (1, 0),
            "the type declares both fields, so the record is a candidate that reached no verdict"
        );
        assert!(findings.is_empty());
    }

    #[test]
    fn realized_by_locator_paths_flattens_all_three_categories() {
        let paths =
            realized_by_locator_paths("spec:docs/rfc/0001-x.md, code:src/lib.rs, test:tests/it.rs");
        assert_eq!(
            paths,
            vec!["docs/rfc/0001-x.md", "src/lib.rs", "tests/it.rs"]
        );
    }

    #[test]
    fn a_locator_cited_by_two_records_is_a_promotion_candidate_observed_failing() {
        let a = record(
            "docs/adr/0001-x.md",
            "adr",
            "> Realized-by: code:src/shared.rs\n",
        );
        let b = record(
            "docs/adr/0002-y.md",
            "adr",
            "> Realized-by: code:src/shared.rs\n",
        );
        let (exec, findings) =
            embodiment_locator_promotion_candidate(&[a, b], &embodiment_config());
        assert_eq!(examined(&exec), 2);
        assert_eq!(findings.len(), 1);
        assert!(findings[0].message.contains("src/shared.rs"));
    }

    #[test]
    fn one_record_citing_the_same_locator_under_two_categories_is_not_a_promotion_candidate() {
        // A single record's own code: and test: locators both naming the
        // same file is one record, not two independent citers -- promotion
        // exists for cross-record duplication, never cross-category
        // duplication within a single record's own Realized-by.
        let a = record(
            "docs/adr/0001-x.md",
            "adr",
            "> Realized-by: code:src/shared.rs, test:src/shared.rs\n",
        );
        let (_, findings) = embodiment_locator_promotion_candidate(&[a], &embodiment_config());
        assert!(findings.is_empty(), "unexpected findings: {findings:?}");
    }

    #[test]
    fn a_locator_cited_by_only_one_record_is_not_a_promotion_candidate() {
        let a = record(
            "docs/adr/0001-x.md",
            "adr",
            "> Realized-by: code:src/a.rs\n",
        );
        let (_, findings) = embodiment_locator_promotion_candidate(&[a], &embodiment_config());
        assert!(findings.is_empty());
    }

    #[test]
    fn a_one_directional_supersession_claim_is_a_reciprocity_violation() {
        // 0002 claims to supersede 0001, but 0001 doesn't reciprocally name
        // 0002 -- a reader of 0001 would never know it was superseded.
        let old = record(
            "docs/adr/ADR-1-x.md",
            "adr",
            "> Supersedes / Superseded-by: —\n",
        );
        let new = record(
            "docs/adr/ADR-2-y.md",
            "adr",
            "> Supersedes / Superseded-by: ADR-0001\n",
        );

        let config = config_with_types(vec![(
            "adr",
            type_config_pointer(&[], Some(&["Supersedes / Superseded-by"]), None, None),
        )]);
        let records = [old, new];
        let index = build_normalized_index(&records);
        let (exec, findings) = supersession_reciprocity(&records, &config, &index);
        let population = exec
            .population
            .expect("the rule must state what it was handed");
        assert_eq!(population.eligible(), 2, "both records declare the slot");
        assert_eq!(
            population.examined(),
            2,
            "the reciprocating half answered its slot with the em dash"
        );
        assert_eq!(findings.len(), 1);
        assert!(findings[0].message.contains("ADR-0001"));
    }

    #[test]
    fn a_reciprocated_supersession_produces_no_finding() {
        let old = record(
            "docs/adr/ADR-1-x.md",
            "adr",
            "> Supersedes / Superseded-by: ADR-0002\n",
        );
        let new = record(
            "docs/adr/ADR-2-y.md",
            "adr",
            "> Supersedes / Superseded-by: ADR-0001\n",
        );

        let config = config_with_types(vec![(
            "adr",
            type_config_pointer(&[], Some(&["Supersedes / Superseded-by"]), None, None),
        )]);
        let records = [old, new];
        let index = build_normalized_index(&records);
        let (exec, findings) = supersession_reciprocity(&records, &config, &index);
        let population = exec
            .population
            .expect("the rule must state what it was handed");
        assert_eq!((population.eligible(), population.examined()), (2, 2));
        assert!(findings.is_empty(), "unexpected findings: {findings:?}");
    }

    #[test]
    fn a_type_declaring_a_custom_supersession_field_name_is_read_by_that_name_observed_failing() {
        let old = record("docs/adr/ADR-1-x.md", "adr", "> Replaces: —\n");
        let new = record("docs/adr/ADR-2-y.md", "adr", "> Replaces: ADR-0001\n");

        let config = config_with_types(vec![(
            "adr",
            crate::config::RecordTypeConfig {
                relation_fields: Some(crate::config::RelationFields {
                    supersession: Some("Replaces".to_string()),
                    ..Default::default()
                }),
                ..type_config_pointer(&[], Some(&["Replaces"]), None, None)
            },
        )]);
        let records = [old, new];
        let index = build_normalized_index(&records);
        let (exec, findings) = supersession_reciprocity(&records, &config, &index);
        let population = exec
            .population
            .expect("the rule must state what it was handed");
        assert_eq!(population.eligible(), 2, "both records declare the slot");
        assert_eq!(findings.len(), 1, "{findings:?}");
        assert!(findings[0].message.contains("ADR-0001"));
    }

    #[test]
    fn an_em_dash_supersession_value_answers_its_slot() {
        let r = record(
            "docs/adr/ADR-1-x.md",
            "adr",
            "> Supersedes / Superseded-by: —\n",
        );
        let config = config_with_types(vec![(
            "adr",
            type_config_pointer(&[], Some(&["Supersedes / Superseded-by"]), None, None),
        )]);
        let records = [r];
        let index = build_normalized_index(&records);
        let (exec, findings) = supersession_reciprocity(&records, &config, &index);
        let population = exec
            .population
            .expect("the rule must state what it was handed");
        assert_eq!(
            (population.eligible(), population.examined()),
            (1, 1),
            "the em dash is a written answer, not an unfilled slot"
        );
        assert!(findings.is_empty());
    }

    #[test]
    fn relation_target_status_undeclared_flags_a_resolving_reference_to_an_undeclaring_type_observed_failing(
    ) {
        let target = record("docs/rfc/RFC-1-x.md", "rfc", "> Date: 2026-01-01\n");
        let source = record("docs/specs/SPEC-1-x.md", "spec", "> Implements: RFC-1\n");
        let config = config_with_types(vec![
            (
                "spec",
                type_config_pointer(&[], None, Some(&["Implements"]), Some(&[])),
            ),
            // "rfc" declares no Status at all -- ADR-60's gate would silently
            // exclude any rule reading it, which is exactly what this rule
            // exists to surface.
            ("rfc", type_config_pointer(&["Date"], None, None, None)),
        ]);
        let records = [target, source];
        let index = build_normalized_index(&records);

        let (exec, findings) = relation_target_status_undeclared(&records, &config, &index);
        assert_eq!(examined(&exec), 1);
        assert_eq!(findings.len(), 1, "{findings:?}");
        assert!(findings[0].message.contains("RFC-1"), "{findings:?}");
        assert!(findings[0].message.contains("Status"), "{findings:?}");
    }

    #[test]
    fn relation_target_status_undeclared_is_silent_when_the_target_declares_status() {
        let target = record("docs/rfc/RFC-1-x.md", "rfc", "> Status: Draft\n");
        let source = record("docs/specs/SPEC-1-x.md", "spec", "> Implements: RFC-1\n");
        let config = config_with_types(vec![
            (
                "spec",
                type_config_pointer(&[], None, Some(&["Implements"]), Some(&[])),
            ),
            ("rfc", type_config_pointer(&["Status"], None, None, None)),
        ]);
        let records = [target, source];
        let index = build_normalized_index(&records);

        let (exec, findings) = relation_target_status_undeclared(&records, &config, &index);
        assert_eq!(examined(&exec), 1);
        assert!(findings.is_empty(), "{findings:?}");
    }
}
