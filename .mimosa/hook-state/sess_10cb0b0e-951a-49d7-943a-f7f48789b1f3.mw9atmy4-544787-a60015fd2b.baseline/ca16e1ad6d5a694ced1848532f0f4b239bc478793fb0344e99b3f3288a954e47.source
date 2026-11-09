//! linop-sync:端到端加密的配置同步快照。
//!
//! 对应 navop 的加密跨设备同步(Navop Cloud Sync / Folder/Git/WebDAV):
//! 把 connections.toml / databases.toml / telnet.toml 打包为单一
//! AES-256-GCM 加密文件,密钥由 Argon2id(passphrase + 随机盐)派生,
//! 口令不落盘不出本机;产物可放到任何同步介质(Git 仓库/网盘文件夹/
//! WebDAV),由用户自行投递。
//!
//! 文件布局:`"LNSYNC1\n"` + version(u8) + salt(16B) + nonce(12B) + ct。

use std::path::Path;

use aes_gcm::aead::Aead;
use aes_gcm::{Aes256Gcm, KeyInit};
use argon2::Argon2;

#[derive(Debug, thiserror::Error)]
pub enum SyncError {
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("{0}")]
    BadFormat(&'static str),
    #[error("wrong passphrase")]
    WrongPassphrase,
    #[error("bundle is corrupted: {0}")]
    Corrupt(&'static str),
    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("profile error: {0}")]
    Profile(#[from] linop_core::CoreError),
}

pub type Result<T> = std::result::Result<T, SyncError>;

/// 文件魔数与格式版本。
const MAGIC: &[u8] = b"LNSYNC1\n";
const FORMAT_VERSION: u8 = 1;
const SALT_LEN: usize = 16;
const NONCE_LEN: usize = 12;

/// 冲突策略:应用快照时同名文件如何处理。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ApplyMode {
    /// 保留本地已有文件,只写入本地缺失的。
    SkipExisting,
    /// 全部覆盖为快照内容。
    Replace,
}

/// 同步载荷:各配置文件的原文(TOML)。None = 该文件不存在/未选。
#[derive(Debug, Clone, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct SyncPayload {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub connections: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub databases: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub telnet: Option<String>,
}

impl SyncPayload {
    /// 是否至少包含一个配置。
    pub fn is_empty(&self) -> bool {
        self.connections.is_none() && self.databases.is_none() && self.telnet.is_none()
    }
}

/// 从配置目录收集快照(缺的文件记 None)。
pub fn collect_from(config_dir: &Path) -> SyncPayload {
    let read = |name: &str| -> Option<String> {
        std::fs::read_to_string(config_dir.join(name))
            .ok()
            .filter(|s| !s.trim().is_empty())
    };
    SyncPayload {
        connections: read("connections.toml"),
        databases: read("databases.toml"),
        telnet: read("telnet.toml"),
    }
}

/// 把快照写回配置目录;返回实际写入的文件名列表。
pub fn apply_to(payload: &SyncPayload, config_dir: &Path, mode: ApplyMode) -> Result<Vec<String>> {
    std::fs::create_dir_all(config_dir)?;
    let mut written = Vec::new();
    let mut put = |name: &str, content: &Option<String>| -> Result<()> {
        let Some(content) = content else {
            return Ok(());
        };
        let path = config_dir.join(name);
        if mode == ApplyMode::SkipExisting && path.exists() {
            return Ok(());
        }
        std::fs::write(&path, content)?;
        written.push(name.to_string());
        Ok(())
    };
    put("connections.toml", &payload.connections)?;
    put("databases.toml", &payload.databases)?;
    put("telnet.toml", &payload.telnet)?;
    Ok(written)
}

/// 加密载荷为 bundle 字节。
pub fn encrypt(payload: &SyncPayload, passphrase: &str) -> Result<Vec<u8>> {
    let plaintext = serde_json::to_vec(payload)?;
    encrypt_bytes(&plaintext, passphrase)
}

/// 解密 bundle 并解析载荷。
pub fn decrypt(bundle: &[u8], passphrase: &str) -> Result<SyncPayload> {
    let plaintext = decrypt_bytes(bundle, passphrase)?;
    Ok(serde_json::from_slice(&plaintext)?)
}

/// 原始字节加密(magic + version + salt + nonce + ct)。
fn encrypt_bytes(plaintext: &[u8], passphrase: &str) -> Result<Vec<u8>> {
    let mut salt = [0u8; SALT_LEN];
    let mut nonce = [0u8; NONCE_LEN];
    getrandom::getrandom(&mut salt).map_err(|_| SyncError::Corrupt("rng failed"))?;
    getrandom::getrandom(&mut nonce).map_err(|_| SyncError::Corrupt("rng failed"))?;
    let key = derive_key(passphrase, &salt);
    let cipher = Aes256Gcm::new((&key).into());
    let ct = cipher
        .encrypt((&nonce).into(), plaintext)
        .map_err(|_| SyncError::Corrupt("encrypt failed"))?;

    let mut out = Vec::with_capacity(MAGIC.len() + 1 + SALT_LEN + NONCE_LEN + ct.len());
    out.extend_from_slice(MAGIC);
    out.push(FORMAT_VERSION);
    out.extend_from_slice(&salt);
    out.extend_from_slice(&nonce);
    out.extend_from_slice(&ct);
    Ok(out)
}

