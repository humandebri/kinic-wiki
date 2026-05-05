use std::path::PathBuf;

use clap::Parser;
use tempfile::tempdir;
use vfs_types::{Node, NodeKind, SearchPreviewMode};

use crate::cli::{
    AuditFailOnArg, Cli, Command, ConnectionArgs, GlobNodeTypeArg, NodeKindArg,
    SearchPreviewModeArg, SkillCommand, SkillIndexCommand, SkillLocalCommand, SkillPolicyCommand,
    SkillPublicCommand, SkillVersionsCommand,
};
use crate::commands::run_command;
use crate::commands_fs_tests::MockClient;
use crate::skill_registry::parse_skill_manifest;

fn test_cli(command: Command) -> Cli {
    Cli {
        connection: ConnectionArgs {
            local: false,
            canister_id: Some("aaaaa-aa".to_string()),
            identity_pem: None,
        },
        command,
    }
}

fn test_node(path: &str, content: &str) -> Node {
    Node {
        path: path.to_string(),
        kind: NodeKind::File,
        content: content.to_string(),
        created_at: 1,
        updated_at: 2,
        etag: "etag-test".to_string(),
        metadata_json: "{}".to_string(),
    }
}

#[tokio::test]
async fn append_node_command_calls_canister_append() {
    let dir = tempdir().expect("temp dir should exist");
    let input = PathBuf::from(dir.path()).join("append.md");
    std::fs::write(&input, "\nnext").expect("input should write");
    let client = MockClient::default();

    run_command(
        &client,
        test_cli(Command::AppendNode {
            path: "/Wiki/foo.md".to_string(),
            input,
            kind: Some(NodeKindArg::File),
            metadata_json: Some("{\"k\":1}".to_string()),
            expected_etag: Some("etag-1".to_string()),
            separator: Some("\n".to_string()),
            json: false,
        }),
    )
    .await
    .expect("append command should succeed");

    let appends = client.appends.lock().expect("appends should lock");
    assert_eq!(appends.len(), 1);
    assert_eq!(appends[0].path, "/Wiki/foo.md");
    assert_eq!(appends[0].expected_etag.as_deref(), Some("etag-1"));
    assert_eq!(appends[0].separator.as_deref(), Some("\n"));
}

#[tokio::test]
async fn write_node_command_supports_source_kind() {
    let dir = tempdir().expect("temp dir should exist");
    let input = PathBuf::from(dir.path()).join("source.md");
    std::fs::write(&input, "# Source\n\nBody").expect("input should write");
    let client = MockClient::default();

    run_command(
        &client,
        test_cli(Command::WriteNode {
            path: "/Sources/raw/source/source.md".to_string(),
            kind: NodeKindArg::Source,
            input,
            metadata_json: "{}".to_string(),
            expected_etag: None,
            json: false,
        }),
    )
    .await
    .expect("write source command should succeed");

    let writes = client.writes.lock().expect("writes should lock");
    assert_eq!(writes.len(), 1);
    assert_eq!(writes[0].path, "/Sources/raw/source/source.md");
    assert_eq!(writes[0].kind, NodeKind::Source);
}

#[tokio::test]
async fn skill_policy_command_reads_policy() {
    let client = MockClient::default();

    run_command(
        &client,
        test_cli(Command::Skill {
            command: SkillCommand::Policy {
                command: SkillPolicyCommand::Policy { json: true },
            },
        }),
    )
    .await
    .expect("policy should render");
}

#[tokio::test]
async fn skill_policy_whoami_command_reads_roles_policy_and_principal() {
    let client = MockClient::default();

    run_command(
        &client,
        test_cli(Command::Skill {
            command: SkillCommand::Policy {
                command: SkillPolicyCommand::Whoami { json: true },
            },
        }),
    )
    .await
    .expect("whoami should render");
}

#[tokio::test]
async fn skill_import_writes_registry_nodes() {
    let dir = tempdir().expect("temp dir should exist");
    std::fs::write(dir.path().join("SKILL.md"), "# Skill\n\nUse wiki evidence.")
        .expect("skill should write");
    let client = MockClient::default();

    run_command(
        &client,
        test_cli(Command::Skill {
            command: SkillCommand::Import {
                source: Some(dir.path().to_string_lossy().into_owned()),
                github: None,
                path: None,
                ref_name: "HEAD".to_string(),
                id: "acme/legal-review".to_string(),
                json: true,
            },
        }),
    )
    .await
    .expect("skill import should succeed");

    let writes = client.writes.lock().expect("writes should lock");
    assert_eq!(writes.len(), 5);
    assert!(
        writes
            .iter()
            .any(|write| write.path == "/Wiki/skills/acme/legal-review/SKILL.md")
    );
    assert!(
        writes
            .iter()
            .any(|write| write.path == "/Wiki/skills/acme/legal-review/manifest.md")
    );
}

#[tokio::test]
async fn skill_import_preserves_source_manifest_and_optional_files() {
    let dir = tempdir().expect("temp dir should exist");
    std::fs::write(
        dir.path().join("SKILL.md"),
        "# Skill\n\nUse source manifest.",
    )
    .expect("skill should write");
    std::fs::write(
        dir.path().join("manifest.md"),
        "---\nkind: kinic.skill\nschema_version: 1\nid: acme/legal-review\nversion: 9.9.9\npublisher: acme\nentry: SKILL.md\nknowledge: []\npermissions:\n  file_read: true\n  network: false\n  shell: false\nprovenance:\n  source: local\n  source_ref: pinned\n---\n# Source Manifest\n",
    )
    .expect("manifest should write");
    std::fs::write(dir.path().join("provenance.md"), "# Source Provenance\n")
        .expect("provenance should write");
    std::fs::write(dir.path().join("evals.md"), "# Source Evals\n").expect("evals should write");
    let client = MockClient::default();

    run_command(
        &client,
        test_cli(Command::Skill {
            command: SkillCommand::Import {
                source: Some(dir.path().to_string_lossy().into_owned()),
                github: None,
                path: None,
                ref_name: "HEAD".to_string(),
                id: "acme/legal-review".to_string(),
                json: true,
            },
        }),
    )
    .await
    .expect("skill import should succeed");

    let writes = client.writes.lock().expect("writes should lock");
    let manifest = writes
        .iter()
        .find(|write| write.path == "/Wiki/skills/acme/legal-review/manifest.md")
        .expect("manifest write should exist");
    assert!(manifest.content.contains("version: 9.9.9"));
    let provenance = writes
        .iter()
        .find(|write| write.path == "/Wiki/skills/acme/legal-review/provenance.md")
        .expect("provenance write should exist");
    assert_eq!(provenance.content, "# Source Provenance\n");
    let evals = writes
        .iter()
        .find(|write| write.path == "/Wiki/skills/acme/legal-review/evals.md")
        .expect("evals write should exist");
    assert_eq!(evals.content, "# Source Evals\n");
}

