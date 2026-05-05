// Where: crates/vfs_cli_app/src/skill_registry/command.rs
// What: Skill Registry CLI command dispatch.
// Why: Keep the top-level skill_registry module as a facade over domain submodules.
use anyhow::{Result, anyhow};
use vfs_client::VfsApi;
use wiki_domain::{PUBLIC_SKILL_REGISTRY_ROOT, SKILL_REGISTRY_ROOT};

use crate::cli::{
    SkillCommand, SkillIndexCommand, SkillLocalCommand, SkillPublicCommand, SkillVersionsCommand,
};

use super::*;

pub async fn run_skill_command(client: &impl VfsApi, command: SkillCommand) -> Result<()> {
    match command {
        SkillCommand::Policy { command } => {
            run_skill_policy_command(client, command).await?;
        }
        SkillCommand::Index { command } => {
            run_skill_index_command(client, command).await?;
        }
        SkillCommand::Local { command } => {
            run_skill_local_command(client, command).await?;
        }
        SkillCommand::Public { command } => {
            run_skill_public_command(client, command).await?;
        }
        SkillCommand::Versions { command } => {
            run_skill_versions_command(client, command, SKILL_REGISTRY_ROOT, "skill").await?;
        }
        SkillCommand::Import {
            source,
            github,
            path,
            ref_name,
            id,
            json,
        } => {
            let inspect =
                import_skill_command(client, source, github, path, &ref_name, &id).await?;
            print_json_or_message(
                json,
                &inspect,
                &format!("skill imported: {} -> {}", id, inspect.base_path),
            )?;
        }
        SkillCommand::Update { id, ref_name, json } => {
            let inspect = update_github_skill(client, &id, &ref_name).await?;
            print_json_or_message(
                json,
                &inspect,
                &format!("skill updated: {} -> {}", id, inspect.base_path),
            )?;
        }
        SkillCommand::Inspect { id, json } => {
            let inspect = inspect_skill(client, &id).await?;
            print_json_or_message(json, &inspect, &format!("skill inspected: {id}"))?;
        }
        SkillCommand::List { prefix, json } => {
            let list = list_skills(client, &prefix).await?;
            print_json_or_message(json, &list, &format!("{} skills", list.len()))?;
        }
        SkillCommand::Audit { id, fail_on, json } => {
            let audit = audit_skill(client, &id).await?;
            let should_fail = fail_on.is_some_and(|level| audit_fails_on(&audit, level));
            print_json_or_message(
                json,
                &audit,
                if audit.ok {
                    "skill audit ok"
                } else {
                    "skill audit warnings"
                },
            )?;
            if should_fail {
                return Err(anyhow!("skill audit failed for {id}"));
            }
        }
        SkillCommand::Install {
            id,
            output,
            skills_dir,
            lockfile,
            json,
        } => {
            let output = install_output_path(&id, output, skills_dir)?;
            let result = install_skill(client, &id, &output, lockfile).await?;
            print_json_or_message(json, &result, &format!("skill installed: {id}"))?;
        }
    }
    Ok(())
}

async fn run_skill_index_command(client: &impl VfsApi, command: SkillIndexCommand) -> Result<()> {
    match command {
        SkillIndexCommand::List { index, json } => {
            let entries = load_skill_index(&index).await?;
            print_json_or_message(json, &entries, &format!("{} indexed skills", entries.len()))?;
        }
        SkillIndexCommand::Inspect { id, index, json } => {
            let entry = load_skill_index_entry(&index, &id).await?;
            let inspect = inspect_skill_at_root(client, &entry.id, entry.catalog.root()).await?;
            print_json_or_message(json, &inspect, &format!("skill inspected: {id}"))?;
        }
        SkillIndexCommand::Install {
            id,
            index,
            output,
            lockfile,
            json,
        } => {
            let entry = load_skill_index_entry(&index, &id).await?;
            let result = install_skill_from_index_entry(client, &entry, &output, lockfile).await?;
            print_json_or_message(json, &result, &format!("skill installed: {id}"))?;
        }
        SkillIndexCommand::InstallEnabled {
            index,
            skills_dir,
            lockfile,
            json,
        } => {
            let entries = load_skill_index(&index).await?;
            let result = install_enabled_skill_index(client, &entries, &skills_dir, lockfile).await;
            match result {
                Ok(result) => {
                    print_json_or_message(json, &result, "indexed skills installed")?;
                }
                Err((result, error)) => {
                    print_json_or_message(json, &result, "indexed skill install failed")?;
                    return Err(error);
                }
            }
        }
    }
    Ok(())
}

