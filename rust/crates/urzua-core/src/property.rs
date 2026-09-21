//! `SPEC-4`'s acceptance suite, first slice (`MILE-101`).
//!
//! One property, generated over a curated alphabet, run twice: against the
//! matcher the rules use and against a deliberately narrowed one that must
//! fail it. The mutation is a *parameter*, never an edit to shipping code, so
//! the suite proves it can fail without anyone having to remember to break the
//! engine by hand and put it back.
//!
//! This is a slice, not the milestone. `SPEC-4` also specifies fixture corpora
//! and a `manifest.yaml`; neither is here. What is here is the shape the rest
//! should follow: a stated property, an input space wide enough to contain the
//! cases a real corpus has, and a demonstration that a wrong implementation is
//! caught.
//!
//! The generator is hand-rolled rather than drawn from a property-testing
//! library. A dependency needs an ADR under the `ADR-17`/`ADR-21` precedent,
//! and a library's main value here would be shrinking, which a fixed alphabet
//! and short strings do not need. The trigger for revisiting that is the first
//! case whose counterexample is too large to read.

use std::collections::HashSet;

/// Characters chosen because each one has broken a case-insensitive comparison
/// somewhere: a Latin letter with an accent in both cases, the German sharp s
/// (whose uppercase is two characters), the Turkish dotted capital I (whose
/// lowercase is two code points), a CJK character with no case at all, a
/// combining mark that makes two visually identical strings differ, and an RTL
/// character that reorders display without changing bytes.
const ALPHABET: &[char] = &[
    'a', 'B', 'z', '-', '_', 'é', 'É', 'ß', 'İ', '漢', '\u{0301}', '\u{05D0}',
];

/// A deterministic, seeded generator. Reproducibility matters more than
/// statistical quality here: a failing case has to be re-runnable from its seed
/// alone, or the report is not evidence.
struct Gen(u64);

impl Gen {
    fn next(&mut self) -> u64 {
        // xorshift64*. Small, no dependency, and adequate for picking indices.
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        x.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }

    fn below(&mut self, n: usize) -> usize {
        (self.next() % n as u64) as usize
    }

    fn field_name(&mut self) -> String {
        let len = 1 + self.below(6);
        (0..len)
            .map(|_| ALPHABET[self.below(ALPHABET.len())])
            .collect()
    }
}

/// The property, stated once and checked against whatever matcher is passed in.
///
/// > A field name the config declared is matched by the same name a record
/// > writes, whatever case the author used.
///
/// This is the claim `fold_field_name` exists to make good on. It is *not* the
/// "accepted or reported, never neither" property the plan for this slice
/// stated -- that one is vacuous against a `bool` and is checked against the
/// rule instead, in `every_field_a_record_carries_is_either_accepted_or_reported`.
///
/// Returns the first counterexample rather than panicking, so the same function
/// serves the passing check and the mutation check.
fn first_counterexample(
    matcher: impl Fn(&str, &HashSet<String>) -> bool,
    cases: usize,
    seed: u64,
) -> Option<(String, String)> {
    let mut gen = Gen(seed);

    for _ in 0..cases {
        let declared = gen.field_name();
        let allowed: HashSet<String> = [crate::rules::fold_field_name(&declared)]
            .into_iter()
            .collect();

        // The record writes the same name the config declared, in whatever case
        // the author happened to use. A corpus does this constantly: `Status`
        // declared, `status` written.
        let written = if gen.below(2) == 0 {
            declared.clone()
        } else {
            declared.to_uppercase()
        };

        if !matcher(&written, &allowed) {
            return Some((declared, written));
        }
    }
    None
}

