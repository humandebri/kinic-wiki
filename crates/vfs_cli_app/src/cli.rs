// Where: crates/vfs_cli_app/src/cli.rs
// What: clap definitions for the FS-first CLI surface.
// Why: Agents need direct node operations and path-based mirror sync commands.
use clap::{Parser, Subcommand, ValueEnum};
use std::path::PathBuf;
use vfs_cli::cli::VfsCommand;
pub use vfs_cli::cli::{ConnectionArgs, GlobNodeTypeArg, NodeKindArg, SearchPreviewModeArg};
use wiki_domain::{
    DEFAULT_MIRROR_ROOT, PUBLIC_SKILL_REGISTRY_ROOT, SKILL_REGISTRY_ROOT, WIKI_ROOT_PATH,
};

#[derive(Parser, Debug)]
#[command(name = "vfs-cli")]
#[command(about = "Agent-facing CLI for the Kinic FS-first wiki")]
pub struct Cli {
    #[command(flatten)]
    pub connection: ConnectionArgs,

    #[command(subcommand)]
    pub command: Command,
}

#[derive(Subcommand, Debug, Clone)]
pub enum Command {
    Skill {
        #[command(subcommand)]
        command: SkillCommand,
    },
    Github {
        #[command(subcommand)]
        command: GitHubCommand,
    },
    RebuildIndex,
    RebuildScopeIndex {
        #[arg(long)]
        scope: String,
    },
    GenerateConversationWiki {
        #[arg(long)]
        source_path: String,
        #[arg(long)]
        json: bool,
    },
    ReadNode {
        #[arg(long)]
        path: String,
        #[arg(long)]
        json: bool,
    },
    ListNodes {
        #[arg(long, default_value = WIKI_ROOT_PATH)]
        prefix: String,
        #[arg(long)]
        recursive: bool,
        #[arg(long)]
        json: bool,
    },
    ListChildren {
        #[arg(long, default_value = WIKI_ROOT_PATH)]
        path: String,
        #[arg(long)]
        json: bool,
    },
    WriteNode {
        #[arg(long)]
        path: String,
        #[arg(long, value_enum, default_value_t = NodeKindArg::File)]
        kind: NodeKindArg,
        #[arg(long)]
        input: PathBuf,
        #[arg(long, default_value = "{}")]
        metadata_json: String,
        #[arg(long)]
        expected_etag: Option<String>,
        #[arg(long)]
        json: bool,
    },
    AppendNode {
        #[arg(long)]
        path: String,
        #[arg(long)]
        input: PathBuf,
        #[arg(long, value_enum)]
        kind: Option<NodeKindArg>,
        #[arg(long)]
        metadata_json: Option<String>,
        #[arg(long)]
        expected_etag: Option<String>,
        #[arg(long)]
        separator: Option<String>,
        #[arg(long)]
        json: bool,
    },
    EditNode {
        #[arg(long)]
        path: String,
        #[arg(long)]
        old_text: String,
        #[arg(long)]
        new_text: String,
        #[arg(long)]
        expected_etag: Option<String>,
        #[arg(long)]
        replace_all: bool,
        #[arg(long)]
        json: bool,
    },
    DeleteNode {
        #[arg(long)]
        path: String,
        #[arg(long)]
        expected_etag: Option<String>,
        #[arg(long)]
        json: bool,
    },
    DeleteTree {
        #[arg(long)]
        path: String,
        #[arg(long)]
        json: bool,
    },
    MkdirNode {
        #[arg(long)]
        path: String,
        #[arg(long)]
        json: bool,
    },
    MoveNode {
        #[arg(long)]
        from_path: String,
        #[arg(long)]
        to_path: String,
        #[arg(long)]
        expected_etag: Option<String>,
        #[arg(long)]
        overwrite: bool,
        #[arg(long)]
        json: bool,
    },
    GlobNodes {
        pattern: String,
        #[arg(long, default_value = WIKI_ROOT_PATH)]
        path: String,
        #[arg(long, value_enum)]
        node_type: Option<GlobNodeTypeArg>,
        #[arg(long)]
        json: bool,
    },
    RecentNodes {
        #[arg(long, help = "Maximum 100; 0 is treated as 1 by the canister")]
        limit: u32,
        #[arg(long, default_value = WIKI_ROOT_PATH)]
        path: String,
        #[arg(long)]
        json: bool,
    },
    ReadNodeContext {
        #[arg(long)]
        path: String,
        #[arg(long, default_value_t = 20)]
        link_limit: u32,
        #[arg(long)]
        json: bool,
    },
    GraphNeighborhood {
        #[arg(long)]
        center_path: String,
        #[arg(long, default_value_t = 1)]
        depth: u32,
        #[arg(long, default_value_t = 100)]
        limit: u32,
        #[arg(long)]
        json: bool,
    },
    GraphLinks {
        #[arg(long, default_value = WIKI_ROOT_PATH)]
        prefix: String,
        #[arg(long, default_value_t = 100)]
        limit: u32,
        #[arg(long)]
        json: bool,
    },
    IncomingLinks {
        #[arg(long)]
        path: String,
        #[arg(long, default_value_t = 20)]
        limit: u32,
        #[arg(long)]
        json: bool,
    },
    OutgoingLinks {
        #[arg(long)]
        path: String,
        #[arg(long, default_value_t = 20)]
        limit: u32,
        #[arg(long)]
        json: bool,
    },
    MultiEditNode {
        #[arg(long)]
        path: String,
        #[arg(long)]
        edits_file: PathBuf,
        #[arg(long)]
        expected_etag: Option<String>,
        #[arg(long)]
        json: bool,
    },
    SearchRemote {
        query_text: String,
        #[arg(long, default_value = WIKI_ROOT_PATH)]
        prefix: String,
        #[arg(
            long,
            default_value_t = 10,
            help = "Maximum 100; 0 is treated as 1 by the canister. Search preview defaults to light."
        )]
        top_k: u32,
        #[arg(long, value_enum)]
        preview_mode: Option<SearchPreviewModeArg>,
        #[arg(long)]
        json: bool,
    },
    SearchPathRemote {
        query_text: String,
        #[arg(long, default_value = WIKI_ROOT_PATH)]
        prefix: String,
        #[arg(
            long,
            default_value_t = 10,
            help = "Maximum 100; 0 is treated as 1 by the canister"
        )]
        top_k: u32,
        #[arg(long, value_enum)]
        preview_mode: Option<SearchPreviewModeArg>,
        #[arg(long)]
        json: bool,
    },
    LintLocal {
        #[arg(long)]
        vault_path: PathBuf,
        #[arg(long, default_value = DEFAULT_MIRROR_ROOT)]
        mirror_root: String,
        #[arg(long)]
        json: bool,
    },
    Status {
        #[arg(long)]
        vault_path: Option<PathBuf>,
        #[arg(long, default_value = DEFAULT_MIRROR_ROOT)]
        mirror_root: String,
        #[arg(long)]
        json: bool,
    },
    Pull {
        #[arg(long)]
        vault_path: PathBuf,
        #[arg(long, default_value = DEFAULT_MIRROR_ROOT)]
        mirror_root: String,
        #[arg(long)]
        resync: bool,
    },
    Push {
        #[arg(long)]
        vault_path: PathBuf,
        #[arg(long, default_value = DEFAULT_MIRROR_ROOT)]
        mirror_root: String,
    },
}