#[tokio::test]
async fn skill_import_saves_previous_current_files_as_version() {
    let dir = tempdir().expect("temp dir should exist");
    std::fs::write(dir.path().join("SKILL.md"), "# New Skill\n").expect("skill should write");
    let client = MockClient {
        nodes: vec![
            test_node(
                "/Wiki/skills/acme/legal-review/manifest.md",
                "---\nkind: kinic.skill\nschema_version: 1\nid: acme/legal-review\nversion: 0.1.0\npublisher: acme\nentry: SKILL.md\nknowledge: []\npermissions: {}\nprovenance:\n  source: local\n  source_ref: old\n---\n# Old Manifest\n",
            ),
            test_node("/Wiki/skills/acme/legal-review/SKILL.md", "# Old Skill\n"),
            test_node(
                "/Wiki/skills/acme/legal-review/provenance.md",
                "# Old Provenance\n",
            ),
            test_node("/Wiki/skills/acme/legal-review/evals.md", "# Old Evals\n"),
        ],
        ..MockClient::default()
    };

    run_command(
        &client,
        test_cli(Command::Skill {
            command: SkillCommand::Import {
                source: Some(dir.path().to_string_lossy().into_owned()),
                github: None,
                path: None,
                ref_name: "HEAD".to_string(),
                id: "acme/legal-review".to_string(),
                json: true,
            },
        }),
    )
    .await
    .expect("skill import should succeed");

    let writes = client.writes.lock().expect("writes should lock");
    let old_skill = writes
        .iter()
        .find(|write| {
            write
                .path
                .starts_with("/Wiki/skills/acme/legal-review/versions/")
                && write.path.ends_with("/SKILL.md")
        })
        .expect("versioned old skill should be written");
    assert_eq!(old_skill.content, "# Old Skill\n");
    assert!(old_skill.path.contains("etag-test"));
    let current_skill = writes
        .iter()
        .find(|write| write.path == "/Wiki/skills/acme/legal-review/SKILL.md")
        .expect("current skill should be written");
    assert_eq!(current_skill.content, "# New Skill\n");
}

#[tokio::test]
async fn skill_import_skips_version_when_existing_manifest_is_missing() {
    let dir = tempdir().expect("temp dir should exist");
    std::fs::write(dir.path().join("SKILL.md"), "# New Skill\n").expect("skill should write");
    let client = MockClient {
        nodes: vec![test_node(
            "/Wiki/skills/acme/legal-review/SKILL.md",
            "# Old Skill\n",
        )],
        ..MockClient::default()
    };

    run_command(
        &client,
        test_cli(Command::Skill {
            command: SkillCommand::Import {
                source: Some(dir.path().to_string_lossy().into_owned()),
                github: None,
                path: None,
                ref_name: "HEAD".to_string(),
                id: "acme/legal-review".to_string(),
                json: true,
            },
        }),
    )
    .await
    .expect("skill import should succeed");

    let writes = client.writes.lock().expect("writes should lock");
    assert!(
        writes
            .iter()
            .all(|write| !write.path.contains("/versions/"))
    );
}

#[tokio::test]
async fn skill_import_quotes_generated_manifest_source() {
    let dir = tempdir().expect("temp dir should exist");
    let source_dir = dir.path().join("true # source");
    std::fs::create_dir(&source_dir).expect("source dir should write");
    std::fs::write(source_dir.join("SKILL.md"), "# Skill\n").expect("skill should write");
    let source = source_dir.to_string_lossy().into_owned();
    let client = MockClient::default();

    run_command(
        &client,
        test_cli(Command::Skill {
            command: SkillCommand::Import {
                source: Some(source.clone()),
                github: None,
                path: None,
                ref_name: "HEAD".to_string(),
                id: "acme/legal-review".to_string(),
                json: true,
            },
        }),
    )
    .await
    .expect("skill import should succeed");

    let writes = client.writes.lock().expect("writes should lock");
    let manifest = writes
        .iter()
        .find(|write| write.path == "/Wiki/skills/acme/legal-review/manifest.md")
        .expect("manifest write should exist");
    let manifest = parse_skill_manifest(&manifest.content).expect("manifest should parse");

    assert_eq!(
        manifest.provenance.get("source").map(String::as_str),
        Some(source.as_str())
    );
}

#[tokio::test]
async fn skill_import_rejects_manifest_id_mismatch() {
    let dir = tempdir().expect("temp dir should exist");
    std::fs::write(dir.path().join("SKILL.md"), "# Skill").expect("skill should write");
    std::fs::write(
        dir.path().join("manifest.md"),
        "---\nkind: kinic.skill\nschema_version: 1\nid: other/legal-review\nversion: 0.1.0\npublisher: other\nentry: SKILL.md\n---\n# Manifest\n",
    )
    .expect("manifest should write");
    let client = MockClient::default();

    let error = run_command(
        &client,
        test_cli(Command::Skill {
            command: SkillCommand::Import {
                source: Some(dir.path().to_string_lossy().into_owned()),
                github: None,
                path: None,
                ref_name: "HEAD".to_string(),
                id: "acme/legal-review".to_string(),
                json: false,
            },
        }),
    )
    .await
    .expect_err("mismatched manifest id should fail");

    assert!(error.to_string().contains("source manifest id must match"));
}

#[tokio::test]
async fn skill_inspect_reports_missing_optional_files() {
    let client = MockClient {
        nodes: vec![vfs_types::Node {
            path: "/Wiki/skills/acme/legal-review/manifest.md".to_string(),
            kind: NodeKind::File,
            content: "---\nkind: kinic.skill\nschema_version: 1\nid: acme/legal-review\nversion: 0.1.0\npublisher: acme\nentry: SKILL.md\nknowledge: []\npermissions:\n  file_read: true\n  network: false\n  shell: false\nprovenance:\n  source: local\n  source_ref: local\n---\n# Manifest\n".to_string(),
            created_at: 1,
            updated_at: 1,
            etag: "etag".to_string(),
            metadata_json: "{}".to_string(),
        }],
        ..Default::default()
    };

    run_command(
        &client,
        test_cli(Command::Skill {
            command: SkillCommand::Inspect {
                id: "acme/legal-review".to_string(),
                json: true,
            },
        }),
    )
    .await
    .expect("skill inspect should succeed");
}

#[tokio::test]
async fn skill_audit_fail_on_error_returns_error_for_missing_files() {
    let client = MockClient {
        nodes: vec![vfs_types::Node {
            path: "/Wiki/skills/acme/legal-review/manifest.md".to_string(),
            kind: NodeKind::File,
            content: "---\nkind: kinic.skill\nschema_version: 1\nid: acme/legal-review\nversion: 0.1.0\npublisher: acme\nentry: SKILL.md\nknowledge: []\npermissions:\n  file_read: true\n  network: false\n  shell: false\nprovenance:\n  source: local\n  source_ref: local\n---\n# Manifest\n".to_string(),
            created_at: 1,
            updated_at: 1,
            etag: "etag".to_string(),
            metadata_json: "{}".to_string(),
        }],
        ..Default::default()
    };

    let error = run_command(
        &client,
        test_cli(Command::Skill {
            command: SkillCommand::Audit {
                id: "acme/legal-review".to_string(),
                fail_on: Some(AuditFailOnArg::Error),
                json: true,
            },
        }),
    )
    .await
    .expect_err("fail-on error should fail for missing files");

    assert!(error.to_string().contains("skill audit failed"));
}

