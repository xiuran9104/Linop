//! 连接 profile 数据模型。

use std::path::PathBuf;

/// 认证方式。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Auth {
    /// 密码认证。密码为空时由前端在连接前向用户询问。
    Password { password: String },
    /// 私钥文件认证(ed25519/rsa/ecdsa 等 OpenSSH 格式)。
    Key {
        path: PathBuf,
        passphrase: Option<String>,
    },
}

impl Auth {
    pub fn password(password: impl Into<String>) -> Self {
        Auth::Password {
            password: password.into(),
        }
    }

    pub fn key(path: impl Into<PathBuf>) -> Self {
        Auth::Key {
            path: path.into(),
            passphrase: None,
        }
    }

    pub fn key_with_passphrase(path: impl Into<PathBuf>, passphrase: impl Into<String>) -> Self {
        Auth::Key {
            path: path.into(),
            passphrase: Some(passphrase.into()),
        }
    }
}

/// 一条本地端口转发规则:本地 `local_port` 上的连接经 SSH 隧道
/// 转发到远端机器视角下的 `remote_host:remote_port`。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PortForward {
    pub local_port: u16,
    pub remote_host: String,
    pub remote_port: u16,
}

impl PortForward {
    pub fn new(local_port: u16, remote_host: impl Into<String>, remote_port: u16) -> Self {
        PortForward {
            local_port,
            remote_host: remote_host.into(),
            remote_port,
        }
    }
}

/// 一条 SSH 连接 profile。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Connection {
    /// 唯一名称,同时作为 TOML 表名。
    pub name: String,
    pub host: String,
    pub port: u16,
    pub username: String,
    pub auth: Auth,
    /// 建立连接后自动/手动启动的本地端口转发规则。
    pub forwards: Vec<PortForward>,
    pub notes: Option<String>,
}

impl Connection {
    pub fn new(
        name: impl Into<String>,
        host: impl Into<String>,
        username: impl Into<String>,
        auth: Auth,
    ) -> Self {
        Connection {
            name: name.into(),
            host: host.into(),
            port: 22,
            username: username.into(),
            auth,
            forwards: Vec::new(),
            notes: None,
        }
    }

    /// TOML 序列化用中间结构。
    pub(crate) fn to_raw(&self) -> RawConnection {
        RawConnection {
            host: self.host.clone(),
            port: Some(self.port),
            username: self.username.clone(),
            password: match &self.auth {
                Auth::Password { password } if !password.is_empty() => Some(password.clone()),
                _ => None,
            },
            key_path: match &self.auth {
                Auth::Key { path, .. } => Some(path.display().to_string()),
                _ => None,
            },
            key_passphrase: match &self.auth {
                Auth::Key {
                    passphrase: Some(p),
                    ..
                } => Some(p.clone()),
                _ => None,
            },
            forwards: self
                .forwards
                .iter()
                .map(|f| RawForward {
                    local_port: f.local_port,
                    remote_host: f.remote_host.clone(),
                    remote_port: f.remote_port,
                })
                .collect(),
            notes: self.notes.clone(),
        }
    }
}

/// 密钥认证优先于密码:两者都配置时使用密钥。
impl From<&RawConnection> for Connection {
    fn from(raw: &RawConnection) -> Self {
        let auth = if let Some(path) = &raw.key_path {
            Auth::Key {
                path: PathBuf::from(path),
                passphrase: raw.key_passphrase.clone(),
            }
        } else {
            Auth::Password {
                password: raw.password.clone().unwrap_or_default(),
            }
        };
        Connection {
            name: String::new(), // 由 Registry 按 TOML 表名补齐
            host: raw.host.clone(),
            port: raw.port.unwrap_or(22),
            username: raw.username.clone(),
            auth,
            forwards: raw
                .forwards
                .iter()
                .map(|f| PortForward {
                    local_port: f.local_port,
                    remote_host: f.remote_host.clone(),
                    remote_port: f.remote_port,
                })
                .collect(),
            notes: raw.notes.clone(),
        }
    }
}

/// TOML 中的连接表结构。
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub(crate) struct RawForward {
    pub local_port: u16,
    pub remote_host: String,
    pub remote_port: u16,
}

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub(crate) struct RawConnection {
    pub host: String,
    pub port: Option<u16>,
    pub username: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub password: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub key_path: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub key_passphrase: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub forwards: Vec<RawForward>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub notes: Option<String>,
}
