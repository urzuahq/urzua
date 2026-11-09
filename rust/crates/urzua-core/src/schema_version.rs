//! Config schema versions: the declared support window and the one upgrade
//! step each release carries (`ADR-77`, `RFC-49`, `SPEC-3`).
//!
//! A release reads the current version `N` and the one before it, `N-1`. A
//! config at `N-1` is upgraded in memory to `N` before any rule runs
//! (`upgrade`), and `urzua migrate config` writes the same upgrade to the file
//! (`migrate_text`). Anything outside the window is a structured error
//! (`SchemaVersionError`). There is no chain of steps: the one step below is
//! the only one in the code, and the next break replaces it.
//!
//! Pure: no I/O. Callers hand in text and write the result.

use crate::config::CURRENT_SCHEMA_VERSION;
use serde::Serialize;
use yaml_serde::{Mapping, Value};

/// The oldest version this build reads.
pub const OLDEST_SUPPORTED_SCHEMA_VERSION: u32 = CURRENT_SCHEMA_VERSION - 1;

/// The command that rewrites an `N-1` config to `N`.
pub const MIGRATE_COMMAND: &str = "urzua migrate config";

/// The rule that reports an upgraded config. Each upgrade step enables it, so
/// a config the next release finds at `N-1` is reported, not silently read.
pub const OUTDATED_RULE: &str = crate::rules::RULE_CONFIG_SCHEMA_VERSION_OUTDATED;

/// The versions this build reads, oldest first.
pub fn supported_versions() -> Vec<u32> {
    (OLDEST_SUPPORTED_SCHEMA_VERSION..=CURRENT_SCHEMA_VERSION).collect()
}

/// One release that reads YAML configs: the versions it reads and the one it
/// writes. The source of `SPEC-3`'s "Supported schema versions" table (a test
/// keeps the two equal) and of the `migration` a rejected version is pointed
/// to.
pub struct Release {
    pub release: &'static str,
    pub reads: &'static [u32],
    pub writes: u32,
}

/// Every release that read `.urzua/config.yaml`, oldest first. A release that
/// read only the TOML config (`0.3.0` and earlier, `ADR-52`) is not listed:
/// nothing since reads that file, so no command migrates it (`ADR-54`).
pub const RELEASES: &[Release] = &[Release {
    release: "0.4.0",
    reads: &[1, 2],
    writes: 2,
}];

/// Where to go from a version outside the window.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Migration {
    /// The oldest release that still reads `found`; `None` when none does.
    pub release: Option<String>,
    /// The command to run with that release; `None` when none applies.
    pub command: Option<String>,
    /// What to do, in a sentence.
    pub note: String,
}

/// A config whose `schema_version` is outside the window (`ADR-77`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct SchemaVersionError {
    pub found: u64,
    pub supported: Vec<u32>,
    pub current: u32,
    pub migration: Migration,
}

impl SchemaVersionError {
    pub fn new(found: u64) -> Self {
        SchemaVersionError {
            found,
            supported: supported_versions(),
            current: CURRENT_SCHEMA_VERSION,
            migration: migration_for(found),
        }
    }
}

impl std::fmt::Display for SchemaVersionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let supported: Vec<String> = self.supported.iter().map(u32::to_string).collect();
        write!(
            f,
            "schema_version {} is outside what this build of urzua reads (supported: {}; current: {}) -- {}",
            self.found,
            supported.join(", "),
            self.current,
            self.migration.note
        )
    }
}

fn migration_for(found: u64) -> Migration {
    if found > u64::from(CURRENT_SCHEMA_VERSION) {
        return Migration {
            release: None,
            command: None,
            note: format!(
                "this config is newer than this build; upgrade urzua to a release that reads schema_version {found}"
            ),
        };
    }
    let reader = RELEASES
        .iter()
        .find(|r| u32::try_from(found).is_ok_and(|v| r.reads.contains(&v)));
    match reader {
        Some(r) => Migration {
            release: Some(r.release.to_string()),
            command: Some(MIGRATE_COMMAND.to_string()),
            note: format!(
                "install urzua {} and run `{MIGRATE_COMMAND}` to reach schema_version {}; repeat with the release that reads that version until the config is at a version this build reads",
                r.release, r.writes
            ),
        },
        None => Migration {
            release: None,
            command: None,
            note: format!("no release of urzua reads schema_version {found}"),
        },
    }
}

