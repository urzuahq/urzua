//! Blank, placeholder, and pending are three distinct states, not one.
//! Collapsing them is the single most-repeated bug class this project's own
//! design draws on: a check that treats "unedited template text" the same
//! as "explicitly not applicable" the same as "empty" cannot tell an author
//! who forgot a field from one who filled it in correctly.

use crate::FieldState;

/// Literal, unedited template text -- copied verbatim from
/// `.urzua/templates/adr.md` and `.urzua/templates/rfc.md`. Exact match only
/// (case-insensitive): a real value that happens to contain "name" as a
/// substring must not be flagged.
const PLACEHOLDER_TOKENS: &[&str] = &["name", "name(s)", "yyyy-mm-dd", "tbd", "todo"];

/// Classify a header field's value. `None` (the field is absent entirely) is
/// [`FieldState::Blank`], same as an empty string -- absence and emptiness
/// are the same defect from a reader's point of view.
pub fn classify(value: Option<&str>) -> FieldState {
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

    if lower.contains("pending") || lower == "todo" {
        return FieldState::Pending;
    }

    // An em dash is this corpus's own explicit "not applicable" convention
    // (e.g. `Supersedes / Superseded-by: —` on a record nothing supersedes)
    // -- a deliberate value, not an omission. Treated as Present.
    FieldState::Present
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
        assert_eq!(classify(Some("(project lead)")), FieldState::Present);
        assert_eq!(classify(Some("username123")), FieldState::Present);
    }
}