#[derive(Subcommand, Debug, Clone)]
pub enum SkillCommand {
    Policy {
        #[command(subcommand)]
        command: SkillPolicyCommand,
    },
    Index {
        #[command(subcommand)]
        command: SkillIndexCommand,
    },
    Local {
        #[command(subcommand)]
        command: SkillLocalCommand,
    },
    Public {
        #[command(subcommand)]
        command: SkillPublicCommand,
    },
    Versions {
        #[command(subcommand)]
        command: SkillVersionsCommand,
    },
    Import {
        #[arg(
            long,
            conflicts_with = "github",
            help = "Local skill directory containing SKILL.md"
        )]
        source: Option<String>,
        #[arg(
            long,
            conflicts_with = "source",
            help = "GitHub source in owner/repo or owner/repo:path form"
        )]
        github: Option<String>,
        #[arg(long, help = "Repository-relative skill path for --github")]
        path: Option<String>,
        #[arg(
            long = "ref",
            default_value = "HEAD",
            help = "GitHub commit, tag, or branch ref"
        )]
        ref_name: String,
        #[arg(long)]
        id: String,
        #[arg(long)]
        json: bool,
    },
    Update {
        id: String,
        #[arg(
            long = "ref",
            default_value = "HEAD",
            help = "GitHub commit, tag, or branch ref"
        )]
        ref_name: String,
        #[arg(long)]
        json: bool,
    },
    Inspect {
        id: String,
        #[arg(long)]
        json: bool,
    },
    List {
        #[arg(long, default_value = SKILL_REGISTRY_ROOT)]
        prefix: String,
        #[arg(long)]
        json: bool,
    },
    Audit {
        id: String,
        #[arg(long, value_enum)]
        fail_on: Option<AuditFailOnArg>,
        #[arg(long)]
        json: bool,
    },
    Install {
        id: String,
        #[arg(long, help = "Exact output directory for the installed skill files")]
        output: Option<PathBuf>,
        #[arg(
            long,
            help = "Skills root; writes to <skills-dir>/<publisher>/<name>. Use either --output or --skills-dir."
        )]
        skills_dir: Option<PathBuf>,
        #[arg(
            long,
            help = "Write install lock metadata to the installed skill directory"
        )]
        lockfile: bool,
        #[arg(long)]
        json: bool,
    },
}

