// Where: crates/vfs_cli_app/src/identity.rs
// What: Load the active icp-cli identity for authenticated canister calls.
// Why: kinic-vfs-cli must sign with the same caller selected by `icp identity default`.
use anyhow::{Context, Result, anyhow, bail};
use candid::Principal;
use ic_agent::identity::{DelegatedIdentity, Identity, SignedDelegation};
use serde::Deserialize;
use serde_json::Value;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::process::Command;
use vfs_client::identity_from_pem;

const REFRESH_SKEW_NS: u64 = 5 * 60 * 1_000_000_000;

pub async fn load_default_icp_identity() -> Result<Box<dyn Identity>> {
    let identity_name = default_identity_name().await?;
    load_icp_identity(&identity_name).await
}

pub async fn default_identity_name() -> Result<String> {
    let identity_name = command_stdout("icp", &["identity", "default"])
        .await
        .context("failed to read active icp-cli identity")?;
    let identity_name = identity_name.trim();
    if identity_name.is_empty() {
        bail!("active icp-cli identity is empty");
    }
    Ok(identity_name.to_string())
}

async fn load_icp_identity(identity_name: &str) -> Result<Box<dyn Identity>> {
    match identity_kind(identity_name)? {
        Some(IdentityKind::InternetIdentity) => load_internet_identity(identity_name),
        Some(IdentityKind::PendingDelegation) => Err(refresh_required(identity_name, "pending")),
        Some(IdentityKind::Anonymous) => {
            bail!("selected icp identity `{identity_name}` is anonymous")
        }
        Some(IdentityKind::Other) | None => load_exported_identity(identity_name).await,
    }
}

async fn load_exported_identity(identity_name: &str) -> Result<Box<dyn Identity>> {
    let pem = command_stdout_bytes("icp", &["identity", "export", identity_name])
        .await
        .with_context(|| format!("failed to export icp-cli identity `{identity_name}`"))?;
    identity_from_pem(&pem)
}

fn load_internet_identity(identity_name: &str) -> Result<Box<dyn Identity>> {
    let dir = identity_dir()?;
    let metadata = identity_metadata(&dir, identity_name)?;
    let session_identity = load_session_identity(&dir, identity_name, &metadata)?;
    let chain = load_delegation_chain(&dir, identity_name)?;
    if chain.delegations.is_empty() {
        return Err(refresh_required(identity_name, "missing delegation"));
    }
    let now = now_ns()?;
    let signed_delegations = chain
        .delegations
        .into_iter()
        .map(|delegation| delegation.into_signed_delegation(identity_name, now))
        .collect::<Result<Vec<_>>>()?;
    let session_public_key = session_identity
        .public_key()
        .ok_or_else(|| anyhow!("icp identity `{identity_name}` session key has no public key"))?;
    let last_pubkey = &signed_delegations
        .last()
        .expect("checked non-empty above")
        .delegation
        .pubkey;
    if *last_pubkey != session_public_key {
        bail!("icp identity `{identity_name}` delegation does not target its session public key");
    }
    Ok(Box::new(DelegatedIdentity::new_unchecked(
        decode_hex(&chain.public_key).context("failed to decode II publicKey")?,
        session_identity,
        signed_delegations,
    )))
}

fn identity_kind(identity_name: &str) -> Result<Option<IdentityKind>> {
    let dir = identity_dir()?;
    let Some(value) = identity_list(&dir)? else {
        return Ok(None);
    };
    let kind = value
        .get("identities")
        .and_then(|identities| identities.get(identity_name))
        .and_then(|identity| identity.get("kind"))
        .and_then(Value::as_str)
        .map(IdentityKind::from_str)
        .unwrap_or(IdentityKind::Other);
    Ok(Some(kind))
}

fn identity_metadata(dir: &Path, identity_name: &str) -> Result<Value> {
    let value = identity_list(dir)?.ok_or_else(|| anyhow!("failed to load icp identity list"))?;
    value
        .get("identities")
        .and_then(|identities| identities.get(identity_name))
        .cloned()
        .ok_or_else(|| anyhow!("icp identity `{identity_name}` is not listed"))
}

