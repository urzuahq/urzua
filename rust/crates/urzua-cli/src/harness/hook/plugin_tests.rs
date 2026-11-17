//! The agent plugin in `plugins/<adapter>/` registers the same hooks as
//! `urzua harness install`: its hooks file is the adapter's settings fragment
//! rendered with the built-in limits and the hook timeout at the top of its range, and its commands are the bundle's
//! lifecycle commands, byte for byte. Its manifest and the marketplace entry are
//! checked against the documented shape as far as it can be checked offline.
//! `URZUA_WRITE_PLUGIN=1` rewrites the generated files instead of comparing.

use std::path::{Path, PathBuf};

use super::super::bundle::ADAPTERS;
use super::super::json::{self, J};

fn repo() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../..")
}

fn plugin_dir(adapter: &str) -> PathBuf {
    repo().join("plugins").join(adapter)
}

/// The limits the plugin's hooks are rendered with: the built-in ones, except
/// the hook timeout at the top of its range. A plugin cannot read a
/// repository's configuration, so the host's timeout must outlast any
/// `hook_timeout_seconds` a repository declares; the binary reads it and ends
/// the wait itself, with a refusal, before then.
pub(super) fn plugin_limits() -> urzua_core::harness::Limits {
    let top =
        urzua_core::harness::limits::limit_info(urzua_core::harness::limits::HOOK_TIMEOUT_SECONDS)
            .and_then(|i| i.range)
            .map(|(_, high)| high)
            .expect("the hook timeout has a range");
    urzua_core::harness::Limits::read(
        &yaml_serde::from_str(&format!("hook_timeout_seconds: {top}")).expect("yaml"),
    )
    .expect("the top of the range is a valid value")
}

fn generated(adapter: &super::super::bundle::Adapter) -> Vec<(PathBuf, String)> {
    let fragment =
        super::super::render_fragment(adapter, &plugin_limits()).expect("the fragment renders");
    let mut hooks = json::parse(&fragment)
        .expect("the fragment is JSON")
        .pretty();
    hooks.push('\n');
    let dir = plugin_dir(adapter.name);
    let mut out = vec![(dir.join("hooks/hooks.json"), hooks)];
    for (name, content) in adapter.commands.iter().filter(|(n, _)| *n != "draft.md") {
        out.push((dir.join("commands").join(name), content.to_string()));
    }
    out
}

#[test]
fn the_plugin_registers_what_the_install_does_but_with_the_timeout_at_the_top_of_its_range_because_it_cannot_read_the_repository(
) {
    for adapter in ADAPTERS.iter().filter(|a| a.manifest().plugin.is_some()) {
        for (path, want) in generated(adapter) {
            if std::env::var_os("URZUA_WRITE_PLUGIN").is_some() {
                std::fs::create_dir_all(path.parent().unwrap()).unwrap();
                std::fs::write(&path, &want).unwrap();
            }
            let have = std::fs::read_to_string(&path)
                .unwrap_or_else(|e| panic!("{}: {e}", path.display()));
            assert_eq!(
                have,
                want,
                "{} is not generated from the bundle (run the test with URZUA_WRITE_PLUGIN=1)",
                path.display()
            );
        }
        let commands: Vec<String> = std::fs::read_dir(plugin_dir(adapter.name).join("commands"))
            .unwrap()
            .map(|e| e.unwrap().file_name().to_string_lossy().to_string())
            .collect();
        assert_eq!(commands.len(), adapter.commands.len() - 1, "{commands:?}");
    }
}

fn read(path: &Path) -> J {
    json::parse(&std::fs::read_to_string(path).unwrap()).unwrap()
}

/// A plugin or marketplace name as the plugin reference allows it.
fn valid_name(name: &str) -> bool {
    !name.is_empty()
        && name
            .chars()
            .next()
            .is_some_and(|c| c.is_ascii_alphanumeric())
        && name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '-'))
        && !["claude-", "anthropic-", "anthropics-", "cc-plugin-"]
            .iter()
            .any(|p| name.starts_with(p))
}

#[test]
fn the_manifest_and_marketplace_have_the_documented_shape() {
    let market = read(&repo().join(".claude-plugin/marketplace.json"));
    let name = market.get("name").and_then(J::as_str).unwrap();
    assert!(valid_name(name));
    assert!(market
        .get("owner")
        .and_then(|o| o.get("name"))
        .and_then(J::as_str)
        .is_some());
    let Some(J::Arr(entries)) = market.get("plugins") else {
        panic!("plugins is a list")
    };
    for adapter in ADAPTERS {
        let Some(plugin) = adapter.manifest().plugin else {
            continue;
        };
        let entry = entries
            .iter()
            .find(|e| {
                e.get("source").and_then(J::as_str) == Some(&format!("./plugins/{}", adapter.name))
            })
            .expect("a marketplace entry for the plugin");
        let entry_name = entry.get("name").and_then(J::as_str).unwrap();
        assert_eq!(
            plugin.id,
            format!("{entry_name}@{name}"),
            "the id the adapter looks for in the settings"
        );
        let manifest = read(&plugin_dir(adapter.name).join(".claude-plugin/plugin.json"));
        assert_eq!(
            manifest.get("name").and_then(J::as_str),
            Some(entry_name),
            "the manifest name is the entry name"
        );
        assert!(valid_name(entry_name));
        assert!(manifest
            .get("author")
            .and_then(|a| a.get("name"))
            .and_then(J::as_str)
            .is_some());
        for key in ["homepage", "repository"] {
            if let Some(url) = manifest.get(key).and_then(J::as_str) {
                assert!(url.starts_with("https://"), "{key}");
            }
        }
        let hooks = read(&plugin_dir(adapter.name).join("hooks/hooks.json"));
        assert!(
            matches!(hooks.get("hooks"), Some(J::Obj(_))),
            "a hooks file wraps the event map in `hooks`"
        );
        assert!(
            !std::fs::read_to_string(plugin_dir(adapter.name).join("hooks/hooks.json"))
                .unwrap()
                .contains("CLAUDE_PLUGIN_ROOT"),
            "the hooks call the binary, not a file of the plugin"
        );
    }
}
