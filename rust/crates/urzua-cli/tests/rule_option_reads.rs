//! `check` reads a rule's options through the typed accessor
//! (`Config::enabled_rule_opts`), never by re-reading a `RuleSetting` field with
//! `.and_then(|s| s.<option>...)` and `unwrap_or_default()`, which cannot tell
//! an absent option from an empty one (`MILE-174`, `BUG-44`).

use urzua_core::rules::RULE_TABLE;

fn re_reads(source: &str, options: &[&str]) -> Vec<String> {
    source
        .lines()
        .filter(|l| {
            options
                .iter()
                .any(|o| l.contains(&format!("|s| s.{o}")) || l.contains(&format!("setting.{o}")))
        })
        .map(|l| l.trim().to_string())
        .collect()
}

fn all_options() -> Vec<&'static str> {
    let mut v: Vec<&str> = RULE_TABLE
        .iter()
        .flat_map(|d| d.options.iter().copied())
        .collect();
    v.sort();
    v.dedup();
    v
}

#[test]
fn check_rs_does_not_re_read_a_rule_option_field() {
    let source = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/commands/check.rs"),
    )
    .unwrap();
    let found = re_reads(&source, &all_options());
    assert!(
        found.is_empty(),
        "check.rs re-reads rule options by hand: {found:?}"
    );
}

#[test]
fn the_scan_is_observed_failing_on_a_planted_re_read() {
    let planted =
        "let p = config.rules.get(R).and_then(|s| s.claim_paths.clone()).unwrap_or_default();\n";
    assert_eq!(re_reads(planted, &all_options()).len(), 1);
}
