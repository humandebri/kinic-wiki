// Where: crates/wiki_search/src/lib.rs
// What: Thin adapter from wiki projection docs to ic_hybrid_engine.
// Why: Keep search indexing and lexical retrieval isolated from wiki store logic.
mod search;

pub use crate::search::WikiSearch;
