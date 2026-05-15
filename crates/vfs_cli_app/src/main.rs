// Where: crates/vfs_cli_app/src/main.rs
// What: Binary entrypoint for the single published kinic-vfs-cli executable.
// Why: Wiki operations and Skill Registry operations share connection, identity, and DB selection.
use anyhow::{Result, bail};
use clap::Parser;
use vfs_cli::cli::IdentityModeArg;
use vfs_cli::commands::{print_database_current, run_database_unlink};
use vfs_cli::connection::{
    ResolvedConnection, resolve_connection, resolve_connection_optional_canister,
};
use vfs_cli_app::cli::{Cli, Command, DatabaseCommand};
use vfs_cli_app::commands::run_command;
use vfs_cli_app::identity::load_default_icp_identity;
use vfs_client::{CanisterVfsClient, VfsApi};

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    if let Command::Database { command } = &cli.command {
        match command {
            DatabaseCommand::Current { json } => {
                let connection = resolve_connection_optional_canister(
                    cli.connection.local,
                    cli.connection.replica_host.clone(),
                    cli.connection.canister_id.clone(),
                    cli.connection.database_id.clone(),
                )?;
                print_database_current(&connection, *json)?;
                return Ok(());
            }
            DatabaseCommand::Unlink => {
                run_database_unlink()?;
                return Ok(());
            }
            _ => {}
        }
    }
    let connection = resolve_connection(
        cli.connection.local,
        cli.connection.replica_host.clone(),
        cli.connection.canister_id.clone(),
        cli.connection.database_id.clone(),
    )?;
    let client =
        client_for_command(&cli.command, cli.connection.identity_mode, &connection).await?;
    run_command(&client, cli, &connection).await
}

async fn client_for_command(
    command: &Command,
    identity_mode: IdentityModeArg,
    connection: &ResolvedConnection,
) -> Result<CanisterVfsClient> {
    match identity_mode {
        IdentityModeArg::Identity => signed_client(connection).await,
        IdentityModeArg::Anonymous => {
            if command.requires_identity() {
                bail!("--identity-mode anonymous cannot run commands that require identity");
            }
            anonymous_client(connection).await
        }
        IdentityModeArg::Auto => {
            if command.requires_identity() || command.auto_uses_identity_without_target_database() {
                return signed_client(connection).await;
            }
            let Some(database_id) = connection.database_id.as_deref() else {
                return anonymous_client(connection).await;
            };
            if !command.uses_target_database_read() {
                return anonymous_client(connection).await;
            }
            let anonymous = anonymous_client(connection).await?;
            if anonymous.status(database_id).await.is_err() {
                return signed_client(connection).await;
            }
            let signed = signed_client(connection).await?;
            let databases = signed.list_databases().await.map_err(|error| {
                anyhow::anyhow!("failed to check selected identity database membership: {error}")
            })?;
            if databases
                .iter()
                .any(|database| database.database_id == database_id)
            {
                Ok(signed)
            } else {
                Ok(anonymous)
            }
        }
    }
}

async fn anonymous_client(connection: &ResolvedConnection) -> Result<CanisterVfsClient> {
    CanisterVfsClient::new(&connection.replica_host, &connection.canister_id).await
}

async fn signed_client(connection: &ResolvedConnection) -> Result<CanisterVfsClient> {
    let identity = load_default_icp_identity().await?;
    CanisterVfsClient::new_with_boxed_identity(
        &connection.replica_host,
        &connection.canister_id,
        identity,
    )
    .await
}
