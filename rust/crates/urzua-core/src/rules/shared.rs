use super::*;

/// `census_records` for every rule that reads a record's header: the one place
/// a `header_shape: none` type (`ADR-50`) is classified `OutOfScope`
/// (`MILE-119`). Such a type has nowhere for a header to be, so its
/// `is_unreadable()` is true by construction -- not a parse failure -- and no
/// edit to the record could make the rule apply; the contradiction of declaring
/// fields on it is `config.header-none-has-no-required-fields`'s to name.
/// Gating here rather than in each rule is the point: `BUG-98`, `BUG-124`,
/// `BUG-137`, `BUG-142` and `BUG-147` were each one rule that forgot.
///
/// A rule that never reads a header (`identity.collision`,
/// `filename.title-consistency`, `revision-log.change-class-required`) uses
/// `census_records` directly. The all-rules test in `check_integration.rs`
/// iterates `ALL_RULES` and names each such exemption.
pub(super) fn scoped_census_records<'a, T>(
    config: &Config,
    unit: PopulationUnit,
    candidates: Vec<T>,
    record_of: impl Fn(&T) -> &'a Record,
    mut body: impl FnMut(&T) -> Outcome,
) -> (Population, Vec<PathBuf>) {
    census_records(
        unit,
        candidates,
        |candidate| record_of(candidate).path.clone(),
        |candidate| {
            let record = record_of(candidate);
            if config
                .record_types
                .get(&record.record_type)
                .is_some_and(|t| t.has_no_header())
            {
                return Outcome::OutOfScope;
            }
            body(candidate)
        },
    )
}

