//! linop-sftp: 基于 russh-sftp 的文件操作层。
//!
//! 对应 navop 的 `sftp` / `sftp_view` / `sftp_transfer` crate:
//! 目录浏览、上传、下载,以及面向 TUI 的条目模型。

// 错误枚举聚合多个上游错误类型,按值返回换取使用简单性。
#![allow(clippy::result_large_err)]

use std::path::Path;

use russh_sftp::client::SftpSession;
use tokio::io::AsyncReadExt;

#[derive(Debug, thiserror::Error)]
pub enum SftpError {
    #[error("sftp error: {0}")]
    Sftp(#[from] russh_sftp::client::error::Error),
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("ssh error: {0}")]
    Ssh(#[from] linop_ssh::SshError),
    #[error("invalid path: {0}")]
    InvalidPath(String),
}

pub type Result<T> = std::result::Result<T, SftpError>;

/// 面向 UI 的目录条目。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Entry {
    pub name: String,
    pub is_dir: bool,
    pub size: u64,
    /// 人类可读大小(如 "1.2 KiB"),用于表格展示。
    pub size_h: String,
}

/// 字节数转人类可读大小(如 "1.2 KiB")。
pub fn human_size(size: u64) -> String {
    const UNITS: [&str; 5] = ["B", "KiB", "MiB", "GiB", "TiB"];
    let mut value = size as f64;
    let mut unit = 0;
    while value >= 1024.0 && unit < UNITS.len() - 1 {
        value /= 1024.0;
        unit += 1;
    }
    if unit == 0 {
        format!("{size} B")
    } else {
        format!("{value:.1} {}", UNITS[unit])
    }
}

/// SFTP 会话包装。
pub struct Sftp {
    session: SftpSession,
    /// 当前目录(绝对路径,以 '/' 开头)。
    cwd: String,
}

impl Sftp {
    /// 在 SSH 会话上打开 SFTP 子系统并进入用户主目录。
    pub async fn open(ssh: &linop_ssh::Session) -> Result<Self> {
        let stream = ssh.open_sftp_stream().await?;
        let session = SftpSession::new(stream).await?;
        let mut sftp = Self {
            session,
            cwd: "/".to_string(),
        };
        sftp.cwd = sftp.canonicalize(".").await?;
        Ok(sftp)
    }

    pub fn cwd(&self) -> &str {
        &self.cwd
    }

    /// 规范化路径:相对路径基于 cwd。
    fn resolve(&self, path: &str) -> String {
        resolve_against(&self.cwd, path)
    }

    pub async fn canonicalize(&self, path: &str) -> Result<String> {
        Ok(self.session.canonicalize(self.resolve(path)).await?)
    }

    /// 列出目录(按 名称 排序,目录优先)。
    pub async fn list_dir(&self, path: &str) -> Result<Vec<Entry>> {
        let full = self.resolve(path);
        let mut entries: Vec<Entry> = self
            .session
            .read_dir(&full)
            .await?
            .map(|dirent| {
                let metadata = dirent.metadata();
                let size = metadata.size.unwrap_or(0);
                Entry {
                    name: dirent.file_name(),
                    is_dir: metadata.is_dir(),
                    size,
                    size_h: human_size(size),
                }
            })
            .filter(|e| e.name != "." && e.name != "..")
            .collect();
        entries.sort_by(|a, b| b.is_dir.cmp(&a.is_dir).then_with(|| a.name.cmp(&b.name)));
        Ok(entries)
    }

    /// 进入目录并更新 cwd,返回新目录的条目。
    pub async fn enter(&mut self, path: &str) -> Result<Vec<Entry>> {
        let full = self.canonicalize(path).await?;
        let entries = self.list_dir(&full).await?;
        self.cwd = full;
        Ok(entries)
    }

    pub async fn go_parent(&mut self) -> Result<Vec<Entry>> {
        let parent = parent_of(&self.cwd);
        self.enter(&parent).await
    }

