//! SPEC-164 plain adjacency graph (`eq_graph_nodes` / `eq_graph_edges`).
//!
//! Request reads and writes use these queries when `eq_graph_cutover.status`
//! is `ready`. `EDGEQUAKE_GRAPH_READ` does not move a ready ledger off these
//! tables. A flag cannot select these tables while the ledger status is `age`.
//! Serve reads do not fall back to Apache AGE once `eq_graph_*` exists.

pub(in crate::adapters::postgres::graph) mod counts;
pub(in crate::adapters::postgres::graph) mod cutover;
pub(in crate::adapters::postgres::graph) mod facets;
pub(in crate::adapters::postgres::graph) mod ledger;
pub(in crate::adapters::postgres::graph) mod read;
mod rows;
pub(in crate::adapters::postgres::graph) mod scan;
pub(in crate::adapters::postgres::graph) mod search;
pub(in crate::adapters::postgres::graph) mod write;

pub use cutover::{finish_plain_graph_cutover, CutoverReport};
pub(in crate::adapters::postgres::graph) use ledger::{
    refuse_cypher_opt_out, register_graph, tables_present,
};
