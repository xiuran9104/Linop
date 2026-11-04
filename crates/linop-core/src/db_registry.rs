//! 数据库 profile 模型与注册表(`databases.toml`)。

use std::collections::BTreeMap;
use std::path::PathBuf;

use crate::{CoreError, Result};

/// 一个数据库连接 profile。
/// - `sqlite`:本地文件(`path`);
/// - `redis`:TCP 服务(`host:port`,可选 `password`);
///
/// 驱动字段为后续 MySQL/PostgreSQL 等保留(navop 用扩展驱动支持更多数据库)。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DbProfile {
    pub name: String,
    pub driver: DbDriver,
    /// SQLite:数据库文件路径。
    pub path: PathBuf,
    /// Redis:主机与端口(缺省 6379)。
    pub host: Option<String>,
    pub port: Option<u16>,
    /// Redis:AUTH 口令(可选)。
    pub password: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DbDriver {
    Sqlite,
    Redis,
}

impl DbDriver {
    pub fn as_str(&self) -> &'static str {
        match self {
            DbDriver::Sqlite => "sqlite",
            DbDriver::Redis => "redis",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        match value.trim().to_ascii_lowercase().as_str() {
            "sqlite" => Some(DbDriver::Sqlite),
            "redis" => Some(DbDriver::Redis),
            _ => None,
        }
    }
}

impl DbProfile {
    pub fn sqlite(name: impl Into<String>, path: impl Into<PathBuf>) -> Self {
        DbProfile {
            name: name.into(),
            driver: DbDriver::Sqlite,
            path: path.into(),
            host: None,
            port: None,
            password: None,
        }
    }

    pub fn redis(
        name: impl Into<String>,
        host: impl Into<String>,
        port: u16,
        password: Option<String>,
    ) -> Self {
        DbProfile {
            name: name.into(),
            driver: DbDriver::Redis,
            path: PathBuf::new(),
            host: Some(host.into()),
            port: Some(port),
            password,
        }
    }

    /// Redis 端点 `host:port`(端口缺省 6379)。
    pub fn redis_endpoint(&self) -> String {
        format!(
            "{}:{}",
            self.host.clone().unwrap_or_default(),
            self.port.unwrap_or(6379)
        )
    }
}

/// 数据库注册表,按名称排序。
#[derive(Debug, Clone, Default)]
pub struct DbRegistry {
    profiles: Vec<DbProfile>,
}

/// databases.toml 中的条目结构。
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct RawDb {
    driver: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    path: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    host: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    port: Option<u16>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    password: Option<String>,
}

/// 默认数据库配置路径:`~/.config/linop/databases.toml`。
pub fn default_databases_path() -> PathBuf {
    crate::config_dir().join("databases.toml")
}

impl FromIterator<DbProfile> for DbRegistry {
    fn from_iter<I: IntoIterator<Item = DbProfile>>(iter: I) -> Self {
        let mut reg = Self::default();
        for profile in iter {
            reg.upsert(profile);
        }
        reg
    }
}

impl DbRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn profiles(&self) -> &[DbProfile] {
        &self.profiles
    }

    pub fn get(&self, name: &str) -> Option<&DbProfile> {
        self.profiles.iter().find(|p| p.name == name)
    }

    pub fn upsert(&mut self, profile: DbProfile) {
        match self.profiles.iter_mut().find(|p| p.name == profile.name) {
            Some(slot) => *slot = profile,
            None => {
                self.profiles.push(profile);
                self.profiles.sort_by(|a, b| a.name.cmp(&b.name));
            }
        }
    }

    pub fn remove(&mut self, name: &str) -> Result<DbProfile> {
        let idx = self
            .profiles
            .iter()
            .position(|p| p.name == name)
            .ok_or_else(|| CoreError::NotFound(name.to_string()))?;
        Ok(self.profiles.remove(idx))
    }

    /// 加载;文件不存在返回空表,驱动名非法的条目报错。
    pub fn load(path: impl AsRef<std::path::Path>) -> Result<Self> {
        let path = path.as_ref();
        if !path.exists() {
            return Ok(Self::default());
        }
        let text = std::fs::read_to_string(path)?;
        let raw: BTreeMap<String, RawDb> =
            toml::from_str(&text).map_err(|source| CoreError::Parse {
                path: path.to_path_buf(),
                source,
            })?;
        let mut profiles = Vec::new();
        for (name, entry) in raw {
            let driver = DbDriver::parse(&entry.driver).ok_or_else(|| {
                CoreError::NotFound(format!("数据库 {name} 的驱动不支持:{}", entry.driver))
            })?;
            profiles.push(DbProfile {
                name,
                driver,
                path: PathBuf::from(entry.path.unwrap_or_default()),
                host: entry.host,
                port: entry.port,
                password: entry.password,
            });
        }
        profiles.sort_by(|a, b| a.name.cmp(&b.name));
        Ok(Self { profiles })
    }

    pub fn save(&self, path: impl AsRef<std::path::Path>) -> Result<()> {
        let path = path.as_ref();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let raw: BTreeMap<String, RawDb> = self
            .profiles
            .iter()
            .map(|p| {
                (
                    p.name.clone(),
                    RawDb {
                        driver: p.driver.as_str().to_string(),
                        path: if p.path.as_os_str().is_empty() {
                            None
                        } else {
                            Some(p.path.display().to_string())
                        },
                        host: p.host.clone(),
                        port: p.port,
                        password: p.password.clone(),
                    },
                )
            })
            .collect();
        let text = toml::to_string_pretty(&raw)?;
        let tmp = path.with_extension("toml.tmp");
        std::fs::write(&tmp, text)?;
        std::fs::rename(&tmp, path)?;
        Ok(())
    }

    pub fn load_default() -> Result<Self> {
        Self::load(default_databases_path())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip_databases_toml() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("databases.toml");
        let mut reg = DbRegistry::new();
        reg.upsert(DbProfile::sqlite("local", "/tmp/app.db"));
        reg.upsert(DbProfile::sqlite("zeta", "/home/me/z.db"));
        reg.save(&path).unwrap();

        let loaded = DbRegistry::load(&path).unwrap();
        assert_eq!(loaded.profiles(), reg.profiles());
        // 按名称排序。
        assert_eq!(loaded.profiles()[0].name, "local");
        assert_eq!(loaded.profiles()[1].name, "zeta");
    }

    #[test]
    fn missing_file_and_bad_driver() {
        assert!(
            DbRegistry::load("/nonexistent/databases.toml")
                .unwrap()
                .profiles()
                .is_empty()
        );

        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("d.toml");
        std::fs::write(&path, "[x]\ndriver = \"oracle\"\npath = \"/a\"").unwrap();
        assert!(DbRegistry::load(&path).is_err());
    }

    #[test]
    fn driver_parse() {
        assert_eq!(DbDriver::parse("SQLite"), Some(DbDriver::Sqlite));
        assert_eq!(DbDriver::parse("Redis"), Some(DbDriver::Redis));
        assert_eq!(DbDriver::parse("mysql"), None);
    }

    #[test]
    fn redis_profile_roundtrip() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("databases.toml");
        let mut reg = DbRegistry::new();
        reg.upsert(DbProfile::redis(
            "cache",
            "10.0.0.5",
            6380,
            Some("secret".into()),
        ));
        reg.save(&path).unwrap();

        let loaded = DbRegistry::load(&path).unwrap();
        assert_eq!(loaded.profiles(), reg.profiles());
        let cache = loaded.get("cache").unwrap();
        assert_eq!(cache.redis_endpoint(), "10.0.0.5:6380");
    }
}