/// One `(record, field)` candidate per field a record's type declares in
/// `fields_by_type` (`BUG-105`): the same construction four rules duplicated,
/// against `HashMap`s of either `Vec<String>` or `HashSet<String>`.
pub(super) fn field_slots<'a, T, C>(
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
pub(super) fn pointer_and_narrative_slots<'a>(
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
pub(super) fn project_by_type<T>(
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
pub(super) fn required_fields_by_type(config: &Config) -> HashMap<String, Vec<String>> {
    project_by_type(config, |cfg| Some(cfg.required_fields.clone()))
}

/// A type's `header_layout`, by type name, when declared (`ADR-59`).
pub(super) fn header_layout_by_type(config: &Config) -> HashMap<String, HeaderLayout> {
    project_by_type(config, |cfg| cfg.header_layout)
}

/// A type's declared field set, by type name, only for a type that declares
/// `known_fields` (`ADR-59`): `header.field-set-consistency` skips a type
/// declaring only `required_fields` by design, and a map with an entry for
/// every type would silently stop skipping it.
pub(super) fn known_fields_by_type(
    config: &Config,
) -> HashMap<String, std::collections::BTreeSet<crate::values::FieldName>> {
    project_by_type(config, |cfg| {
        cfg.known_fields.as_ref().map(|_| cfg.declared_fields())
    })
}

/// Every field a type declares (`required_fields` ∪ `known_fields`), by type
/// name (`ADR-59`, `BUG-106`).
pub(super) fn declared_fields_by_type(
    config: &Config,
) -> HashMap<String, std::collections::BTreeSet<crate::values::FieldName>> {
    project_by_type(config, |cfg| Some(cfg.declared_fields()))
}

/// A type's `pointer_fields`, by type name, when declared (`ADR-59`).
pub(super) fn pointer_fields_by_type(config: &Config) -> HashMap<String, Vec<String>> {
    project_by_type(config, |cfg| cfg.pointer_fields.clone())
}

/// A type's `narrative_fields`, by type name, when declared (`ADR-59`).
pub(super) fn narrative_fields_by_type(config: &Config) -> HashMap<String, Vec<String>> {
    project_by_type(config, |cfg| cfg.narrative_fields.clone())
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
    pub(super) enforce_clean_format: bool,
    /// Does `narrative_field_stale` apply to fields of this kind?
    pub(super) check_target_staleness: bool,
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
pub(super) fn fields_with_capability(
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
pub fn record_id(record: &Record) -> Option<String> {
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
pub(super) fn strip_possessive(token: &str) -> &str {
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
///
/// Shares its segment grammar with [`crate::new_record::parse_record_filename`]
/// (`BUG-133`): two independent copies of "is this a valid record identifier"
/// is how they drifted apart once already (`BUG-114`).
pub(super) fn is_record_reference(token: &str) -> bool {
    use crate::new_record::{is_digit_segment, is_prefix_segment};

    let segments: Vec<&str> = token.split('-').collect();
    let Some((num, prefix_segments)) = segments.split_last() else {
        return false;
    };
    is_digit_segment(num)
        && !prefix_segments.is_empty()
        && prefix_segments.iter().all(|s| is_prefix_segment(s))
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

/// The references a declared pointer/narrative field carries, or the
/// `Outcome` its absence means -- the guard chain `pointer_resolution`,
/// `pointer_target_status`, `narrative_field_stale`, and
/// `relation_target_status_undeclared` each independently re-derived before
/// their own per-reference loop. `Unreadable` if the header didn't parse;
/// `Absent` if the field is unwritten, or written but yields no reference
/// (`narrative_field_stale`'s BUG-39 case: prose that extracts nothing is
/// unexamined, not silently present).
pub(super) fn declared_field_references(
    record: &Record,
    field_name: &str,
) -> Result<Vec<String>, Outcome> {
    if record.header.is_unreadable() {
        return Err(Outcome::Unreadable);
    }
    let Some(value) = record.header.get(field_name) else {
        return Err(Outcome::Absent);
    };
    let references = extract_references(value);
    if references.is_empty() {
        return Err(Outcome::Absent);
    }
    Ok(references)
}

/// The walk `pointer_target_status` and `relation_target_status_undeclared`
/// both do before their own, differing judgment of a resolved target's
/// status: resolve the declared field's references, look each up in the
/// index, and name the field that plays the `Status` role for the target's
/// type. A lookup miss is silently skipped in both -- `pointer_resolution`
/// already reports a dangling reference, so this is never that rule's job.
pub(super) fn for_each_resolved_status_target(
    record: &Record,
    field_name: &str,
    config: &Config,
    index: &RecordIndex,
    mut body: impl FnMut(&str, &Record, &str),
) -> Result<(), Outcome> {
    let references = declared_field_references(record, field_name)?;
    for reference in references {
        let Some(target) = index.get(&crate::values::RecordId::new(&reference)) else {
            continue;
        };
        let target_status_field = relation_field_name(
            config,
            &target.record_type,
            crate::config::RelationRole::Status,
        );
        body(&reference, target, target_status_field);
    }
    Ok(())
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
pub(super) fn declared_slots_for_roles<'a>(
    records: &'a [Record],
    config: &Config,
    roles: &[crate::config::RelationRole],
) -> Vec<&'a Record> {
    let declared_by_type = declared_fields_by_type(config);
    records
        .iter()
        .filter(|record| {
            config
                .record_types
                .get(&record.record_type)
                .is_some_and(|t| {
                    let Some(declared) = declared_by_type.get(&record.record_type) else {
                        return false;
                    };
                    roles
                        .iter()
                        .all(|role| declared.contains(t.relation_field(*role)))
                })
        })
        .collect()
}

/// A declared field's value, or the `Outcome` its absence means (`RFC-40`):
/// `Unreadable` if the header itself didn't parse, `Absent` otherwise.
pub(super) fn declared_value<'a>(record: &'a Record, key: &str) -> Result<&'a str, Outcome> {
    match record.header.read_declared(key) {
        crate::header::FieldRead::Present(value) => Ok(value),
        crate::header::FieldRead::Missing => Err(Outcome::Absent),
        crate::header::FieldRead::Unreadable => Err(Outcome::Unreadable),
    }
}

/// `declared_value`, additionally gated on `record`'s own type declaring
/// `key` at all (`ADR-60`): a type that never declares `Status` is not this
/// rule's business to judge, the same as any other undeclared field --
/// `Status` is adopter vocabulary like every other field `ADR-53` governs,
/// not an engine-reserved one.
pub(super) fn declared_cross_record_value<'a>(
    declared_by_type: &HashMap<String, std::collections::BTreeSet<crate::values::FieldName>>,
    record: &'a Record,
    key: &str,
) -> Result<&'a str, Outcome> {
    let declared = declared_by_type
        .get(&record.record_type)
        .is_some_and(|fields| fields.contains(&crate::values::FieldName::from(key)));
    if !declared {
        return Err(Outcome::Absent);
    }
    declared_value(record, key)
}

/// Shared by `boundary.published-path-reference` and `citation.resolution` so
/// the two rules agree on what counts as a cited identifier.
/// The record-identifier prefixes this repository's configuration declares,
/// longest first so `DOC-ADR` is matched before `ADR`. The type's effective
/// prefix, the same one `urzua new` mints and `check` resolves with.
pub(super) fn declared_prefixes(config: &Config) -> Vec<String> {
    let mut prefixes: Vec<String> = config
        .record_types
        .iter()
        .map(|(name, rt)| {
            rt.prefix
                .clone()
                .unwrap_or_else(|| name.to_ascii_uppercase())
        })
        .collect();
    prefixes.sort_by(|a, b| b.len().cmp(&a.len()).then_with(|| a.cmp(b)));
    prefixes.dedup();
    prefixes
}

/// Every identifier on `line` that is a declared prefix, a hyphen and a
/// number, standing on its own: not the tail of a longer word (`XADR-26`), a
/// URL or path segment (`x.org/ADR-1`) or a snake_case or dotted name, and not
/// run into letters after the number (`ADR-26x`). A trailing `.` is allowed, so
/// `(ADR-1.2)` reports `ADR-1`. An identifier split across lines is not
/// detected: each line is scanned alone.
///
/// Deliberately wider than the evidence collector's structured-marker matcher
/// (`RFC-11` section 3): this answers "was a record cited here at all", prose
/// included, and must never be the matcher that decides a record was realized.
pub(super) fn cited_identifiers(line: &str, prefixes: &[String]) -> Vec<String> {
    let bytes = line.as_bytes();
    let mut out = Vec::new();
    let mut i = 0;
    'scan: while i < bytes.len() {
        // Not the tail of a word, a hyphenated name, a URL or path segment, a
        // snake_case name or a dotted one: `/`, `_` and `.` join an identifier
        // to what precedes it the way a letter, digit or `-` does.
        let standing_alone = line.is_char_boundary(i)
            && !line[..i]
                .chars()
                .next_back()
                .is_some_and(|c| c.is_alphanumeric() || matches!(c, '-' | '/' | '_' | '.'));
        if standing_alone {
            for prefix in prefixes {
                let p = prefix.as_bytes();
                if !bytes[i..].starts_with(p) || bytes.get(i + p.len()) != Some(&b'-') {
                    continue;
                }
                let digits_at = i + p.len() + 1;
                let digits = bytes[digits_at..]
                    .iter()
                    .take_while(|b| b.is_ascii_digit())
                    .count();
                let end = digits_at + digits;
                if digits == 0
                    || line[end..]
                        .chars()
                        .next()
                        .is_some_and(char::is_alphanumeric)
                {
                    continue;
                }
                out.push(line[i..end].to_string());
                i = end;
                continue 'scan;
            }
        }
        i += 1;
    }
    out
}