#[derive(Subcommand, Debug, Clone)]
pub enum SkillIndexCommand {
    List {
        #[arg(long, default_value = "./skills.index.toml")]
        index: PathBuf,
        #[arg(long)]
        json: bool,
    },
    Inspect {
        id: String,
        #[arg(long, default_value = "./skills.index.toml")]
        index: PathBuf,
        #[arg(long)]
        json: bool,
    },
    Install {
        id: String,
        #[arg(long, default_value = "./skills.index.toml")]
        index: PathBuf,
        #[arg(long, help = "Exact output directory for the installed skill files")]
        output: PathBuf,
        #[arg(
            long,
            help = "Write install lock metadata to the installed skill directory"
        )]
        lockfile: bool,
        #[arg(long)]
        json: bool,
    },
    InstallEnabled {
        #[arg(long, default_value = "./skills.index.toml")]
        index: PathBuf,
        #[arg(long, help = "Skills root; writes to <skills-dir>/<publisher>/<name>.")]
        skills_dir: PathBuf,
        #[arg(
            long,
            help = "Write install lock metadata to each installed skill directory"
        )]
        lockfile: bool,
        #[arg(long)]
        json: bool,
    },
}

#[derive(Subcommand, Debug, Clone)]
pub enum SkillLocalCommand {
    Audit {
        dir: PathBuf,
        #[arg(long)]
        json: bool,
    },
    Diff {
        dir: PathBuf,
        #[arg(long)]
        json: bool,
    },
    Install {
        dir: PathBuf,
        #[arg(long, help = "Skills root; writes to <skills-dir>/<publisher>/<name>.")]
        skills_dir: PathBuf,
        #[arg(long)]
        json: bool,
    },
}

#[derive(Subcommand, Debug, Clone)]
pub enum SkillPublicCommand {
    Promote {
        id: String,
        #[arg(long)]
        json: bool,
    },
    List {
        #[arg(long, default_value = PUBLIC_SKILL_REGISTRY_ROOT)]
        prefix: String,
        #[arg(long)]
        json: bool,
    },
    Inspect {
        id: String,
        #[arg(long)]
        json: bool,
    },
    Install {
        id: String,
        #[arg(long, help = "Exact output directory for the installed skill files")]
        output: Option<PathBuf>,
        #[arg(
            long,
            help = "Skills root; writes to <skills-dir>/<publisher>/<name>. Use either --output or --skills-dir."
        )]
        skills_dir: Option<PathBuf>,
        #[arg(
            long,
            help = "Write install lock metadata to the installed skill directory"
        )]
        lockfile: bool,
        #[arg(long)]
        json: bool,
    },
    Revoke {
        id: String,
        #[arg(long)]
        json: bool,
    },
    Versions {
        #[command(subcommand)]
        command: SkillVersionsCommand,
    },
}

#[derive(Subcommand, Debug, Clone)]
pub enum SkillVersionsCommand {
    List {
        id: String,
        #[arg(long)]
        json: bool,
    },
    Inspect {
        id: String,
        version: String,
        #[arg(long)]
        json: bool,
    },
}

#[derive(Subcommand, Debug, Clone)]
pub enum GitHubCommand {
    Ingest {
        #[command(subcommand)]
        command: GitHubIngestCommand,
    },
}

#[derive(Subcommand, Debug, Clone)]
pub enum GitHubIngestCommand {
    Issue {
        target: String,
        #[arg(long)]
        json: bool,
    },
    Pr {
        target: String,
        #[arg(long)]
        json: bool,
    },
}

