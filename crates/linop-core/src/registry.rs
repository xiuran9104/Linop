//! 连接注册表:从 TOML 文件加载/保存全部连接 profile。

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use crate::model::{Connection, RawConnection};
use crate::{CoreError, Result};

/// Linop 配置目录(默认 `~/.config/linop`,可用 `LINOP_CONFIG_DIR` 覆盖,
/// 便于测试与 navop 式的多环境隔离)。
pub fn config_dir() -> PathBuf {
    if let Ok(dir) = std::env::var("LINOP_CONFIG_DIR") {
        return PathBuf::from(dir);
    }
    dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from(".config"))
        .join("linop")
}

pub fn default_connections_path() -> PathBuf {
    config_dir().join("connections.toml")
}

/// TOFU(trust-on-first-use)主机指纹库,由 linop-ssh 维护。
pub fn known_hosts_path() -> PathBuf {
    config_dir().join("known_hosts")
}

/// 会话录制(.cast,asciinema v2)目录。
pub fn recordings_dir() -> PathBuf {
    config_dir().join("recordings")
}

/// 录制目录里最新的 .cast 文件(回放默认目标);目录为空返回 None。
pub fn newest_recording() -> Option<PathBuf> {
    let dir = recordings_dir();
    let mut best: Option<(std::time::SystemTime, PathBuf)> = None;
    for entry in std::fs::read_dir(&dir).ok()?.flatten() {
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("cast") {
            continue;
        }
        let Ok(modified) = entry.metadata().and_then(|m| m.modified()) else {
            continue;
        };
        if best.as_ref().is_none_or(|(t, _)| modified > *t) {
            best = Some((modified, path));
        }
    }
    best.map(|(_, path)| path)
}

/// 连接注册表,保持按名称排序。
#[derive(Debug, Clone, Default)]
pub struct Registry {
    connections: Vec<Connection>,
}

impl Registry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn connections(&self) -> &[Connection] {
        &self.connections
    }

    pub fn get(&self, name: &str) -> Option<&Connection> {
        self.connections.iter().find(|c| c.name == name)
    }

    /// 插入或更新同名连接。
    pub fn upsert(&mut self, conn: Connection) {
        match self.connections.iter_mut().find(|c| c.name == conn.name) {
            Some(slot) => *slot = conn,
            None => {
                self.connections.push(conn);
                self.connections.sort_by(|a, b| a.name.cmp(&b.name));
            }
        }
    }

    pub fn remove(&mut self, name: &str) -> Result<Connection> {
        let idx = self
            .connections
            .iter()
            .position(|c| c.name == name)
            .ok_or_else(|| CoreError::NotFound(name.to_string()))?;
        Ok(self.connections.remove(idx))
    }

    /// 从 TOML 文件加载;文件不存在时返回空注册表。
    pub fn load(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref();
        if !path.exists() {
            return Ok(Self::default());
        }
        let text = std::fs::read_to_string(path)?;
        let raw: BTreeMap<String, RawConnection> =
            toml::from_str(&text).map_err(|source| CoreError::Parse {
                path: path.to_path_buf(),
                source,
            })?;
        let connections = raw
            .iter()
            .map(|(name, r)| {
                let mut conn = Connection::from(r);
                conn.name = name.clone();
                conn
            })
            .collect();
        Ok(Self { connections })
    }

    /// 保存到 TOML 文件(原子写:先写临时文件再 rename)。
    pub fn save(&self, path: impl AsRef<Path>) -> Result<()> {
        let path = path.as_ref();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let raw: BTreeMap<String, RawConnection> = self
            .connections
            .iter()
            .map(|c| (c.name.clone(), c.to_raw()))
            .collect();
        let text = toml::to_string_pretty(&raw)?;
        let tmp = path.with_extension("toml.tmp");
        std::fs::write(&tmp, text)?;
        std::fs::rename(&tmp, path)?;
        Ok(())
    }

    /// 加载默认路径(`~/.config/linop/connections.toml`)。
    pub fn load_default() -> Result<Self> {
        Self::load(default_connections_path())
    }
}

impl<'a> IntoIterator for &'a Registry {
    type Item = &'a Connection;
    type IntoIter = std::slice::Iter<'a, Connection>;

    fn into_iter(self) -> Self::IntoIter {
        self.connections.iter()
    }
}

impl FromIterator<Connection> for Registry {
    fn from_iter<I: IntoIterator<Item = Connection>>(iter: I) -> Self {
        let mut reg = Self::default();
        for conn in iter {
            reg.upsert(conn);
        }
        reg
    }
}

/// 由连接 profile 构造 `user@host:port` 展示串。
pub fn endpoint(conn: &Connection) -> String {
    format!("{}@{}:{}", conn.username, conn.host, conn.port)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::Auth;

    fn sample() -> Vec<Connection> {
        vec![
            {
                let mut c =
                    Connection::new("web-1", "192.168.1.10", "root", Auth::password("secret"));
                c.port = 2222;
                c.forwards
                    .push(crate::model::PortForward::new(8080, "127.0.0.1", 80));
                c
            },
            Connection::new(
                "db-1",
                "db.internal",
                "admin",
                Auth::key("/home/me/.ssh/id_ed25519"),
            ),
        ]
    }

    #[test]
    fn roundtrip_toml() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("connections.toml");
        let reg: Registry = sample().into_iter().collect();
        reg.save(&path).unwrap();

        let loaded = Registry::load(&path).unwrap();
        assert_eq!(loaded.connections(), reg.connections());
    }

    #[test]
    fn load_missing_file_is_empty() {
        let reg = Registry::load("/nonexistent/linop/connections.toml").unwrap();
        assert!(reg.connections().is_empty());
    }

    #[test]
    fn key_auth_survives_roundtrip() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("c.toml");
        let reg: Registry = vec![Connection::new(
            "k",
            "h",
            "u",
            Auth::key_with_passphrase("/path/key", "pp"),
        )]
        .into_iter()
        .collect();
        reg.save(&path).unwrap();
        let loaded = Registry::load(&path).unwrap();
        assert_eq!(
            loaded.get("k").unwrap().auth,
            Auth::key_with_passphrase("/path/key", "pp")
        );
    }

    #[test]
    fn upsert_and_remove() {
        let mut reg = Registry::new();
        reg.upsert(sample().remove(0));
        assert_eq!(reg.connections().len(), 1);
        reg.upsert(Connection::new(
            "web-1",
            "new-host",
            "root",
            Auth::password("x"),
        ));
        assert_eq!(reg.connections().len(), 1);
        assert_eq!(reg.get("web-1").unwrap().host, "new-host");
        assert!(reg.remove("web-1").is_ok());
        assert!(reg.remove("web-1").is_err());
    }

    #[test]
    fn config_dir_env_override() {
        // 注意:env 变量全局生效,串行运行下安全;单独验证一次后还原。
        let dir = tempfile::tempdir().unwrap();
        let key = "LINOP_CONFIG_DIR";
        let old = std::env::var(key).ok();
        unsafe { std::env::set_var(key, dir.path()) };
        assert_eq!(config_dir(), dir.path());
        assert!(default_connections_path().starts_with(dir.path()));
        unsafe {
            match old {
                Some(v) => std::env::set_var(key, v),
                None => std::env::remove_var(key),
            }
        }
    }

    #[test]
    fn endpoint_display() {
        let reg: Registry = sample().into_iter().collect();
        assert_eq!(
            endpoint(reg.get("web-1").unwrap()),
            "root@192.168.1.10:2222"
        );
    }
}
