// Where: crates/wiki_store/src/hashing.rs
// What: Stable hashing and normalization helpers for sections and rendered pages.
// Why: Revision diffs and system page etags depend on deterministic text normalization.
use sha2::{Digest, Sha256};

pub fn normalize_text(text: &str) -> String {
    text.lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .collect::<Vec<_>>()
        .join("\n")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

pub fn sha256_hex(text: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(text.as_bytes());
    hex::encode(hasher.finalize())
}
