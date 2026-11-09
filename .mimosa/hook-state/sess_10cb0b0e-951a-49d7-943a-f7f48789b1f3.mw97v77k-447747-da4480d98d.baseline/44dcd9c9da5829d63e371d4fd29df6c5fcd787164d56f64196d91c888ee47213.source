//! Linop connections.toml → navop 连接导入扩展(wasm32-wasip2 组件)。
//!
//! 实现 navop `connection-importer` WIT world:guest 侧 descriptor/scan/
//! preview 均返回 JSON 字符串,形状由 vendored 的 connection-import-protocol
//! 类型保证(host 解码端权威模型)。
//!
//! 分层:
//! - `pure`:解析 Linop 的 connections.toml 原文 → ImportRecord(native 可测);
//! - guest 胶水:WIT 导出,host `read-file` 取候选文件后交给 pure。

use std::borrow::Cow;

pub mod protocol;

use crate::protocol as proto;

/// 扩展与 importer 的稳定 id(与 extension.json 一致)。
pub const EXTENSION_ID: &str = "com.linop.importer.config";
pub const IMPORTER_ID: &str = "linop-config";
pub const SOURCE_LABEL: &str = "Linop";

// ---------------------------------------------------------------------------
// 纯逻辑:TOML 解析 → 协议记录
// ---------------------------------------------------------------------------

/// connections.toml 的条目形状(linop-core RawConnection 的镜像,
/// 独立声明以便 wasm 组件自包含)。
#[derive(Debug, Default, serde::Deserialize)]
pub struct RawForward {
    pub local_port: Option<u16>,
    #[serde(default)]
    pub remote_host: Option<String>,
    pub remote_port: Option<u16>,
    // 反向转发(-R)字段
    #[serde(default)]
    pub bind_host: Option<String>,
    #[serde(default)]
    pub bind_port: Option<u16>,
    #[serde(default)]
    pub target_host: Option<String>,
    #[serde(default)]
    pub target_port: Option<u16>,
}

#[derive(Debug, Default, serde::Deserialize)]
pub struct RawConnection {
    pub host: Option<String>,
    pub port: Option<u16>,
    pub username: Option<String>,
    #[serde(default)]
    pub password: Option<String>,
    #[serde(default)]
    pub key_path: Option<String>,
    #[serde(default)]
    pub key_passphrase: Option<String>,
    #[serde(default)]
    pub forwards: Vec<RawForward>,
    #[serde(default)]
    pub remote_forwards: Vec<RawForward>,
    #[serde(default)]
    pub dynamic_ports: Vec<u16>,
    #[serde(default)]
    pub notes: Option<String>,
}

