// Where: crates/vfs_cli_app/src/main.rs
// What: Binary entrypoint for the agent-facing wiki CLI.
// Why: Agents need one executable that can read remote pages and sync the local mirror.
use anyhow::Result;
use clap::Parser;
use vfs_cli::connection::resolve_connection;
use vfs_cli_app::aeo_generate::{AeoGenerateArgs, run_aeo_generate};
use vfs_cli_app::cli::Cli;
use vfs_cli_app::cli::Command;
use vfs_cli_app::commands::run_command;
use vfs_client::CanisterVfsClient;

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    if let Command::AeoGenerate {
        repo,
        out,
        project_name,
    } = &cli.command
    {
        let report = run_aeo_generate(AeoGenerateArgs {
            repo: repo.clone(),
            out: out.clone(),
            project_name: project_name.clone(),
        })?;
        println!("{}", serde_json::to_string_pretty(&report)?);
        return Ok(());
    }
    let connection = resolve_connection(cli.connection.local, cli.connection.canister_id.clone())?;
    let client = CanisterVfsClient::new(&connection.replica_host, &connection.canister_id).await?;
    run_command(&client, cli).await
}
