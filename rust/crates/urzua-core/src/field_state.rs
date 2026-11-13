//! Blank, placeholder, and pending are three distinct states, not one.
//! Collapsing them is the single most-repeated bug class this project's own
//! design draws on: a check that treats "unedited template text" the same
//! as "explicitly not applicable" the same as "empty" cannot tell an author
//! who forgot a field from one who filled it in correctly.

use crate::FieldState;

/// Literal, unedited template text -- copied verbatim from
/// `.urzua/templates/adr.md` and `.urzua/templates/rfc.md`. Exact match only
/// (case-insensitive): a real value that happens to contain "name" as a
/// substring must not be flagged. `(project lead)`/`(session author)` are
/// this project's own retired placeholder convention (BUG-5) -- used before
/// MILE-78's identity backfill, and the reason a hand-edit reverting
/// `Author` back to one of them (the ADR-38 incident) classified as
/// `Present`, indistinguishable from a real name, rather than `Placeholder`.
const PLACEHOLDER_TOKENS: &[&str] = &[
    "name",
    "name(s)",
    "yyyy-mm-dd",
    "tbd",
    "todo",
    "(project lead)",
    "(session author)",
];

/// Classify a header field's value. `None` (the field is absent entirely) is
/// [`FieldState::Blank`], same as an empty string -- absence and emptiness
/// are the same defect from a reader's point of view.
pub fn classify(value: Option<&str>) -> FieldState {
    classify_with(value, None)
}

/// [`classify`], judging "pending" by the type's declared vocabulary when it
/// declares one (BUG-65): whole values only, replacing the compiled-in
/// whole-word `pending` match. A type declaring none keeps that match, so an
/// undeclared type behaves exactly as before.
pub fn classify_with(
    value: Option<&str>,
    declared: Option<&crate::status_model::StatusDecl>,
) -> FieldState {
    let Some(value) = value else {
        return FieldState::Blank;
    };
    let trimmed = value.trim();

    if trimmed.is_empty() {
        return FieldState::Blank;
    }

    let lower = trimmed.to_ascii_lowercase();
    if PLACEHOLDER_TOKENS.contains(&lower.as_str()) || is_placeholder_reference(&lower) {
        return FieldState::Placeholder;
    }

    let pending = match declared.and_then(|d| d.is_pending_value(trimmed)) {
        Some(declared_verdict) => declared_verdict,
        None => has_pending_word(&lower) || lower == "todo",
    };
    if pending {
        return FieldState::Pending;
    }

    // An em dash is this corpus's own explicit "not applicable" convention
    // (e.g. `Supersedes / Superseded-by: —` on a record nothing supersedes)
    // -- a deliberate value, not an omission. Treated as Present.
    FieldState::Present
}

/// The compiled-in vocabulary read against the WHOLE trimmed value, case
/// insensitively: `TBD`, `TODO`, `name` and the other placeholder tokens are a
/// Placeholder, and `pending` alone is Pending. A sentence that merely contains
/// one of them ("pending the upstream fix") is `None`: that is a real value.
pub fn whole_value_default_state(value: &str) -> Option<FieldState> {
    let lower = value.trim().to_ascii_lowercase();
    if PLACEHOLDER_TOKENS.contains(&lower.as_str()) {
        Some(FieldState::Placeholder)
    } else if lower == "pending" {
        Some(FieldState::Pending)
    } else {
        None
    }
}

/// [`classify_with`] for a type whose placeholder vocabulary is its template
/// (`BUG-22`): a value identical after trimming to `template_value`, the
/// template's text for the same field, is a Placeholder. The compiled-in tokens
/// and the `nnnn` heuristic do not apply, and a value is Pending only when the
/// type's declared `pending` vocabulary says so. A field the template does not
/// carry has no template value and is never a Placeholder.
pub fn classify_against_template(
    value: Option<&str>,
    template_value: Option<&str>,
    declared: Option<&crate::status_model::StatusDecl>,
) -> FieldState {
    let Some(value) = value else {
        return FieldState::Blank;
    };
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return FieldState::Blank;
    }
    if template_value.is_some_and(|t| t.trim() == trimmed) {
        return FieldState::Placeholder;
    }
    if declared.and_then(|d| d.is_pending_value(trimmed)) == Some(true) {
        return FieldState::Pending;
    }
    FieldState::Present
}

