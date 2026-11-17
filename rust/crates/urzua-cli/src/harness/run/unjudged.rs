//! What decides an edit when the engine gives no verdict (it could not be run,
//! crashed, or answered with something that is not a verdict): the same inputs
//! the engine would have read. Every path is placed as the engine places it
//! (made absolute against the request's directory, its links resolved), and
//! every configuration that governs it is found as the engine finds it (at or
//! above the file, as written and as resolved, and the session's own). Then one
//! rule: the edit goes through only when every configuration found is read with
//! certainty as letting an unjudged edit through, and silently only when there
//! is none at all.

use std::path::{Path, PathBuf};

use super::{absolute, resolve_links};

#[derive(Debug, PartialEq, Eq)]
pub(crate) enum Unjudged {
    /// No configuration governs any of the paths, the directory or the project.
    NotGoverned,
    /// Every configuration found lets an unjudged edit through.
    Open,
    /// A configuration refuses an unjudged edit, cannot be read, or holds a
    /// value of a shape the rule does not accept; or a path cannot be placed.
    Closed,
}

fn nearest_config(dir: &Path) -> Option<PathBuf> {
    dir.ancestors()
        .map(crate::discovery::default_config_path)
        .find(|p| p.exists())
}

fn configs_holding(path: &Path, out: &mut Vec<PathBuf>) {
    for a in path.ancestors() {
        let c = crate::discovery::default_config_path(a);
        if c.exists() && !out.contains(&c) {
            out.push(c);
        }
    }
}

/// The decision for an edit of `paths` from `cwd` in the session of `project`.
pub(crate) fn unjudged_policy(cwd: &Path, project: Option<&str>, paths: &[String]) -> Unjudged {
    let mut configs: Vec<PathBuf> = Vec::new();
    for dir in std::iter::once(cwd.to_path_buf()).chain(project.map(PathBuf::from)) {
        if let Some(c) = nearest_config(&dir) {
            if !configs.contains(&c) {
                configs.push(c);
            }
        }
    }
    for p in paths {
        let abs = absolute(cwd, p);
        let Some(resolved) = resolve_links(&abs) else {
            return Unjudged::Closed;
        };
        configs_holding(&abs, &mut configs);
        configs_holding(&resolved, &mut configs);
    }
    if configs.is_empty() {
        return Unjudged::NotGoverned;
    }
    if configs.iter().all(|c| config_open(c)) {
        Unjudged::Open
    } else {
        Unjudged::Closed
    }
}

fn config_open(path: &Path) -> bool {
    std::fs::read_to_string(path)
        .ok()
        .and_then(|t| {
            yaml_serde::from_str::<yaml_serde::Value>(urzua_core::config::strip_bom(&t)).ok()
        })
        .is_some_and(|doc| open(&doc))
}

/// A configuration lets an unjudged edit through only when neither
/// `acceptance` nor `protected-paths` is enabled without `on_unjudged: allow`
/// (their default being deny), and `editor-hook` is not enabled with an
/// `on_unjudged` other than `allow` (its default). A guardrail is off only when
/// `enabled` is absent, null or `false`; `on_unjudged` allows only as exactly
/// the string `allow`. The hook command's text fallback follows the same rule.
pub(crate) fn open(doc: &yaml_serde::Value) -> bool {
    use yaml_serde::Value as V;
    let guardrails = match doc.get("harness") {
        None | Some(V::Null) => return true,
        Some(h @ V::Mapping(_)) => match h.get("guardrails") {
            None | Some(V::Null) => return true,
            Some(g @ V::Mapping(_)) => g,
            Some(_) => return false,
        },
        Some(_) => return false,
    };
    let rule = |name: &str, allow_when_unset: bool| {
        let g = match guardrails.get(name) {
            None | Some(V::Null) => return true,
            Some(g @ V::Mapping(_)) => g,
            Some(_) => return false,
        };
        let off = matches!(
            g.get("enabled"),
            None | Some(V::Null) | Some(V::Bool(false))
        );
        let allows = match g.get("on_unjudged") {
            None | Some(V::Null) => allow_when_unset,
            Some(V::String(s)) => s == "allow",
            Some(_) => false,
        };
        off || allows
    };
    rule("acceptance", false) && rule("protected-paths", false) && rule("editor-hook", true)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_configuration_is_open_only_when_it_parses_and_says_so() {
        let doc = |t: &str| yaml_serde::from_str::<yaml_serde::Value>(t).unwrap();
        for (text, want) in [
            (
                "harness:\n  guardrails:\n    acceptance:\n      enabled: true\n",
                false,
            ),
            (
                "harness:\n  guardrails:\n    acceptance: {enabled: true, on_unjudged: 'allow'}\n",
                true,
            ),
            (
                "harness:\n  guardrails:\n    protected-paths:\n      enabled: false\n",
                true,
            ),
            (
                "harness:\n  guardrails:\n    protected-paths: {enabled: 'yes'}\n",
                false,
            ),
            (
                "harness:\n  guardrails:\n    editor-hook: {enabled: true}\n",
                true,
            ),
            (
                "harness:\n  guardrails:\n    editor-hook: {enabled: true, on_unjudged: deny}\n",
                false,
            ),
            ("schema_version: 2\n", true),
        ] {
            assert_eq!(open(&doc(text)), want, "{text}");
        }
    }
}