/// 原始字节解密(校验魔数/版本;认证失败 = 口令错误)。
fn decrypt_bytes(bundle: &[u8], passphrase: &str) -> Result<Vec<u8>> {
    let header = MAGIC.len() + 1 + SALT_LEN + NONCE_LEN;
    if bundle.len() < header {
        return Err(SyncError::BadFormat("too short"));
    }
    if &bundle[..MAGIC.len()] != MAGIC {
        return Err(SyncError::BadFormat("magic mismatch"));
    }
    if bundle[MAGIC.len()] != FORMAT_VERSION {
        return Err(SyncError::BadFormat("unsupported version"));
    }
    let mut salt = [0u8; SALT_LEN];
    salt.copy_from_slice(&bundle[MAGIC.len() + 1..MAGIC.len() + 1 + SALT_LEN]);
    let mut nonce = [0u8; NONCE_LEN];
    nonce.copy_from_slice(&bundle[MAGIC.len() + 1 + SALT_LEN..header]);
    let key = derive_key(passphrase, &salt);
    let cipher = Aes256Gcm::new((&key).into());
    cipher
        .decrypt((&nonce).into(), &bundle[header..])
        .map_err(|_| SyncError::WrongPassphrase)
}

/// Argon2id 派生 32B 会话密钥(默认参数:m=19MiB, t=2, p=1)。
fn derive_key(passphrase: &str, salt: &[u8]) -> [u8; 32] {
    let mut key = [0u8; 32];
    Argon2::default()
        .hash_password_into(passphrase.as_bytes(), salt, &mut key)
        .expect("argon2 derive into fixed-size buffer cannot fail");
    key
}

#[cfg(test)]
mod tests {
    use super::*;

    fn payload() -> SyncPayload {
        SyncPayload {
            connections: Some(
                "[web-1]\nhost = \"10.0.0.1\"\nusername = \"root\"\npassword = \"s3cret\"\n".into(),
            ),
            databases: Some(
                "[cache]\ndriver = \"redis\"\nhost = \"10.0.0.2\"\nport = 6379\n".into(),
            ),
            telnet: Some("[box]\nhost = \"10.0.0.3\"\nport = 23\n".into()),
        }
    }

    #[test]
    fn encrypt_decrypt_roundtrip() {
        let bundle = encrypt(&payload(), "correct horse").unwrap();
        // 魔数可见、密文长度合理。
        assert!(bundle.starts_with(MAGIC));
        assert!(bundle.len() > 64);
        let decoded = decrypt(&bundle, "correct horse").unwrap();
        assert_eq!(decoded, payload());
    }

    #[test]
    fn wrong_passphrase_is_rejected() {
        let bundle = encrypt(&payload(), "right").unwrap();
        assert!(matches!(
            decrypt(&bundle, "wrong"),
            Err(SyncError::WrongPassphrase)
        ));
    }

    #[test]
    fn tampered_ciphertext_is_rejected() {
        let mut bundle = encrypt(&payload(), "pw").unwrap();
        let last = bundle.len() - 1;
        bundle[last] ^= 0x01;
        let err = decrypt(&bundle, "pw").unwrap_err();
        assert!(
            matches!(err, SyncError::WrongPassphrase | SyncError::Corrupt(_)),
            "{err:?}"
        );
    }

    #[test]
    fn not_a_bundle_is_bad_format() {
        assert!(matches!(
            decrypt(b"hello world this is not a bundle", "pw"),
            Err(SyncError::BadFormat(_))
        ));
        // 截断。
        let bundle = encrypt(&payload(), "pw").unwrap();
        assert!(matches!(
            decrypt(&bundle[..20], "pw"),
            Err(SyncError::BadFormat(_))
        ));
    }

    #[test]
    fn same_passphrase_same_key_but_unique_nonce() {
        let a = encrypt(&payload(), "pw").unwrap();
        let b = encrypt(&payload(), "pw").unwrap();
        // 相同口令+载荷:盐与 nonce 随机 ⇒ 密文不同(防重放/指纹)。
        assert_ne!(a, b);
        // 都能解开。
        assert_eq!(decrypt(&a, "pw").unwrap(), payload());
        assert_eq!(decrypt(&b, "pw").unwrap(), payload());
    }

    #[test]
    fn collect_and_apply_roundtrip_with_modes() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("connections.toml"),
            "[local]\nhost=\"h\"\nusername=\"u\"\npassword=\"p\"\n",
        )
        .unwrap();
        let collected = collect_from(dir.path());
        assert!(collected.connections.is_some());
        assert!(collected.databases.is_none());
        assert!(collected.telnet.is_none());

        // 应用到新目录:全部写入。
        let target = tempfile::tempdir().unwrap();
        let bundle_payload = payload();
        let written = apply_to(&bundle_payload, target.path(), ApplyMode::Replace).unwrap();
        assert_eq!(
            written,
            vec!["connections.toml", "databases.toml", "telnet.toml"]
        );
        let again = collect_from(target.path());
        assert_eq!(again, bundle_payload);

        // SkipExisting:本地 connections 保留。
        let local = collect_from(dir.path());
        let written = apply_to(&bundle_payload, dir.path(), ApplyMode::SkipExisting).unwrap();
        assert_eq!(written, vec!["databases.toml", "telnet.toml"]);
        assert_eq!(collect_from(dir.path()).connections, local.connections);
    }
}
