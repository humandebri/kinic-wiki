// Where: crates/vfs_cli_app/src/skill_registry/manifest.rs
// What: Skill manifest frontmatter types and v1 parser.
// Why: Manifest schema stays in the skill domain, separate from VFS policy and command flow.
use anyhow::{Result, anyhow};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

use super::{SCHEMA_VERSION, SKILL_KIND, frontmatter};

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct SkillManifest {
    pub kind: String,
    pub schema_version: u32,
    pub id: String,
    pub version: String,
    pub publisher: String,
    pub entry: String,
    pub knowledge: Vec<String>,
    pub permissions: BTreeMap<String, bool>,
    pub provenance: BTreeMap<String, String>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(super) struct RawSkillManifest {
    pub(super) kind: String,
    pub(super) schema_version: u32,
    pub(super) id: String,
    pub(super) version: String,
    pub(super) publisher: String,
    pub(super) entry: String,
    #[serde(default)]
    pub(super) knowledge: Vec<String>,
    #[serde(default)]
    pub(super) permissions: BTreeMap<String, bool>,
    #[serde(default)]
    pub(super) provenance: BTreeMap<String, String>,
}

pub fn parse_skill_manifest(content: &str) -> Result<SkillManifest> {
    let frontmatter =
        frontmatter(content).ok_or_else(|| anyhow!("manifest frontmatter missing"))?;
    let raw: RawSkillManifest = serde_yaml::from_str(frontmatter)
        .map_err(|error| anyhow!("manifest YAML invalid: {error}"))?;
    if raw.kind != SKILL_KIND {
        return Err(anyhow!("manifest kind must be {SKILL_KIND}"));
    }
    if raw.schema_version.to_string() != SCHEMA_VERSION {
        return Err(anyhow!("manifest schema_version must be {SCHEMA_VERSION}"));
    }
    Ok(SkillManifest {
        kind: raw.kind,
        schema_version: 1,
        id: raw.id,
        version: raw.version,
        publisher: raw.publisher,
        entry: raw.entry,
        knowledge: raw.knowledge,
        permissions: raw.permissions,
        provenance: raw.provenance,
    })
}