fn identity_list(dir: &Path) -> Result<Option<Value>> {
    let path = dir.join("identity_list.json");
    if !path.is_file() {
        return Ok(None);
    }
    let bytes = std::fs::read(&path)
        .with_context(|| format!("failed to read icp identity list {}", path.display()))?;
    serde_json::from_slice(&bytes)
        .with_context(|| format!("failed to parse icp identity list {}", path.display()))
        .map(Some)
}

fn load_session_identity(
    dir: &Path,
    identity_name: &str,
    metadata: &Value,
) -> Result<Box<dyn Identity>> {
    let storage_kind = metadata
        .get("storage")
        .and_then(|storage| storage.get("kind"))
        .and_then(Value::as_str)
        .unwrap_or("plaintext");
    match storage_kind {
        "plaintext" => {
            let path = dir.join("keys").join(format!("{identity_name}.pem"));
            let pem = std::fs::read(&path)
                .with_context(|| format!("failed to read II session key {}", path.display()))?;
            identity_from_pem(&pem)
        }
        "keyring" => bail!(
            "icp identity `{identity_name}` stores its II session key in keyring; re-link with `icp identity link ii {identity_name} --storage plaintext` or use a PEM identity"
        ),
        "password" => bail!(
            "icp identity `{identity_name}` stores its II session key encrypted; password storage is not supported by kinic-vfs-cli yet"
        ),
        other => {
            bail!("unsupported II session key storage `{other}` for icp identity `{identity_name}`")
        }
    }
}

fn load_delegation_chain(dir: &Path, identity_name: &str) -> Result<DelegationChainJson> {
    let path = dir
        .join("delegations")
        .join(format!("{identity_name}.json"));
    if !path.is_file() {
        return Err(refresh_required(identity_name, "missing delegation"));
    }
    let bytes = std::fs::read(&path)
        .with_context(|| format!("failed to read II delegation {}", path.display()))?;
    serde_json::from_slice(&bytes)
        .with_context(|| format!("failed to parse II delegation {}", path.display()))
}

fn identity_dir() -> Result<PathBuf> {
    if let Ok(path) = std::env::var("ICP_CLI_IDENTITY_DIR") {
        return Ok(PathBuf::from(path));
    }
    let home = std::env::var_os("HOME").ok_or_else(|| anyhow!("HOME is not set"))?;
    let home = PathBuf::from(home);
    #[cfg(target_os = "macos")]
    {
        Ok(home
            .join("Library")
            .join("Application Support")
            .join("org.dfinity.icp-cli")
            .join("identity"))
    }
    #[cfg(not(target_os = "macos"))]
    {
        Ok(home
            .join(".local")
            .join("share")
            .join("org.dfinity.icp-cli")
            .join("identity"))
    }
}

fn refresh_required(identity_name: &str, reason: &str) -> anyhow::Error {
    anyhow!(
        "icp identity `{identity_name}` delegation is {reason}; run `icp identity login {identity_name}`"
    )
}

fn now_ns() -> Result<u64> {
    let duration = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .context("system clock is before Unix epoch")?;
    u64::try_from(duration.as_nanos()).context("current time does not fit in u64 nanoseconds")
}