#[derive(Subcommand, Debug, Clone)]
pub enum SkillPolicyCommand {
    Enable {
        #[arg(long)]
        json: bool,
    },
    Whoami {
        #[arg(long)]
        json: bool,
    },
    Policy {
        #[arg(long)]
        json: bool,
    },
    List {
        #[arg(long)]
        json: bool,
    },
    Explain {
        principal: String,
        #[arg(long)]
        json: bool,
    },
    Grant {
        principal: String,
        role: String,
        #[arg(long)]
        json: bool,
    },
    Revoke {
        principal: String,
        role: String,
        #[arg(long)]
        json: bool,
    },
}

#[derive(ValueEnum, Debug, Clone, Copy, PartialEq, Eq)]
pub enum AuditFailOnArg {
    Error,
    Warning,
}

impl Command {
    pub fn as_vfs_command(&self) -> Option<VfsCommand> {
        match self {
            Self::ReadNode { path, json } => Some(VfsCommand::ReadNode {
                path: path.clone(),
                json: *json,
            }),
            Self::ListNodes {
                prefix,
                recursive,
                json,
            } => Some(VfsCommand::ListNodes {
                prefix: prefix.clone(),
                recursive: *recursive,
                json: *json,
            }),
            Self::ListChildren { path, json } => Some(VfsCommand::ListChildren {
                path: path.clone(),
                json: *json,
            }),
            Self::WriteNode {
                path,
                kind,
                input,
                metadata_json,
                expected_etag,
                json,
            } => Some(VfsCommand::WriteNode {
                path: path.clone(),
                kind: *kind,
                input: input.clone(),
                metadata_json: metadata_json.clone(),
                expected_etag: expected_etag.clone(),
                json: *json,
            }),
            Self::AppendNode {
                path,
                input,
                kind,
                metadata_json,
                expected_etag,
                separator,
                json,
            } => Some(VfsCommand::AppendNode {
                path: path.clone(),
                input: input.clone(),
                kind: *kind,
                metadata_json: metadata_json.clone(),
                expected_etag: expected_etag.clone(),
                separator: separator.clone(),
                json: *json,
            }),
            Self::EditNode {
                path,
                old_text,
                new_text,
                expected_etag,
                replace_all,
                json,
            } => Some(VfsCommand::EditNode {
                path: path.clone(),
                old_text: old_text.clone(),
                new_text: new_text.clone(),
                expected_etag: expected_etag.clone(),
                replace_all: *replace_all,
                json: *json,
            }),
            Self::DeleteNode {
                path,
                expected_etag,
                json,
            } => Some(VfsCommand::DeleteNode {
                path: path.clone(),
                expected_etag: expected_etag.clone(),
                json: *json,
            }),
            Self::DeleteTree { path, json } => Some(VfsCommand::DeleteTree {
                path: path.clone(),
                json: *json,
            }),
            Self::MkdirNode { path, json } => Some(VfsCommand::MkdirNode {
                path: path.clone(),
                json: *json,
            }),
            Self::MoveNode {
                from_path,
                to_path,
                expected_etag,
                overwrite,
                json,
            } => Some(VfsCommand::MoveNode {
                from_path: from_path.clone(),
                to_path: to_path.clone(),
                expected_etag: expected_etag.clone(),
                overwrite: *overwrite,
                json: *json,
            }),
            Self::GlobNodes {
                pattern,
                path,
                node_type,
                json,
            } => Some(VfsCommand::GlobNodes {
                pattern: pattern.clone(),
                path: path.clone(),
                node_type: *node_type,
                json: *json,
            }),
            Self::RecentNodes { limit, path, json } => Some(VfsCommand::RecentNodes {
                limit: *limit,
                path: path.clone(),
                json: *json,
            }),
            Self::ReadNodeContext {
                path,
                link_limit,
                json,
            } => Some(VfsCommand::ReadNodeContext {
                path: path.clone(),
                link_limit: *link_limit,
                json: *json,
            }),
            Self::GraphNeighborhood {
                center_path,
                depth,
                limit,
                json,
            } => Some(VfsCommand::GraphNeighborhood {
                center_path: center_path.clone(),
                depth: *depth,
                limit: *limit,
                json: *json,
            }),
            Self::GraphLinks {
                prefix,
                limit,
                json,
            } => Some(VfsCommand::GraphLinks {
                prefix: prefix.clone(),
                limit: *limit,
                json: *json,
            }),
            Self::IncomingLinks { path, limit, json } => Some(VfsCommand::IncomingLinks {
                path: path.clone(),
                limit: *limit,
                json: *json,
            }),
            Self::OutgoingLinks { path, limit, json } => Some(VfsCommand::OutgoingLinks {
                path: path.clone(),
                limit: *limit,
                json: *json,
            }),
            Self::MultiEditNode {
                path,
                edits_file,
                expected_etag,
                json,
            } => Some(VfsCommand::MultiEditNode {
                path: path.clone(),
                edits_file: edits_file.clone(),
                expected_etag: expected_etag.clone(),
                json: *json,
            }),
            Self::SearchRemote {
                query_text,
                prefix,
                top_k,
                preview_mode,
                json,
            } => Some(VfsCommand::SearchRemote {
                query_text: query_text.clone(),
                prefix: prefix.clone(),
                top_k: *top_k,
                preview_mode: *preview_mode,
                json: *json,
            }),
            Self::SearchPathRemote {
                query_text,
                prefix,
                top_k,
                preview_mode,
                json,
            } => Some(VfsCommand::SearchPathRemote {
                query_text: query_text.clone(),
                prefix: prefix.clone(),
                top_k: *top_k,
                preview_mode: *preview_mode,
                json: *json,
            }),
            _ => None,
        }
    }
}

