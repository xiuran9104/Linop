//! linop-dbx:数据库模块 —— DBX 内核(linop-db-core)作为**驱动执行层**。
//!
//! 设计(用户决策:DBX 原始前端/存储不要,改 GPUI 原生):
//! - 连接配置归 Linop 自己管(`databases.toml`,见 linop-core::DbProfile);
//! - `AppState` 仅作技术性运行时(连接池/超时/风控/取消 —— DBX 的执行
//!   路径全部复用),放在 Linop 数据目录下的空 `dbx.db` 上,不承载
//!   DBX 的任何业务数据;连接在会话内以内存方式注入 `configs`;
//! - 查询走 `linop_db_core::query::execute_sql_statement`,表元数据走
//!   `list_tables_core` —— 与 DBX 桌面版同一条执行代码。

use std::path::{Path, PathBuf};

use linop_db_core::connection::AppState;
use linop_db_core::persistence::storage::Storage;
use linop_db_types::models::connection::ConnectionConfig;

#[derive(Debug, thiserror::Error)]
pub enum DbxError {
    #[error("dbx: {0}")]
    Dbx(String),
    #[error("io: {0}")]
    Io(#[from] std::io::Error),
    #[error("json: {0}")]
    Json(#[from] serde_json::Error),
    #[error("unknown connection: {0}")]
    UnknownConnection(String),
    #[error("unsupported driver: {0}")]
    UnsupportedDriver(String),
    #[error("无法解析 HOME 目录(设置 HOME 环境变量)")]
    NoDataDir,
}

pub type Result<T> = std::result::Result<T, DbxError>;

/// 连接摘要。
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct DbxConnectionSummary {
    pub name: String,
    /// 驱动标识字符串(如 "postgres"、"mysql";DbDriver 未实现 Serialize)。
    pub driver: String,
    pub host: String,
    pub port: u16,
    pub database: Option<String>,
}

/// DBX 执行运行时。**非 Send 友好**:AppState 持 tokio 资源,
/// 使用时保持在创建它的 runtime 上(或 Arc 化后跨 await 共享)。
pub struct Dbx {
    state: std::sync::Arc<tokio::sync::Mutex<AppState>>,
    /// 连接配置(Linop databases.toml 注入的镜像)。
    profiles: tokio::sync::Mutex<Vec<linop_core::DbProfile>>,
    data_dir: PathBuf,
}

/// Linop 数据目录下的 DBX 技术存储路径。
pub fn default_store_path() -> Result<PathBuf> {
    let home = std::env::var_os("HOME")
        .map(PathBuf::from)
        .ok_or(DbxError::NoDataDir)?;
    Ok(home.join(".linop-dev").join("linop-db-data").join("dbx.db"))
}

impl Dbx {
    /// 打开运行时:`store_path` 是技术性 dbx.db(空库即可);
    /// `profiles` 来自 linop-core 的 databases.toml,会话内注入。
    pub async fn open(store_path: &Path, profiles: Vec<linop_core::DbProfile>) -> Result<Self> {
        if let Some(parent) = store_path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let storage = Storage::open_unmigrated(store_path)
            .await
            .map_err(DbxError::Dbx)?;
        let state = std::sync::Arc::new(tokio::sync::Mutex::new(AppState::new(storage)));
        // 核心方言注册(linop-db-web 同款全局初始化)。
        linop_db_core::sql_dialect::dialect_loader::register_core_dialects();
        for profile in &profiles {
            Self::inject(&state, profile).await?;
        }
        Ok(Self {
            state,
            profiles: tokio::sync::Mutex::new(profiles),
            data_dir: store_path.to_path_buf(),
        })
    }

    /// 把 Linop profile 构造成 DBX ConnectionConfig 并注入运行时。
    async fn inject(state: &std::sync::Arc<tokio::sync::Mutex<AppState>>, profile: &linop_core::DbProfile) -> Result<()> {
        let config: ConnectionConfig = serde_json::from_value(serde_json::json!({
            "id": format!("linop:{}", profile.name),
            "name": profile.name,
            "db_type": profile.driver.as_str(),
            "host": profile.host.clone().unwrap_or_default(),
            "port": profile.port.unwrap_or(match profile.driver {
                linop_core::DbDriver::Postgres => 5432,
                linop_core::DbDriver::MySql => 3306,
                linop_core::DbDriver::Redis => 6379,
                linop_core::DbDriver::Sqlite => 0,
            }),
            "username": profile.user.clone().unwrap_or_default(),
            "password": profile.password.clone().unwrap_or_default(),
            "database": profile.database.clone().unwrap_or_default(),
            "ssl": false,
        }))?;
        state
            .lock()
            .await
            .configs
            .write()
            .await
            .insert(config.id.clone(), config);
        Ok(())
    }

    async fn connection_id(&self, name: &str) -> Result<String> {
        let guard = self.profiles.lock().await;
        let profile = guard
            .iter()
            .find(|p| p.name == name)
            .ok_or_else(|| DbxError::UnknownConnection(name.to_string()))?;
        Ok(format!("linop:{}", profile.name))
    }

    async fn database_of(&self, name: &str) -> String {
        self.profiles
            .lock()
            .await
            .iter()
            .find(|p| p.name == name)
            .and_then(|p| p.database.clone())
            .unwrap_or_default()
    }

    /// 连接摘要。
    pub async fn connections(&self) -> Vec<DbxConnectionSummary> {
        self.profiles
            .lock()
            .await
            .iter()
            .map(|p| DbxConnectionSummary {
                name: p.name.clone(),
                driver: p.driver.as_str().to_string(),
                host: p.host.clone().unwrap_or_default(),
                port: p.port.unwrap_or_default(),
                database: p.database.clone(),
            })
            .collect()
    }

    /// 会话内追加一个连接(不入库;连接配置持久化归 databases.toml)。
    pub async fn add_profile(&self, profile: linop_core::DbProfile) -> Result<()> {
        Self::inject(&self.state, &profile).await?;
        self.profiles.lock().await.push(profile);
        Ok(())
    }

    /// 表列表(走 DBX 驱动的元数据路径)。
    pub async fn list_tables(&self, name: &str) -> Result<Vec<linop_db_core::types::TableInfo>> {
        let id = self.connection_id(name).await?;
        let database = self.database_of(name).await;
        let guard = self.state.lock().await;
        linop_db_core::schema::list_tables_core(
            &guard,
            &id,
            &database,
            "",
            None,
            None,
            None,
            None,
            None,
        )
        .await
        .map_err(DbxError::Dbx)
    }

    /// 执行 SQL(默认只读风控、超时与取消都来自 DBX 执行路径)。
    pub async fn query(&self, name: &str, sql: &str) -> Result<linop_db_core::db::QueryResult> {
        let id = self.connection_id(name).await?;
        let database = self.database_of(name).await;
        let guard = self.state.lock().await;
        linop_db_core::query::execute_sql_statement(&guard, &id, &database, sql, None, None)
            .await
            .map_err(DbxError::Dbx)
    }

    /// 技术存储路径(诊断)。
    pub fn store_path(&self) -> &Path {
        &self.data_dir
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 空技术库冒烟:open 创建 dbx.db,连接列表来自注入的 profiles。
    #[tokio::test]
    async fn open_empty_store_lists_injected_profiles() {
        let dir = tempfile::tempdir().unwrap();
        let store = dir.path().join("dbx.db");
        let profiles = vec![linop_core::DbProfile::postgres(
            "pg",
            "127.0.0.1",
            5433,
            "app",
            Some("pw".into()),
            Some("orders".into()),
        )];
        let dbx = Dbx::open(&store, profiles).await.unwrap();
        assert!(store.exists(), "open 应创建技术性 dbx.db");
        let conns = dbx.connections().await;
        assert_eq!(conns.len(), 1);
        assert_eq!(conns[0].name, "pg");
        assert_eq!(conns[0].driver, "postgres");
        assert_eq!(conns[0].database.as_deref(), Some("orders"));
    }

    #[tokio::test]
    async fn unknown_connection_is_reported() {
        let dir = tempfile::tempdir().unwrap();
        let dbx = Dbx::open(&dir.path().join("dbx.db"), Vec::new())
            .await
            .unwrap();
        let err = dbx.connection_id("nope").await.unwrap_err();
        assert!(err.to_string().contains("nope"));
    }
}
