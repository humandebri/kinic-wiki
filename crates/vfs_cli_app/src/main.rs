// Where: crates/vfs_cli_app/src/main.rs
// What: Binary entrypoint for the agent-facing wiki CLI.
// Why: Agents need one executable that can read remote pages and sync the local mirror.
use anyhow::Result;
use clap::Parser;
use vfs_cli::commands::{print_database_current, run_database_unlink};
use vfs_cli::connection::{resolve_connection, resolve_connection_optional_canister};
use vfs_cli_app::cli::{Cli, Command, DatabaseCommand};
use vfs_cli_app::commands::run_command;
use vfs_client::CanisterVfsClient;

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    if let Command::Database { command } = &cli.command {
        match command {
            DatabaseCommand::Current { json } => {
                let connection = resolve_connection_optional_canister(
                    cli.connection.local,
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
        cli.connection.canister_id.clone(),
        cli.connection.database_id.clone(),
    )?;
    let client = CanisterVfsClient::new(&connection.replica_host, &connection.canister_id).await?;
    run_command(&client, cli, &connection).await
}