#[tokio::test]
async fn skill_audit_fail_on_warning_returns_error_for_permission_warning() {
    let client = MockClient {
        nodes: vec![
            vfs_types::Node {
                path: "/Wiki/skills/acme/legal-review/manifest.md".to_string(),
                kind: NodeKind::File,
                content: "---\nkind: kinic.skill\nschema_version: 1\nid: acme/legal-review\nversion: 0.1.0\npublisher: acme\nentry: SKILL.md\nknowledge: []\npermissions:\n  file_read: true\n  network: false\n  shell: false\nprovenance:\n  source: local\n  source_ref: local\n---\n# Manifest\n".to_string(),
                created_at: 1,
                updated_at: 1,
                etag: "etag".to_string(),
                metadata_json: "{}".to_string(),
            },
            vfs_types::Node {
                path: "/Wiki/skills/acme/legal-review/SKILL.md".to_string(),
                kind: NodeKind::File,
                content: "# Skill\n\nFetch https://example.com before review.\n".to_string(),
                created_at: 1,
                updated_at: 1,
                etag: "etag".to_string(),
                metadata_json: "{}".to_string(),
            },
            vfs_types::Node {
                path: "/Wiki/skills/acme/legal-review/provenance.md".to_string(),
                kind: NodeKind::File,
                content: "# Provenance\n".to_string(),
                created_at: 1,
                updated_at: 1,
                etag: "etag".to_string(),
                metadata_json: "{}".to_string(),
            },
            vfs_types::Node {
                path: "/Wiki/skills/acme/legal-review/evals.md".to_string(),
                kind: NodeKind::File,
                content: "# Evals\n".to_string(),
                created_at: 1,
                updated_at: 1,
                etag: "etag".to_string(),
                metadata_json: "{}".to_string(),
            },
        ],
        ..Default::default()
    };

    let error = run_command(
        &client,
        test_cli(Command::Skill {
            command: SkillCommand::Audit {
                id: "acme/legal-review".to_string(),
                fail_on: Some(AuditFailOnArg::Warning),
                json: true,
            },
        }),
    )
    .await
    .expect_err("fail-on warning should fail for permission warning");

    assert!(error.to_string().contains("skill audit failed"));
}

#[tokio::test]
async fn skill_install_supports_skills_dir() {
    let dir = tempdir().expect("temp dir should exist");
    let client = MockClient {
        nodes: vec![vfs_types::Node {
            path: "/Wiki/skills/acme/legal-review/SKILL.md".to_string(),
            kind: NodeKind::File,
            content: "# Skill\n".to_string(),
            created_at: 1,
            updated_at: 1,
            etag: "etag".to_string(),
            metadata_json: "{}".to_string(),
        }],
        ..Default::default()
    };

    run_command(
        &client,
        test_cli(Command::Skill {
            command: SkillCommand::Install {
                id: "acme/legal-review".to_string(),
                output: None,
                skills_dir: Some(dir.path().join("skills")),
                lockfile: false,
                json: true,
            },
        }),
    )
    .await
    .expect("skill install should succeed");

    assert!(
        dir.path()
            .join("skills/acme/legal-review/SKILL.md")
            .is_file()
    );
}