#[cfg(test)]
mod github_cli_tests {
    use super::{Cli, Command, SkillCommand, SkillLocalCommand, SkillVersionsCommand};
    use clap::Parser;

    #[test]
    fn skill_import_accepts_github_source() {
        let cli = Cli::try_parse_from([
            "vfs-cli",
            "--canister-id",
            "aaaaa-aa",
            "skill",
            "import",
            "--github",
            "owner/repo",
            "--path",
            "skills/foo",
            "--ref",
            "main",
            "--id",
            "acme/foo",
        ])
        .expect("CLI should parse");
        let Command::Skill {
            command:
                SkillCommand::Import {
                    source,
                    github,
                    path,
                    ref_name,
                    id,
                    ..
                },
        } = cli.command
        else {
            panic!("expected skill import command");
        };
        assert_eq!(source, None);
        assert_eq!(github.as_deref(), Some("owner/repo"));
        assert_eq!(path.as_deref(), Some("skills/foo"));
        assert_eq!(ref_name, "main");
        assert_eq!(id, "acme/foo");
    }

    #[test]
    fn skill_import_rejects_source_and_github_together() {
        let error = Cli::try_parse_from([
            "vfs-cli",
            "--canister-id",
            "aaaaa-aa",
            "skill",
            "import",
            "--source",
            "./skills/foo",
            "--github",
            "owner/repo",
            "--id",
            "acme/foo",
        ])
        .expect_err("source and github should conflict");
        assert!(error.to_string().contains("cannot be used with"));
    }

    #[test]
    fn skill_local_commands_parse() {
        let cli = Cli::try_parse_from([
            "vfs-cli",
            "--canister-id",
            "aaaaa-aa",
            "skill",
            "local",
            "audit",
            "./foo",
            "--json",
        ])
        .expect("CLI should parse");
        let Command::Skill {
            command:
                SkillCommand::Local {
                    command: SkillLocalCommand::Audit { dir, json },
                },
        } = cli.command
        else {
            panic!("expected skill local audit command");
        };
        assert_eq!(dir, std::path::PathBuf::from("./foo"));
        assert!(json);

        let cli = Cli::try_parse_from([
            "vfs-cli",
            "--canister-id",
            "aaaaa-aa",
            "skill",
            "local",
            "diff",
            "./foo",
            "--json",
        ])
        .expect("CLI should parse");
        assert!(matches!(
            cli.command,
            Command::Skill {
                command: SkillCommand::Local {
                    command: SkillLocalCommand::Diff { .. }
                }
            }
        ));

        let cli = Cli::try_parse_from([
            "vfs-cli",
            "--canister-id",
            "aaaaa-aa",
            "skill",
            "local",
            "install",
            "./foo",
            "--skills-dir",
            "./skills",
            "--json",
        ])
        .expect("CLI should parse");
        assert!(matches!(
            cli.command,
            Command::Skill {
                command: SkillCommand::Local {
                    command: SkillLocalCommand::Install { .. }
                }
            }
        ));
    }

