use clap::Parser;
use wiki_types::NodeKind;

use crate::cli::{Cli, Command, ConnectionArgs};
use crate::commands::run_command;
use crate::commands_fs_tests::MockClient;
use crate::maintenance::{append_log, normalize_log_kind, rebuild_index};

fn node(path: &str, kind: NodeKind, content: &str) -> wiki_types::Node {
    wiki_types::Node {
        path: path.to_string(),
        kind,
        content: content.to_string(),
        created_at: 1,
        updated_at: 2,
        etag: format!("etag-{path}"),
        metadata_json: "{}".to_string(),
    }
}

fn test_cli(command: Command) -> Cli {
    Cli {
        connection: ConnectionArgs {
            local: false,
            canister_id: Some("aaaaa-aa".to_string()),
        },
        command,
    }
}

#[tokio::test]
async fn rebuild_index_renders_sections_from_existing_wiki_nodes() {
    let client = MockClient {
        nodes: vec![
            node(
                "/Wiki/sources/alpha.md",
                NodeKind::File,
                "# Alpha\n\nAlpha summary",
            ),
            node(
                "/Wiki/entities/openai.md",
                NodeKind::File,
                "# OpenAI\n\nEntity summary",
            ),
            node(
                "/Wiki/concepts/tool-calling.md",
                NodeKind::File,
                "# Tool Calling\n\nConcept summary",
            ),
            node("/Wiki/lint/r.md", NodeKind::File, "# Lint\n\nLint summary"),
        ],
        ..Default::default()
    };

    rebuild_index(&client)
        .await
        .expect("rebuild index should succeed");

    let writes = client.writes.lock().expect("writes should lock");
    let index = writes.last().expect("index write should exist");
    assert_eq!(index.path, "/Wiki/index.md");
    assert!(index.content.contains("## Sources"));
    assert!(index.content.contains("## Entities"));
    assert!(index.content.contains("## Concepts"));
    assert!(!index.content.contains("## Queries"));
    assert!(!index.content.contains("## Lint Reports"));
}

#[tokio::test]
async fn append_log_appends_formatted_entry() {
    let client = MockClient {
        nodes: vec![node("/Wiki/log.md", NodeKind::File, "# Log\n")],
        ..Default::default()
    };

    append_log(
        &client,
        "query",
        "Topic note",
        &["/Wiki/topic.md".to_string()],
        &["/Wiki/topic.md".to_string()],
        None,
    )
    .await
    .expect("append log should succeed");

    let writes = client.writes.lock().expect("writes should lock");
    let log = writes.last().expect("log write should exist");
    assert_eq!(log.path, "/Wiki/log.md");
    assert!(log.content.contains("query | Topic note"));
    assert!(log.content.contains("target_paths: /Wiki/topic.md"));
}

#[tokio::test]
async fn rebuild_index_command_dispatches() {
    let client = MockClient::default();

    run_command(&client, test_cli(Command::RebuildIndex))
        .await
        .expect("rebuild index command should succeed");

    let writes = client.writes.lock().expect("writes should lock");
    assert_eq!(writes.last().expect("index write").path, "/Wiki/index.md");
}

#[tokio::test]
async fn append_log_command_dispatches() {
    let client = MockClient::default();

    run_command(
        &client,
        test_cli(Command::AppendLog {
            kind: "research-note".to_string(),
            title: "Integrate".to_string(),
            target_paths: vec!["/Wiki/topic.md".to_string()],
            updated_paths: vec!["/Wiki/topic.md".to_string()],
            failure: None,
        }),
    )
    .await
    .expect("append log command should succeed");

    let writes = client.writes.lock().expect("writes should lock");
    assert_eq!(writes.last().expect("log write").path, "/Wiki/log.md");
}

#[test]
fn append_log_kind_accepts_freeform_and_rejects_invalid_values() {
    assert_eq!(
        normalize_log_kind(" research-note ").expect("freeform kind should pass"),
        "research-note"
    );
    assert!(normalize_log_kind("").is_err());
    assert!(normalize_log_kind("   ").is_err());
    assert!(normalize_log_kind("bad\nkind").is_err());
}

#[test]
fn workflow_commands_are_not_in_cli_anymore() {
    for removed in [
        "ingest-source",
        "build-ingest-context",
        "build-crystallize-context",
        "build-query-context",
        "build-integrate-context",
        "build-lint-context",
        "apply-workflow-result",
        "apply-integrate",
    ] {
        let parsed = Cli::try_parse_from(["wiki-cli", "--canister-id", "aaaaa-aa", removed]);
        assert!(parsed.is_err(), "{removed} should be removed");
    }
}

#[test]
fn connection_flags_are_optional_in_cli() {
    let parsed = Cli::try_parse_from(["wiki-cli", "rebuild-index"]);
    assert!(parsed.is_ok(), "connection flags should be optional");
}
