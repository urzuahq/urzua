//! When a case may expect an empty population (`empty_population_ok_without`).
//!
//! A rule whose population is the files of one directory examines nothing once
//! a release has deleted them. The case accepts that only when no file the rule
//! would examine exists, so the condition must mirror the rule's own population
//! exactly: `claim.status-agreement` examines one claim per distinct id listed
//! in a fragment's `closes`, so a fragment that closes nothing (`closes: []`)
//! adds nothing to it. A condition wider than the rule (any `.md` file) fails a
//! pull request whose only fragment closes nothing.
//!
//! Shared by the `acceptance` runner and tested in `acceptance_population.rs`.

use std::path::Path;

/// True when some file with `extension` sits directly under `dir` of `root`
/// and, when `with_key` is given, declares that frontmatter key with a
/// non-empty list (or non-blank scalar). Such a file is one the rule examines,
/// so the population is then required to be non-empty.
pub fn population_exists(root: &Path, dir: &str, extension: &str, with_key: Option<&str>) -> bool {
    std::fs::read_dir(root.join(dir)).is_ok_and(|rd| {
        rd.flatten()
            .filter(|e| e.path().extension().is_some_and(|x| x == extension))
            .any(|e| match with_key {
                None => true,
                Some(key) => std::fs::read_to_string(e.path())
                    .is_ok_and(|text| declares_non_empty(&text, key)),
            })
    })
}

/// Whether the leading `---` frontmatter block of `text` maps `key` to a
/// non-empty value. A file with no frontmatter, or one that does not parse,
/// declares nothing.
fn declares_non_empty(text: &str, key: &str) -> bool {
    let Some(rest) = text.strip_prefix("---\n") else {
        return false;
    };
    let Some(end) = rest.find("\n---") else {
        return false;
    };
    let Ok(yaml_serde::Value::Mapping(map)) =
        yaml_serde::from_str::<yaml_serde::Value>(&rest[..end])
    else {
        return false;
    };
    match map.get(yaml_serde::Value::String(key.into())) {
        Some(yaml_serde::Value::Sequence(items)) => !items.is_empty(),
        Some(yaml_serde::Value::String(s)) => !s.trim().is_empty(),
        _ => false,
    }
}
