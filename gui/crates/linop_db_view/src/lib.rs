//! Linop 数据库面板(DBX 内核,GPUI 原生)。

rust_i18n::i18n!("locales", fallback = "en");

pub mod connection_form;
pub mod console_tab;
pub mod data_tab;
pub mod mcp;
pub mod panel;
pub mod structure_tab;

use std::path::PathBuf;
use std::sync::Arc;

use domain_api::{ConnectionOpener, Domain, DomainId, DomainRegistry};
use gpui::AppContext;
use linop_core::DbRegistry;
use one_core::storage::ConnectionType;

/// 进程级共享运行时:面板与 MCP 工具共用一套(via 隧道只建立一次)。
static SHARED_RUNTIME: tokio::sync::OnceCell<Arc<DbxRuntime>> = tokio::sync::OnceCell::const_new();

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
        let conns = linop_core::Registry::load_default()
            .map_err(|e| anyhow::anyhow!("connections.toml 加载失败(via 隧道需要):{e}"))?;
        let dbx = linop_dbx::Dbx::open(&store, profiles, &conns)
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

    /// 会话内追加连接(via 跳板从 connections.toml 解析)。
    pub async fn add_profile(&self, profile: linop_core::DbProfile) -> anyhow::Result<()> {
        let conns = linop_core::Registry::load_default()
            .map_err(|e| anyhow::anyhow!("connections.toml 加载失败(via 隧道需要):{e}"))?;
        self.0
            .add_profile(profile, &conns)
            .await
            .map_err(|e| anyhow::anyhow!("{}", e))
    }

    /// 进程级单例:databases.toml + connections.toml 打开,首个调用方初始化,
    /// 后续共享(via 隧道只建立一套)。初始化失败不缓存,下次调用重试;
    /// 面板与 MCP 工具都应经由 gpui_tokio 的 Tokio runtime 调用,
    /// 保证 DBX 资源(SSH 隧道/连接池)始终创建并轮询于同一 runtime。
    pub async fn shared() -> anyhow::Result<Arc<Self>> {
        Ok(SHARED_RUNTIME
            .get_or_try_init(|| async {
                let profiles = DbRegistry::load_default()
                    .map_err(|e| anyhow::anyhow!("{}", e))?
                    .profiles()
                    .to_vec();
                let runtime = Self::open_with(default_store_path(), profiles).await?;
                Ok::<_, anyhow::Error>(Arc::new(runtime))
            })
            .await?
            .clone())
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
        mcp::register_mcp_toolsets();
        registry.register_form_provider(Arc::new(connection_form::DbxConnectionFormProvider));
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
            let view = cx.new(|cx| panel::DbxPanel::new(window, cx));
            one_core::tab_container::TabItem::from_dyn(item_id, "dbx", std::sync::Arc::new(view))
        }),
    );
    true
}