#[derive(Debug, thiserror::Error)]
pub enum ParseError {
    #[error("toml parse: {0}")]
    Toml(#[from] toml::de::Error),
}

/// 解析整份 connections.toml(name → RawConnection)。
pub fn parse_connections(
    text: &str,
) -> Result<std::collections::BTreeMap<String, RawConnection>, ParseError> {
    Ok(toml::from_str(text)?)
}

/// 认证方法:密码被扣留时按协议约定返回 null + 状态位。
fn auth_method(
    raw: &RawConnection,
    include_passwords: bool,
) -> (proto::SshImportAuthMethod, proto::PasswordImportStatus) {
    if let Some(key_path) = &raw.key_path {
        let passphrase = raw
            .key_passphrase
            .clone()
            .filter(|p| include_passwords && !p.is_empty());
        let status = if raw.key_passphrase.as_deref().is_some_and(|p| !p.is_empty()) {
            if include_passwords {
                proto::PasswordImportStatus::Included
            } else {
                proto::PasswordImportStatus::PermissionDenied
            }
        } else {
            proto::PasswordImportStatus::Missing
        };
        return (
            proto::SshImportAuthMethod::PrivateKey {
                key_path: key_path.clone(),
                passphrase,
            },
            status,
        );
    }
    let password = raw.password.clone().filter(|p| !p.is_empty());
    match password {
        Some(pw) if include_passwords => (
            proto::SshImportAuthMethod::Password { password: Some(pw) },
            proto::PasswordImportStatus::Included,
        ),
        Some(_) => (
            proto::SshImportAuthMethod::Password { password: None },
            proto::PasswordImportStatus::PermissionDenied,
        ),
        None => (
            proto::SshImportAuthMethod::AutoPublicKey,
            proto::PasswordImportStatus::Missing,
        ),
    }
}

fn warning(code: &str, message: impl Into<String>) -> proto::ImportWarning {
    proto::ImportWarning {
        code: code.to_string(),
        message: message.into(),
    }
}

/// 构造全部导入记录:每条 SSH 连接一条,转发规则各一条。
pub fn build_records(
    connections: &std::collections::BTreeMap<String, RawConnection>,
    include_passwords: bool,
) -> Vec<proto::ImportRecord> {
    let mut records = Vec::new();
    for (name, raw) in connections {
        let Some(host) = raw.host.clone() else {
            continue; // 无 host 的畸形条目跳过
        };
        let username = raw.username.clone().unwrap_or_else(|| "root".into());
        let (auth, password_status) = auth_method(raw, include_passwords);
        let source_id = format!("linop:{name}");

        records.push(proto::ImportRecord {
            id: source_id.clone(),
            importer_id: IMPORTER_ID.to_string(),
            source_label: SOURCE_LABEL.to_string(),
            source_id: Some(source_id.clone()),
            kind: proto::ImportRecordKind::Ssh,
            display_name: name.clone(),
            database: None,
            ssh: Some(proto::SshImportRecord {
                name: name.clone(),
                host,
                port: raw.port,
                username,
                group_path: Some("Linop".to_string()),
                auth_method: auth,
                init_script: None,
                jump_server: None,
                proxy: None,
            }),
            port_forwarding: None,
            quick_command: None,
            workspace: None,
            password_status,
            warnings: raw
                .notes
                .as_ref()
                .map(|n| vec![warning("linop.notes", format!("连接备注:{n}"))])
                .unwrap_or_default(),
        });

        for (idx, fwd) in raw.forwards.iter().enumerate() {
            let (Some(local_port), Some(remote_host), Some(remote_port)) =
                (fwd.local_port, fwd.remote_host.clone(), fwd.remote_port)
            else {
                continue;
            };
            records.push(proto::ImportRecord {
                id: format!("{source_id}:forward:{idx}"),
                importer_id: IMPORTER_ID.to_string(),
                source_label: SOURCE_LABEL.to_string(),
                source_id: Some(source_id.clone()),
                kind: proto::ImportRecordKind::PortForwarding,
                display_name: format!("{name}:{local_port}→{remote_host}:{remote_port}"),
                database: None,
                ssh: None,
                port_forwarding: Some(proto::PortForwardingImportRecord {
                    name: format!("{name}-local-{local_port}"),
                    ssh_source_id: source_id.clone(),
                    kind: proto::PortForwardingImportKind::Local,
                    bind_host: "127.0.0.1".into(),
                    bind_port: local_port,
                    target_host: remote_host,
                    target_port: remote_port,
                }),
                quick_command: None,
                workspace: None,
                password_status: proto::PasswordImportStatus::Missing,
                warnings: Vec::new(),
            });
        }

        for (idx, fwd) in raw.remote_forwards.iter().enumerate() {
            let (Some(bind_port), Some(target_host), Some(target_port)) =
                (fwd.bind_port, fwd.target_host.clone(), fwd.target_port)
            else {
                continue;
            };
            records.push(proto::ImportRecord {
                id: format!("{source_id}:remote:{idx}"),
                importer_id: IMPORTER_ID.to_string(),
                source_label: SOURCE_LABEL.to_string(),
                source_id: Some(source_id.clone()),
                kind: proto::ImportRecordKind::PortForwarding,
                display_name: format!("{name}:{bind_port}←{target_host}:{target_port}"),
                database: None,
                ssh: None,
                port_forwarding: Some(proto::PortForwardingImportRecord {
                    name: format!("{name}-remote-{bind_port}"),
                    ssh_source_id: source_id.clone(),
                    kind: proto::PortForwardingImportKind::Remote,
                    bind_host: fwd.bind_host.clone().unwrap_or_else(|| "0.0.0.0".into()),
                    bind_port,
                    target_host,
                    target_port,
                }),
                quick_command: None,
                workspace: None,
                password_status: proto::PasswordImportStatus::Missing,
                warnings: Vec::new(),
            });
        }

        for port in &raw.dynamic_ports {
            records.push(proto::ImportRecord {
                id: format!("{source_id}:dynamic:{port}"),
                importer_id: IMPORTER_ID.to_string(),
                source_label: SOURCE_LABEL.to_string(),
                source_id: Some(source_id.clone()),
                kind: proto::ImportRecordKind::PortForwarding,
                display_name: format!("{name}:socks-{port}"),
                database: None,
                ssh: None,
                port_forwarding: Some(proto::PortForwardingImportRecord {
                    name: format!("{name}-socks-{port}"),
                    ssh_source_id: source_id.clone(),
                    kind: proto::PortForwardingImportKind::Dynamic,
                    bind_host: "127.0.0.1".into(),
                    bind_port: *port,
                    target_host: String::new(),
                    target_port: 0,
                }),
                quick_command: None,
                workspace: None,
                password_status: proto::PasswordImportStatus::Missing,
                warnings: vec![warning(
                    "linop.dynamic_target",
                    "SOCKS 动态转发无固定目标,目标字段留空",
                )],
            });
        }
    }
    records
}

// ---------------------------------------------------------------------------
// guest 胶水:wasm 组件导出
// ---------------------------------------------------------------------------

#[cfg(target_arch = "wasm32")]
mod guest {
    wit_bindgen::generate!({
        path: "wit",
        world: "connection-importer",
    });

