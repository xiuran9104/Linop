//! Telnet profile 注册表(`telnet.toml`)。

use std::collections::BTreeMap;
use std::path::PathBuf;

use crate::{CoreError, Result};

/// 一条 Telnet 目标(NVT,无认证)。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TelnetProfile {
    pub name: String,
    pub host: String,
    pub port: u16,
}

impl TelnetProfile {
    pub fn new(name: impl Into<String>, host: impl Into<String>, port: u16) -> Self {
        TelnetProfile {
            name: name.into(),
            host: host.into(),
            port,
        }
    }
}

/// telnet.toml 中的条目结构。
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct RawTelnet {
    host: String,
    #[serde(default = "default_port")]
    port: u16,
}

fn default_port() -> u16 {
    23
}

/// 默认配置路径:`~/.config/linop/telnet.toml`。
pub fn default_telnet_path() -> PathBuf {
    crate::config_dir().join("telnet.toml")
}

/// Telnet 注册表,按名称排序。
#[derive(Debug, Clone, Default)]
pub struct TelnetRegistry {
    profiles: Vec<TelnetProfile>,
}

impl FromIterator<TelnetProfile> for TelnetRegistry {
    fn from_iter<I: IntoIterator<Item = TelnetProfile>>(iter: I) -> Self {
        let mut reg = Self::default();
        for profile in iter {
            reg.upsert(profile);
        }
        reg
    }
}

impl TelnetRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn profiles(&self) -> &[TelnetProfile] {
        &self.profiles
    }

    pub fn get(&self, name: &str) -> Option<&TelnetProfile> {
        self.profiles.iter().find(|p| p.name == name)
    }

    pub fn upsert(&mut self, profile: TelnetProfile) {
        match self.profiles.iter_mut().find(|p| p.name == profile.name) {
            Some(slot) => *slot = profile,
            None => {
                self.profiles.push(profile);
                self.profiles.sort_by(|a, b| a.name.cmp(&b.name));
            }
        }
    }

    pub fn remove(&mut self, name: &str) -> Result<TelnetProfile> {
        let idx = self
            .profiles
            .iter()
            .position(|p| p.name == name)
            .ok_or_else(|| CoreError::NotFound(name.to_string()))?;
        Ok(self.profiles.remove(idx))
    }

    /// 加载;文件不存在返回空表。
    pub fn load(path: impl AsRef<std::path::Path>) -> Result<Self> {
        let path = path.as_ref();
        if !path.exists() {
            return Ok(Self::default());
        }
        let text = std::fs::read_to_string(path)?;
        let raw: BTreeMap<String, RawTelnet> =
            toml::from_str(&text).map_err(|source| CoreError::Parse {
                path: path.to_path_buf(),
                source,
            })?;
        let profiles = raw
            .into_iter()
            .map(|(name, entry)| TelnetProfile {
                name,
                host: entry.host,
                port: entry.port,
            })
            .collect();
        Ok(Self { profiles })
    }

    pub fn save(&self, path: impl AsRef<std::path::Path>) -> Result<()> {
        let path = path.as_ref();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let raw: BTreeMap<String, RawTelnet> = self
            .profiles
            .iter()
            .map(|p| {
                (
                    p.name.clone(),
                    RawTelnet {
                        host: p.host.clone(),
                        port: p.port,
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
        Self::load(default_telnet_path())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip_telnet_toml() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("telnet.toml");
        let reg: TelnetRegistry = vec![
            TelnetProfile::new("switch", "10.0.0.2", 23),
            TelnetProfile::new("bbs", "bbs.example.org", 2323),
        ]
        .into_iter()
        .collect();
        reg.save(&path).unwrap();
        let loaded = TelnetRegistry::load(&path).unwrap();
        assert_eq!(loaded.profiles(), reg.profiles());
        // 端口缺省 23 已落盘。
        assert_eq!(loaded.get("switch").unwrap().port, 23);
    }

    #[test]
    fn missing_file_and_remove() {
        assert!(
            TelnetRegistry::load("/nonexistent/telnet.toml")
                .unwrap()
                .profiles()
                .is_empty()
        );
        let mut reg: TelnetRegistry = vec![TelnetProfile::new("x", "h", 23)].into_iter().collect();
        assert!(reg.remove("x").is_ok());
        assert!(reg.remove("x").is_err());
    }
}