async fn run_skill_local_command(client: &impl VfsApi, command: SkillLocalCommand) -> Result<()> {
    match command {
        SkillLocalCommand::Audit { dir, json } => {
            let audit = audit_local_skill(&dir).await?;
            print_json_or_message(
                json,
                &audit,
                if audit.ok {
                    "local skill audit ok"
                } else {
                    "local skill audit warnings"
                },
            )?;
        }
        SkillLocalCommand::Diff { dir, json } => {
            let diff = diff_local_skill(client, &dir).await?;
            print_json_or_message(json, &diff, "local skill diff computed")?;
        }
        SkillLocalCommand::Install {
            dir,
            skills_dir,
            json,
        } => {
            let result = install_local_skill(&dir, &skills_dir).await?;
            print_json_or_message(
                json,
                &result,
                &format!("local skill installed: {}", result.id),
            )?;
        }
    }
    Ok(())
}

async fn run_skill_public_command(client: &impl VfsApi, command: SkillPublicCommand) -> Result<()> {
    match command {
        SkillPublicCommand::Promote { id, json } => {
            let inspect = promote_public_skill(client, &id).await?;
            print_json_or_message(
                json,
                &inspect,
                &format!("skill promoted to public: {} -> {}", id, inspect.base_path),
            )?;
        }
        SkillPublicCommand::List { prefix, json } => {
            let list = list_skills(client, &prefix).await?;
            print_json_or_message(json, &list, &format!("{} public skills", list.len()))?;
        }
        SkillPublicCommand::Inspect { id, json } => {
            let inspect = inspect_public_skill(client, &id).await?;
            print_json_or_message(json, &inspect, &format!("public skill inspected: {id}"))?;
        }
        SkillPublicCommand::Install {
            id,
            output,
            skills_dir,
            lockfile,
            json,
        } => {
            let output = install_output_path(&id, output, skills_dir)?;
            let result = install_public_skill(client, &id, &output, lockfile).await?;
            print_json_or_message(json, &result, &format!("public skill installed: {id}"))?;
        }
        SkillPublicCommand::Revoke { id, json } => {
            let value = revoke_public_skill(client, &id).await?;
            print_json_or_message(json, &value, &format!("public skill revoked: {id}"))?;
        }
        SkillPublicCommand::Versions { command } => {
            run_skill_versions_command(client, command, PUBLIC_SKILL_REGISTRY_ROOT, "public skill")
                .await?;
        }
    }
    Ok(())
}

async fn run_skill_versions_command(
    client: &impl VfsApi,
    command: SkillVersionsCommand,
    root: &str,
    label: &str,
) -> Result<()> {
    match command {
        SkillVersionsCommand::List { id, json } => {
            let versions = list_skill_versions(client, root, &id).await?;
            print_json_or_message(
                json,
                &versions,
                &format!("{} {} versions", versions.len(), label),
            )?;
        }
        SkillVersionsCommand::Inspect { id, version, json } => {
            let inspect = inspect_skill_version(client, root, &id, &version).await?;
            print_json_or_message(
                json,
                &inspect,
                &format!("{label} version inspected: {id} {version}"),
            )?;
        }
    }
    Ok(())
}
