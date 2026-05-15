// Where: crates/vfs_cli_app/src/client_mode.rs
// What: Select whether a command should use anonymous or signed canister calls.
// Why: `auto` mode needs one consistent policy for public and private database reads.
use crate::cli::Command;
use anyhow::{Result, bail};
use vfs_cli::cli::IdentityModeArg;
use vfs_client::VfsApi;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClientMode {
    Anonymous,
    Identity,
}

pub async fn select_client_mode(
    command: &Command,
    identity_mode: IdentityModeArg,
    database_id: Option<&str>,
    anonymous: &impl VfsApi,
    identity: &impl VfsApi,
) -> Result<ClientMode> {
    match identity_mode {
        IdentityModeArg::Identity => Ok(ClientMode::Identity),
        IdentityModeArg::Anonymous => {
            if command.requires_identity() {
                bail!("--identity-mode anonymous cannot run commands that require identity");
            }
            Ok(ClientMode::Anonymous)
        }
        IdentityModeArg::Auto => select_auto_mode(command, database_id, anonymous, identity).await,
    }
}

async fn select_auto_mode(
    command: &Command,
    database_id: Option<&str>,
    anonymous: &impl VfsApi,
    identity: &impl VfsApi,
) -> Result<ClientMode> {
    if command.requires_identity() || command.auto_uses_identity_without_target_database() {
        return Ok(ClientMode::Identity);
    }
    let Some(database_id) = database_id else {
        return Ok(ClientMode::Anonymous);
    };
    if !command.uses_target_database_read() {
        return Ok(ClientMode::Anonymous);
    }
    if anonymous.status(database_id).await.is_err() {
        return Ok(ClientMode::Identity);
    }
    let databases = identity.list_databases().await.map_err(|error| {
        anyhow::anyhow!("failed to check selected identity database membership: {error}")
    })?;
    if databases
        .iter()
        .any(|database| database.database_id == database_id)
    {
        Ok(ClientMode::Identity)
    } else {
        Ok(ClientMode::Anonymous)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cli::{Cli, DatabaseCommand};
    use anyhow::{Result, anyhow};
    use async_trait::async_trait;
    use clap::Parser;
    use vfs_types::{DatabaseStatus, DatabaseSummary, Status};

    struct MockClient {
        status_ok: bool,
        databases: Vec<DatabaseSummary>,
    }

    #[async_trait]
    impl VfsApi for MockClient {
        async fn status(&self, _database_id: &str) -> Result<Status> {
            if self.status_ok {
                Ok(Status {
                    file_count: 0,
                    source_count: 0,
                })
            } else {
                Err(anyhow!("not visible"))
            }
        }

        async fn list_databases(&self) -> Result<Vec<DatabaseSummary>> {
            Ok(self.databases.clone())
        }

        async fn read_node(
            &self,
            _database_id: &str,
            _path: &str,
        ) -> Result<Option<vfs_types::Node>> {
            unimplemented!()
        }

        async fn list_nodes(
            &self,
            _request: vfs_types::ListNodesRequest,
        ) -> Result<Vec<vfs_types::NodeEntry>> {
            unimplemented!()
        }

        async fn list_children(
            &self,
            _request: vfs_types::ListChildrenRequest,
        ) -> Result<Vec<vfs_types::ChildNode>> {
            unimplemented!()
        }

        async fn write_node(
            &self,
            _request: vfs_types::WriteNodeRequest,
        ) -> Result<vfs_types::WriteNodeResult> {
            unimplemented!()
        }

        async fn append_node(
            &self,
            _request: vfs_types::AppendNodeRequest,
        ) -> Result<vfs_types::WriteNodeResult> {
            unimplemented!()
        }

        async fn edit_node(
            &self,
            _request: vfs_types::EditNodeRequest,
        ) -> Result<vfs_types::EditNodeResult> {
            unimplemented!()
        }

        async fn delete_node(
            &self,
            _request: vfs_types::DeleteNodeRequest,
        ) -> Result<vfs_types::DeleteNodeResult> {
            unimplemented!()
        }

        async fn move_node(
            &self,
            _request: vfs_types::MoveNodeRequest,
        ) -> Result<vfs_types::MoveNodeResult> {
            unimplemented!()
        }

        async fn mkdir_node(
            &self,
            _request: vfs_types::MkdirNodeRequest,
        ) -> Result<vfs_types::MkdirNodeResult> {
            unimplemented!()
        }

        async fn glob_nodes(
            &self,
            _request: vfs_types::GlobNodesRequest,
        ) -> Result<Vec<vfs_types::GlobNodeHit>> {
            unimplemented!()
        }

        async fn recent_nodes(
            &self,
            _request: vfs_types::RecentNodesRequest,
        ) -> Result<Vec<vfs_types::RecentNodeHit>> {
            unimplemented!()
        }

        async fn multi_edit_node(
            &self,
            _request: vfs_types::MultiEditNodeRequest,
        ) -> Result<vfs_types::MultiEditNodeResult> {
            unimplemented!()
        }

        async fn search_nodes(
            &self,
            _request: vfs_types::SearchNodesRequest,
        ) -> Result<Vec<vfs_types::SearchNodeHit>> {
            unimplemented!()
        }

        async fn search_node_paths(
            &self,
            _request: vfs_types::SearchNodePathsRequest,
        ) -> Result<Vec<vfs_types::SearchNodeHit>> {
            unimplemented!()
        }

        async fn export_snapshot(
            &self,
            _request: vfs_types::ExportSnapshotRequest,
        ) -> Result<vfs_types::ExportSnapshotResponse> {
            unimplemented!()
        }

        async fn fetch_updates(
            &self,
            _request: vfs_types::FetchUpdatesRequest,
        ) -> Result<vfs_types::FetchUpdatesResponse> {
            unimplemented!()
        }
    }

    #[tokio::test]
    async fn auto_uses_identity_for_private_database_read() {
        let command = read_command();
        let anonymous = MockClient {
            status_ok: false,
            databases: vec![],
        };
        let identity = MockClient {
            status_ok: true,
            databases: vec![],
        };
        let mode = select_client_mode(
            &command,
            IdentityModeArg::Auto,
            Some("db"),
            &anonymous,
            &identity,
        )
        .await
        .unwrap();
        assert_eq!(mode, ClientMode::Identity);
    }

    #[tokio::test]
    async fn auto_uses_identity_for_public_member_database_read() {
        let command = read_command();
        let anonymous = MockClient {
            status_ok: true,
            databases: vec![],
        };
        let identity = MockClient {
            status_ok: true,
            databases: vec![database("db")],
        };
        let mode = select_client_mode(
            &command,
            IdentityModeArg::Auto,
            Some("db"),
            &anonymous,
            &identity,
        )
        .await
        .unwrap();
        assert_eq!(mode, ClientMode::Identity);
    }

    #[tokio::test]
    async fn auto_uses_anonymous_for_public_non_member_database_read() {
        let command = read_command();
        let anonymous = MockClient {
            status_ok: true,
            databases: vec![],
        };
        let identity = MockClient {
            status_ok: true,
            databases: vec![database("other")],
        };
        let mode = select_client_mode(
            &command,
            IdentityModeArg::Auto,
            Some("db"),
            &anonymous,
            &identity,
        )
        .await
        .unwrap();
        assert_eq!(mode, ClientMode::Anonymous);
    }

    #[tokio::test]
    async fn anonymous_rejects_mutating_command() {
        let command = Cli::parse_from([
            "kinic-vfs-cli",
            "write-node",
            "--path",
            "/Wiki/a.md",
            "--input",
            "a.md",
        ])
        .command;
        let client = MockClient {
            status_ok: true,
            databases: vec![],
        };
        let error = select_client_mode(
            &command,
            IdentityModeArg::Anonymous,
            Some("db"),
            &client,
            &client,
        )
        .await
        .unwrap_err()
        .to_string();
        assert!(error.contains("--identity-mode anonymous"));
    }

    fn read_command() -> Command {
        Cli::parse_from(["kinic-vfs-cli", "read-node", "--path", "/Wiki/index.md"]).command
    }

    fn database(database_id: &str) -> DatabaseSummary {
        DatabaseSummary {
            database_id: database_id.to_string(),
            status: DatabaseStatus::Hot,
            role: vfs_types::DatabaseRole::Reader,
            logical_size_bytes: 0,
            archived_at_ms: None,
            deleted_at_ms: None,
        }
    }

    #[test]
    fn database_list_auto_prefers_identity() {
        let command = Command::Database {
            command: DatabaseCommand::List { json: false },
        };
        assert!(command.auto_uses_identity_without_target_database());
    }
}
