//! 解析 OpenSSH `~/.ssh/config` 的子集,转换为连接 profile。
//!
//! 支持指令:Host / HostName / User / Port / IdentityFile;
//! 通配符别名(`*`/`?`)与未知指令忽略。HostName 缺省用别名本身,
//! User 缺省取当前用户,`~` 前缀展开为家目录。

use std::path::PathBuf;

use crate::model::{Auth, Connection};

/// 默认 ssh config 路径(`LINOP_SSH_CONFIG` 可覆盖,便于测试)。
pub fn default_ssh_config_path() -> PathBuf {
    if let Ok(path) = std::env::var("LINOP_SSH_CONFIG") {
        return PathBuf::from(path);
    }
    ssh_dir().join("config")
}

fn ssh_dir() -> PathBuf {
    dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".ssh")
}

fn expand_tilde(path: &str) -> PathBuf {
    if let Some(rest) = path.strip_prefix("~/") {
        dirs::home_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join(rest)
    } else {
        PathBuf::from(path)
    }
}

/// 解析 ssh config 文本为连接列表(按出现顺序)。
pub fn parse_ssh_config(text: &str) -> Vec<Connection> {
    let mut connections: Vec<Connection> = Vec::new();
    // 当前块 accumulators:同一 Host 块内后写的值覆盖先写的。
    let mut names: Vec<String> = Vec::new();
    let mut host_name: Option<String> = None;
    let mut user: Option<String> = None;
    let mut port: Option<u16> = None;
    let mut identity: Option<PathBuf> = None;

    let flush = |names: &mut Vec<String>,
                 host_name: &mut Option<String>,
                 user: &mut Option<String>,
                 port: &mut Option<u16>,
                 identity: &mut Option<PathBuf>,
                 connections: &mut Vec<Connection>| {
        for alias in names.drain(..) {
            let auth = match identity.take() {
                Some(path) => Auth::key(path),
                None => Auth::password(String::new()),
            };
            let mut conn = Connection::new(
                alias.clone(),
                host_name.clone().unwrap_or(alias),
                user.clone().unwrap_or_else(current_username),
                auth,
            );
            if let Some(port) = *port {
                conn.port = port;
            }
            connections.push(conn);
        }
        // 同一块的多别名各自独立使用这些值:重置后进入下一块。
        *host_name = None;
        *user = None;
        *port = None;
        *identity = None;
    };

    for raw in text.lines() {
        let line = raw.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let (keyword, value) = match line.split_once('=') {
            Some((k, v)) => (k.trim(), v.trim()),
            None => match line.split_once(char::is_whitespace) {
                Some((k, v)) => (k, v.trim()),
                None => continue,
            },
        };
        match keyword.to_ascii_lowercase().as_str() {
            "host" => {
                flush(
                    &mut names,
                    &mut host_name,
                    &mut user,
                    &mut port,
                    &mut identity,
                    &mut connections,
                );
                names = value
                    .split_whitespace()
                    .filter(|alias| !alias.contains(['*', '?']))
                    .map(str::to_string)
                    .collect();
            }
            "hostname" => host_name = Some(value.to_string()),
            "user" => user = Some(value.to_string()),
            "port" => port = value.parse().ok(),
            "identityfile" => identity = Some(expand_tilde(value)),
            _ => {}
        }
    }
    flush(
        &mut names,
        &mut host_name,
        &mut user,
        &mut port,
        &mut identity,
        &mut connections,
    );
    connections
}

fn current_username() -> String {
    std::env::var("USER")
        .or_else(|_| std::env::var("LOGNAME"))
        .unwrap_or_else(|_| "root".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_basic_blocks_with_defaults() {
        let text = "\
# 注释行
Host web-1
  HostName 192.168.1.10
  Port 2222
  User root
  IdentityFile ~/.ssh/id_ed25519

Host plain
  HostName plain.example.com
";
        let conns = parse_ssh_config(text);
        assert_eq!(conns.len(), 2);

        let web = &conns[0];
        assert_eq!(web.name, "web-1");
        assert_eq!(web.host, "192.168.1.10");
        assert_eq!(web.port, 2222);
        assert_eq!(web.username, "root");
        assert_eq!(
            web.auth,
            Auth::key(dirs::home_dir().unwrap().join(".ssh/id_ed25519"))
        );

        // 缺省:HostName=别名,User=当前用户,端口 22,密码留空。
        let plain = &conns[1];
        assert_eq!(plain.host, "plain.example.com");
        assert_eq!(plain.port, 22);
        assert_eq!(plain.username, current_username());
        assert_eq!(plain.auth, Auth::password(String::new()));
    }

    #[test]
    fn skips_wildcard_hosts_and_keeps_literals() {
        let text = "Host *\n  User fallback\n\nHost real\n  HostName 10.0.0.1\n";
        let conns = parse_ssh_config(text);
        assert_eq!(conns.len(), 1);
        assert_eq!(conns[0].name, "real");
        // 通配符块的 User 不外溢到后续块(每块独立)。
        assert_eq!(conns[0].username, current_username());
    }

    #[test]
    fn multiple_aliases_on_one_line() {
        let conns = parse_ssh_config("Host a b\n  HostName 10.0.0.9\n");
        assert_eq!(conns.len(), 2);
        assert_eq!(conns[0].name, "a");
        assert_eq!(conns[1].name, "b");
        assert_eq!(conns[1].host, "10.0.0.9");
    }

    #[test]
    fn equals_sign_syntax() {
        let conns = parse_ssh_config("Host=x\nHostName=10.0.0.2\nPort=2222\n");
        assert_eq!(conns[0].name, "x");
        assert_eq!(conns[0].host, "10.0.0.2");
        assert_eq!(conns[0].port, 2222);
    }
}
