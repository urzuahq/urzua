//! One module per command: each owns its own logic and its own
//! `Report`-implementing type(s), next to the function that builds them.

pub mod audit;
pub mod check;
pub mod doctor;
pub mod explain;
pub mod export;
pub mod fix;
pub mod graph;
pub mod index;
pub mod init;
pub mod mcp;
pub mod migrate;
pub mod new;
mod new_refs;
pub mod refs;
pub mod reverify;
pub mod roadmap;
pub mod rules;
pub mod waive;
