//! SPEC-164 plain adjacency graph (`eq_graph_nodes` / `eq_graph_edges`).
//!
//! Request reads use these queries only when `eq_graph_cutover.status = ready`.
//! `EDGEQUAKE_GRAPH_READ` does not move a ready ledger back onto Apache AGE.
//! A flag cannot select plain reads while the ledger is still `age`.

pub(in crate::adapters::postgres::graph) mod counts;
pub(in crate::adapters::postgres::graph) mod cutover;
pub(in crate::adapters::postgres::graph) mod ledger;
pub(in crate::adapters::postgres::graph) mod read;
mod rows;
pub(in crate::adapters::postgres::graph) mod scan;
pub(in crate::adapters::postgres::graph) mod search;
pub(in crate::adapters::postgres::graph) mod write;

pub use cutover::{finish_plain_graph_cutover, CutoverReport};
pub use ledger::graph_cutover_is_ready;
pub(in crate::adapters::postgres::graph) use ledger::{
    refuse_cypher_opt_out, register_graph, tables_present,
};
