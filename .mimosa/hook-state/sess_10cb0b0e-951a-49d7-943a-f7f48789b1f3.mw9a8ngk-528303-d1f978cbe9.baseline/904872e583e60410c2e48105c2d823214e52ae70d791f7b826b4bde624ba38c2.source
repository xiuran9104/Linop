//! TOFU(trust-on-first-use)known_hosts 存储,借鉴 navop 的
//! known_hosts 管理:首次连接记录主机指纹,之后校验指纹是否变化。

use std::collections::BTreeMap;
use std::path::PathBuf;

use crate::SshError;

/// 主机密钥校验结论。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Decision {
    /// 首次见到该主机:记录指纹并放行。
    NewlyTrusted,
    /// 指纹与记录一致。
    Trusted,
    /// 指纹变化,疑似中间人攻击。
    Mismatch { known: String, got: String },
}

/// 校验策略。
pub enum KnownHostsPolicy {
    /// 全部信任(危险,仅用于测试)。
    AcceptAll,
    /// TOFU:信任首次指纹,之后必须匹配。
    TrustOnFirstUse(KnownHosts),
}

/// 指纹库,内存 map + 可选持久化文件。
#[derive(Debug, Clone, Default)]
pub struct KnownHosts {
    path: Option<PathBuf>,
    entries: BTreeMap<String, String>,
}

impl KnownHosts {
    pub fn in_memory() -> Self {
        Self::default()
    }

    /// 内存指纹库,但持久化到指定文件。
    pub fn with_path(path: impl Into<PathBuf>) -> Self {
        Self {
            path: Some(path.into()),
            entries: BTreeMap::new(),
        }
    }

    /// 从 linop 配置目录加载(`~/.config/linop/known_hosts`)。
    pub fn load_default() -> Self {
        let path = linop_core::known_hosts_path();
        let entries = std::fs::read_to_string(&path)
            .map(|text| parse_known_hosts(&text))
            .unwrap_or_default();
        Self {
            path: Some(path),
            entries,
        }
    }

    pub fn fingerprint_of(&self, host: &str, port: u16) -> Option<&str> {
        self.entries.get(&host_key(host, port)).map(|s| s.as_str())
    }

    /// 校验并按需记录指纹。变化时返回 `Decision::Mismatch` 且不落盘。
    pub fn decide(&mut self, host: &str, port: u16, fingerprint: &str) -> Decision {
        let key = host_key(host, port);
        match self.entries.get(&key) {
            Some(known) if known == fingerprint => Decision::Trusted,
            Some(known) => Decision::Mismatch {
                known: known.clone(),
                got: fingerprint.to_string(),
            },
            None => {
                self.entries.insert(key, fingerprint.to_string());
                Decision::NewlyTrusted
            }
        }
    }

    /// 持久化到关联文件(若存在)。
    pub fn save(&self) -> Result<(), SshError> {
        if let Some(path) = &self.path {
            if let Some(parent) = path.parent() {
                std::fs::create_dir_all(parent)?;
            }
            let text = self
                .entries
                .iter()
                .map(|(k, v)| format!("{k} {v}\n"))
                .collect::<String>();
            std::fs::write(path, text)?;
        }
        Ok(())
    }
}

fn host_key(host: &str, port: u16) -> String {
    if port == 22 {
        host.to_string()
    } else {
        format!("[{host}]:{port}")
    }
}

fn parse_known_hosts(text: &str) -> BTreeMap<String, String> {
    text.lines()
        .filter_map(|line| line.split_once(' '))
        .map(|(k, v)| (k.trim().to_string(), v.trim().to_string()))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tofu_flow() {
        let mut hosts = KnownHosts::in_memory();
        assert_eq!(
            hosts.decide("a.com", 22, "SHA256:aaa"),
            Decision::NewlyTrusted
        );
        assert_eq!(hosts.decide("a.com", 22, "SHA256:aaa"), Decision::Trusted);
        assert_eq!(
            hosts.decide("a.com", 22, "SHA256:bbb"),
            Decision::Mismatch {
                known: "SHA256:aaa".into(),
                got: "SHA256:bbb".into()
            }
        );
    }

    #[test]
    fn distinct_ports_are_distinct_hosts() {
        let mut hosts = KnownHosts::in_memory();
        hosts.decide("a.com", 22, "SHA256:aaa");
        assert_eq!(hosts.fingerprint_of("a.com", 2222), None);
        assert_eq!(hosts.fingerprint_of("[a.com]:2222", 0), None); // host_key 仅由构造方调用
    }

    #[test]
    fn persist_roundtrip() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("known_hosts");
        let mut hosts = KnownHosts {
            path: Some(path.clone()),
            entries: BTreeMap::new(),
        };
        hosts.decide("b.com", 2222, "SHA256:xyz");
        hosts.save().unwrap();
        let text = std::fs::read_to_string(&path).unwrap();
        assert_eq!(text, "[b.com]:2222 SHA256:xyz\n");
    }
}