    use super::*;

    struct LinopImporter;

    impl Guest for LinopImporter {
        fn descriptor() -> String {
            let descriptor = proto::ImporterDescriptor {
                id: IMPORTER_ID.to_string(),
                display_name: "Linop".to_string(),
                description: Some(
                    "Import SSH connections and port forwards from Linop's connections.toml".into(),
                ),
                icon: None,
                vendor: Some("Linop".into()),
                supported_platforms: vec![
                    proto::Platform::Linux,
                    proto::Platform::Macos,
                    proto::Platform::Windows,
                ],
                output_kinds: vec![
                    proto::ImportRecordKind::Ssh,
                    proto::ImportRecordKind::PortForwarding,
                ],
                capabilities: proto::ImporterCapabilities {
                    supports_scan: true,
                    supports_password_import: true,
                    supports_manual_file_pick: false,
                    supports_manual_directory_pick: false,
                    manual_file_pick_prompt: None,
                    manual_directory_pick_prompt: None,
                    supports_incremental_preview: false,
                },
            };
            serde_json::to_string(&descriptor).expect("descriptor serializes")
        }

        fn scan() -> String {
            let candidates =
                onet::extension::connection_import_host::list_candidate_files(IMPORTER_ID);
            let mut warnings = Vec::new();
            let mut discovered = Vec::new();
            let mut estimated: Option<u32> = None;
            for candidate in &candidates {
                match onet::extension::connection_import_host::read_file(&candidate.id) {
                    Ok(bytes) => {
                        let text = String::from_utf8_lossy(&bytes).into_owned();
                        match crate::parse_connections(&text) {
                            Ok(connections) => {
                                let count = connections.len() as u32;
                                estimated = Some(estimated.unwrap_or(0) + count);
                            }
                            Err(e) => warnings.push(warning("linop.parse", e.to_string())),
                        }
                        discovered.push(proto::DiscoveredFile {
                            candidate_id: candidate.id.clone(),
                            display_path: candidate.path.clone(),
                        });
                    }
                    Err(_) => warnings.push(warning(
                        "linop.read",
                        format!("无法读取 {}", candidate.path),
                    )),
                }
            }
            let report = proto::ImportScanReport {
                importer_id: IMPORTER_ID.to_string(),
                availability: if estimated.is_some_and(|n| n > 0) {
                    proto::ImporterAvailability::Available {
                        estimated_count: estimated,
                    }
                } else {
                    proto::ImporterAvailability::NoData
                },
                discovered_files: discovered,
                warnings,
                discovered_workspace_paths: Vec::new(),
            };
            serde_json::to_string(&report).expect("scan report serializes")
        }

