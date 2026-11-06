//! `.urzua/harness.lock` (`ADR-89` section 1): the bundle version, the adapters,
//! and a hash of every installed file, including the hook entries and the
//! instructions block the install merged into files a person also edits. It is
//! what `status` compares the repository with, so drift is detectable.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

pub const LOCK_PATH: &str = ".urzua/harness.lock";

/// A hook entry merged into a shared settings file.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EntryHash {
    pub file: String,
    /// The hook event, such as `PreToolUse`.
    pub event: String,
    /// The matcher of the group the entry sits in; absent for a group with none.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub matcher: Option<String>,
    /// The word after `urzua harness run`: how the entry is found again.
    pub handler: String,
    pub hash: String,
}

/// The instructions block in a shared file.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BlockHash {
    pub file: String,
    pub hash: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Lock {
    pub schema: u32,
    /// The engine version that wrote the lock (the bundle is versioned with it).
    pub bundle_version: String,
    pub adapters: Vec<String>,
    /// Whole files the install owns, by path.
    pub files: BTreeMap<String, String>,
    #[serde(default)]
    pub entries: Vec<EntryHash>,
    #[serde(default)]
    pub blocks: Vec<BlockHash>,
}

impl EntryHash {
    /// `event|matcher|handler`, for display only.
    pub fn key(&self) -> String {
        format!(
            "{}|{}|{}",
            self.event,
            self.matcher.as_deref().unwrap_or(""),
            self.handler
        )
    }
}

impl Lock {
    pub fn new(adapters: Vec<String>) -> Self {
        Lock {
            schema: 1,
            bundle_version: env!("CARGO_PKG_VERSION").to_string(),
            adapters,
            files: BTreeMap::new(),
            entries: Vec::new(),
            blocks: Vec::new(),
        }
    }

    pub fn to_text(&self) -> String {
        let mut me = self.clone();
        me.entries.sort_by(|a, b| {
            (&a.file, &a.event, &a.matcher, &a.handler)
                .cmp(&(&b.file, &b.event, &b.matcher, &b.handler))
        });
        me.blocks.sort_by(|a, b| a.file.cmp(&b.file));
        format!(
            "{}\n",
            serde_json::to_string_pretty(&me).expect("a lock serializes")
        )
    }

    pub fn parse(text: &str) -> Result<Lock, String> {
        let lock: Lock = serde_json::from_str(text)
            .map_err(|e| format!("{LOCK_PATH} is not a harness lock: {e}"))?;
        if lock.schema != 1 {
            return Err(format!(
                "{LOCK_PATH} has schema {}, which this build does not read (it reads 1)",
                lock.schema
            ));
        }
        Ok(lock)
    }
}

pub fn hash(text: &str) -> String {
    format!("sha256:{}", urzua_io::sha256_hex_of_str(text))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_lock_round_trips_and_is_written_in_a_stable_order() {
        let mut lock = Lock::new(vec!["a".into()]);
        lock.files.insert("z".into(), hash("z"));
        lock.files.insert("a".into(), hash("a"));
        let entry = |handler: &str| EntryHash {
            file: "s".into(),
            event: "E".into(),
            matcher: None,
            handler: handler.into(),
            hash: hash(handler),
        };
        lock.entries.push(entry("k2"));
        lock.entries.push(entry("k1"));
        let text = lock.to_text();
        assert!(text.ends_with("}\n"));
        let back = Lock::parse(&text).unwrap();
        assert_eq!(back.to_text(), text);
        assert_eq!(back.entries[0].handler, "k1");
    }

    #[test]
    fn a_lock_of_another_schema_or_no_lock_at_all_is_refused() {
        assert!(Lock::parse("{}").is_err());
        assert!(Lock::parse("not json").is_err());
        let other = Lock::new(vec![])
            .to_text()
            .replace("\"schema\": 1", "\"schema\": 9");
        assert!(Lock::parse(&other).unwrap_err().contains("schema 9"));
    }
}
