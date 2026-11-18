//! One module per command: each owns its own logic and its own
//! `Report`-implementing type(s), next to the function that builds them.

pub mod audit;
pub mod board;
mod board_assets;
mod board_page;
pub mod check;
pub(crate) mod config_lines;
pub mod doctor;
pub(crate) mod evidence_log;
pub mod explain;
pub mod explain_types;
pub mod export;
pub mod fix;
pub(crate) mod fix_conflicts;
pub(crate) mod fix_pin;
pub mod graph;
pub mod hooks;
pub mod hooks_verify;
pub mod index;
pub mod init;
pub mod init_greenfield;
pub mod mcp;
mod merge_pins;
pub mod migrate;
pub mod migrate_filenames;
pub mod new;
mod new_refs;
pub mod refs;
pub mod reverify;
pub mod roadmap;
pub mod rules;
pub mod waive;
