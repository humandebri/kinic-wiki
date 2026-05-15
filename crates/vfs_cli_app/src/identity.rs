// Where: crates/vfs_cli_app/src/identity.rs
// What: Load the active icp-cli identity for authenticated canister calls.
// Why: kinic-vfs-cli updates must use the caller selected by `icp identity default`.
use anyhow::{Context, Result, anyhow, bail};
use tokio::process::Command;

pub async fn export_default_identity_pem() -> Result<Vec<u8>> {
    let identity_name = command_stdout("icp", &["identity", "default"])
        .await
        .context("failed to read active icp-cli identity")?;
    let identity_name = identity_name.trim();
    if identity_name.is_empty() {
        bail!("active icp-cli identity is empty");
    }
    command_stdout_bytes("icp", &["identity", "export", identity_name])
        .await
        .with_context(|| format!("failed to export icp-cli identity `{identity_name}`"))
}

async fn command_stdout(command: &str, args: &[&str]) -> Result<String> {
    let bytes = command_stdout_bytes(command, args).await?;
    String::from_utf8(bytes).context("command output was not UTF-8")
}

async fn command_stdout_bytes(command: &str, args: &[&str]) -> Result<Vec<u8>> {
    let output = Command::new(command)
        .args(args)
        .output()
        .await
        .with_context(|| format!("failed to run `{}`", command_line(command, args)))?;
    if !output.status.success() {
        return Err(anyhow!(
            "`{}` failed: {}",
            command_line(command, args),
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }
    Ok(output.stdout)
}

fn command_line(command: &str, args: &[&str]) -> String {
    std::iter::once(command)
        .chain(args.iter().copied())
        .collect::<Vec<_>>()
        .join(" ")
}
