//! `harness.limits` (`ADR-101` amendment of 2026-11-15, `SPEC-3`): the numbers
//! that bound how much of one agent call the harness reads, and the editor hook's
//! timeout.
//!
//! Every input cap is a security floor: its built-in value is both the default
//! and the minimum, so configuration may raise it but a lower value is a load
//! error. What lies beyond a cap is unchanged by raising it: it is an edit that
//! was not judged, handled by the guardrail's `on_unjudged`.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// One key of `harness.limits`.
pub struct LimitInfo {
    pub key: &'static str,
    /// The built-in value: the default, and for a floor also the minimum.
    pub default: u64,
    /// `Some((min, max))` for a plainly configurable value; `None` for a
    /// security floor, which takes any value from `default` up.
    pub range: Option<(u64, u64)>,
    /// A floor's ceiling, where raising it far enough would bring back the
    /// failure the floor guards against.
    pub ceiling: Option<u64>,
    pub what: &'static str,
}

pub const HOOK_TIMEOUT_SECONDS: &str = "hook_timeout_seconds";
pub const PATHS_PER_CALL: &str = "paths_per_call";
pub const RECORDS_PER_CALL: &str = "records_per_call";
pub const LOOSE_STRINGS_PER_CALL: &str = "loose_strings_per_call";
pub const INPUT_DEPTH: &str = "input_depth";
pub const PATH_LENGTH: &str = "path_length";
pub const STRING_LENGTH: &str = "string_length";
pub const SYMLINK_HOPS: &str = "symlink_hops";
pub const JSON_DEPTH: &str = "json_depth";
pub const MCP_LINE_BYTES: &str = "mcp_line_bytes";
pub const GATE_WALK_FILES: &str = "gate_walk_files";

/// Every key `harness.limits` reads.
pub const LIMITS: &[LimitInfo] = &[
    LimitInfo {
        key: HOOK_TIMEOUT_SECONDS,
        default: 120,
        range: Some((10, 600)),
        ceiling: None,
        what: "seconds the agent waits for the editor hook",
    },
    LimitInfo {
        key: PATHS_PER_CALL,
        default: 512,
        range: None,
        ceiling: None,
        what: "files of one call judged against the gates",
    },
    LimitInfo {
        key: RECORDS_PER_CALL,
        default: 8,
        range: None,
        ceiling: None,
        what: "record-checking writes of one MCP call judged",
    },
    LimitInfo {
        key: LOOSE_STRINGS_PER_CALL,
        default: 512,
        range: None,
        ceiling: None,
        what: "other strings of one MCP call looked at as possible paths",
    },
    LimitInfo {
        key: INPUT_DEPTH,
        default: 24,
        range: None,
        // The translator's scan recurses per level under Python's default
        // recursion limit of 1000; 256 keeps it far below that.
        ceiling: Some(256),
        what: "nesting levels of one MCP call's input read",
    },
    LimitInfo {
        key: PATH_LENGTH,
        default: 4096,
        range: None,
        ceiling: None,
        what: "characters of a string read as a path",
    },
    LimitInfo {
        key: STRING_LENGTH,
        default: 65536,
        range: None,
        ceiling: None,
        what: "characters of a string looked into for paths",
    },
    LimitInfo {
        key: SYMLINK_HOPS,
        default: 40,
        range: None,
        ceiling: None,
        what: "symbolic links followed resolving one path",
    },
    LimitInfo {
        key: JSON_DEPTH,
        default: 128,
        range: None,
        // The parser, writer and drop recurse per level; 1024 levels stay far
        // inside the smallest thread stack urzua runs on.
        ceiling: Some(1024),
        what: "nesting levels of a settings file read",
    },
    LimitInfo {
        key: MCP_LINE_BYTES,
        default: 1024 * 1024,
        range: None,
        ceiling: None,
        what: "bytes of one request line `urzua mcp` reads",
    },
    LimitInfo {
        key: GATE_WALK_FILES,
        default: 50_000,
        range: None,
        ceiling: None,
        what: "files walked under the path-gate entries",
    },
];

pub fn limit_info(key: &str) -> Option<&'static LimitInfo> {
    LIMITS.iter().find(|l| l.key == key)
}

/// `harness.limits` as declared. Only declared keys are held, so a
/// configuration that declares none round-trips to none.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Limits {
    declared: BTreeMap<&'static str, u64>,
}

impl Limits {
    /// The effective value of `key`: the declared value, else the built-in one.
    ///
    /// # Panics
    /// When `key` is not one of [`LIMITS`], which is a programming error.
    pub fn get(&self, key: &str) -> u64 {
        let info = limit_info(key).unwrap_or_else(|| panic!("unknown harness limit '{key}'"));
        self.declared.get(info.key).copied().unwrap_or(info.default)
    }

    /// [`Self::get`] as a `usize`, saturating on a narrower platform.
    pub fn get_usize(&self, key: &str) -> usize {
        usize::try_from(self.get(key)).unwrap_or(usize::MAX)
    }

    pub fn is_empty(&self) -> bool {
        self.declared.is_empty()
    }

