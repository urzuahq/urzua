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
pub const RULE_RELATION_PAIR_RECIPROCITY: &str = "relation.pair-reciprocity";
pub const RULE_RELATION_CROSS_TYPE_DISALLOWED: &str = "relation.cross-type-disallowed";
pub const RULE_RELATION_TARGET_STATUS_UNDECLARED: &str = "relation.target-status-undeclared";
pub const RULE_CONFIG_SCOPE_MATCHES_NOTHING: &str = "config.scope-matches-nothing";
pub const RULE_FIELD_UNTRIMMED_VALUE: &str = "field.untrimmed-value";
pub const RULE_HEADER_FIELD_CASE_MISMATCH: &str = "header.field-case-mismatch";
pub const RULE_CONFIG_RELATION_FIELD_NOT_KNOWN: &str = "config.relation-field-not-known";
pub const RULE_CONFIG_KNOWN_FIELDS_DECLARATION_MISSING: &str =
    "config.known-fields-declaration-missing";
pub const RULE_FIELD_LEADING_RESERVED_INDICATOR: &str = "field.leading-reserved-indicator";
pub const RULE_HEADER_CASE_VARIANT_KEY: &str = "header.case-variant-key";
pub const RULE_CITATION_RESOLUTION: &str = "citation.resolution";
pub const RULE_BOUNDARY_PUBLISHED_PATH_REFERENCE: &str = "boundary.published-path-reference";
pub const RULE_DIFF_SOURCE_REQUIRES_RECORD: &str = "diff.source-requires-record";
pub const RULE_DIFF_SOURCE_AND_RECORDS_MIXED: &str = "diff.source-and-records-mixed";

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
    RULE_FIELD_LEADING_RESERVED_INDICATOR,
    RULE_HEADER_CASE_VARIANT_KEY,
    RULE_RELATION_PAIR_RECIPROCITY,
    RULE_RELATION_CROSS_TYPE_DISALLOWED,
    RULE_CITATION_RESOLUTION,
    RULE_BOUNDARY_PUBLISHED_PATH_REFERENCE,
    RULE_DIFF_SOURCE_REQUIRES_RECORD,
    RULE_DIFF_SOURCE_AND_RECORDS_MIXED,
];

/// One rule's id and its adopter-facing description -- the single place this
/// text lives; `SPEC-2`'s own rule table is generated from it.
pub struct RuleMeta {
    pub id: &'static str,
    pub description: &'static str,
}

