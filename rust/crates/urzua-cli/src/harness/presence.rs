//! What the binary can tell, from files it can read, about how the harness is
//! wired: files an older bundle left in the repository, and whether the hooks
//! are registered by the repository's settings, by an agent plugin, by both or
//! by neither. It never claims more than a file says: a plugin enabled in a
//! settings file may still not be installed on this machine.

use std::path::{Path, PathBuf};

use super::bundle;
use super::json::J;
use super::settings;

/// The leftovers of an older bundle for `adapters`, each with the steps that
/// remove it by hand. There is no migration command (`ADR-89`, amendment of
/// 2026-11-17).
pub fn leftovers(root: &Path, adapters: &[String]) -> Vec<String> {
    let mut out = Vec::new();
    for name in adapters {
        let Some(manifest) = bundle::adapter(name).map(bundle::Adapter::manifest) else {
            continue;
        };
        if manifest.retired_files.is_empty() {
            continue;
        }
        let file = &manifest.settings_file;
        let doc = std::fs::read_to_string(root.join(file))
            .ok()
            .and_then(|t| super::json::parse(&t).ok());
        for entry in doc
            .as_ref()
            .map(|d| settings::commands_naming(d, &manifest.retired_files))
            .unwrap_or_default()
        {
            out.push(format!(
                "{file} still holds the hook entry {entry} that an older bundle wrote: it runs a script from the repository against a pinned sha256, and the script is no longer installed. Remove that entry from {file} by hand, then run `urzua harness update` (`urzua harness install` where there is no lock) to write the entry that calls the binary"
            ));
        }
        for f in &manifest.retired_files {
            if root.join(f).exists() {
                out.push(format!(
                    "{f} is left from an older bundle: nothing installs, runs or pins it now. Remove it by hand: git rm {f}"
                ));
            }
        }
    }
    out
}

/// Which registration of the hooks is in effect, as far as files show.
#[derive(serde::Serialize, Debug, PartialEq, Eq)]
pub struct Registration {
    pub adapter: String,
    /// `repository-settings`, `plugin`, `both` or `none`.
    pub in_effect: &'static str,
    /// The repository's settings file holds urzua's hook entries.
    pub repository_settings: bool,
    /// The settings file that enables the plugin, when one does.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub plugin_enabled_in: Option<String>,
    pub note: String,
}

fn expand(file: &str) -> Option<PathBuf> {
    match file.strip_prefix("~/") {
        Some(rest) => std::env::var_os("HOME").map(|h| PathBuf::from(h).join(rest)),
        None => Some(PathBuf::from(file)),
    }
}

/// The settings file that decides whether `id` is enabled, highest precedence
/// first, when one names it as enabled.
fn plugin_enabled(root: &Path, plugin: &bundle::Plugin) -> Option<String> {
    for file in &plugin.settings_files {
        let path = expand(file).map(|p| if p.is_absolute() { p } else { root.join(p) })?;
        let Some(doc) = std::fs::read_to_string(&path)
            .ok()
            .and_then(|t| super::json::parse(&t).ok())
        else {
            continue;
        };
        match doc.get(&plugin.enabled_key).and_then(|m| m.get(&plugin.id)) {
            Some(J::Bool(true)) => return Some(file.clone()),
            Some(_) => return None,
            None => {}
        }
    }
    None
}

fn has_our_entries(root: &Path, file: &str) -> bool {
    let Some(J::Obj(events)) = std::fs::read_to_string(root.join(file))
        .ok()
        .and_then(|t| super::json::parse(&t).ok())
        .and_then(|d| d.get("hooks").cloned())
    else {
        return false;
    };
    events.iter().any(|(_, groups)| match groups {
        J::Arr(groups) => groups.iter().any(|g| match g.get("hooks") {
            Some(J::Arr(hooks)) => hooks.iter().any(|h| settings::handler_of(h).is_some()),
            _ => false,
        }),
        _ => false,
    })
}

pub fn registration(root: &Path, adapter: &str) -> Option<Registration> {
    let manifest = bundle::adapter(adapter)?.manifest();
    let repository_settings = has_our_entries(root, &manifest.settings_file);
    let plugin_enabled_in = manifest
        .plugin
        .as_ref()
        .and_then(|p| plugin_enabled(root, p));
    let (in_effect, note) = match (repository_settings, &plugin_enabled_in) {
        (true, Some(_)) => ("both", "both register the same hooks; the hook judges each call once and the second registration prints the first one's answer"),
        (true, None) => ("repository-settings", "the hooks run for a session the agent opened in this repository"),
        (false, Some(_)) => ("plugin", "a settings file enables the plugin; whether the agent has it installed on this machine is not something the binary can read"),
        (false, None) => ("none", "no hook entry of urzua is in the repository's settings and no settings file this binary reads enables the plugin: no guardrail runs for an agent here"),
    };
    Some(Registration {
        adapter: adapter.to_string(),
        in_effect,
        repository_settings,
        plugin_enabled_in,
        note: note.to_string(),
    })
}
