//! One module per command: each owns its own logic and its own
//! `Report`-implementing type(s), next to the function that builds them.

pub mod audit;
pub mod check;
pub mod doctor;
pub mod explain;
pub mod fix;
pub mod graph;
pub mod init;
pub mod migrate;
pub mod new;