        fn preview(options: onet::extension::connection_import::ImportOptions) -> String {
            let candidates =
                onet::extension::connection_import_host::list_candidate_files(IMPORTER_ID);
            let mut records = Vec::new();
            for candidate in &candidates {
                let Ok(bytes) = onet::extension::connection_import_host::read_file(&candidate.id)
                else {
                    continue;
                };
                let text = String::from_utf8_lossy(&bytes).into_owned();
                match crate::parse_connections(&text) {
                    Ok(connections) => records.extend(crate::build_records(
                        &connections,
                        options.include_passwords,
                    )),
                    Err(_) => continue, // scan 已报 warning
                }
            }
            serde_json::to_string(&records).expect("records serialize")
        }
    }

    export!(LinopImporter);
}

/// 非 wasm 目标(native 测试/CI):guest 不导出,只暴露纯逻辑。
#[cfg(not(target_arch = "wasm32"))]
pub use pure_api::*;

#[cfg(not(target_arch = "wasm32"))]
mod pure_api {
    /// native 环境下的 descriptor JSON(与 wasm 导出一致,供宿主测试比对)。
    pub fn descriptor_json() -> String {
        use crate::protocol as proto;
        let descriptor = proto::ImporterDescriptor {
            id: crate::IMPORTER_ID.to_string(),
            display_name: "Linop".to_string(),
            description: Some(
                "Import SSH connections and port forwards from Linop's connections.toml".into(),
            ),
            icon: None,
            vendor: Some("Linop".into()),
            supported_platforms: vec![
                proto::Platform::Linux,
                proto::Platform::Macos,
                proto::Platform::Windows,
            ],
            output_kinds: vec![
                proto::ImportRecordKind::Ssh,
                proto::ImportRecordKind::PortForwarding,
            ],
            capabilities: proto::ImporterCapabilities {
                supports_scan: true,
                supports_password_import: true,
                supports_manual_file_pick: false,
                supports_manual_directory_pick: false,
                manual_file_pick_prompt: None,
                manual_directory_pick_prompt: None,
                supports_incremental_preview: false,
            },
        };
        serde_json::to_string(&descriptor).expect("descriptor serializes")
    }

    // 供外部(native 测试/宿主)复用;crate 内部未必全部使用。
    #[allow(unused_imports)]
    pub use super::{EXTENSION_ID, IMPORTER_ID, SOURCE_LABEL, build_records, parse_connections};
}

#[cfg(not(target_arch = "wasm32"))]
use serde_json as _;

#[allow(unused)]
fn _keep_cow_import() -> Cow<'static, str> {
    Cow::Borrowed("")
}

#[cfg(test)]
mod tests {
    use super::*;
    use proto::{ImportRecordKind, PasswordImportStatus, SshImportAuthMethod};

    fn toml_text() -> String {
        r#"
[web-1]
host = "192.168.1.10"
port = 2222
username = "deploy"
password = "secret"

[[web-1.forwards]]
local_port = 8080
remote_host = "127.0.0.1"
remote_port = 80

[db-1]
host = "db.internal"
username = "admin"
key_path = "~/.ssh/id_ed25519"
notes = "生产库"
"#
        .to_string()
    }

