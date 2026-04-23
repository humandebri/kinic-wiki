// Where: crates/vfs_cli_app/src/connection.rs
// What: Compatibility re-export for the legacy wiki_cli connection module path.
// Why: Shared VFS connection resolution now lives in vfs_cli.
pub use vfs_cli::connection::*;
