//! linop-core: 连接配置模型与注册表。
//!
//! 借鉴 navop 的 `connection_form` / `core` crate:连接以 profile 形式
//! 持久化在 TOML 文件中,供 SSH/SFTP/端口转发等上层模块共用。

mod model;
mod registry;
pub mod ssh_config;
mod telnet_registry;

pub use model::{Auth, Connection, PortForward, RemoteForward};
pub use registry::{
    Registry, config_dir, default_connections_path, endpoint, known_hosts_path, newest_recording,
    recordings_dir,
};
pub use telnet_registry::{TelnetProfile, TelnetRegistry, default_telnet_path};

use std::fmt;

#[derive(Debug, thiserror::Error)]
pub enum CoreError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("config parse error in {path}: {source}")]
    Parse {
        path: std::path::PathBuf,
        source: toml::de::Error,
    },
    #[error("config serialize error: {0}")]
    Serialize(#[from] toml::ser::Error),
    #[error("duplicate connection name: {0}")]
    Duplicate(String),
    #[error("connection not found: {0}")]
    NotFound(String),
}

pub type Result<T> = std::result::Result<T, CoreError>;

/// 连接的唯一标识(名称)。
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ConnId(pub String);

impl fmt::Display for ConnId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}