    #[test]
    fn skill_versions_commands_parse() {
        let cli = Cli::try_parse_from([
            "vfs-cli",
            "--canister-id",
            "aaaaa-aa",
            "skill",
            "versions",
            "list",
            "acme/foo",
            "--json",
        ])
        .expect("CLI should parse");
        assert!(matches!(
            cli.command,
            Command::Skill {
                command: SkillCommand::Versions {
                    command: SkillVersionsCommand::List { .. }
                }
            }
        ));

        let cli = Cli::try_parse_from([
            "vfs-cli",
            "--canister-id",
            "aaaaa-aa",
            "skill",
            "public",
            "versions",
            "inspect",
            "acme/foo",
            "20260505T010203Z-etag",
            "--json",
        ])
        .expect("CLI should parse");
        assert!(matches!(
            cli.command,
            Command::Skill {
                command: SkillCommand::Public {
                    command: super::SkillPublicCommand::Versions {
                        command: SkillVersionsCommand::Inspect { .. }
                    }
                }
            }
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::{Cli, Command};
    use clap::{CommandFactory, Parser};

    #[test]
    fn main_cli_help_does_not_list_beam_bench() {
        let mut command = Cli::command();
        let help = command.render_long_help().to_string();

        assert!(!help.contains("beam-bench"));
    }

    #[test]
    fn skill_install_help_explains_output_choices() {
        let mut command = Cli::command();
        let skill = command
            .find_subcommand_mut("skill")
            .expect("skill subcommand should exist");
        let install = skill
            .find_subcommand_mut("install")
            .expect("install subcommand should exist");
        let help = install.render_long_help().to_string();

        assert!(help.contains("Exact output directory"));
        assert!(help.contains("Use either --output or --skills-dir"));
    }

    #[test]
    fn skill_policy_help_lists_policy_commands() {
        let mut command = Cli::command();
        let skill = command
            .find_subcommand_mut("skill")
            .expect("skill subcommand should exist");
        let policy = skill
            .find_subcommand_mut("policy")
            .expect("policy subcommand should exist");
        let help = policy.render_long_help().to_string();

        assert!(help.contains("enable"));
        assert!(help.contains("policy"));
        assert!(help.contains("grant"));
        assert!(help.contains("revoke"));
    }

    #[test]
    fn main_cli_help_mentions_identity_pem_env() {
        let mut command = Cli::command();
        let help = command.render_long_help().to_string();

        assert!(help.contains("VFS_IDENTITY_PEM"));
        assert!(help.contains("--identity-pem"));
    }

    #[test]
    fn main_cli_parses_identity_pem() {
        let cli = Cli::parse_from([
            "vfs-cli",
            "--identity-pem",
            "./identity.pem",
            "skill",
            "policy",
            "whoami",
            "--json",
        ]);

        assert_eq!(
            cli.connection.identity_pem.as_deref(),
            Some(std::path::Path::new("./identity.pem"))
        );
    }

    #[test]
    fn main_cli_parses_link_commands() {
        let cli = Cli::parse_from([
            "vfs-cli",
            "read-node-context",
            "--path",
            "/Wiki/a.md",
            "--link-limit",
            "7",
            "--json",
        ]);
        let Command::ReadNodeContext {
            path,
            link_limit,
            json,
        } = cli.command
        else {
            panic!("expected read-node-context command");
        };
        assert_eq!(path, "/Wiki/a.md");
        assert_eq!(link_limit, 7);
        assert!(json);

        let cli = Cli::parse_from([
            "vfs-cli",
            "graph-neighborhood",
            "--center-path",
            "/Wiki/a.md",
            "--depth",
            "2",
            "--limit",
            "9",
        ]);
        let Command::GraphNeighborhood {
            center_path,
            depth,
            limit,
            json,
        } = cli.command
        else {
            panic!("expected graph-neighborhood command");
        };
        assert_eq!(center_path, "/Wiki/a.md");
        assert_eq!(depth, 2);
        assert_eq!(limit, 9);
        assert!(!json);
    }

    #[test]
    fn main_cli_parses_conversation_wiki_command() {
        let cli = Cli::parse_from([
            "vfs-cli",
            "generate-conversation-wiki",
            "--source-path",
            "/Sources/raw/chatgpt-abc/chatgpt-abc.md",
            "--json",
        ]);
        let Command::GenerateConversationWiki { source_path, json } = cli.command else {
            panic!("expected generate-conversation-wiki command");
        };
        assert_eq!(source_path, "/Sources/raw/chatgpt-abc/chatgpt-abc.md");
        assert!(json);
    }
}