    #[test]
    fn parses_and_builds_ssh_record_with_exact_json_shape() {
        let connections = parse_connections(&toml_text()).unwrap();
        assert_eq!(connections.len(), 2);

        let records = build_records(&connections, true);
        let ssh = records
            .iter()
            .find(|r| r.display_name == "web-1")
            .expect("web-1 record");
        assert!(matches!(ssh.kind, ImportRecordKind::Ssh));
        assert_eq!(ssh.password_status, PasswordImportStatus::Included);
        assert!(matches!(
            &ssh.ssh.as_ref().unwrap().auth_method,
            SshImportAuthMethod::Password { password: Some(pw) } if pw == "secret"
        ));

        // 宿主解码端权威形状:逐字段 JSON 断言。
        let json = serde_json::to_value(ssh).unwrap();
        assert_eq!(json["kind"], "ssh");
        assert_eq!(json["ssh"]["auth_method"]["password"]["password"], "secret");
        assert_eq!(json["ssh"]["port"], 2222);
        assert_eq!(json["password_status"], "included");
        assert_eq!(json["ssh"]["group_path"], "Linop");

        // validate_shape(协议 crate 的守门函数)必须通过。
        ssh.validate_shape().unwrap();
    }

    #[test]
    fn key_auth_maps_to_private_key_with_path() {
        let connections = parse_connections(&toml_text()).unwrap();
        let records = build_records(&connections, false);
        let db = records.iter().find(|r| r.display_name == "db-1").unwrap();
        let json = serde_json::to_value(db).unwrap();
        assert_eq!(
            json["ssh"]["auth_method"],
            serde_json::json!({"private_key": {"key_path": "~/.ssh/id_ed25519", "passphrase": null}})
        );
        assert_eq!(json["password_status"], "missing");
        assert_eq!(json["warnings"][0]["code"], "linop.notes");
        db.validate_shape().unwrap();
    }

    #[test]
    fn withheld_password_becomes_null_with_permission_denied() {
        let connections = parse_connections(&toml_text()).unwrap();
        let records = build_records(&connections, false);
        let ssh = records.iter().find(|r| r.display_name == "web-1").unwrap();
        let json = serde_json::to_value(ssh).unwrap();
        assert_eq!(
            json["ssh"]["auth_method"]["password"]["password"],
            serde_json::Value::Null
        );
        assert_eq!(json["password_status"], "permission_denied");
        ssh.validate_shape().unwrap();
    }

    #[test]
    fn forwards_become_port_forwarding_records() {
        let connections = parse_connections(&toml_text()).unwrap();
        let records = build_records(&connections, true);
        let fwd = records
            .iter()
            .find(|r| matches!(r.kind, ImportRecordKind::PortForwarding))
            .expect("至少一条转发");
        let json = serde_json::to_value(fwd).unwrap();
        assert_eq!(json["kind"], "port_forwarding");
        assert_eq!(json["port_forwarding"]["kind"], "local");
        assert_eq!(json["port_forwarding"]["bind_port"], 8080);
        assert_eq!(json["port_forwarding"]["target_host"], "127.0.0.1");
        assert_eq!(json["port_forwarding"]["ssh_source_id"], "linop:web-1");
        fwd.validate_shape().unwrap();
    }

    #[test]
    fn descriptor_json_matches_protocol_shape() {
        let descriptor: proto::ImporterDescriptor =
            serde_json::from_str(&crate::pure_api::descriptor_json()).unwrap();
        assert_eq!(descriptor.id, IMPORTER_ID);
        assert!(descriptor.capabilities.supports_scan);
        let json = serde_json::to_value(&descriptor).unwrap();
        assert_eq!(json["supported_platforms"][0], "linux");
    }

    #[test]
    fn sample_from_linop_core_registry_roundtrips() {
        // 与 linop-core 注册表真实落盘格式对齐。
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("connections.toml");
        let conn = linop_core::Connection::new(
            "prod",
            "10.0.0.9",
            "ops",
            linop_core::Auth::password("pw9"),
        );
        let reg: linop_core::Registry = vec![conn].into_iter().collect();
        reg.save(&path).unwrap();
        let text = std::fs::read_to_string(&path).unwrap();

        let connections = parse_connections(&text).unwrap();
        assert_eq!(connections.len(), 1);
        let records = build_records(&connections, true);
        let ssh = &records[0];
        assert_eq!(ssh.display_name, "prod");
        assert!(matches!(
            &ssh.ssh.as_ref().unwrap().auth_method,
            SshImportAuthMethod::Password { password: Some(pw) } if pw == "pw9"
        ));
        ssh.validate_shape().unwrap();
    }
}
