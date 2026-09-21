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
/// > A field name matches the declaration when it is the same name, and only
/// > then. A different spelling is a different field.
///
/// Comparing loosely would be the engine guessing which two spellings are
/// really one name, which `ADR-57` decided is not its call to make.
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
        let allowed: HashSet<String> = [declared.clone()].into_iter().collect();

        // Half the cases write the declared name back exactly and must match.
        // The other half change its case, which is now a *different* name and
        // must not: accepting it is how `Maße` and `Masse` become one field.
        let same_case = gen.below(2) == 0;
        let written = if same_case {
            declared.clone()
        } else {
            declared.to_uppercase()
        };

        let matched = matcher(&written, &allowed);
        // An uppercase form that is byte-identical (no cased characters at all,
        // e.g. `漢-漢`) is the same name, not a counterexample.
        let expected = same_case || written == declared;
        if matched != expected {
            return Some((declared, written));
        }
    }
    None
}

/// The mutation the property must reject. A suite that cannot tell this from
/// exact comparison is not testing anything.
fn case_insensitive_matcher(key: &str, allowed: &HashSet<String>) -> bool {
    allowed
        .iter()
        .any(|d| d.to_lowercase() == key.to_lowercase())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rules::field_is_declared;

    /// The mutation, run as a parameter rather than as an edit to `rules.rs`.
    /// If this ever passes, the property has stopped discriminating and every
    /// other assertion in this file is worthless.
    #[test]
    fn the_property_rejects_a_case_insensitive_matcher() {
        let found = first_counterexample(case_insensitive_matcher, 500, 0xC0FFEE);
        assert!(
            found.is_some(),
            "a case-insensitive matcher must fail the property -- it accepts a \
             spelling the adopter did not declare"
        );
    }

    /// The same property against the matcher the rules actually use.
    #[test]
    fn a_field_name_matches_its_declaration_exactly_and_only_exactly() {
        if let Some((declared, written)) = first_counterexample(field_is_declared, 500, 0xC0FFEE) {
            panic!(
                "type declared {declared:?}, record wrote {written:?}, and the \
                 matcher disagreed with exact comparison"
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
    /// Kept alongside the exact-match property rather than instead of it: this one
    /// catches a field going missing, the other catches it being misjudged, and
    /// neither implies the other.
    ///
    /// The first version of this asserted `accepted || reported`, which are
    /// exact complements -- a tautology that could not fail, inside the suite
    /// built to catch exactly that. It now predicts the finding count from the
    /// declared name and holds the parser to recovering the key.
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
                [declared.clone()].into_iter().collect::<HashSet<String>>(),
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

            // Predicted from the declaration, not from the rule's own
            // output: a prediction the rule cannot influence is the only kind
            // that can catch it doing nothing.
            let expected = usize::from(written != declared);
            assert_eq!(
                findings.len(),
                expected,
                "declared {declared:?}, wrote {written:?}: expected {expected} finding(s)"
            );
        }
    }

    /// **Every `Record`- or `Field`-unit rule must be able to report a gap.**
    ///
    /// `eligible` is the candidate list's length, built by the runner, so a rule
    /// cannot influence it. `examined` is body-reported and cannot be
    /// recomputed -- the design says so explicitly and claims assertability for
    /// `eligible` only. Nothing enforced the boundary it drew: `field.quality`
    /// and `field.pending` returned `Examined` unconditionally, so their
    /// populations were pinned to 100% and reported nothing, for as long as they
    /// had populations at all.
    ///
    /// A population that can never show a gap is not a measurement, and it is
    /// invisible to `MILE-106`, whose signal is `eligible > 0 && examined == 0`.
    /// This is the test-time half of that check: `MILE-106` asks whether a rule
    /// *did* judge nothing; this asks whether it *could* ever report having
    /// judged less than it was handed.
    ///
    /// `RecordType` rules are excluded deliberately: a declaration cannot be
    /// missing from the table it *is*, so `eligible == examined` is correct
    /// there rather than tautological. `Path` is excluded for the same reason --
    /// a filename is judged without opening anything, so every candidate gets a
    /// verdict.
    ///
    /// One adversarial record serves every rule: a filename carrying no number,
    /// a header that does not parse, and therefore no field of any kind. Each
    /// rule's type declares the fields it needs, so `eligible > 0` throughout
    /// and every gap comes from the record, never from an empty population.
    #[test]
    fn every_record_and_field_rule_can_report_examined_below_eligible() {
        use crate::config::{Config, RecordTypeConfig};
        use crate::header::HeaderLayout;
        use crate::rules;
        use std::collections::HashMap;

        let path = std::path::PathBuf::from("docs/notes/no-number-here.md");
        let body = "this is not a header at all\n";
        let record = crate::record::Record {
            path: path.clone(),
            record_type: "note".to_string(),
            type_prefix: "NOTE".to_string(),
            header: crate::header::parse(body),
        };
        let records = [record];

        let fields =
            |names: &[&str]| -> Vec<String> { names.iter().map(|s| s.to_string()).collect() };
        let declared = fields(&[
            "Status",
            "Embodiment",
            "Realized-by",
            "Supersedes / Superseded-by",
            "Derives-from",
            "Blocked-on",
        ]);

        let by_type = |v: Vec<String>| -> HashMap<String, Vec<String>> {
            [("note".to_string(), v)].into_iter().collect()
        };

        let config = Config {
            schema_version: 2,
            rules: HashMap::new(),
            record_types: [(
                "note".to_string(),
                RecordTypeConfig {
                    dir: "docs/notes".to_string(),
                    required_fields: declared.clone(),
                    header_shape: Default::default(),
                    prefix: None,
                    header_layout: Some(HeaderLayout::OnePerLine),
                    known_fields: Some(declared.clone()),
                    pointer_fields: Some(fields(&["Derives-from"])),
                    narrative_fields: Some(fields(&["Blocked-on"])),
                    spec: None,
                },
            )]
            .into_iter()
            .collect(),
        };

        let full_text: HashMap<std::path::PathBuf, String> =
            [(path.clone(), body.to_string())].into_iter().collect();
        let allowed: HashMap<String, std::collections::HashSet<String>> =
            [("note".to_string(), declared.iter().cloned().collect())]
                .into_iter()
                .collect();
        let layouts: HashMap<String, HeaderLayout> =
            [("note".to_string(), HeaderLayout::OnePerLine)]
                .into_iter()
                .collect();
        let pointers = by_type(fields(&["Derives-from"]));
        let narratives = by_type(fields(&["Blocked-on"]));
        let required = by_type(declared.clone());

        let cases: Vec<(
            &str,
            (crate::report::RuleExecution, Vec<crate::report::Finding>),
        )> = vec![
            (
                "header.required-fields",
                rules::header_required_fields(&records, &required),
            ),
            (
                "header.layout-consistency",
                rules::header_layout_consistency(&records, &layouts),
            ),
            (
                "header.field-set-consistency",
                rules::header_field_set_consistency(&records, &allowed),
            ),
            (
                "header.pointer-field-clean",
                rules::header_pointer_field_clean(&records, &config),
            ),
            ("field.quality", rules::field_quality(&records, &required)),
            ("field.pending", rules::field_pending(&records, &required)),
            (
                "filename.title-consistency",
                rules::filename_title_consistency(&records, &full_text),
            ),
            (
                "revision-log.change-class-required",
                rules::revision_log_change_class(&records, &full_text),
            ),
            ("identity.collision", rules::identity_collision(&records)),
            (
                "pointer.resolution",
                rules::pointer_resolution(&records, &pointers, &narratives),
            ),
            (
                "pointer.target-status",
                rules::pointer_target_status(&records, &pointers, &narratives, &[]),
            ),
            (
                "narrative-field.stale",
                rules::narrative_field_stale(&records, &config, &[]),
            ),
            (
                "relation.supersession-reciprocity",
                rules::supersession_reciprocity(&records, &config),
            ),
            (
                "embodiment.consistency",
                rules::embodiment_consistency(&records, &config, &Default::default()),
            ),
            (
                "embodiment.locator-exists",
                rules::embodiment_locator_exists(&records, &config, &|_| true),
            ),
            (
                "embodiment.locator-promotion-candidate",
                rules::embodiment_locator_promotion_candidate(&records, &config),
            ),
        ];

        let mut tautological = Vec::new();
        for (id, (exec, _)) in cases {
            let population = exec
                .population
                .unwrap_or_else(|| panic!("{id} reported no population"));
            assert!(
                population.eligible() > 0,
                "{id}: the fixture must hand it candidates, or the gap below is vacuous"
            );
            if population.examined() >= population.eligible() {
                tautological.push(format!(
                    "{id} ({}/{})",
                    population.eligible(),
                    population.examined()
                ));
            }
        }

        assert!(
            tautological.is_empty(),
            "these rules judged every candidate of a record with no parseable header, \
             no filename number and no fields -- their populations cannot report a gap, \
             so MILE-106 can never see them: {tautological:?}"
        );
    }

    /// A seed reproduces its counterexample exactly, or a failure report is not
    /// evidence anyone can act on.
    #[test]
    fn a_seed_reproduces_its_counterexample() {
        let a = first_counterexample(case_insensitive_matcher, 500, 0xC0FFEE);
        let b = first_counterexample(case_insensitive_matcher, 500, 0xC0FFEE);
        assert_eq!(a, b);
        assert!(a.is_some());
    }
}
