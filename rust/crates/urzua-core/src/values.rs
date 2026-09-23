//! Newtypes for values drawn from the adopter's declared vocabulary
//! (`RFC-39`/`ADR-59`): the comparison rule lives in the type's `Eq`/`Hash`,
//! not rediscovered at each call site.

/// A field name as the adopter wrote it. Compares exactly (`ADR-57`) via the
/// derived `Eq`/`Hash` on the wrapped string -- no custom comparison logic is
/// written here. The type's value is that a bare `&str` can no longer be
/// substituted at a comparison site without an explicit `.as_str()`, which is
/// what makes a loose comparison (`eq_ignore_ascii_case`, `to_lowercase`)
/// visible in review instead of silent.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct FieldName(String);

impl FieldName {
    pub fn new(s: impl Into<String>) -> Self {
        FieldName(s.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for FieldName {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<&str> for FieldName {
    fn from(s: &str) -> Self {
        FieldName(s.to_string())
    }
}

impl From<String> for FieldName {
    fn from(s: String) -> Self {
        FieldName(s)
    }
}

impl std::borrow::Borrow<str> for FieldName {
    fn borrow(&self) -> &str {
        &self.0
    }
}

/// A record identifier (`ADR-34`), compared by numeric value (`BUG-2`):
/// `ADR-0034` and `ADR-34` are one id. Only the normalized form is kept --
/// there is no un-normalized form to accidentally compare, which is what
/// made `normalize_id` a discipline to remember at thirteen call sites
/// instead of a fact the type enforces. `Display` shows the normalized form,
/// matching what `identity.collision`'s finding message already showed
/// before this type existed (it formatted the map key, which was always
/// normalized) -- so introducing this type changes no output.
#[derive(Debug, Clone)]
pub struct RecordId {
    normalized: String,
}

impl RecordId {
    pub fn new(raw: &str) -> Self {
        RecordId {
            normalized: normalize(raw),
        }
    }

    pub fn as_str(&self) -> &str {
        &self.normalized
    }
}

impl PartialEq for RecordId {
    fn eq(&self, other: &Self) -> bool {
        self.normalized == other.normalized
    }
}

impl Eq for RecordId {}

impl PartialOrd for RecordId {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for RecordId {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.normalized.cmp(&other.normalized)
    }
}

impl std::hash::Hash for RecordId {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.normalized.hash(state);
    }
}

impl std::fmt::Display for RecordId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.normalized)
    }
}

impl std::borrow::Borrow<str> for RecordId {
    fn borrow(&self) -> &str {
        &self.normalized
    }
}

/// Numeric-value equality for an id/reference like `ADR-0034` or `ADR-34`
/// (`BUG-2`): strips the numeric part's leading zeros so a filename's
/// padding and a hand-typed reference's padding never have to match exactly.
/// Falls back to the original string unchanged if the numeric part doesn't
/// parse (defensive only -- callers already validate theirs).
///
/// Splits at the *last* hyphen, not the first: a multi-segment prefix like
/// `DOC-ADR` (`BUG-111`/`BUG-114`, already supported by
/// `parse_record_filename`/`is_record_reference`) puts the number after
/// every prefix hyphen, not just the first one -- splitting on the first
/// hyphen left `DOC-ADR-02` and `DOC-ADR-2` normalizing to two different,
/// unequal strings instead of the same id.
fn normalize(id: &str) -> String {
    match id.rsplit_once('-') {
        Some((prefix, number)) => match number.parse::<u64>() {
            Ok(n) => format!("{prefix}-{n}"),
            Err(_) => id.to_string(),
        },
        None => id.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn field_name_compares_exactly() {
        assert_eq!(FieldName::from("Status"), FieldName::from("Status"));
        assert_ne!(FieldName::from("Status"), FieldName::from("status"));
    }

    #[test]
    fn record_id_compares_by_numeric_value() {
        assert_eq!(RecordId::new("ADR-0034"), RecordId::new("ADR-34"));
        assert_ne!(RecordId::new("ADR-34"), RecordId::new("ADR-35"));
    }

    #[test]
    fn record_id_display_shows_the_normalized_form() {
        assert_eq!(RecordId::new("ADR-0034").to_string(), "ADR-34");
    }

    #[test]
    fn record_id_falls_back_to_the_original_string_when_unparseable() {
        assert_eq!(
            RecordId::new("not-an-id-at-all").to_string(),
            "not-an-id-at-all"
        );
    }

    /// `BUG-111`/`BUG-114`: a multi-segment prefix like `DOC-ADR` is already
    /// a supported shape elsewhere (`parse_record_filename`,
    /// `is_record_reference`). The number always follows the *last* hyphen,
    /// so a zero-padded and unpadded reference to the same such record must
    /// still normalize equal.
    #[test]
    fn record_id_compares_by_numeric_value_with_a_hyphenated_prefix_observed_failing() {
        assert_eq!(RecordId::new("DOC-ADR-02"), RecordId::new("DOC-ADR-2"));
    }
}