/// The narrowed matcher the mutation test runs against: it compares raw bytes,
/// dropping the case fold. A suite that cannot tell this from the real matcher
/// is not testing anything.
fn case_sensitive_matcher(key: &str, allowed: &HashSet<String>) -> bool {
    allowed.contains(key)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rules::field_is_declared;

    /// The mutation, run as a parameter rather than as an edit to `rules.rs`.
    /// If this ever passes, the property has stopped discriminating and every
    /// other assertion in this file is worthless.
    #[test]
    fn the_property_rejects_a_matcher_that_forgot_to_fold_case() {
        let found = first_counterexample(case_sensitive_matcher, 500, 0xC0FFEE);
        assert!(
            found.is_some(),
            "a matcher comparing raw bytes must fail the property -- \
             if it passes, the property is not testing the fold"
        );
    }

    /// The same property against the matcher the rules actually use.
    ///
    /// **Expected to fail today, and that is the point** (`BUG-97`): the fold is
    /// `to_ascii_lowercase`, so `É` does not fold to `é` and a field the config
    /// declared is reported undeclared. Marked `should_panic` so the suite is
    /// green while the defect stands and turns red the moment it is fixed --
    /// at which point this attribute comes off and the assertion stands alone.
    #[test]
    #[should_panic(expected = "BUG-97")]
    fn a_declared_field_name_is_always_matched_by_the_name_a_record_writes() {
        if let Some((declared, written)) = first_counterexample(field_is_declared, 500, 0xC0FFEE) {
            panic!(
                "BUG-97: type declared {declared:?}, record wrote {written:?}, \
                 and the rule reports it undeclared -- the fold is ASCII-only"
            );
        }
    }

    /// The generator has to actually reach the characters the property exists
    /// for. A curated alphabet that never emits its interesting half would make
    /// every assertion above pass for the wrong reason.
    #[test]
    fn the_generator_reaches_its_non_ascii_alphabet() {
        let mut gen = Gen(0xC0FFEE);
        let produced: String = (0..500).map(|_| gen.field_name()).collect();
        for ch in ['é', 'É', 'ß', 'İ', '漢', '\u{0301}', '\u{05D0}'] {
            assert!(
                produced.contains(ch),
                "the generator never produced {ch:?} in 500 names"
            );
        }
    }

    /// The plan for this slice stated the property as *"any generated field
    /// name is accepted as declared or reported as undeclared; never neither"*.
    /// At the matcher level that is vacuous -- a `bool` is one or the other by
    /// construction -- so it only has content against the rule, where "neither"
    /// means the field was silently dropped from the report.
    ///
    /// Kept alongside the fold property rather than instead of it: this one
    /// catches a field going missing, the other catches it being misjudged, and
    /// neither implies the other.
    ///
    /// The first version of this asserted `accepted || reported`, which are
    /// exact complements -- a tautology that could not fail, inside the suite
    /// built to catch exactly that. It now predicts the finding count from the
    /// fold and holds the parser to recovering the key.
    #[test]
    fn every_field_a_record_carries_is_either_accepted_or_reported() {
        use crate::rules::header_field_set_consistency;
        use std::collections::HashMap;

        let mut gen = Gen(0xA11CE);
        for _ in 0..200 {
            let declared = gen.field_name();
            let written = gen.field_name();

            let mut allowed_by_type = HashMap::new();
            allowed_by_type.insert(
                "note".to_string(),
                [crate::rules::fold_field_name(&declared)]
                    .into_iter()
                    .collect::<HashSet<String>>(),
            );

            let body = format!("> {written}: x\n");
            let record = crate::record::Record {
                path: std::path::PathBuf::from("docs/notes/0001-x.md"),
                record_type: "note".to_string(),
                type_prefix: "NOTE".to_string(),
                header: crate::header::parse(&body),
            };

            // The parser must recover the key before the rule can be held to
            // anything about it. Excusing the unparsed case instead is what
            // made the first version of this assertion unfailable.
            assert_eq!(
                record.header.fields.len(),
                1,
                "declared {declared:?}, wrote {written:?}: the parser did not \
                 recover the generated field"
            );
            assert_eq!(record.header.fields[0].key, written);

            let (_, findings) = header_field_set_consistency(&[record], &allowed_by_type);

            // Predicted from the fold the rule uses, not from the rule's own
            // output: a prediction the rule cannot influence is the only kind
            // that can catch it doing nothing.
            let expected = usize::from(
                crate::rules::fold_field_name(&written) != crate::rules::fold_field_name(&declared),
            );
            assert_eq!(
                findings.len(),
                expected,
                "declared {declared:?}, wrote {written:?}: expected {expected} finding(s)"
            );
        }
    }

    /// A seed reproduces its counterexample exactly, or a failure report is not
    /// evidence anyone can act on.
    #[test]
    fn a_seed_reproduces_its_counterexample() {
        let a = first_counterexample(case_sensitive_matcher, 500, 0xC0FFEE);
        let b = first_counterexample(case_sensitive_matcher, 500, 0xC0FFEE);
        assert_eq!(a, b);
        assert!(a.is_some());
    }
}
