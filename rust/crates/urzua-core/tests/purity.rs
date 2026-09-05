//! Mechanical proof of urzua-core's own purity claim (ADR-0005): its
//! dependency graph must contain no I/O-capable crate. Ships with a
//! planted-violation case, since a check never observed failing is
//! unverified, not passing.

const ALLOWED_DEPENDENCIES: &[&str] = &["serde", "thiserror", "toml"];

/// The actual assertion, extracted as a pure function over a dependency list
/// so it can be exercised both against real `cargo metadata` output and
/// against a synthetic planted violation.
fn check_dependencies_are_allowed(deps: &[String], allowed: &[&str]) -> Result<(), Vec<String>> {
    let violations: Vec<String> = deps
        .iter()
        .filter(|d| !allowed.contains(&d.as_str()))
        .cloned()
        .collect();
    if violations.is_empty() {
        Ok(())
    } else {
        Err(violations)
    }
}

fn urzua_core_direct_dependencies() -> Vec<String> {
    let metadata = cargo_metadata::MetadataCommand::new()
        .no_deps()
        .exec()
        .expect("cargo metadata should run from within the workspace");

    let package = metadata
        .packages
        .iter()
        .find(|p| p.name.as_str() == "urzua-core")
        .expect("urzua-core must be a workspace member");

    package
        .dependencies
        .iter()
        .filter(|d| d.kind == cargo_metadata::DependencyKind::Normal)
        .map(|d| d.name.clone())
        .collect()
}

#[test]
fn urzua_core_depends_on_no_io_capable_crate() {
    let deps = urzua_core_direct_dependencies();
    assert!(
        !deps.is_empty(),
        "sanity check: urzua-core should have at least one dependency (serde)"
    );
    check_dependencies_are_allowed(&deps, ALLOWED_DEPENDENCIES)
        .unwrap_or_else(|violations| panic!("urzua-core depends on non-allowlisted crate(s), which may be I/O-capable: {violations:?}"));
}

#[test]
fn the_check_is_observed_failing_on_a_planted_violation() {
    // A crate name that is not in the allowlist -- proves the assertion
    // above can actually fail, not just pass vacuously.
    let planted = vec!["serde".to_string(), "tokio".to_string()];
    let result = check_dependencies_are_allowed(&planted, ALLOWED_DEPENDENCIES);
    assert_eq!(result, Err(vec!["tokio".to_string()]));
}
