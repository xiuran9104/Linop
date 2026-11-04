//! linop-ssh: 基于 russh 的异步 SSH 客户端。
//!
//! 对应 navop 的 `ssh` / `terminal` / `port_forwarding` crate 的职责:
//! 连接与认证、交互式 shell、SFTP 通道、本地端口转发。

mod client;
mod forward;
mod host_keys;
mod shell;
mod socks;

pub use client::{ClientHandler, Session};
pub use forward::{LocalForward, RemoteForward};
pub use host_keys::{Decision, KnownHosts, KnownHostsPolicy};
pub use shell::{ShellChannel, ShellReader, ShellWriter};
pub use socks::DynamicForward;

use linop_core::Auth;

#[derive(Debug, thiserror::Error)]
pub enum SshError {
    #[error("ssh transport error: {0}")]
    Transport(#[from] russh::Error),
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("authentication failed for {user}@{host}")]
    AuthFailed { user: String, host: String },
    #[error("host key mismatch for {host}: known {known}, got {got}")]
    HostKeyMismatch {
        host: String,
        known: String,
        got: String,
    },
    #[error("invalid key file {path}: {source}")]
    KeyFile {
        path: std::path::PathBuf,
        source: russh::keys::Error,
    },
    #[error("port forward listen failed: {0}")]
    Listen(std::io::Error),
    #[error("core error: {0}")]
    Core(#[from] linop_core::CoreError),
}

pub type Result<T> = std::result::Result<T, SshError>;

/// 默认终端类型(xterm-256color,与 navop 终端一致)。
pub const TERM_TYPE: &str = "xterm-256color";

/// 私钥是否需要口令:密钥认证、未配置口令、且文件实际为加密格式。
/// 供前端在连接前弹口令输入框。
pub fn key_needs_passphrase(auth: &Auth) -> bool {
    let Auth::Key { path, passphrase } = auth else {
        return false;
    };
    passphrase.is_none()
        && matches!(
            russh::keys::load_secret_key(path, None),
            Err(russh::keys::Error::KeyIsEncrypted)
        )
}

/// 加载私钥文件并构造认证用密钥。
pub(crate) fn load_key(auth: &Auth) -> Result<russh::keys::PrivateKeyWithHashAlg> {
    let Auth::Key { path, passphrase } = auth else {
        unreachable!("load_key only handles Auth::Key");
    };
    let key = russh::keys::load_secret_key(path, passphrase.as_deref()).map_err(|source| {
        SshError::KeyFile {
            path: path.clone(),
            source,
        }
    })?;
    Ok(russh::keys::PrivateKeyWithHashAlg::new(
        std::sync::Arc::new(key),
        Some(russh::keys::HashAlg::Sha256),
    ))
}