fn decode_hex(value: &str) -> Result<Vec<u8>> {
    hex::decode(value).with_context(|| "invalid hex string")
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum IdentityKind {
    InternetIdentity,
    PendingDelegation,
    Anonymous,
    Other,
}

impl IdentityKind {
    fn from_str(value: &str) -> Self {
        match value {
            "internet-identity" => Self::InternetIdentity,
            "pending-delegation" => Self::PendingDelegation,
            "anonymous" => Self::Anonymous,
            _ => Self::Other,
        }
    }
}

#[derive(Debug, Deserialize)]
struct DelegationChainJson {
    #[serde(rename = "publicKey")]
    public_key: String,
    delegations: Vec<SignedDelegationJson>,
}

#[derive(Debug, Deserialize)]
struct SignedDelegationJson {
    signature: String,
    delegation: DelegationJson,
}

impl SignedDelegationJson {
    fn into_signed_delegation(self, identity_name: &str, now: u64) -> Result<SignedDelegation> {
        let expiration = parse_expiration(&self.delegation.expiration)
            .with_context(|| format!("invalid II delegation expiration for `{identity_name}`"))?;
        if expiration <= now.saturating_add(REFRESH_SKEW_NS) {
            return Err(refresh_required(identity_name, "expired"));
        }
        let targets = self
            .delegation
            .targets
            .unwrap_or_default()
            .into_iter()
            .map(|target| Principal::from_text(&target).context("invalid delegation target"))
            .collect::<Result<Vec<_>>>()?;
        Ok(SignedDelegation {
            signature: decode_hex(&self.signature).context("failed to decode II signature")?,
            delegation: ic_agent::identity::Delegation {
                pubkey: decode_hex(&self.delegation.pubkey)
                    .context("failed to decode II delegation pubkey")?,
                expiration,
                targets: (!targets.is_empty()).then_some(targets),
            },
        })
    }
}

#[derive(Debug, Deserialize)]
struct DelegationJson {
    pubkey: String,
    expiration: ExpirationJson,
    targets: Option<Vec<String>>,
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum ExpirationJson {
    String(String),
    Number(u64),
}

fn parse_expiration(expiration: &ExpirationJson) -> Result<u64> {
    match expiration {
        ExpirationJson::Number(value) => Ok(*value),
        ExpirationJson::String(value) => u64::from_str_radix(value, 16)
            .or_else(|_| value.parse())
            .with_context(|| format!("invalid expiration `{value}`")),
    }
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

#[cfg(test)]
mod tests {
    use super::*;

    const SECP256K1_PEM: &str = "-----BEGIN EC PARAMETERS-----
BgUrgQQACg==
-----END EC PARAMETERS-----
-----BEGIN EC PRIVATE KEY-----
MHQCAQEEIAgy7nZEcVHkQ4Z1Kdqby8SwyAiyKDQmtbEHTIM+WNeBoAcGBSuBBAAK
oUQDQgAEgO87rJ1ozzdMvJyZQ+GABDqUxGLvgnAnTlcInV3NuhuPv4O3VGzMGzeB
N3d26cRxD99TPtm8uo2OuzKhSiq6EQ==
-----END EC PRIVATE KEY-----
";

    #[test]
    fn parses_decimal_and_hex_expiration() {
        assert_eq!(
            parse_expiration(&ExpirationJson::String("10".to_string())).unwrap(),
            16
        );
        assert_eq!(parse_expiration(&ExpirationJson::Number(42)).unwrap(), 42);
    }

    #[test]
    fn unchecked_delegated_identity_accepts_certificate_signature_shape() {
        let session = identity_from_pem(SECP256K1_PEM.as_bytes()).unwrap();
        let session_public_key = hex::encode(session.public_key().unwrap());
        let json = format!(
            r#"{{
              "publicKey": "303c300c060a2b0601040183b8430102032c000a000000000000000701010001",
              "delegations": [{{
                "signature": "d9d9f7a26b6365727469666963617465",
                "delegation": {{
                  "pubkey": "{session_public_key}",
                  "expiration": "ffffffffffffffff",
                  "targets": null
                }}
              }}]
            }}"#
        );
        let chain: DelegationChainJson = serde_json::from_str(&json).unwrap();
        let signed = chain.delegations.into_iter().next().unwrap();
        let signed = signed.into_signed_delegation("ii", 0).unwrap();
        let identity = DelegatedIdentity::new_unchecked(
            decode_hex(&chain.public_key).unwrap(),
            session,
            vec![signed],
        );
        assert!(identity.sender().is_ok());
    }

    #[test]
    fn expired_delegation_mentions_icp_login() {
        let json = r#"{
          "signature": "00",
          "delegation": {
            "pubkey": "00",
            "expiration": "01",
            "targets": null
          }
        }"#;
        let signed: SignedDelegationJson = serde_json::from_str(json).unwrap();
        let error = signed
            .into_signed_delegation("kinic-ii", u64::MAX - REFRESH_SKEW_NS)
            .unwrap_err()
            .to_string();
        assert!(error.contains("icp identity login kinic-ii"));
    }
}