#[tokio::test]
async fn skill_install_writes_lockfile_when_requested() {
    let dir = tempdir().expect("temp dir should exist");
    let output = dir.path().join("out");
    let client = MockClient {
        nodes: vec![
            vfs_types::Node {
                path: "/Wiki/skills/acme/legal-review/SKILL.md".to_string(),
                kind: NodeKind::File,
                content: "# Skill\n".to_string(),
                created_at: 1,
                updated_at: 1,
                etag: "skill-etag".to_string(),
                metadata_json: "{}".to_string(),
            },
            vfs_types::Node {
                path: "/Wiki/skills/acme/legal-review/manifest.md".to_string(),
                kind: NodeKind::File,
                content: "---\nkind: kinic.skill\nschema_version: 1\nid: acme/legal-review\nversion: 0.1.0\npublisher: acme\nentry: SKILL.md\nknowledge: []\npermissions: {}\nprovenance:\n  source: local\n---\n# Manifest\n".to_string(),
                created_at: 1,
                updated_at: 2,
                etag: "manifest-etag".to_string(),
                metadata_json: "{}".to_string(),
            },
        ],
        ..Default::default()
    };

    run_command(
        &client,
        test_cli(Command::Skill {
            command: SkillCommand::Install {
                id: "acme/legal-review".to_string(),
                output: Some(output.clone()),
                skills_dir: None,
                lockfile: true,
                json: true,
            },
        }),
    )
    .await
    .expect("skill install should succeed");

    let lockfile =
        std::fs::read_to_string(output.join("skill.lock.json")).expect("lockfile should exist");
    assert!(lockfile.contains(r#""manifest_etag": "manifest-etag""#));
    assert!(lockfile.contains(r#""source_path": "/Wiki/skills/acme/legal-review""#));
}

#[tokio::test]
async fn skill_local_audit_accepts_skill_file_and_warns_on_missing_optional_files() {
    let dir = tempdir().expect("temp dir should exist");
    std::fs::write(dir.path().join("SKILL.md"), "# Skill\n").expect("skill should write");
    let client = MockClient::default();

    run_command(
        &client,
        test_cli(Command::Skill {
            command: SkillCommand::Local {
                command: SkillLocalCommand::Audit {
                    dir: dir.path().to_path_buf(),
                    json: true,
                },
            },
        }),
    )
    .await
    .expect("local audit should succeed");
}

#[tokio::test]
async fn skill_local_audit_requires_skill_file() {
    let dir = tempdir().expect("temp dir should exist");
    let client = MockClient::default();

    let error = run_command(
        &client,
        test_cli(Command::Skill {
            command: SkillCommand::Local {
                command: SkillLocalCommand::Audit {
                    dir: dir.path().to_path_buf(),
                    json: true,
                },
            },
        }),
    )
    .await
    .expect_err("missing SKILL.md should fail");

    assert!(error.to_string().contains("SKILL.md missing"));
}

#[tokio::test]
async fn skill_local_audit_reports_invalid_manifest_without_registry_writes() {
    let dir = tempdir().expect("temp dir should exist");
    std::fs::write(dir.path().join("SKILL.md"), "# Skill\n").expect("skill should write");
    std::fs::write(
        dir.path().join("manifest.md"),
        "---\nkind: wrong\n---\n# Manifest\n",
    )
    .expect("manifest should write");
    let client = MockClient::default();

    run_command(
        &client,
        test_cli(Command::Skill {
            command: SkillCommand::Local {
                command: SkillLocalCommand::Audit {
                    dir: dir.path().to_path_buf(),
                    json: true,
                },
            },
        }),
    )
    .await
    .expect("invalid manifest should be reported as audit warning");

    assert!(client.writes.lock().expect("writes should lock").is_empty());
}

#[tokio::test]
async fn skill_local_diff_requires_lockfile() {
    let dir = tempdir().expect("temp dir should exist");
    let client = MockClient::default();

    let error = run_command(
        &client,
        test_cli(Command::Skill {
            command: SkillCommand::Local {
                command: SkillLocalCommand::Diff {
                    dir: dir.path().to_path_buf(),
                    json: true,
                },
            },
        }),
    )
    .await
    .expect_err("missing lockfile should fail");

    assert!(error.to_string().contains("skill.lock.json missing"));
}

#[tokio::test]
async fn skill_local_diff_compares_local_files_with_registry_source_path() {
    let dir = tempdir().expect("temp dir should exist");
    std::fs::write(dir.path().join("SKILL.md"), "# Local Skill\n").expect("skill should write");
    std::fs::write(dir.path().join("manifest.md"), "# Manifest\n").expect("manifest should write");
    std::fs::write(dir.path().join("evals.md"), "# Local Evals\n").expect("evals should write");
    std::fs::write(
        dir.path().join("skill.lock.json"),
        "{\n  \"id\": \"acme/legal-review\",\n  \"version\": \"0.1.0\",\n  \"source_path\": \"/Wiki/skills/acme/legal-review\",\n  \"manifest_etag\": \"manifest-etag\",\n  \"installed_at\": \"2026-01-01T00:00:00Z\"\n}\n",
    )
    .expect("lockfile should write");
    let client = MockClient {
        nodes: vec![
            test_node(
                "/Wiki/skills/acme/legal-review/SKILL.md",
                "# Remote Skill\n",
            ),
            test_node("/Wiki/skills/acme/legal-review/manifest.md", "# Manifest\n"),
            test_node(
                "/Wiki/skills/acme/legal-review/provenance.md",
                "# Provenance\n",
            ),
        ],
        ..MockClient::default()
    };

    run_command(
        &client,
        test_cli(Command::Skill {
            command: SkillCommand::Local {
                command: SkillLocalCommand::Diff {
                    dir: dir.path().to_path_buf(),
                    json: true,
                },
            },
        }),
    )
    .await
    .expect("local diff should succeed");
}

#[tokio::test]
async fn skill_local_install_uses_manifest_id() {
    let dir = tempdir().expect("temp dir should exist");
    let skills_dir = dir.path().join("skills");
    let skill_dir = dir.path().join("work");
    std::fs::create_dir(&skill_dir).expect("skill dir should write");
    std::fs::write(skill_dir.join("SKILL.md"), "# Skill\n").expect("skill should write");
    std::fs::write(
        skill_dir.join("manifest.md"),
        "---\nkind: kinic.skill\nschema_version: 1\nid: acme/local-review\nversion: 0.1.0\npublisher: acme\nentry: SKILL.md\nknowledge: []\npermissions: {}\nprovenance:\n  source: local\n  source_ref: local\n---\n# Manifest\n",
    )
    .expect("manifest should write");
    let client = MockClient::default();

    run_command(
        &client,
        test_cli(Command::Skill {
            command: SkillCommand::Local {
                command: SkillLocalCommand::Install {
                    dir: skill_dir,
                    skills_dir: skills_dir.clone(),
                    json: true,
                },
            },
        }),
    )
    .await
    .expect("local install should succeed");

    assert!(skills_dir.join("acme/local-review/SKILL.md").is_file());
}

#[tokio::test]
async fn skill_local_install_falls_back_to_lockfile_id() {
    let dir = tempdir().expect("temp dir should exist");
    let skills_dir = dir.path().join("skills");
    let skill_dir = dir.path().join("work");
    std::fs::create_dir(&skill_dir).expect("skill dir should write");
    std::fs::write(skill_dir.join("SKILL.md"), "# Skill\n").expect("skill should write");
    std::fs::write(
        skill_dir.join("skill.lock.json"),
        "{\n  \"id\": \"acme/lock-review\",\n  \"version\": \"0.1.0\",\n  \"source_path\": \"/Wiki/skills/acme/lock-review\",\n  \"manifest_etag\": \"manifest-etag\",\n  \"installed_at\": \"2026-01-01T00:00:00Z\"\n}\n",
    )
    .expect("lockfile should write");
    let client = MockClient::default();

    run_command(
        &client,
        test_cli(Command::Skill {
            command: SkillCommand::Local {
                command: SkillLocalCommand::Install {
                    dir: skill_dir,
                    skills_dir: skills_dir.clone(),
                    json: true,
                },
            },
        }),
    )
    .await
    .expect("local install should succeed");

    assert!(skills_dir.join("acme/lock-review/SKILL.md").is_file());
}

#[tokio::test]
async fn skill_versions_list_reads_version_nodes() {
    let client = MockClient {
        nodes: vec![
            test_node(
                "/Wiki/skills/acme/legal-review/versions/20260505T010203Z-etag/manifest.md",
                "---\nkind: kinic.skill\nschema_version: 1\nid: acme/legal-review\nversion: 0.1.0\npublisher: acme\nentry: SKILL.md\nknowledge: []\npermissions: {}\nprovenance:\n  source: local\n  source_ref: old\n---\n# Manifest\n",
            ),
            test_node(
                "/Wiki/skills/acme/legal-review/versions/20260505T010203Z-etag/SKILL.md",
                "# Old Skill\n",
            ),
        ],
        ..MockClient::default()
    };

    run_command(
        &client,
        test_cli(Command::Skill {
            command: SkillCommand::Versions {
                command: SkillVersionsCommand::List {
                    id: "acme/legal-review".to_string(),
                    json: true,
                },
            },
        }),
    )
    .await
    .expect("versions list should succeed");

    let lists = client.lists.lock().expect("lists should lock");
    assert_eq!(lists[0].prefix, "/Wiki/skills/acme/legal-review/versions");
}

#[tokio::test]
async fn skill_versions_inspect_reads_version_manifest() {
    let client = MockClient {
        nodes: vec![
            test_node(
                "/Wiki/skills/acme/legal-review/versions/20260505T010203Z-etag/manifest.md",
                "---\nkind: kinic.skill\nschema_version: 1\nid: acme/legal-review\nversion: 0.1.0\npublisher: acme\nentry: SKILL.md\nknowledge: []\npermissions: {}\nprovenance:\n  source: local\n  source_ref: old\n---\n# Manifest\n",
            ),
            test_node(
                "/Wiki/skills/acme/legal-review/versions/20260505T010203Z-etag/SKILL.md",
                "# Old Skill\n",
            ),
        ],
        ..MockClient::default()
    };

    run_command(
        &client,
        test_cli(Command::Skill {
            command: SkillCommand::Versions {
                command: SkillVersionsCommand::Inspect {
                    id: "acme/legal-review".to_string(),
                    version: "20260505T010203Z-etag".to_string(),
                    json: true,
                },
            },
        }),
    )
    .await
    .expect("versions inspect should succeed");
}

#[tokio::test]
async fn skill_versions_inspect_missing_version_fails() {
    let client = MockClient {
        nodes: vec![test_node("/Wiki/other.md", "# Other\n")],
        ..MockClient::default()
    };

    let error = run_command(
        &client,
        test_cli(Command::Skill {
            command: SkillCommand::Versions {
                command: SkillVersionsCommand::Inspect {
                    id: "acme/legal-review".to_string(),
                    version: "20260505T010203Z-etag".to_string(),
                    json: true,
                },
            },
        }),
    )
    .await
    .expect_err("missing version should fail");

    assert!(error.to_string().contains("version not found"));
}

#[tokio::test]
async fn skill_public_versions_list_uses_public_root() {
    let client = MockClient {
        nodes: vec![test_node(
            "/Wiki/public-skills/acme/legal-review/versions/20260505T010203Z-etag/manifest.md",
            "---\nkind: kinic.skill\nschema_version: 1\nid: acme/legal-review\nversion: 0.1.0\npublisher: acme\nentry: SKILL.md\nknowledge: []\npermissions: {}\nprovenance:\n  source: local\n  source_ref: old\n---\n# Manifest\n",
        )],
        ..MockClient::default()
    };

    run_command(
        &client,
        test_cli(Command::Skill {
            command: SkillCommand::Public {
                command: SkillPublicCommand::Versions {
                    command: SkillVersionsCommand::List {
                        id: "acme/legal-review".to_string(),
                        json: true,
                    },
                },
            },
        }),
    )
    .await
    .expect("public versions list should succeed");

    let lists = client.lists.lock().expect("lists should lock");
    assert_eq!(
        lists[0].prefix,
        "/Wiki/public-skills/acme/legal-review/versions"
    );
}

#[tokio::test]
async fn skill_index_inspect_uses_public_root() {
    let dir = tempdir().expect("temp dir should exist");
    let index = dir.path().join("skills.index.toml");
    std::fs::write(
        &index,
        "version = 1\n\n[[skills]]\nid = \"acme/legal-review\"\ncatalog = \"public\"\n",
    )
    .expect("index should write");
    let client = MockClient {
        nodes: vec![test_node(
            "/Wiki/public-skills/acme/legal-review/manifest.md",
            "---\nkind: kinic.skill\nschema_version: 1\nid: acme/legal-review\nversion: 0.1.0\npublisher: acme\nentry: SKILL.md\nknowledge: []\npermissions: {}\nprovenance:\n  source: local\n  source_ref: local\n---\n# Manifest\n",
        )],
        ..MockClient::default()
    };

    run_command(
        &client,
        test_cli(Command::Skill {
            command: SkillCommand::Index {
                command: SkillIndexCommand::Inspect {
                    id: "acme/legal-review".to_string(),
                    index,
                    json: true,
                },
            },
        }),
    )
    .await
    .expect("index inspect should succeed");
}

#[tokio::test]
async fn skill_index_install_writes_public_lockfile() {
    let dir = tempdir().expect("temp dir should exist");
    let index = dir.path().join("skills.index.toml");
    let output = dir.path().join("installed");
    std::fs::write(
        &index,
        "version = 1\n\n[[skills]]\nid = \"acme/legal-review\"\ncatalog = \"public\"\n",
    )
    .expect("index should write");
    let client = MockClient {
        nodes: vec![
            test_node(
                "/Wiki/public-skills/acme/legal-review/SKILL.md",
                "# Public Skill\n",
            ),
            test_node(
                "/Wiki/public-skills/acme/legal-review/manifest.md",
                "---\nkind: kinic.skill\nschema_version: 1\nid: acme/legal-review\nversion: 0.1.0\npublisher: acme\nentry: SKILL.md\nknowledge: []\npermissions: {}\nprovenance:\n  source: local\n  source_ref: local\n---\n# Manifest\n",
            ),
        ],
        ..MockClient::default()
    };

    run_command(
        &client,
        test_cli(Command::Skill {
            command: SkillCommand::Index {
                command: SkillIndexCommand::Install {
                    id: "acme/legal-review".to_string(),
                    index,
                    output: output.clone(),
                    lockfile: true,
                    json: true,
                },
            },
        }),
    )
    .await
    .expect("index install should succeed");

    let lockfile =
        std::fs::read_to_string(output.join("skill.lock.json")).expect("lockfile should exist");
    assert!(lockfile.contains(r#""catalog": "public""#));
    assert!(lockfile.contains(r#""source_path": "/Wiki/public-skills/acme/legal-review""#));
}

#[tokio::test]
async fn skill_index_install_enabled_skips_disabled_entries() {
    let dir = tempdir().expect("temp dir should exist");
    let index = dir.path().join("skills.index.toml");
    let skills_dir = dir.path().join("skills");
    std::fs::write(
        &index,
        "version = 1\n\n[[skills]]\nid = \"acme/enabled\"\npriority = 10\n\n[[skills]]\nid = \"acme/disabled\"\nenabled = false\npriority = 20\n",
    )
    .expect("index should write");
    let client = MockClient {
        nodes: vec![
            test_node("/Wiki/skills/acme/enabled/SKILL.md", "# Enabled\n"),
            test_node(
                "/Wiki/skills/acme/enabled/manifest.md",
                "---\nkind: kinic.skill\nschema_version: 1\nid: acme/enabled\nversion: 0.1.0\npublisher: acme\nentry: SKILL.md\nknowledge: []\npermissions: {}\nprovenance:\n  source: local\n  source_ref: local\n---\n# Manifest\n",
            ),
        ],
        ..MockClient::default()
    };

    run_command(
        &client,
        test_cli(Command::Skill {
            command: SkillCommand::Index {
                command: SkillIndexCommand::InstallEnabled {
                    index,
                    skills_dir: skills_dir.clone(),
                    lockfile: true,
                    json: true,
                },
            },
        }),
    )
    .await
    .expect("enabled install should succeed");

    assert!(skills_dir.join("acme/enabled/SKILL.md").is_file());
    assert!(!skills_dir.join("acme/disabled/SKILL.md").is_file());
}

#[tokio::test]
async fn skill_public_promote_requires_clean_private_audit() {
    let client = MockClient {
        nodes: vec![
            test_node(
                "/Wiki/skills/acme/legal-review/manifest.md",
                "---\nkind: kinic.skill\nschema_version: 1\nid: acme/legal-review\nversion: 0.1.0\npublisher: acme\nentry: SKILL.md\nknowledge:\n  - /Wiki/legal/contracts.md\npermissions:\n  file_read: true\n  network: false\n  shell: false\nprovenance:\n  source: local\n  source_ref: local\n---\n# Manifest\n",
            ),
            test_node("/Wiki/skills/acme/legal-review/SKILL.md", "# Skill\n"),
            test_node(
                "/Wiki/skills/acme/legal-review/provenance.md",
                "# Provenance\n",
            ),
            test_node("/Wiki/skills/acme/legal-review/evals.md", "# Evals\n"),
            test_node("/Wiki/legal/contracts.md", "# Contracts\n"),
            test_node(
                "/Sources/raw/skill-imports/acme-legal-review/acme-legal-review.md",
                "# Raw\n",
            ),
        ],
        ..MockClient::default()
    };

    run_command(
        &client,
        test_cli(Command::Skill {
            command: SkillCommand::Public {
                command: SkillPublicCommand::Promote {
                    id: "acme/legal-review".to_string(),
                    json: true,
                },
            },
        }),
    )
    .await
    .expect("clean skill should promote");

    let writes = client.writes.lock().expect("writes should lock");
    assert!(
        writes
            .iter()
            .any(|write| { write.path == "/Wiki/public-skills/acme/legal-review/SKILL.md" })
    );
    assert!(
        writes
            .iter()
            .any(|write| { write.path == "/Wiki/public-skills/acme/legal-review/manifest.md" })
    );
}

#[tokio::test]
async fn skill_public_promote_saves_previous_public_version() {
    let client = MockClient {
        nodes: vec![
            test_node(
                "/Wiki/skills/acme/legal-review/manifest.md",
                "---\nkind: kinic.skill\nschema_version: 1\nid: acme/legal-review\nversion: 0.2.0\npublisher: acme\nentry: SKILL.md\nknowledge: []\npermissions:\n  file_read: true\n  network: false\n  shell: false\nprovenance:\n  source: local\n  source_ref: new\n---\n# New Manifest\n",
            ),
            test_node("/Wiki/skills/acme/legal-review/SKILL.md", "# New Skill\n"),
            test_node(
                "/Wiki/skills/acme/legal-review/provenance.md",
                "# New Provenance\n",
            ),
            test_node("/Wiki/skills/acme/legal-review/evals.md", "# New Evals\n"),
            test_node(
                "/Wiki/public-skills/acme/legal-review/manifest.md",
                "---\nkind: kinic.skill\nschema_version: 1\nid: acme/legal-review\nversion: 0.1.0\npublisher: acme\nentry: SKILL.md\nknowledge: []\npermissions: {}\nprovenance:\n  source: local\n  source_ref: old\n---\n# Old Public Manifest\n",
            ),
            test_node(
                "/Wiki/public-skills/acme/legal-review/SKILL.md",
                "# Old Public Skill\n",
            ),
            test_node(
                "/Sources/raw/skill-imports/acme-legal-review/acme-legal-review.md",
                "# Raw\n",
            ),
        ],
        ..MockClient::default()
    };

    run_command(
        &client,
        test_cli(Command::Skill {
            command: SkillCommand::Public {
                command: SkillPublicCommand::Promote {
                    id: "acme/legal-review".to_string(),
                    json: true,
                },
            },
        }),
    )
    .await
    .expect("clean skill should promote");

    let writes = client.writes.lock().expect("writes should lock");
    let old_public = writes
        .iter()
        .find(|write| {
            write
                .path
                .starts_with("/Wiki/public-skills/acme/legal-review/versions/")
                && write.path.ends_with("/SKILL.md")
        })
        .expect("old public skill should be versioned");
    assert_eq!(old_public.content, "# Old Public Skill\n");
}

#[tokio::test]
async fn skill_public_promote_fails_on_audit_warning() {
    let client = MockClient {
        nodes: vec![
            test_node(
                "/Wiki/skills/acme/legal-review/manifest.md",
                "---\nkind: kinic.skill\nschema_version: 1\nid: acme/legal-review\nversion: 0.1.0\npublisher: acme\nentry: SKILL.md\nknowledge: []\npermissions:\n  file_read: true\n  network: false\n  shell: false\nprovenance:\n  source: local\n  source_ref: local\n---\n# Manifest\n",
            ),
            test_node(
                "/Wiki/skills/acme/legal-review/SKILL.md",
                "# Skill\n\nfetch(\"https://example.com\")\n",
            ),
            test_node(
                "/Wiki/skills/acme/legal-review/provenance.md",
                "# Provenance\n",
            ),
            test_node("/Wiki/skills/acme/legal-review/evals.md", "# Evals\n"),
            test_node(
                "/Sources/raw/skill-imports/acme-legal-review/acme-legal-review.md",
                "# Raw\n",
            ),
        ],
        ..MockClient::default()
    };

    let error = run_command(
        &client,
        test_cli(Command::Skill {
            command: SkillCommand::Public {
                command: SkillPublicCommand::Promote {
                    id: "acme/legal-review".to_string(),
                    json: true,
                },
            },
        }),
    )
    .await
    .expect_err("warning should block promote");

    assert!(error.to_string().contains("clean audit"));
    assert!(client.writes.lock().expect("writes should lock").is_empty());
}

#[tokio::test]
async fn skill_public_install_writes_catalog_lockfile() {
    let dir = tempdir().expect("temp dir should exist");
    let output = dir.path().join("public-skill");
    let client = MockClient {
        nodes: vec![
            test_node(
                "/Wiki/public-skills/acme/legal-review/SKILL.md",
                "# Public Skill\n",
            ),
            test_node(
                "/Wiki/public-skills/acme/legal-review/manifest.md",
                "---\nkind: kinic.skill\nschema_version: 1\nid: acme/legal-review\nversion: 0.1.0\npublisher: acme\nentry: SKILL.md\nknowledge: []\npermissions: {}\nprovenance:\n  source: local\n---\n# Manifest\n",
            ),
        ],
        ..MockClient::default()
    };

    run_command(
        &client,
        test_cli(Command::Skill {
            command: SkillCommand::Public {
                command: SkillPublicCommand::Install {
                    id: "acme/legal-review".to_string(),
                    output: Some(output.clone()),
                    skills_dir: None,
                    lockfile: true,
                    json: true,
                },
            },
        }),
    )
    .await
    .expect("public install should succeed");

    let lockfile =
        std::fs::read_to_string(output.join("skill.lock.json")).expect("lockfile should exist");
    assert!(lockfile.contains(r#""catalog": "public""#));
    assert!(lockfile.contains(r#""source_path": "/Wiki/public-skills/acme/legal-review""#));
}

#[tokio::test]
async fn skill_public_revoke_deletes_public_files_only() {
    let client = MockClient {
        nodes: vec![
            test_node(
                "/Wiki/public-skills/acme/legal-review/SKILL.md",
                "# Skill\n",
            ),
            test_node(
                "/Wiki/public-skills/acme/legal-review/manifest.md",
                "# Manifest\n",
            ),
            test_node("/Wiki/skills/acme/legal-review/SKILL.md", "# Private\n"),
        ],
        ..MockClient::default()
    };

    run_command(
        &client,
        test_cli(Command::Skill {
            command: SkillCommand::Public {
                command: SkillPublicCommand::Revoke {
                    id: "acme/legal-review".to_string(),
                    json: true,
                },
            },
        }),
    )
    .await
    .expect("public revoke should succeed");

    let deletes = client.deletes.lock().expect("deletes should lock");
    assert!(
        deletes
            .iter()
            .any(|delete| { delete.path == "/Wiki/public-skills/acme/legal-review/SKILL.md" })
    );
    assert!(
        deletes
            .iter()
            .all(|delete| !delete.path.starts_with("/Wiki/skills/"))
    );
}

#[tokio::test]
async fn skill_install_rejects_output_and_skills_dir_together() {
    let dir = tempdir().expect("temp dir should exist");
    let client = MockClient::default();

    let error = run_command(
        &client,
        test_cli(Command::Skill {
            command: SkillCommand::Install {
                id: "acme/legal-review".to_string(),
                output: Some(dir.path().join("out")),
                skills_dir: Some(dir.path().join("skills")),
                lockfile: false,
                json: false,
            },
        }),
    )
    .await
    .expect_err("output and skills-dir should conflict");

    assert!(
        error
            .to_string()
            .contains("either --output or --skills-dir")
    );
}

#[tokio::test]
async fn skill_install_missing_skill_does_not_create_output() {
    let dir = tempdir().expect("temp dir should exist");
    let output = dir.path().join("out");
    let client = MockClient {
        nodes: vec![vfs_types::Node {
            path: "/Wiki/skills/acme/legal-review/manifest.md".to_string(),
            kind: NodeKind::File,
            content: "---\nkind: kinic.skill\nschema_version: 1\nid: acme/legal-review\nversion: 0.1.0\npublisher: acme\nentry: SKILL.md\nknowledge: []\npermissions:\n  file_read: true\n  network: false\n  shell: false\nprovenance:\n  source: local\n  source_ref: local\n---\n# Manifest\n".to_string(),
            created_at: 1,
            updated_at: 1,
            etag: "etag".to_string(),
            metadata_json: "{}".to_string(),
        }],
        ..Default::default()
    };

    let error = run_command(
        &client,
        test_cli(Command::Skill {
            command: SkillCommand::Install {
                id: "acme/legal-review".to_string(),
                output: Some(output.clone()),
                skills_dir: None,
                lockfile: false,
                json: false,
            },
        }),
    )
    .await
    .expect_err("missing SKILL.md should fail");

    assert!(error.to_string().contains("SKILL.md missing"));
    assert!(!output.exists());
}

#[tokio::test]
async fn skill_list_reads_manifest_nodes() {
    let client = MockClient {
        nodes: vec![vfs_types::Node {
            path: "/Wiki/skills/acme/legal-review/manifest.md".to_string(),
            kind: NodeKind::File,
            content: "---\nkind: kinic.skill\nschema_version: 1\nid: acme/legal-review\nversion: 0.1.0\npublisher: acme\nentry: SKILL.md\nknowledge: []\npermissions:\n  file_read: true\n  network: false\n  shell: false\nprovenance:\n  source: local\n  source_ref: local\n---\n# Manifest\n".to_string(),
            created_at: 1,
            updated_at: 1,
            etag: "etag".to_string(),
            metadata_json: "{}".to_string(),
        }],
        ..Default::default()
    };

    run_command(
        &client,
        test_cli(Command::Skill {
            command: SkillCommand::List {
                prefix: "/Wiki/skills".to_string(),
                json: true,
            },
        }),
    )
    .await
    .expect("skill list should succeed");

    let lists = client.lists.lock().expect("lists should lock");
    assert_eq!(lists[0].prefix, "/Wiki/skills");
    assert!(lists[0].recursive);
}

#[tokio::test]
async fn append_node_command_supports_source_kind() {
    let dir = tempdir().expect("temp dir should exist");
    let input = PathBuf::from(dir.path()).join("append-source.md");
    std::fs::write(&input, "\nnext").expect("input should write");
    let client = MockClient::default();

    run_command(
        &client,
        test_cli(Command::AppendNode {
            path: "/Sources/raw/source/source.md".to_string(),
            input,
            kind: Some(NodeKindArg::Source),
            metadata_json: Some("{}".to_string()),
            expected_etag: Some("etag-1".to_string()),
            separator: Some("\n".to_string()),
            json: false,
        }),
    )
    .await
    .expect("append source command should succeed");

    let appends = client.appends.lock().expect("appends should lock");
    assert_eq!(appends.len(), 1);
    assert_eq!(appends[0].path, "/Sources/raw/source/source.md");
    assert_eq!(appends[0].kind, Some(NodeKind::Source));
}

#[tokio::test]
async fn list_children_command_calls_canister_children() {
    let client = MockClient::default();

    run_command(
        &client,
        test_cli(Command::ListChildren {
            path: "/Wiki".to_string(),
            json: true,
        }),
    )
    .await
    .expect("list children command should succeed");

    let requests = client.child_lists.lock().expect("child lists should lock");
    assert_eq!(requests.len(), 1);
    assert_eq!(requests[0].path, "/Wiki");
}

#[tokio::test]
async fn write_node_command_rejects_noncanonical_source_path() {
    let dir = tempdir().expect("temp dir should exist");
    let input = PathBuf::from(dir.path()).join("source.md");
    std::fs::write(&input, "# Source\n\nBody").expect("input should write");
    let client = MockClient::default();

    let error = run_command(
        &client,
        test_cli(Command::WriteNode {
            path: "/Sources/raw/source.md".to_string(),
            kind: NodeKindArg::Source,
            input,
            metadata_json: "{}".to_string(),
            expected_etag: None,
            json: false,
        }),
    )
    .await
    .expect_err("noncanonical source path should fail");

    assert!(
        error.to_string().contains("canonical form")
            || error.to_string().contains("source path must stay under")
    );
}

#[tokio::test]
async fn append_node_command_rejects_noncanonical_source_path_when_kind_is_explicit() {
    let dir = tempdir().expect("temp dir should exist");
    let input = PathBuf::from(dir.path()).join("append-source.md");
    std::fs::write(&input, "\nnext").expect("input should write");
    let client = MockClient::default();

    let error = run_command(
        &client,
        test_cli(Command::AppendNode {
            path: "/Wiki/topic.md".to_string(),
            input,
            kind: Some(NodeKindArg::Source),
            metadata_json: Some("{}".to_string()),
            expected_etag: Some("etag-1".to_string()),
            separator: Some("\n".to_string()),
            json: false,
        }),
    )
    .await
    .expect_err("explicit source kind with wiki path should fail");

    assert!(error.to_string().contains("source path must stay under"));
}

#[tokio::test]
async fn edit_node_command_calls_canister_edit() {
    let client = MockClient::default();

    run_command(
        &client,
        test_cli(Command::EditNode {
            path: "/Wiki/foo.md".to_string(),
            old_text: "before".to_string(),
            new_text: "after".to_string(),
            expected_etag: Some("etag-1".to_string()),
            replace_all: true,
            json: false,
        }),
    )
    .await
    .expect("edit command should succeed");

    let edits = client.edits.lock().expect("edits should lock");
    assert_eq!(edits.len(), 1);
    assert!(edits[0].replace_all);
    assert_eq!(edits[0].old_text, "before");
    assert_eq!(edits[0].new_text, "after");
}

#[tokio::test]
async fn mkdir_node_command_calls_canister_mkdir() {
    let client = MockClient::default();

    run_command(
        &client,
        test_cli(Command::MkdirNode {
            path: "/Wiki/new-dir".to_string(),
            json: false,
        }),
    )
    .await
    .expect("mkdir command should succeed");

    let mkdirs = client.mkdirs.lock().expect("mkdirs should lock");
    assert_eq!(mkdirs.len(), 1);
    assert_eq!(mkdirs[0].path, "/Wiki/new-dir");
}

#[tokio::test]
async fn move_node_command_calls_canister_move() {
    let client = MockClient::default();

    run_command(
        &client,
        test_cli(Command::MoveNode {
            from_path: "/Wiki/from.md".to_string(),
            to_path: "/Wiki/to.md".to_string(),
            expected_etag: Some("etag-1".to_string()),
            overwrite: true,
            json: false,
        }),
    )
    .await
    .expect("move command should succeed");

    let moves = client.moves.lock().expect("moves should lock");
    assert_eq!(moves.len(), 1);
    assert_eq!(moves[0].from_path, "/Wiki/from.md");
    assert_eq!(moves[0].to_path, "/Wiki/to.md");
    assert!(moves[0].overwrite);
}

#[tokio::test]
async fn move_node_command_allows_canonical_source_target() {
    let client = MockClient {
        nodes: vec![vfs_types::Node {
            path: "/Sources/raw/source/source.md".to_string(),
            kind: NodeKind::Source,
            content: "# Source".to_string(),
            created_at: 1,
            updated_at: 2,
            etag: "etag-1".to_string(),
            metadata_json: "{}".to_string(),
        }],
        ..Default::default()
    };

    run_command(
        &client,
        test_cli(Command::MoveNode {
            from_path: "/Sources/raw/source/source.md".to_string(),
            to_path: "/Sources/raw/renamed/renamed.md".to_string(),
            expected_etag: Some("etag-1".to_string()),
            overwrite: false,
            json: false,
        }),
    )
    .await
    .expect("source move should succeed");

    let moves = client.moves.lock().expect("moves should lock");
    assert_eq!(moves.len(), 1);
    assert_eq!(moves[0].to_path, "/Sources/raw/renamed/renamed.md");
}

#[tokio::test]
async fn move_node_command_rejects_noncanonical_source_target() {
    let client = MockClient {
        nodes: vec![vfs_types::Node {
            path: "/Sources/raw/source/source.md".to_string(),
            kind: NodeKind::Source,
            content: "# Source".to_string(),
            created_at: 1,
            updated_at: 2,
            etag: "etag-1".to_string(),
            metadata_json: "{}".to_string(),
        }],
        ..Default::default()
    };

    let error = run_command(
        &client,
        test_cli(Command::MoveNode {
            from_path: "/Sources/raw/source/source.md".to_string(),
            to_path: "/Sources/raw/renamed/wrong.md".to_string(),
            expected_etag: Some("etag-1".to_string()),
            overwrite: false,
            json: false,
        }),
    )
    .await
    .expect_err("noncanonical source move should fail");

    assert!(error.to_string().contains("canonical form"));
    assert!(client.moves.lock().expect("moves should lock").is_empty());
}

#[tokio::test]
async fn glob_node_command_calls_canister_glob() {
    let client = MockClient::default();

    run_command(
        &client,
        test_cli(Command::GlobNodes {
            pattern: "**/*.md".to_string(),
            path: "/Wiki".to_string(),
            node_type: Some(GlobNodeTypeArg::Directory),
            json: false,
        }),
    )
    .await
    .expect("glob command should succeed");

    let globs = client.globs.lock().expect("globs should lock");
    assert_eq!(globs.len(), 1);
    assert_eq!(globs[0].pattern, "**/*.md");
}

#[tokio::test]
async fn recent_node_command_calls_canister_recent() {
    let client = MockClient::default();

    run_command(
        &client,
        test_cli(Command::RecentNodes {
            limit: 5,
            path: "/Wiki".to_string(),
            json: false,
        }),
    )
    .await
    .expect("recent command should succeed");

    let recents = client.recents.lock().expect("recents should lock");
    assert_eq!(recents.len(), 1);
    assert_eq!(recents[0].limit, 5);
    assert_eq!(recents[0].path.as_deref(), Some("/Wiki"));
}

#[tokio::test]
async fn multi_edit_node_command_calls_canister_multi_edit() {
    let dir = tempdir().expect("temp dir should exist");
    let input = PathBuf::from(dir.path()).join("edits.json");
    std::fs::write(
        &input,
        r#"[{"old_text":"before","new_text":"after"},{"old_text":"alpha","new_text":"beta"}]"#,
    )
    .expect("edits file should write");
    let client = MockClient::default();

    run_command(
        &client,
        test_cli(Command::MultiEditNode {
            path: "/Wiki/foo.md".to_string(),
            edits_file: input,
            expected_etag: Some("etag-1".to_string()),
            json: false,
        }),
    )
    .await
    .expect("multi edit command should succeed");

    let edits = client.multi_edits.lock().expect("multi edits should lock");
    assert_eq!(edits.len(), 1);
    assert_eq!(edits[0].edits.len(), 2);
}

#[tokio::test]
async fn search_path_command_calls_canister_path_search() {
    let client = MockClient::default();

    run_command(
        &client,
        test_cli(Command::SearchPathRemote {
            query_text: "nested".to_string(),
            prefix: "/Wiki".to_string(),
            top_k: 7,
            preview_mode: None,
            json: false,
        }),
    )
    .await
    .expect("search path command should succeed");

    let searches = client
        .path_searches
        .lock()
        .expect("path searches should lock");
    assert_eq!(searches.len(), 1);
    assert_eq!(searches[0].query_text, "nested");
    assert_eq!(searches[0].prefix.as_deref(), Some("/Wiki"));
    assert_eq!(searches[0].top_k, 7);
    assert_eq!(searches[0].preview_mode, None);
}

#[tokio::test]
async fn search_commands_pass_explicit_content_start_preview_mode() {
    let client = MockClient::default();

    run_command(
        &client,
        test_cli(Command::SearchRemote {
            query_text: "body".to_string(),
            prefix: "/Wiki".to_string(),
            top_k: 3,
            preview_mode: Some(SearchPreviewModeArg::ContentStart),
            json: true,
        }),
    )
    .await
    .expect("search command should succeed");
    run_command(
        &client,
        test_cli(Command::SearchPathRemote {
            query_text: "nested".to_string(),
            prefix: "/Wiki".to_string(),
            top_k: 5,
            preview_mode: Some(SearchPreviewModeArg::ContentStart),
            json: true,
        }),
    )
    .await
    .expect("path search command should succeed");

    let searches = client.searches.lock().expect("searches should lock");
    assert_eq!(searches.len(), 1);
    assert_eq!(
        searches[0].preview_mode,
        Some(SearchPreviewMode::ContentStart)
    );
    drop(searches);

    let path_searches = client
        .path_searches
        .lock()
        .expect("path searches should lock");
    assert_eq!(path_searches.len(), 1);
    assert_eq!(
        path_searches[0].preview_mode,
        Some(SearchPreviewMode::ContentStart)
    );
}

#[tokio::test]
async fn delete_tree_command_lists_recursive_and_deletes_deepest_first() {
    let client = MockClient {
        nodes: vec![
            vfs_types::Node {
                path: "/Wiki/tree".to_string(),
                kind: NodeKind::File,
                content: String::new(),
                created_at: 1,
                updated_at: 1,
                etag: "etag-tree".to_string(),
                metadata_json: "{}".to_string(),
            },
            vfs_types::Node {
                path: "/Wiki/tree/leaf.md".to_string(),
                kind: NodeKind::File,
                content: String::new(),
                created_at: 1,
                updated_at: 2,
                etag: "etag-leaf".to_string(),
                metadata_json: "{}".to_string(),
            },
            vfs_types::Node {
                path: "/Wiki/tree/branch/twig.md".to_string(),
                kind: NodeKind::File,
                content: String::new(),
                created_at: 1,
                updated_at: 3,
                etag: "etag-twig".to_string(),
                metadata_json: "{}".to_string(),
            },
        ],
        ..Default::default()
    };

    run_command(
        &client,
        test_cli(Command::DeleteTree {
            path: "/Wiki/tree".to_string(),
            json: false,
        }),
    )
    .await
    .expect("delete tree command should succeed");

    let lists = client.lists.lock().expect("lists should lock");
    assert_eq!(lists.len(), 1);
    assert_eq!(lists[0].prefix, "/Wiki/tree");
    assert!(lists[0].recursive);

    let deletes = client.deletes.lock().expect("deletes should lock");
    let deleted_paths = deletes
        .iter()
        .map(|request| request.path.as_str())
        .collect::<Vec<_>>();
    assert_eq!(
        deleted_paths,
        vec![
            "/Wiki/tree/branch/twig.md",
            "/Wiki/tree/leaf.md",
            "/Wiki/tree"
        ]
    );
    let delete_etags = deletes
        .iter()
        .map(|request| request.expected_etag.as_deref())
        .collect::<Vec<_>>();
    assert_eq!(
        delete_etags,
        vec![Some("etag-twig"), Some("etag-leaf"), Some("etag-tree")]
    );
}

#[tokio::test]
async fn delete_tree_command_succeeds_when_nothing_matches() {
    let client = MockClient::default();

    run_command(
        &client,
        test_cli(Command::DeleteTree {
            path: "/Wiki/missing".to_string(),
            json: false,
        }),
    )
    .await
    .expect("delete tree should allow empty matches");

    let deletes = client.deletes.lock().expect("deletes should lock");
    assert!(deletes.is_empty());
}

#[tokio::test]
async fn delete_tree_command_stops_after_first_delete_failure() {
    let client = MockClient {
        nodes: vec![
            vfs_types::Node {
                path: "/Wiki/tree/a.md".to_string(),
                kind: NodeKind::File,
                content: String::new(),
                created_at: 1,
                updated_at: 1,
                etag: "etag-a".to_string(),
                metadata_json: "{}".to_string(),
            },
            vfs_types::Node {
                path: "/Wiki/tree/deeper/b.md".to_string(),
                kind: NodeKind::File,
                content: String::new(),
                created_at: 1,
                updated_at: 2,
                etag: "etag-b".to_string(),
                metadata_json: "{}".to_string(),
            },
            vfs_types::Node {
                path: "/Wiki/tree/z.md".to_string(),
                kind: NodeKind::File,
                content: String::new(),
                created_at: 1,
                updated_at: 3,
                etag: "etag-z".to_string(),
                metadata_json: "{}".to_string(),
            },
        ],
        delete_fail_paths: ["/Wiki/tree/deeper/b.md".to_string()].into_iter().collect(),
        ..Default::default()
    };

    let error = run_command(
        &client,
        test_cli(Command::DeleteTree {
            path: "/Wiki/tree".to_string(),
            json: false,
        }),
    )
    .await
    .expect_err("delete tree should fail fast");
    assert!(error.to_string().contains("/Wiki/tree/deeper/b.md"));

    let deletes = client.deletes.lock().expect("deletes should lock");
    assert!(deletes.is_empty());
}

#[test]
fn delete_tree_command_parses_from_cli() {
    let parsed = Cli::try_parse_from(["vfs-cli", "delete-tree", "--path", "/Wiki/tree"])
        .expect("delete-tree should parse");
    match parsed.command {
        Command::DeleteTree { path, json } => {
            assert_eq!(path, "/Wiki/tree");
            assert!(!json);
        }
        other => panic!("unexpected command: {other:?}"),
    }
}
