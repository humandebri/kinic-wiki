// Where: crates/vfs_cli_app/src/skill_registry/policy.rs
// What: Skill policy CLI wrappers around generic VFS path policy APIs.
// Why: Skill store policy commands remain domain UX while core canister policy stays generic.
use anyhow::Result;
use serde::Serialize;
use vfs_client::VfsApi;
use wiki_domain::SKILL_REGISTRY_ROOT;

use crate::cli::SkillPolicyCommand;

use super::print_json_or_message;

#[derive(Debug, Serialize, PartialEq, Eq)]
pub(super) struct SkillPolicyExplain {
    pub(super) principal: String,
    pub(super) mode: String,
    pub(super) roles: Vec<String>,
    pub(super) can_read: bool,
    pub(super) can_write: bool,
    pub(super) can_admin: bool,
}

#[derive(Debug, Serialize, PartialEq, Eq)]
pub(super) struct SkillPolicyWhoami {
    pub(super) principal: String,
    pub(super) mode: String,
    pub(super) roles: Vec<String>,
    pub(super) can_read: bool,
    pub(super) can_write: bool,
    pub(super) can_admin: bool,
}

pub(super) async fn run_skill_policy_command(
    client: &impl VfsApi,
    command: SkillPolicyCommand,
) -> Result<()> {
    let path = SKILL_REGISTRY_ROOT;
    match command {
        SkillPolicyCommand::Enable { json } => {
            let policy = client.enable_path_policy(path).await?;
            print_json_or_message(json, &policy, "path policy enabled")?;
        }
        SkillPolicyCommand::Whoami { json } => {
            let roles = client.my_path_policy_roles(path).await?;
            let policy = client.path_policy(path).await?;
            let whoami = skill_policy_whoami(client.local_principal()?, policy.mode, roles);
            print_json_or_message(
                json,
                &whoami,
                &format!(
                    "path policy principal {} roles: {}",
                    whoami.principal,
                    whoami.roles.join(",")
                ),
            )?;
        }
        SkillPolicyCommand::Policy { json } => {
            let policy = client.path_policy(path).await?;
            print_json_or_message(json, &policy, &format!("path policy: {}", policy.mode))?;
        }
        SkillPolicyCommand::List { json } => {
            let entries = client.path_policy_entries(path).await?;
            print_json_or_message(json, &entries, &format!("{} policy entries", entries.len()))?;
        }
        SkillPolicyCommand::Explain { principal, json } => {
            let entries = client.path_policy_entries(path).await?;
            let policy = client.path_policy(path).await?;
            let roles = entries
                .iter()
                .find(|entry| entry.principal == principal)
                .map(|entry| entry.roles.clone())
                .unwrap_or_default();
            let explain = skill_policy_explain(principal, policy.mode, roles);
            print_json_or_message(
                json,
                &explain,
                &format!(
                    "path policy principal {} roles: {}",
                    explain.principal,
                    explain.roles.join(",")
                ),
            )?;
        }
        SkillPolicyCommand::Grant {
            principal,
            role,
            json,
        } => {
            client
                .grant_path_policy_role(path, principal.clone(), role.clone())
                .await?;
            let value = serde_json::json!({ "principal": principal, "role": role });
            print_json_or_message(json, &value, "path policy role granted")?;
        }
        SkillPolicyCommand::Revoke {
            principal,
            role,
            json,
        } => {
            client
                .revoke_path_policy_role(path, principal.clone(), role.clone())
                .await?;
            let value = serde_json::json!({ "principal": principal, "role": role });
            print_json_or_message(json, &value, "path policy role revoked")?;
        }
    }
    Ok(())
}

pub(super) fn skill_policy_whoami(
    principal: String,
    mode: String,
    roles: Vec<String>,
) -> SkillPolicyWhoami {
    SkillPolicyWhoami {
        principal,
        mode,
        can_read: has_policy_capability(&roles, "Reader"),
        can_write: has_policy_capability(&roles, "Writer"),
        can_admin: has_policy_capability(&roles, "Admin"),
        roles,
    }
}

pub(super) fn skill_policy_explain(
    principal: String,
    mode: String,
    roles: Vec<String>,
) -> SkillPolicyExplain {
    SkillPolicyExplain {
        principal,
        mode,
        can_read: has_policy_capability(&roles, "Reader"),
        can_write: has_policy_capability(&roles, "Writer"),
        can_admin: has_policy_capability(&roles, "Admin"),
        roles,
    }
}

pub(super) fn has_policy_capability(roles: &[String], required: &str) -> bool {
    roles.iter().any(|role| {
        role == "Admin"
            || (required == "Writer" && role == "Writer")
            || (required == "Reader" && (role == "Writer" || role == "Reader"))
    })
}
