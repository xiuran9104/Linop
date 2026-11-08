//! Linop 数据库面板(DBX 内核,GPUI 原生)。

pub mod panel;

use std::path::PathBuf;
use std::sync::Arc;

use domain_api::{ConnectionOpener, Domain, DomainId, DomainRegistry, HomeContext};
use gpui::{App, AppContext};
use linop_core::DbRegistry;
use one_core::storage::ConnectionType;

/// 共享的 DBX 运行时(连接池由 DBX 管理)。
pub struct DbxRuntime(pub Arc<linop_dbx::Dbx>);

/// 技术存储路径(与 linop-dbx CLI 一致)。
pub fn default_store_path() -> PathBuf {
    match std::env::var_os("HOME") {
        Some(home) => PathBuf::from(home)
            .join(".linop-dev")
            .join("linop-db-data")
            .join("dbx.db"),
        None => PathBuf::from(".linop-dev").join("linop-db-data").join("dbx.db"),
    }
}

impl DbxRuntime {
    /// 以给定连接清单打开运行时(连接配置归 Linop 的 databases.toml)。
    pub async fn open_with(
        store: PathBuf,
        profiles: Vec<linop_core::DbProfile>,
    ) -> anyhow::Result<Self> {
        let dbx = linop_dbx::Dbx::open(&store, profiles)
            .await
            .map_err(|e| anyhow::anyhow!("{}", e))?;
        Ok(Self(Arc::new(dbx)))
    }

    /// 读取默认 databases.toml 并打开。
    pub async fn open_default() -> anyhow::Result<(Self, Vec<linop_core::DbProfile>)> {
        let profiles = DbRegistry::load_default()
            .map_err(|e| anyhow::anyhow!("{}", e))?
            .profiles()
            .to_vec();
        let runtime = Self::open_with(default_store_path(), profiles.clone()).await?;
        Ok((runtime, profiles))
    }
}

/// 数据域:注册 Database / Redis 连接的打开入口(视图 = DBX 面板)。
pub struct DbxDomain;

impl Domain for DbxDomain {
    fn id(&self) -> DomainId {
        DomainId::DATABASE
    }

    fn init(&self, _cx: &mut gpui::App) -> anyhow::Result<()> {
        Ok(())
    }

    fn register(&self, registry: &mut DomainRegistry) {
        registry.register_connection_opener(
            ConnectionType::Database,
            std::sync::Arc::new(DbxConnectionOpener),
        );
        registry.register_connection_opener(
            ConnectionType::Redis,
            std::sync::Arc::new(DbxRedisConnectionOpener),
        );
    }
}

struct DbxConnectionOpener;

impl ConnectionOpener for DbxConnectionOpener {
    fn open(
        &self,
        connection: &one_core::storage::StoredConnection,
        mode: one_core::tab_container::TabOpenMode,
        home: &mut dyn domain_api::HomeContext,
        window: &mut gpui::Window,
        cx: &mut gpui::App,
    ) -> bool {
        if connection.connection_type != ConnectionType::Database {
            return false;
        }
        open_panel(mode, home, window, cx, "sql")
    }
}

struct DbxRedisConnectionOpener;

impl ConnectionOpener for DbxRedisConnectionOpener {
    fn open(
        &self,
        connection: &one_core::storage::StoredConnection,
        mode: one_core::tab_container::TabOpenMode,
        home: &mut dyn domain_api::HomeContext,
        window: &mut gpui::Window,
        cx: &mut gpui::App,
    ) -> bool {
        if connection.connection_type != ConnectionType::Redis {
            return false;
        }
        open_panel(mode, home, window, cx, "redis")
    }
}

fn open_panel(
    mode: one_core::tab_container::TabOpenMode,
    home: &mut dyn domain_api::HomeContext,
    window: &mut gpui::Window,
    cx: &mut gpui::App,
    kind: &'static str,
) -> bool {
    let tab_id = format!("linop-db-panel-{kind}");
    let item_id = tab_id.clone();
    home.open_tab_lazy(
        tab_id,
        mode,
        window,
        cx,
        Box::new(move |window, cx| {
            let view = cx.new(|cx| panel::DbxPanel::new(kind, window, cx));
            one_core::tab_container::TabItem::from_dyn(item_id, "dbx", std::sync::Arc::new(view))
        }),
    );
    true
}