/// The compiled-in pending reading: `pending` as a whole word (not inside
/// `spending`), in a token that is not a path. A token with a `/` is a path
/// such as `docs/pending-settlement/x.md`, which names a place and declares no
/// unfinished work. Words are delimited by anything that is not a letter or a
/// digit, so `Pending review`, `(pending)` and `Pending-ADR-5` still match.
fn has_pending_word(lower: &str) -> bool {
    lower
        .split_whitespace()
        .filter(|token| !token.contains('/'))
        .any(|token| {
            token
                .split(|c: char| !c.is_alphanumeric())
                .any(|word| word == "pending")
        })
}

/// Matches unedited placeholder references like `RFC-NNNN` or `ADR-NNNN`,
/// copied from the template's `Derives-from: RFC-NNNN (optional)` line. Four
/// literal `n`s in a row is not a real record number in any real value.
fn is_placeholder_reference(lower: &str) -> bool {
    lower.contains("nnnn")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn absent_field_is_blank() {
        assert_eq!(classify(None), FieldState::Blank);
    }

    #[test]
    fn empty_string_is_blank() {
        assert_eq!(classify(Some("")), FieldState::Blank);
        assert_eq!(classify(Some("   ")), FieldState::Blank);
    }

    #[test]
    fn unedited_template_text_is_placeholder() {
        assert_eq!(classify(Some("name")), FieldState::Placeholder);
        assert_eq!(classify(Some("YYYY-MM-DD")), FieldState::Placeholder);
        assert_eq!(classify(Some("RFC-NNNN")), FieldState::Placeholder);
    }

    #[test]
    fn explicit_pending_marker_is_pending_not_placeholder() {
        assert_eq!(classify(Some("Pending review")), FieldState::Pending);
    }

    #[test]
    fn pending_is_a_whole_word_not_a_substring_or_a_path_segment() {
        for value in [
            "docs/spending/report.md",
            "docs/pending-settlement/x.md",
            "Suspending the service",
            "spending",
        ] {
            assert_eq!(classify(Some(value)), FieldState::Present, "{value}");
        }
        for value in [
            "pending",
            "(pending)",
            "Pending-ADR-5",
            "still pending review",
        ] {
            assert_eq!(classify(Some(value)), FieldState::Pending, "{value}");
        }
    }

    #[test]
    fn em_dash_is_present_not_blank_or_placeholder() {
        // This corpus's own convention: an explicit "not applicable," not an
        // omission. Treating it as Blank would flag every ADR/RFC that has
        // never been superseded.
        assert_eq!(classify(Some("—")), FieldState::Present);
    }

    #[test]
    fn a_real_value_that_contains_a_placeholder_substring_is_not_flagged() {
        // "name" is a placeholder token; a real author value that merely
        // contains the substring must not match it.
        assert_eq!(classify(Some("username123")), FieldState::Present);
    }

    #[test]
    fn retired_project_placeholder_conventions_are_placeholder_not_present() {
        // BUG-5, observed failing before the fix: this exact value classified
        // as Present, indistinguishable from a real name -- which is why a
        // hand-edit reverting Author back to it (the ADR-38 incident) was
        // never caught by field.quality, despite field.quality running on
        // every check.
        assert_eq!(classify(Some("(project lead)")), FieldState::Placeholder);
        assert_eq!(classify(Some("(session author)")), FieldState::Placeholder);
        assert_eq!(classify(Some("(Project Lead)")), FieldState::Placeholder);
    }

    #[test]
    fn a_value_equal_to_its_template_text_is_a_placeholder() {
        let t = Some("how/where this was actually discovered");
        assert_eq!(
            classify_against_template(Some("  how/where this was actually discovered "), t, None),
            FieldState::Placeholder
        );
        assert_eq!(
            classify_against_template(Some("found in review"), t, None),
            FieldState::Present
        );
        assert_eq!(classify_against_template(None, t, None), FieldState::Blank);
        // No template value for the field: nothing to be identical to, and
        // none of the compiled-in tokens apply.
        assert_eq!(
            classify_against_template(Some("tbd"), None, None),
            FieldState::Present
        );
    }
}