    /// Every key with its effective value, in [`LIMITS`] order.
    pub fn effective(&self) -> Vec<(&'static str, u64)> {
        LIMITS.iter().map(|l| (l.key, self.get(l.key))).collect()
    }

    /// Reads the `harness.limits` mapping. An unknown key, a non-integer, a
    /// floor's value below its floor and a value outside a range are errors.
    pub fn read(v: &yaml_serde::Value) -> Result<Self, String> {
        let map = v
            .as_mapping()
            .ok_or("harness.limits must be a mapping of limit names to whole numbers")?;
        let mut declared = BTreeMap::new();
        for (k, val) in map {
            let name = k.as_str().unwrap_or("?");
            let Some(info) = limit_info(name) else {
                let hint = super::nearest_name(name, LIMITS.iter().map(|l| l.key))
                    .map(|valid| format!("; did you mean '{valid}'?"))
                    .unwrap_or_default();
                return Err(format!(
                    "harness.limits names '{name}', which this build does not know{hint} (it reads: {})",
                    LIMITS.iter().map(|l| l.key).collect::<Vec<_>>().join(", ")
                ));
            };
            let n = val
                .as_u64()
                .ok_or_else(|| format!("harness.limits.{name} must be a whole number"))?;
            match info.range {
                Some((lo, hi)) if n < lo || n > hi => {
                    return Err(format!(
                        "harness.limits.{name} is {n}; it must be from {lo} to {hi}"
                    ))
                }
                None if n < info.default => {
                    return Err(format!(
                        "harness.limits.{name} is {n}, below its security floor of {}: configuration may raise it but never lower it",
                        info.default
                    ))
                }
                None if info.ceiling.is_some_and(|c| n > c) => {
                    return Err(format!(
                        "harness.limits.{name} is {n}, above its ceiling of {}: deeper recursion risks a stack overflow",
                        info.ceiling.unwrap_or_default()
                    ))
                }
                _ => {}
            }
            declared.insert(info.key, n);
        }
        Ok(Limits { declared })
    }
}

impl<'de> Deserialize<'de> for Limits {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let value = yaml_serde::Value::deserialize(deserializer)?;
        Limits::read(&value).map_err(serde::de::Error::custom)
    }
}

impl Serialize for Limits {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        self.declared.serialize(s)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn read(yaml: &str) -> Result<Limits, String> {
        Limits::read(&yaml_serde::from_str(yaml).unwrap())
    }

    #[test]
    fn nothing_declared_is_every_built_in_value() {
        let l = Limits::default();
        let built_in: Vec<(&str, u64)> = vec![
            ("hook_timeout_seconds", 120),
            ("paths_per_call", 512),
            ("records_per_call", 8),
            ("loose_strings_per_call", 512),
            ("input_depth", 24),
            ("path_length", 4096),
            ("string_length", 65536),
            ("symlink_hops", 40),
            ("json_depth", 128),
            ("mcp_line_bytes", 1_048_576),
            ("gate_walk_files", 50_000),
        ];
        assert_eq!(l.effective(), built_in);
    }

    #[test]
    fn each_key_raised_reads_back_raised() {
        for info in LIMITS {
            let raised = info
                .range
                .map(|(_, hi)| hi)
                .or(info.ceiling)
                .unwrap_or(info.default * 2);
            let l = read(&format!("{}: {raised}", info.key)).unwrap();
            assert_eq!(l.get(info.key), raised, "{}", info.key);
        }
    }

    #[test]
    fn each_floor_below_its_floor_is_an_error_naming_key_and_floor() {
        for info in LIMITS.iter().filter(|l| l.range.is_none()) {
            let e = read(&format!("{}: {}", info.key, info.default - 1)).unwrap_err();
            assert!(e.contains(&format!("harness.limits.{}", info.key)), "{e}");
            assert!(
                e.contains(&format!("security floor of {}", info.default)),
                "{e}"
            );
            assert!(read(&format!("{}: {}", info.key, info.default)).is_ok());
        }
    }

    #[test]
    fn the_hook_timeout_takes_its_range_only() {
        assert_eq!(
            read("hook_timeout_seconds: 30")
                .unwrap()
                .get(HOOK_TIMEOUT_SECONDS),
            30
        );
        for bad in ["9", "601", "-1", "ten", "1.5"] {
            assert!(
                read(&format!("hook_timeout_seconds: {bad}")).is_err(),
                "{bad}"
            );
        }
    }

    #[test]
    fn an_unknown_key_is_an_error_suggesting_the_nearest() {
        let e = read("symlink_hop: 50").unwrap_err();
        assert!(
            e.contains("'symlink_hop'") && e.contains("did you mean 'symlink_hops'?"),
            "{e}"
        );
        assert!(read("[1, 2]").is_err());
    }

    #[test]
    fn a_depth_above_its_ceiling_is_an_error() {
        for (key, ceiling) in [(INPUT_DEPTH, 256), (JSON_DEPTH, 1024)] {
            assert!(read(&format!("{key}: {ceiling}")).is_ok());
            let e = read(&format!("{key}: {}", ceiling + 1)).unwrap_err();
            assert!(e.contains(&format!("ceiling of {ceiling}")), "{e}");
        }
    }
}