/// What the upgrade did to one key.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum ChangeAction {
    /// A key the new version needs, written with the value that keeps the
    /// config's behaviour.
    Added,
    /// A key whose value changed in place.
    Changed,
    Renamed,
    Moved,
    /// A key the new version no longer has.
    Dropped,
}

/// One key the upgrade touched, in the order it was.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Change {
    pub action: ChangeAction,
    /// A dotted path from the top of the document.
    pub key: String,
    /// The new path, for a rename or a move.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub to: Option<String>,
    /// The value written, for an added or changed key.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value: Option<String>,
    pub reason: String,
}

/// That a config was read through the upgrade, kept on the loaded `Config` so
/// `config.schema-version-outdated` can report it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UpgradeInfo {
    pub from: u32,
    pub changes: Vec<Change>,
}

/// How a rule of the previous version is carried into this one.
enum Carry {
    /// Enabled at this level.
    Level(&'static str),
    /// Enabled at a level, and `.1` enabled beside it at its own: the rule was
    /// split in two and the halves keep the severities they had.
    Split(&'static str, &'static str, &'static str),
    /// Not carried: the reason.
    Gone(&'static str),
}

/// The rules `schema_version: 1` ran and the level each ran at. A version-1
/// config had no `rules` table: every rule ran, at a severity the rule body
/// chose. Version 2 (`MILE-80`) made every rule opt-in with a declared level,
/// so a config that was not to change behaviour has to name what it ran. The
/// levels are the version-1 build's own (its `rules.rs` before `MILE-80`).
const V1_RULES: &[(&str, Carry)] = &[
    ("header.required-fields", Carry::Level("error")),
    (
        "header.layout-consistency",
        Carry::Gone("the rule was removed along with the `header_layout` key it read"),
    ),
    ("header.field-set-consistency", Carry::Level("warn")),
    ("type.no-declared-spec", Carry::Level("warn")),
    (
        "header.deprecated-shape",
        Carry::Gone("the rule was removed with the header shapes it reported (blockquote and bold-list)"),
    ),
    ("config.pointer-declaration-missing", Carry::Level("error")),
    ("config.pointer-field-not-known", Carry::Level("error")),
    ("config.pointer-narrative-overlap", Carry::Level("error")),
    (
        // Reported every reference that resolved as a warning and every
        // dangling one as an error. Version 2 reports only the dangling ones.
        "pointer.resolution",
        Carry::Level("error"),
    ),
    ("header.pointer-field-clean", Carry::Level("warn")),
    (
        "narrative-field.stale",
        Carry::Gone("version 2 requires the `terminal_statuses` option, which a version-1 config had no place to declare; add the rule with its statuses"),
    ),
    (
        "field.quality",
        Carry::Split("error", "field.pending", "warn"),
    ),
    ("filename.title-consistency", Carry::Level("error")),
    ("revision-log.change-class-required", Carry::Level("error")),
    ("embodiment.consistency", Carry::Level("warn")),
    (
        "embodiment.locator-promotion-candidate",
        Carry::Level("warn"),
    ),
    ("relation.supersession-reciprocity", Carry::Level("error")),
];

/// Upgrades a parsed `schema_version: 1` document to version 2 and lists what
/// it did. Pure and total on a document that is a version-1 config: the
/// typed parse that follows is what judges the result.
pub fn upgrade(mut doc: Value) -> Result<(Value, Vec<Change>), String> {
    let root = doc
        .as_mapping_mut()
        .ok_or_else(|| "the config is not a mapping".to_string())?;
    let rules_key = Value::String("rules".into());
    if root.contains_key(&rules_key) {
        return Err(
            "a schema_version 1 config has no `rules` table; it is a schema_version 2 key"
                .to_string(),
        );
    }
    refuse_omitted_header_shape(root)?;
    let mut changes = Vec::new();

    root.insert(
        Value::String("schema_version".into()),
        Value::Number(CURRENT_SCHEMA_VERSION.into()),
    );
    changes.push(Change {
        action: ChangeAction::Changed,
        key: "schema_version".to_string(),
        to: None,
        value: Some(CURRENT_SCHEMA_VERSION.to_string()),
        reason: "the config is now written in the current schema".to_string(),
    });

    drop_header_layouts(root, &mut changes);
    root.insert(rules_key, Value::Mapping(version_2_rules(&mut changes)));
    Ok((doc, changes))
}

/// A version-1 build read an omitted `header_shape` as `blockquote`; the
/// default is `yaml-frontmatter` now and `blockquote` is no longer read
/// (`MILE-85`, `ADR-54`: a break is a break). Upgrading such a type would
/// change what it means without saying so, so the upgrade refuses.
fn refuse_omitted_header_shape(root: &Mapping) -> Result<(), String> {
    let Some(types) = root
        .get(Value::String("record_types".into()))
        .and_then(Value::as_mapping)
    else {
        return Ok(());
    };
    let shape = Value::String("header_shape".into());
    let omitted: Vec<String> = types
        .iter()
        .filter(|(_, ty)| ty.as_mapping().is_some_and(|m| !m.contains_key(&shape)))
        .map(|(name, _)| format!("record_types.{}", name.as_str().unwrap_or("?")))
        .collect();
    if omitted.is_empty() {
        return Ok(());
    }
    Err(format!(
        "{} omit{} `header_shape`, which a version-1 build read as `blockquote`; that shape was removed in MILE-85 and the default is now `yaml-frontmatter`, so the upgrade would change what the config means. Rewrite the type's records as YAML frontmatter and declare `header_shape: yaml-frontmatter` (or `none` for a type with no header), then run again",
        omitted.join(", "),
        if omitted.len() == 1 { "s" } else { "" },
    ))
}

/// Removes every record type's `header_layout`, which no rule reads.
fn drop_header_layouts(root: &mut Mapping, changes: &mut Vec<Change>) {
    let Some(types) = root
        .get_mut(Value::String("record_types".into()))
        .and_then(Value::as_mapping_mut)
    else {
        return;
    };
    let layout = Value::String("header_layout".into());
    for (name, ty) in types.iter_mut() {
        let Some(ty) = ty.as_mapping_mut() else {
            continue;
        };
        if ty.remove(&layout).is_some() {
            let name = name.as_str().unwrap_or("?");
            changes.push(Change {
                action: ChangeAction::Dropped,
                key: format!("record_types.{name}.header_layout"),
                to: None,
                value: None,
                reason: "no rule reads it any more; `header.layout-consistency` was removed"
                    .to_string(),
            });
        }
    }
}

/// Names a rule in the version-2 `rules` table, and says so.
fn enable_rule(
    rules: &mut Mapping,
    id: &str,
    level: &str,
    reason: &str,
    changes: &mut Vec<Change>,
) {
    rules.insert(Value::String(id.into()), Value::String(level.into()));
    changes.push(Change {
        action: ChangeAction::Added,
        key: format!("rules.{id}"),
        to: None,
        value: Some(level.to_string()),
        reason: reason.to_string(),
    });
}

/// The `rules` table carrying every rule a version-1 build ran, at the
/// severity it ran it.
fn version_2_rules(changes: &mut Vec<Change>) -> Mapping {
    let mut rules = Mapping::new();
    let ran = "a version-1 build ran this rule at this severity; a version-2 config runs only the rules it names";
    for (id, carry) in V1_RULES {
        match carry {
            Carry::Level(level) => enable_rule(&mut rules, id, level, ran, changes),
            Carry::Split(level, other, other_level) => {
                enable_rule(&mut rules, id, level, ran, changes);
                enable_rule(
                    &mut rules,
                    other,
                    other_level,
                    "the rule was split out of the one above; a version-1 build reported this half at this severity",
                    changes,
                );
            }
            Carry::Gone(reason) => changes.push(Change {
                action: ChangeAction::Dropped,
                key: format!("rules.{id}"),
                to: None,
                value: None,
                reason: (*reason).to_string(),
            }),
        }
    }
    changes.push(Change {
        action: ChangeAction::Dropped,
        key: "rules.pointer.target-status".to_string(),
        to: None,
        value: None,
        reason: "version 1 announced the status of every reference that resolved, which version 2 no longer does; the rule now needs the `not_in` statuses a repository declares"
            .to_string(),
    });
    enable_rule(
        &mut rules,
        OUTDATED_RULE,
        "warn",
        "reports a config read through an upgrade; enabled so the next release reports this one",
        changes,
    );
    rules
}

/// The result of rewriting a version-1 config text.
#[derive(Debug)]
pub struct Migrated {
    pub text: String,
    pub changes: Vec<Change>,
}

/// Rewrites the text of a `schema_version: 1` config to version 2, keeping its
/// comments and layout: it edits lines rather than re-serializing, then checks
/// that the edited text parses to exactly the document `upgrade` computed, so
/// a rewrite that would not load as intended is refused, never written.
pub fn migrate_text(content: &str) -> Result<Migrated, String> {
    let doc: Value = yaml_serde::from_str(content).map_err(|e| e.to_string())?;
    let (upgraded, changes) = upgrade(doc)?;

    let mut lines: Vec<String> = content.lines().map(str::to_string).collect();
    let version_line = lines
        .iter()
        .position(|l| is_schema_version_line(l, OLDEST_SUPPORTED_SCHEMA_VERSION))
        .ok_or_else(|| {
            "the `schema_version` line is not a plain top-level `schema_version: 1`; edit the file by hand".to_string()
        })?;
    let comment = lines[version_line]
        .find('#')
        .map(|at| format!(" {}", &lines[version_line][at..]))
        .unwrap_or_default();
    lines[version_line] = format!("schema_version: {CURRENT_SCHEMA_VERSION}{comment}");

    let dropped_layouts = changes
        .iter()
        .filter(|c| c.key.ends_with(".header_layout"))
        .count();
    let before = lines.len();
    lines.retain(|l| !is_header_layout_line(l));
    if before - lines.len() != dropped_layouts {
        return Err(
            "a `header_layout` key is not a single-line value; edit the file by hand".to_string(),
        );
    }

    let mut block = vec!["rules:".to_string()];
    for c in changes.iter().filter(|c| c.action == ChangeAction::Added) {
        let id = c.key.strip_prefix("rules.").unwrap_or(&c.key);
        block.push(format!("  {id}: {}", c.value.as_deref().unwrap_or("warn")));
    }
    let at = lines
        .iter()
        .position(|l| is_schema_version_line(l, CURRENT_SCHEMA_VERSION))
        .map_or(0, |i| i + 1);
    for (offset, line) in block.into_iter().enumerate() {
        lines.insert(at + offset, line);
    }

    let mut text = lines.join("\n");
    if content.ends_with('\n') {
        text.push('\n');
    }
    let reread: Value = yaml_serde::from_str(&text).map_err(|e| {
        format!("the rewritten config would not parse ({e}); edit the file by hand")
    })?;
    if reread != upgraded {
        return Err(
            "the rewritten config would differ from the upgrade; edit the file by hand".to_string(),
        );
    }
    Ok(Migrated { text, changes })
}

fn is_schema_version_line(line: &str, version: u32) -> bool {
    let Some(rest) = line.strip_prefix("schema_version:") else {
        return false;
    };
    let value = rest.split('#').next().unwrap_or("").trim();
    value == version.to_string()
}

fn is_header_layout_line(line: &str) -> bool {
    let trimmed = line.trim_start();
    line.len() != trimmed.len()
        && trimmed.starts_with("header_layout:")
        && !trimmed["header_layout:".len()..].trim().is_empty()
}

#[cfg(test)]
mod tests {
    use super::*;

    const V1: &str = "# my config\nschema_version: 1   # the version\nrecord_types:\n  adr:\n    dir: docs/adr\n    # layout\n    header_shape: yaml-frontmatter\n    header_layout: one-per-line\n    required_fields: [Status]\n";

    #[test]
    fn the_window_is_the_current_version_and_the_one_before() {
        assert_eq!(
            supported_versions(),
            vec![CURRENT_SCHEMA_VERSION - 1, CURRENT_SCHEMA_VERSION]
        );
    }

    #[test]
    fn a_version_above_the_window_points_at_a_newer_urzua() {
        let e = SchemaVersionError::new(99);
        assert_eq!(e.found, 99);
        assert_eq!(e.current, CURRENT_SCHEMA_VERSION);
        assert_eq!(e.migration.release, None);
        assert!(e.migration.note.contains("upgrade urzua"));
    }

    #[test]
    fn a_version_no_release_reads_says_so() {
        let e = SchemaVersionError::new(0);
        assert_eq!(e.migration.release, None);
        assert_eq!(e.migration.command, None);
        assert!(e.migration.note.contains("no release"));
    }

    #[test]
    fn a_version_an_older_release_reads_names_that_release_and_the_command() {
        let m = migration_for(1);
        assert_eq!(m.release.as_deref(), Some("0.4.0"));
        assert_eq!(m.command.as_deref(), Some(MIGRATE_COMMAND));
    }

    #[test]
    fn every_carried_rule_is_a_shipped_rule() {
        for (id, carry) in V1_RULES {
            let ids: Vec<&str> = match carry {
                Carry::Level(_) => vec![id],
                Carry::Split(_, other, _) => vec![id, other],
                Carry::Gone(_) => vec![],
            };
            for id in ids {
                assert!(
                    crate::rules::ALL_RULES.contains(&id),
                    "{id} is carried into version 2 but this build ships no such rule"
                );
            }
        }
    }

    #[test]
    fn the_upgrade_names_what_a_version_one_build_ran() {
        let doc: Value = yaml_serde::from_str(V1).unwrap();
        let (up, changes) = upgrade(doc).unwrap();
        let rules = up["rules"].as_mapping().unwrap();
        assert_eq!(rules["field.quality"], "error");
        assert_eq!(rules["field.pending"], "warn");
        assert_eq!(rules["header.field-set-consistency"], "warn");
        assert_eq!(rules[OUTDATED_RULE], "warn");
        assert!(!rules.contains_key("narrative-field.stale"));
        assert!(changes
            .iter()
            .any(|c| c.action == ChangeAction::Dropped && c.key == "rules.narrative-field.stale"));
        assert!(changes.iter().any(
            |c| c.action == ChangeAction::Dropped && c.key == "record_types.adr.header_layout"
        ));
        assert!(up["record_types"]["adr"]
            .as_mapping()
            .unwrap()
            .get("header_layout")
            .is_none());
    }

    #[test]
    fn a_version_one_type_that_omits_header_shape_is_refused_not_silently_changed() {
        // A version-1 build read an omitted `header_shape` as `blockquote`;
        // today's default is `yaml-frontmatter`, so upgrading it would change
        // the meaning of the config without saying so.
        let doc: Value = yaml_serde::from_str(
            "schema_version: 1\nrecord_types:\n  adr:\n    dir: docs/adr\n    required_fields: [Status]\n  rfc:\n    dir: docs/rfc\n    header_shape: yaml-frontmatter\n",
        )
        .unwrap();
        let err = upgrade(doc).unwrap_err();
        assert!(err.contains("record_types.adr"), "{err}");
        assert!(err.contains("header_shape"), "{err}");
        assert!(err.contains("blockquote"), "{err}");
        assert!(err.contains("MILE-85"), "{err}");
        assert!(!err.contains("record_types.rfc"), "{err}");
    }

    #[test]
    fn a_version_one_type_that_declares_header_shape_upgrades() {
        for shape in ["yaml-frontmatter", "none"] {
            let doc: Value = yaml_serde::from_str(&format!(
                "schema_version: 1\nrecord_types:\n  adr:\n    dir: docs/adr\n    header_shape: {shape}\n"
            ))
            .unwrap();
            assert!(upgrade(doc).is_ok(), "{shape}");
        }
    }

    #[test]
    fn a_version_one_config_with_a_rules_table_is_refused() {
        let doc: Value =
            yaml_serde::from_str("schema_version: 1\nrecord_types: {}\nrules: {}\n").unwrap();
        assert!(upgrade(doc).unwrap_err().contains("no `rules` table"));
    }

    #[test]
    fn the_text_rewrite_keeps_comments_and_matches_the_upgrade() {
        let m = migrate_text(V1).unwrap();
        assert!(m
            .text
            .starts_with("# my config\nschema_version: 2 # the version\nrules:\n"));
        assert!(m.text.contains("    # layout\n"));
        assert!(!m.text.contains("header_layout"));
        assert!(m.text.ends_with('\n'));
        let again: Value = yaml_serde::from_str(&m.text).unwrap();
        assert_eq!(again["schema_version"], 2);
    }

    fn effective(config: &crate::config::Config) -> serde_json::Value {
        serde_json::to_value(config).unwrap()
    }

    #[test]
    fn a_version_one_config_loads_to_the_same_effective_config_as_its_migration() {
        let in_memory = crate::config::parse(V1).unwrap();
        assert_eq!(in_memory.upgraded_from.as_ref().unwrap().from, 1);
        assert_eq!(in_memory.schema_version, CURRENT_SCHEMA_VERSION);

        let written = migrate_text(V1).unwrap().text;
        let from_file = crate::config::parse(&written).unwrap();
        assert!(from_file.upgraded_from.is_none());
        assert_eq!(effective(&in_memory), effective(&from_file));
    }

    #[test]
    fn rules_only_ever_see_the_current_shape() {
        let c = crate::config::parse(V1).unwrap();
        assert_eq!(
            c.rules["field.quality"].level,
            crate::config::RuleLevel::Error
        );
        assert_eq!(c.rules[OUTDATED_RULE].level, crate::config::RuleLevel::Warn);
    }

    #[test]
    fn a_version_outside_the_window_is_a_structured_error_whatever_else_is_wrong() {
        for found in [0u64, 3, 99] {
            let yaml = format!(
                "schema_version: {found}\nrecord_types:\n  adr:\n    dir: docs/adr\n    nonsense: true\n"
            );
            match crate::config::parse(&yaml).unwrap_err() {
                crate::config::ConfigError::SchemaVersionOutOfWindow(e) => {
                    assert_eq!(e.found, found);
                    assert_eq!(e.supported, supported_versions());
                    assert_eq!(e.current, CURRENT_SCHEMA_VERSION);
                }
                other => panic!("{other}"),
            }
        }
    }

    #[test]
    fn a_config_with_no_integer_version_is_still_a_parse_error_naming_the_field() {
        for yaml in [
            "record_types: {}\n",
            "schema_version: two\nrecord_types: {}\n",
            "schema_version: -1\nrecord_types: {}\n",
        ] {
            let e = crate::config::parse(yaml).unwrap_err();
            assert!(
                matches!(e, crate::config::ConfigError::Parse(_)),
                "{yaml}: {e}"
            );
        }
    }

    #[test]
    fn a_flow_style_schema_version_is_refused_not_guessed() {
        let e = migrate_text("{schema_version: 1, record_types: {}}\n").unwrap_err();
        assert!(e.contains("by hand"), "{e}");
    }

    #[test]
    fn a_multi_line_header_layout_is_refused_not_half_removed() {
        let text = "schema_version: 1\nrecord_types:\n  adr:\n    dir: docs/adr\n    header_shape: yaml-frontmatter\n    header_layout:\n      - a\n";
        let e = migrate_text(text).unwrap_err();
        assert!(e.contains("by hand"), "{e}");
    }
}