/// One entry per `ALL_RULES` member, same order.
pub const RULE_METADATA: &[RuleMeta] = &[
    RuleMeta {
        id: RULE_HEADER_REQUIRED_FIELDS,
        description: "Each type's `required_fields` (SPEC-3) are present; a missing header region or a duplicate key is also reported here.",
    },
    RuleMeta {
        id: RULE_HEADER_LAYOUT_CONSISTENCY,
        description: "A record's header line layout (one-per-line vs. pipe-delimited) matches its type's declared `header_layout`, when one is declared (ADR-38).",
    },
    RuleMeta {
        id: RULE_HEADER_FIELD_SET_CONSISTENCY,
        description: "Every header field belongs to its type's `required_fields` \u{222a} `known_fields`, when `known_fields` is declared (ADR-39).",
    },
    RuleMeta {
        id: RULE_TYPE_NO_DECLARED_SPEC,
        description: "Which configured types currently have no `spec` pointer declared at all (ADR-41/ADR-43) — a signal to review, never a mandate; a type can permanently have none declared.",
    },
    RuleMeta {
        id: RULE_TYPE_DIR_MATCHES_NOTHING,
        description: "A type's declared `dir` does not exist at all (`BUG-56`'s hazard for `dir`, no load-time guard). An existing, empty directory is a legitimate declared-but-unused state and is not flagged.",
    },
    RuleMeta {
        id: RULE_TYPE_RECORD_OUTSIDE_DECLARED_DIR,
        description: "A record-shaped tracked path that no type's `dir` claimed (`BUG-62`) — a config written before `RFC-35` silently loses coverage on upgrade otherwise.",
    },
    RuleMeta {
        id: RULE_IDENTITY_COLLISION,
        description: "Two records resolving to the same identifier — a corpus error the index cannot represent, reported rather than papered over (`BUG-79`).",
    },
    RuleMeta {
        id: RULE_HEADER_DEPRECATED_SHAPE,
        description: "Which configured types still declare a deprecated `header_shape` (`blockquote`/`bold-list`) instead of `yaml-frontmatter` (ADR-33). Parsing support for the deprecated shapes stays; this rule only surfaces which types haven't migrated.",
    },
    RuleMeta {
        id: RULE_CONFIG_POINTER_DECLARATION_MISSING,
        description: "A type declaring either `pointer_fields` or `narrative_fields` must declare both explicitly, even as `[]` (MILE-90/ADR-44) — omitting one is not the same as declaring zero fields of that kind.",
    },
    RuleMeta {
        id: RULE_CONFIG_POINTER_FIELD_NOT_KNOWN,
        description: "Every field named in a type's `pointer_fields`/`narrative_fields` must also appear in that type's own `required_fields`/`known_fields` (MILE-90/ADR-44) — otherwise `header.field-set-consistency` would never have heard of it.",
    },
    RuleMeta {
        id: RULE_CONFIG_POINTER_NARRATIVE_OVERLAP,
        description: "A field must be exactly one kind: named in both `pointer_fields` and `narrative_fields` for the same type is a contradiction (MILE-90/ADR-44).",
    },
    RuleMeta {
        id: RULE_POINTER_RESOLUTION,
        description: "A type's config-declared `pointer_fields` \u{222a} `narrative_fields` (MILE-90/ADR-44) resolves to a real record when it names one — a dangling reference is a finding, a resolved one is silent (`pointer.target-status` and `narrative-field.stale` separately judge what a resolved target's status means). No hardcoded field list — an undeclared type is skipped, not defaulted.",
    },
    RuleMeta {
        id: RULE_POINTER_TARGET_STATUS,
        description: "A reference resolves, but its target's `Status` is one the repository declared unacceptable (`not_in`). The optional `fields` option restricts it to the named declared pointer/narrative fields, so a dependency field (`Implements`) can be judged apart from a lineage field (`Derives-from`); an excluded field is not part of the rule's population, and a `fields` entry naming no declared field is a config error. Split from `pointer.resolution` in `MILE-80`.",
    },
    RuleMeta {
        id: RULE_HEADER_POINTER_FIELD_CLEAN,
        description: "A type's `pointer_fields` entries are clean, comma-separated references only, format-enforced (MILE-90/ADR-44) — an empty entry between commas or freeform trailing prose is flagged. Never applies to `narrative_fields`, which tolerate prose.",
    },
    RuleMeta {
        id: RULE_NARRATIVE_FIELD_STALE,
        description: "A type's config-declared `narrative_fields` pointer (e.g. a milestone's `Blocked-on`) resolves to a target whose `Status` has reached a terminal state (e.g. a cited bug is now `Fixed`) — a signal to re-examine (ADR-42, generalized beyond `Blocked-on` by MILE-90/ADR-44). `terminal_statuses` is declared per repository.",
    },
    RuleMeta {
        id: RULE_FIELD_QUALITY,
        description: "Field presence with real semantics: `blank`, `placeholder`, and `pending` are three distinct states, not one — the single most-repeated bug class in hand-written record linters. An absent field is never silently readable as any of the three.",
    },
    RuleMeta {
        id: RULE_FIELD_PENDING,
        description: "A required field is marked `Pending` — work declared unfinished, as distinct from forgotten. Split from `field.quality` in `BUG-38`.",
    },
    RuleMeta {
        id: RULE_CLAIM_STATUS_AGREEMENT,
        description: "A file outside the corpus claims to close a record whose own `Status` disagrees. Scans `claim_paths`; `closed_statuses` declared.",
    },
    RuleMeta {
        id: RULE_FILENAME_TITLE_CONSISTENCY,
        description: "The filename's display number and the H1's display number agree (ADR-3).",
    },
    RuleMeta {
        id: RULE_REVISION_LOG_CHANGE_CLASS_REQUIRED,
        description: "Every revision-log entry carries a `change_class` (ADR-14); missing or unclassified is blocking.",
    },
    RuleMeta {
        id: RULE_EMBODIMENT_CONSISTENCY,
        description: "A stated `Embodiment` agrees with the tier its `Realized-by` locators compute to (`test:` outranks `code:` outranks `spec:`); a locator that changed per git history since `Realized-by` was last touched overrides the expected value to `Drift detected` unconditionally (RFC-5 tier 1, ADR-18/ADR-32). Requires full git history — a shallow checkout makes this rule silently unable to detect anything.",
    },
    RuleMeta {
        id: RULE_EMBODIMENT_LOCATOR_EXISTS,
        description: "A `Realized-by` locator names a path that is not both git-tracked **and** present on disk, or is empty. Tracked alone passes a staged deletion; on disk alone passes a gitignored file.",
    },
    RuleMeta {
        id: RULE_EMBODIMENT_LOCATOR_PROMOTION_CANDIDATE,
        description: "The same locator cited by more than one record's `Realized-by` is a promotion candidate, surfaced as a finding, never auto-promoted (ADR-18).",
    },
    RuleMeta {
        id: RULE_RELATION_SUPERSESSION_RECIPROCITY,
        description: "If A supersedes B, B points back; status-aware (a claim only binds once the claiming record is itself terminal-accepted). Shared with `audit` (ADR-30) — `check` runs it too, not only `audit`.",
    },
    RuleMeta {
        id: RULE_CONFIG_SCOPE_MATCHES_NOTHING,
        description: "Judges the run itself, not the corpus: whether the configuration's declared rules reached what they were handed, from the other rules' own executions (`MILE-106`).",
    },
    RuleMeta {
        id: RULE_FIELD_UNTRIMMED_VALUE,
        description: "A field value carrying leading/trailing whitespace, reachable only through `yaml-frontmatter` — disclosed rather than silently trimmed.",
    },
    RuleMeta {
        id: RULE_HEADER_FIELD_CASE_MISMATCH,
        description: "A declared field written under a different case than its declaration (`RFC-40`/`ADR-58`) — the one place a case-only miss is distinguished from genuine non-adoption.",
    },
    RuleMeta {
        id: RULE_CONFIG_HEADER_NONE_HAS_NO_REQUIRED_FIELDS,
        description: "A type declaring `header_shape: none` has nowhere for a field to be, so a non-empty `required_fields`, `known_fields`, `pointer_fields`, `narrative_fields`, or `relation_fields` is a self-contradiction (`ADR-50`).",
    },
    RuleMeta {
        id: RULE_CONFIG_RELATION_FIELD_NOT_KNOWN,
        description: "Every field named in a type's `relation_fields` (`RFC-42`/`ADR-61`) must also appear in that type's own `required_fields`/`known_fields`, the same shape as `config.pointer-field-not-known`.",
    },
    RuleMeta {
        id: RULE_CONFIG_KNOWN_FIELDS_DECLARATION_MISSING,
        description: "A type declaring no `known_fields` at all gets no field-set governance from `header.field-set-consistency` (`ADR-53`'s \"declared, not voted\" default) — opt-in, so a repository can require the choice be made explicit rather than left ambiguous by omission (`RFC-43`/`ADR-62`).",
    },
    RuleMeta {
        id: RULE_RELATION_TARGET_STATUS_UNDECLARED,
        description: "A resolved pointer or narrative reference whose target type never declares `Status` at all — `ADR-60`'s gate would otherwise silently exclude it from every status-reading rule, with no disclosure of why (`RFC-45`/`ADR-63`).",
    },
    RuleMeta {
        id: RULE_FIELD_LEADING_RESERVED_INDICATOR,
        description: "A declared field's value starts with a reserved YAML indicator character (`@`, `*`, `&`, `!`, `%`, `|`, `>`), which needs quoting to parse at all — the same avoidable, decorative-prefix hazard `BUG-18` found for `Author`/`Deciders`, generalized so the next instance is caught rather than found by hand.",
    },
    RuleMeta {
        id: RULE_HEADER_CASE_VARIANT_KEY,
        description: "One record's header writes two keys differing only by case (`Status`/`status`), reported as the fact of the difference, never as which spelling is right (`MILE-113`). Opt-in and declaration-free: it reads the keys a record wrote, not a declared vocabulary, and composes beside the exact-match rules without changing them (`ADR-57`). An exact repeat of one spelling stays `header.required-fields`' finding.",
    },
    RuleMeta {
        id: RULE_RELATION_PAIR_RECIPROCITY,
        description: "For each declared `relation_pairs` entry with `reciprocity: required` (`RFC-47`, `MILE-122`): a claim `A.forward = B` is echoed by `B.inverse = A`, and a claim `A.inverse = B` by `B.forward = A`, across record types where the pair allows it. Needs `relation_pairs`; enabling it with none declared is a load error. Beside, not replacing, `relation.supersession-reciprocity`, which keeps the combined `Supersedes / Superseded-by` key.",
    },
    RuleMeta {
        id: RULE_RELATION_CROSS_TYPE_DISALLOWED,
        description: "A claim in a declared pair's field whose target is a different record type, for a pair declaring `cross_type: false` (`RFC-47`, `MILE-122`). Such a claim is excluded from `relation.pair-reciprocity` and reported here, never skipped silently.",
    },
    RuleMeta {
        id: RULE_CITATION_RESOLUTION,
        description: "A decision ID cited anywhere in a record's prose (`ADR-12`, with a configured type prefix, standing alone) resolves to an existing record, compared numerically so zero-padding does not matter (`MILE-136`). The header, which `pointer.resolution` governs, and fenced code blocks are not read; an inline code span is. Opt-in and option-free; its population counts the citations examined, so a run over none is distinguishable from a clean one.",
    },
    RuleMeta {
        id: RULE_BOUNDARY_PUBLISHED_PATH_REFERENCE,
        description: "A record identifier (any identifier carrying a configured type prefix, resolvable or not) is cited from inside a `published_paths` entry (a path prefix, normalised at config load; globs are not supported and a glob is treated literally), which publishes a pointer no external reader can resolve (`RFC-11`/`ADR-9`, `MILE-125`). An identifier split across lines is not detected. A declared path matching no tracked file is itself a finding, never a clean pass; the rule's level is the boundary's severity.",
    },
    RuleMeta {
        id: RULE_DIFF_SOURCE_REQUIRES_RECORD,
        description: "A `--base` diff changes a path under a declared `source_paths` prefix but changes no governed record (a `.md` file directly in a record type's `dir`) (`MILE-124`). One finding per changed source path, so a waiver can scope to one. It asks only that a record changed, never that it is the right one. Judges the changed set, so it reports `not-applicable` on a run with no `--base`, never clean. Deleted paths count as changed.",
    },
    RuleMeta {
        id: RULE_DIFF_SOURCE_AND_RECORDS_MIXED,
        description: "A `--base` diff changes both a governed record and a path under a declared `application_paths` prefix (less any `exclude_paths`), a prompt to split the change (`MILE-142`). One finding naming the first path of each kind, in path order. A governed record is never application source, whatever the prefixes say. Reports `not-applicable` on a run with no `--base`, never clean.",
    },
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
///
/// The diff gates report on a changed source path, which is no record at all,
/// so the diff scope's "only examined records" filter would otherwise drop
/// exactly the finding they exist to make.
pub const RULES_REPORTING_OUTSIDE_THE_CORPUS: &[&str] = &[
    RULE_CLAIM_STATUS_AGREEMENT,
    RULE_DIFF_SOURCE_REQUIRES_RECORD,
    RULE_DIFF_SOURCE_AND_RECORDS_MIXED,
];

/// Rules that need `relation_pairs` declared (`RFC-47`); enabling one with
/// none is a config load error.
pub const RULES_REQUIRING_RELATION_PAIRS: &[&str] = &[
    RULE_RELATION_PAIR_RECIPROCITY,
    RULE_RELATION_CROSS_TYPE_DISALLOWED,
];

pub const IDENTITY_DEPENDENT_RULES: &[&str] = &[
    RULE_POINTER_RESOLUTION,
    RULE_POINTER_TARGET_STATUS,
    RULE_FILENAME_TITLE_CONSISTENCY,
    RULE_RELATION_SUPERSESSION_RECIPROCITY,
    RULE_NARRATIVE_FIELD_STALE,
    // Shares `for_each_resolved_status_target`'s `RecordIndex` lookup with
    // `RULE_POINTER_TARGET_STATUS` above -- a prefix-less corpus builds an
    // empty index (`record_id` returns `None` for every record), so this
    // rule resolves nothing for exactly the same reason its sibling does
    // (`BUG-61`'s shape, missed here when the rule was added).
    RULE_RELATION_TARGET_STATUS_UNDECLARED,
    RULE_RELATION_PAIR_RECIPROCITY,
    RULE_RELATION_CROSS_TYPE_DISALLOWED,
];

#[cfg(test)]
mod tests {
    use super::*;

    /// `RULE_METADATA` feeds a generator, not a rule -- nothing else would
    /// catch it silently under- or over-reporting the rule set it's supposed
    /// to describe completely.
    #[test]
    fn every_rule_in_all_rules_has_metadata_in_the_same_order() {
        let all_rules_order: Vec<&str> = ALL_RULES.to_vec();
        let metadata_order: Vec<&str> = RULE_METADATA.iter().map(|m| m.id).collect();
        assert_eq!(
            all_rules_order, metadata_order,
            "RULE_METADATA must list every ALL_RULES entry, in the same order"
        );
        for meta in RULE_METADATA {
            assert!(
                !meta.description.trim().is_empty(),
                "{} has an empty description",
                meta.id
            );
        }
    }
}
