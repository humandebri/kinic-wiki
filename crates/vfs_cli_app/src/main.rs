// Where: crates/vfs_cli_app/src/main.rs
// What: Binary entrypoint for the agent-facing wiki CLI.
// Why: Agents need one executable that can read remote pages and sync the local mirror.
use anyhow::Result;
use clap::Parser;
use std::env;
use std::path::PathBuf;
use vfs_cli::connection::resolve_connection;
use vfs_cli_app::cli::Cli;
use vfs_cli_app::commands::run_command;
use vfs_client::CanisterVfsClient;

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    let connection = resolve_connection(cli.connection.local, cli.connection.canister_id.clone())?;
    let identity_pem = cli
        .connection
        .identity_pem
        .clone()
        .or_else(|| env::var_os("VFS_IDENTITY_PEM").map(PathBuf::from));
    let identity_configured = identity_pem.is_some();
    let client = CanisterVfsClient::new_with_identity(
        &connection.replica_host,
        &connection.canister_id,
        identity_pem.as_deref(),
    )
    .await?;
    run_command(&client, cli)
        .await
        .map_err(|error| explain_anonymous_path_policy_error(error, identity_configured))
}

fn explain_anonymous_path_policy_error(
    error: anyhow::Error,
    identity_configured: bool,
) -> anyhow::Error {
    if !identity_configured && path_policy_access_denied(&error) {
        eprintln!(
            "hint: no identity PEM was configured; restricted path policy calls use anonymous principal 2vxsx-fae. Set --identity-pem or VFS_IDENTITY_PEM."
        );
    }
    error
}

fn path_policy_access_denied(error: &anyhow::Error) -> bool {
    let message = error.to_string().to_ascii_lowercase();
    message.contains("path policy") && message.contains("access denied")
}

#[cfg(test)]
mod tests {
    use super::path_policy_access_denied;

    #[test]
    fn detects_path_policy_access_denied_errors() {
        let error = anyhow::anyhow!("path policy access denied: Reader role required");
        assert!(path_policy_access_denied(&error));
    }
}