    /// 下载远端文件到本地路径。目录递归下载。
    pub async fn download(&self, remote: &str, local: &Path) -> Result<()> {
        let remote_path = self.resolve(remote);
        let md = self.session.metadata(&remote_path).await?;
        if md.is_dir() {
            std::fs::create_dir_all(local)?;
            for entry in self.list_dir(&remote_path).await? {
                let child_local = local.join(&entry.name);
                let child_remote = format!("{remote_path}/{}", entry.name);
                Box::pin(self.download(&child_remote, &child_local)).await?;
            }
        } else {
            if let Some(parent) = local.parent() {
                std::fs::create_dir_all(parent)?;
            }
            let mut file = self.session.open(&remote_path).await?;
            let mut buf = Vec::new();
            file.read_to_end(&mut buf).await?;
            std::fs::write(local, buf)?;
        }
        Ok(())
    }

    /// 上传本地文件/目录到远端路径。
    pub async fn upload(&self, local: &Path, remote: &str) -> Result<()> {
        let remote_path = self.resolve(remote);
        if local.is_dir() {
            // 目录:确保存在,再递归上传子项。
            if self.session.metadata(&remote_path).await.is_err() {
                self.session.create_dir(&remote_path).await?;
            }
            for entry in std::fs::read_dir(local)? {
                let entry = entry?;
                let child_local = entry.path();
                let child_remote = format!("{remote_path}/{}", entry.file_name().to_string_lossy());
                Box::pin(self.upload(&child_local, &child_remote)).await?;
            }
        } else {
            let data = std::fs::read(local)?;
            self.session.write(&remote_path, &data).await?;
        }
        Ok(())
    }

    pub async fn mkdir(&self, path: &str) -> Result<()> {
        Ok(self.session.create_dir(self.resolve(path)).await?)
    }

    pub async fn remove_file(&self, path: &str) -> Result<()> {
        Ok(self.session.remove_file(self.resolve(path)).await?)
    }

    pub async fn remove_dir(&self, path: &str) -> Result<()> {
        Ok(self.session.remove_dir(self.resolve(path)).await?)
    }

    pub async fn rename(&self, from: &str, to: &str) -> Result<()> {
        Ok(self
            .session
            .rename(self.resolve(from), self.resolve(to))
            .await?)
    }

    /// 读取远端文本文件(远程编辑场景的最小实现)。
    pub async fn read_text(&self, path: &str) -> Result<String> {
        let mut file = self.session.open(self.resolve(path)).await?;
        let mut text = String::new();
        file.read_to_string(&mut text).await?;
        Ok(text)
    }

    pub async fn write_text(&self, path: &str, content: &str) -> Result<()> {
        self.session
            .write(self.resolve(path), content.as_bytes())
            .await?;
        Ok(())
    }

    pub async fn close(&self) -> Result<()> {
        Ok(self.session.close().await?)
    }
}

fn resolve_against(cwd: &str, path: &str) -> String {
    if path.starts_with('/') {
        normalize(path)
    } else {
        normalize(&format!("{cwd}/{path}"))
    }
}

fn normalize(path: &str) -> String {
    let mut parts: Vec<&str> = Vec::new();
    for seg in path.split('/') {
        match seg {
            "" | "." => {}
            ".." => {
                parts.pop();
            }
            other => parts.push(other),
        }
    }
    format!("/{}", parts.join("/"))
}

fn parent_of(path: &str) -> String {
    let normalized = normalize(path);
    match normalized.rfind('/') {
        Some(0) => "/".to_string(),
        Some(idx) => normalize(&normalized[..idx]),
        None => "/".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalize_paths() {
        assert_eq!(normalize("/a/b/../c"), "/a/c");
        assert_eq!(normalize("/a/./b//c"), "/a/b/c");
        assert_eq!(normalize("/../.."), "/");
    }

    #[test]
    fn parent_paths() {
        assert_eq!(parent_of("/a/b"), "/a");
        assert_eq!(parent_of("/a"), "/");
        assert_eq!(parent_of("/"), "/");
    }

    #[test]
    fn human_sizes() {
        assert_eq!(human_size(512), "512 B");
        assert_eq!(human_size(2048), "2.0 KiB");
        assert_eq!(human_size(5 * 1024 * 1024), "5.0 MiB");
    }

    #[test]
    fn resolve_relative_against_cwd() {
        assert_eq!(resolve_against("/home/user", "data"), "/home/user/data");
        assert_eq!(resolve_against("/home/user", "/etc"), "/etc");
        assert_eq!(resolve_against("/home/user", "../x"), "/home/x");
    }
}
