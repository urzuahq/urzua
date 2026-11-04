//! The declared `Embodiment` vocabulary (MILE-144): which state names a
//! repository's evidence computes to, in what order, and which of them assert
//! that work was built.
//!
//! The engine computes a record's state from its evidence (ADR-18): drift
//! overrides everything, otherwise the highest tier with evidence of its kind
//! wins, and no evidence at all computes to the `none` state. Which names those
//! states carry is repository configuration, not engine policy (ADR-64). A
//! repository that declares nothing gets the built-in vocabulary below, so its
//! output is unchanged.
//!
//! ```yaml
//! embodiment:
//!   drift: Drift detected
//!   tiers:                       # highest first
//!     - {state: Verified, evidence: test, claims_built: true}
//!     - {state: Implemented, evidence: code, claims_built: true}
//!     - {state: Specified, evidence: spec}
//!   none: Not started
//! ```
//!
//! `evidence` is the category of `Realized-by` locator (`test:`, `code:`,
//! `spec:`) or citing source comment that lifts a record to the tier. A state a
//! human sets that no evidence can compute (`Deprecated`) is not declared here;
//! it is the `sticky_states` option of `embodiment.consistency` (MILE-148).

use serde::{Deserialize, Serialize};

/// The category of evidence that lifts a record to a tier.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Evidence {
    Test,
    Code,
    Spec,
}

impl Evidence {
    fn name(self) -> &'static str {
        match self {
            Evidence::Test => "test",
            Evidence::Code => "code",
            Evidence::Spec => "spec",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct TierDecl {
    pub state: String,
    pub evidence: Evidence,
    /// The state asserts that the work was built, so a record whose type has
    /// not been accepted cannot state it and a record stating it with no
    /// evidence at all is judged (MILE-138, MILE-161).
    #[serde(default)]
    pub claims_built: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct EmbodimentDecl {
    /// The alarm state; overrides every tier when a cited file has drifted.
    pub drift: String,
    /// Highest tier first.
    pub tiers: Vec<TierDecl>,
    /// The state computed from no evidence.
    pub none: String,
}

impl Default for EmbodimentDecl {
    fn default() -> Self {
        let tier = |state: &str, evidence, claims_built| TierDecl {
            state: state.to_string(),
            evidence,
            claims_built,
        };
        EmbodimentDecl {
            drift: "Drift detected".to_string(),
            tiers: vec![
                tier("Verified", Evidence::Test, true),
                tier("Implemented", Evidence::Code, true),
                tier("Specified", Evidence::Spec, false),
            ],
            none: "Not started".to_string(),
        }
    }
}

impl EmbodimentDecl {
    pub fn is_default(&self) -> bool {
        *self == EmbodimentDecl::default()
    }

    /// The state evidence computes to. `has` says whether any evidence of a
    /// category exists (locators and citing comments alike).
    pub fn state_for(&self, drifted: bool, has: impl Fn(Evidence) -> bool) -> &str {
        if drifted {
            return &self.drift;
        }
        self.tiers
            .iter()
            .find(|t| has(t.evidence))
            .map_or(self.none.as_str(), |t| t.state.as_str())
    }

    /// Every state `state_for` can return, drift first.
    pub fn computed_states(&self) -> impl Iterator<Item = &str> {
        std::iter::once(self.drift.as_str()).chain(self.evidence_states())
    }

    /// The states evidence computes to when nothing has drifted: the tiers in
    /// order, then `none`.
    pub fn evidence_states(&self) -> impl Iterator<Item = &str> {
        self.tiers
            .iter()
            .map(|t| t.state.as_str())
            .chain(std::iter::once(self.none.as_str()))
    }

    pub fn is_computed(&self, state: &str) -> bool {
        self.computed_states().any(|s| s == state)
    }

    /// Whether `state` asserts the work was built.
    pub fn claims_built(&self, state: &str) -> bool {
        self.tiers
            .iter()
            .any(|t| t.claims_built && t.state == state)
    }

    pub fn is_drift(&self, state: &str) -> bool {
        self.drift == state
    }

    /// Names of the built-claiming states, for a message.
    pub fn claim_states(&self) -> Vec<&str> {
        self.tiers
            .iter()
            .filter(|t| t.claims_built)
            .map(|t| t.state.as_str())
            .collect()
    }

    /// A half-reading would be a vocabulary that silently never matches
    /// (ADR-7), so a bad declaration fails the load.
    pub fn validate(&self) -> Result<(), String> {
        if self.tiers.is_empty() {
            return Err("`tiers` must list at least one tier".into());
        }
        let mut seen: Vec<&str> = Vec::new();
        for state in self.computed_states() {
            if state.trim().is_empty() {
                return Err("a state name is blank".into());
            }
            if state != state.trim() {
                return Err(format!(
                    "state '{state}' has leading or trailing whitespace"
                ));
            }
            if seen.contains(&state) {
                return Err(format!(
                    "state '{state}' is declared twice; each of `drift`, `none` and every tier needs its own name"
                ));
            }
            seen.push(state);
        }
        let mut kinds: Vec<Evidence> = Vec::new();
        for t in &self.tiers {
            if kinds.contains(&t.evidence) {
                return Err(format!(
                    "two tiers use `evidence: {}`; a category of evidence lifts a record to one tier",
                    t.evidence.name()
                ));
            }
            kinds.push(t.evidence);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_orders_test_over_code_over_spec() {
        let d = EmbodimentDecl::default();
        let all = |_| true;
        assert_eq!(d.state_for(false, all), "Verified");
        assert_eq!(
            d.state_for(false, |e| matches!(e, Evidence::Code | Evidence::Spec)),
            "Implemented"
        );
        assert_eq!(d.state_for(false, |e| e == Evidence::Spec), "Specified");
        assert_eq!(d.state_for(false, |_| false), "Not started");
        assert_eq!(d.state_for(true, all), "Drift detected");
    }

    #[test]
    fn default_is_valid_and_claims_the_two_built_tiers() {
        let d = EmbodimentDecl::default();
        d.validate().unwrap();
        assert_eq!(d.claim_states(), ["Verified", "Implemented"]);
        assert!(d.is_default());
    }

    #[test]
    fn duplicate_names_and_evidence_are_refused() {
        let d = EmbodimentDecl {
            none: "Verified".into(),
            ..Default::default()
        };
        assert!(d.validate().unwrap_err().contains("twice"));
        let mut d = EmbodimentDecl::default();
        d.tiers[1].evidence = Evidence::Test;
        assert!(d.validate().unwrap_err().contains("evidence"));
        let mut d = EmbodimentDecl::default();
        d.tiers.clear();
        assert!(d.validate().unwrap_err().contains("tiers"));
    }
}
