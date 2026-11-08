//! linop-dbx:Linop 的数据库模块 —— 后端直接使用 DBX(vendored dbx-core)。
//!
//! 集成形态(对应 DBX 的模块边界,见 vendor/dbx/crates/ARCHITECTURE.md):
//! - **共享连接库**:读写 DBX 的 `dbx.db`(桌面 / Web / CLI / MCP 与本模块
//!   看到的是同一份连接配置,`DBX_DATA_DIR` 可重定向,默认
//!   `$XDG_DATA_HOME/com.dbx.app/`);
//! - **查询执行**:`dbx_mcp::LocalBackend` —— 与 DBX CLI/MCP 同一条
//!   dbx-core 执行路径(驱动、超时、风控、结果截断全部复用);
//! - **UI 拉起**:`launch_web()` 启动 DBX 自带 Web UI(dbx-web,浏览器访问,
//!   与桌面版功能一致),实现"一个入口,两套界面"。

use std::path::{Path, PathBuf};
use std::process::Stdio;

use dbx_mcp::{DbxBackend, LocalBackend};

#[derive(Debug, thiserror::Error)]
pub enum DbxError {
    #[error("dbx: {0}")]
    Dbx(String),
    #[error("io: {0}")]
    Io(#[from] std::io::Error),
    #[error("DBX 数据目录无法解析(设置 DBX_DATA_DIR)")]
    NoDataDir,
}

pub type Result<T> = std::result::Result<T, DbxError>;

/// 连接摘要(picker 展示用;完整配置留在 dbx.db 里)。
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct DbxConnectionSummary {
    pub id: String,
    pub name: String,
    /// 数据库类型标识(如 "postgres"、"mysql"、"redis")。
    pub db_type: String,
    pub host: String,
    pub port: u16,
    pub database: Option<String>,
}

/// DBX 后端句柄:包装 LocalBackend,与 DBX 各形态共享 dbx.db。
pub struct Dbx {
    backend: LocalBackend,
    /// dbx.db 路径(诊断/展示)。
    pub db_path: PathBuf,
}

/// 解析 DBX 数据目录(与 dbx-mcp paths.rs 同规则,独立实现避免私有依赖)。
pub fn data_dir() -> Result<PathBuf> {
    if let Some(dir) = std::env::var_os("DBX_DATA_DIR").filter(|v| !v.is_empty()) {
        return Ok(PathBuf::from(dir));
    }
    let base = dirs_or_err()?;
    Ok(base.join("com.dbx.app"))
}

fn dirs_or_err() -> Result<PathBuf> {
    std::env::var_os("XDG_DATA_HOME")
        .filter(|v| !v.is_empty())
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".local/share")))
        .ok_or(DbxError::NoDataDir)
}

impl Dbx {
    /// 打开默认数据目录下的 dbx.db(不存在时创建空库)。
    pub async fn open_default() -> Result<Self> {
        let dir = data_dir()?;
        Self::open_at(&dir.join("dbx.db")).await
    }

    /// 打开指定 dbx.db(测试用 DBX_DATA_DIR 临时目录隔离)。
    pub async fn open_at(path: &Path) -> Result<Self> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let backend = LocalBackend::open(path).await.map_err(DbxError::Dbx)?;
        Ok(Self {
            backend,
            db_path: path.to_path_buf(),
        })
    }

    /// 全部连接摘要(按名称排序)。
    pub async fn connections(&self) -> Result<Vec<DbxConnectionSummary>> {
        let mut list: Vec<DbxConnectionSummary> = self
            .backend
            .load_connections()
            .await
            .map_err(DbxError::Dbx)?
            .into_iter()
            .map(|c| DbxConnectionSummary {
                id: c.id.clone(),
                name: c.name.clone(),
                db_type: c.db_type.as_str().to_string(),
                host: c.host.clone(),
                port: c.port,
                database: c.database.clone(),
            })
            .collect();
        list.sort_by(|a, b| a.name.cmp(&b.name));
        Ok(list)
    }

    /// 按名称查找连接(浏览/查询入口)。
    pub async fn connection_by_name(
        &self,
        name: &str,
    ) -> Result<dbx_types::models::connection::ConnectionConfig> {
        self.backend
            .load_connections()
            .await
            .map_err(DbxError::Dbx)?
            .into_iter()
            .find(|c| c.name == name)
            .ok_or_else(|| {
                DbxError::Dbx(format!("连接 {name} 不存在(在 DBX 中创建后此处立即可见)"))
            })
    }

    /// 列出连接的表。
    pub async fn list_tables(&self, name: &str) -> Result<Vec<dbx_core::types::TableInfo>> {
        let conn = self.connection_by_name(name).await?;
        let database = conn.database.clone().unwrap_or_default();
        self.backend
            .list_tables(&conn, &database, "")
            .await
            .map_err(DbxError::Dbx)
    }

    /// 执行 SQL(与 DBX CLI 同一条执行路径;默认只读风控、行数上限)。
    pub async fn query(
        &self,
        name: &str,
        sql: &str,
        max_rows: usize,
    ) -> Result<dbx_core::db::QueryResult> {
        let conn = self.connection_by_name(name).await?;
        let database = conn.database.clone().unwrap_or_default();
        self.backend
            .execute_query(&conn, &database, sql, Some(max_rows), None)
            .await
            .map_err(DbxError::Dbx)
    }

    /// 启动 DBX Web UI(dbx-web 二进制),返回子进程。
    /// UI 与本模块共享 dbx.db —— TUI 建的连接浏览器里立即可用,反之亦然。
    pub fn launch_web(bin: &Path) -> std::io::Result<std::process::Child> {
        std::process::Command::new(bin)
            .env("DBX_DATA_DIR", data_dir().ok().unwrap_or_default())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 空库冒烟:open 创建 dbx.db,connections 返回空表。
    /// (写入真实连接需要 ConnectionConfig 全量字段,见 e2e 层。)
    #[tokio::test]
    async fn open_creates_and_lists_empty() {
        let dir = tempfile::tempdir().unwrap();
        let dbx = Dbx::open_at(&dir.path().join("dbx.db")).await.unwrap();
        assert!(dbx.db_path.exists(), "open 应创建 dbx.db");
        let conns = dbx.connections().await.unwrap();
        assert!(conns.is_empty());
    }

    #[tokio::test]
    async fn missing_connection_reports_friendly() {
        let dir = tempfile::tempdir().unwrap();
        let dbx = Dbx::open_at(&dir.path().join("dbx.db")).await.unwrap();
        let err = dbx.connection_by_name("nope").await.unwrap_err();
        assert!(err.to_string().contains("在 DBX 中创建"), "{err}");
    }

    #[test]
    fn data_dir_respects_env() {
        let dir = tempfile::tempdir().unwrap();
        let key = "DBX_DATA_DIR";
        let old = std::env::var_os(key);
        unsafe { std::env::set_var(key, dir.path()) };
        assert_eq!(data_dir().unwrap(), dir.path());
        unsafe {
            match old {
                Some(v) => std::env::set_var(key, v),
                None => std::env::remove_var(key),
            }
        }
    }
}
