use gpui::{App, AssetSource};
use std::sync::Arc;

/// 产品身份。三产品只在各自的 `Product` 实现里提供不同值。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AppIdentity {
    pub app_id: &'static str,
    pub display_name: &'static str,
    pub bundle_id: &'static str,
    pub binary_name: &'static str,
    pub data_dir_name: &'static str,
    pub development_data_dir_name: &'static str,
    pub legacy_data_dir_names: &'static [&'static str],
    pub keychain_service: &'static str,
    pub development_keychain_service: &'static str,
    pub dir_migration_marker: &'static str,
    pub portable_marker: &'static str,
    pub mcp_app_name: &'static str,
    pub mcp_legacy_app_name: Option<&'static str>,
    pub mcp_update_channel: &'static str,
    pub file_associations: &'static [&'static str],
}

impl AppIdentity {
    /// 全量版沿用既有身份，避免老用户迁移数据目录与 keychain。
    pub const FULL: Self = Self {
        app_id: "linop",
        display_name: "Linop",
        bundle_id: "com.onetcli.app",
        binary_name: "linop",
        data_dir_name: "linop",
        development_data_dir_name: ".linop-dev",
        legacy_data_dir_names: &["one-hub"],
        keychain_service: "com.linop.desktop",
        development_keychain_service: "com.linop.desktop.dev",
        dir_migration_marker: ".one-hub-migration-complete",
        portable_marker: "linop.portable",
        mcp_app_name: "linop",
        mcp_legacy_app_name: Some("onetcli"),
        mcp_update_channel: "stable",
        file_associations: &["db", "duckdb", "md", "cast"],
    };

    pub const SHELL: Self = Self {
        app_id: "linop-shell",
        display_name: "Linop Shell",
        bundle_id: "com.linop.shell",
        binary_name: "linop-shell",
        data_dir_name: "linop-shell",
        development_data_dir_name: ".linop-shell-dev",
        legacy_data_dir_names: &[],
        keychain_service: "com.linop.desktop.shell",
        development_keychain_service: "com.linop.desktop.shell.dev",
        dir_migration_marker: ".linop-shell-migration-complete",
        portable_marker: "linop-shell.portable",
        mcp_app_name: "linop-shell",
        mcp_legacy_app_name: None,
        mcp_update_channel: "shell",
        file_associations: &[],
    };

    pub const DATA: Self = Self {
        app_id: "linop-data",
        display_name: "Linop Data",
        bundle_id: "com.linop.data",
        binary_name: "linop-data",
        data_dir_name: "linop-data",
        development_data_dir_name: ".linop-data-dev",
        legacy_data_dir_names: &[],
        keychain_service: "com.linop.desktop.data",
        development_keychain_service: "com.linop.desktop.data.dev",
        dir_migration_marker: ".linop-data-migration-complete",
        portable_marker: "linop-data.portable",
        mcp_app_name: "linop-data",
        mcp_legacy_app_name: None,
        mcp_update_channel: "data",
        file_associations: &[],
    };
}

/// 与具体领域无关的产品级功能开关；领域裁剪由 `Product::domains` 决定。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ProductFeatures {
    pub ai: bool,
    pub notes: bool,
    pub team: bool,
    pub cloud_sync: bool,
    pub credential_vault: bool,
    pub toolbox: bool,
    pub public_mcp: bool,
}

impl ProductFeatures {
    pub const FULL: Self = Self {
        ai: true,
        notes: true,
        team: true,
        cloud_sync: true,
        credential_vault: true,
        toolbox: true,
        public_mcp: true,
    };

    pub const SHELL: Self = Self { ..Self::FULL };

    pub const DATA: Self = Self { ..Self::FULL };
}

/// 一个产品由身份、功能开关、资产与领域列表组成。
pub trait Product: 'static {
    type Assets: AssetSource;

    fn identity(&self) -> &'static AppIdentity;

    fn features(&self) -> ProductFeatures;

    fn assets(&self) -> Self::Assets;

    fn domains(&self) -> Vec<Arc<dyn super::Domain>>;

    /// 全量产品由 main 自己的 `public_mcp_runtime` 管理 MCP；独立产品由
    /// `app_kit` 统一管理。
    fn mcp_managed_by_app_kit(&self) -> bool {
        true
    }

    fn init(&self, cx: &mut App) -> anyhow::Result<()> {
        for domain in self.domains() {
            domain.init(cx)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn three_products_have_distinct_identities() {
        assert_ne!(AppIdentity::FULL.app_id, AppIdentity::SHELL.app_id);
        assert_ne!(AppIdentity::FULL.bundle_id, AppIdentity::DATA.bundle_id);
        assert_ne!(
            AppIdentity::SHELL.binary_name,
            AppIdentity::DATA.binary_name
        );
        assert_eq!(AppIdentity::FULL.data_dir_name, "linop");
    }

    #[test]
    fn standalone_products_support_public_mcp() {
        assert!(ProductFeatures::FULL.public_mcp);
        assert!(ProductFeatures::SHELL.public_mcp);
        assert!(ProductFeatures::DATA.public_mcp);
        assert!(ProductFeatures::SHELL.ai);
        assert!(ProductFeatures::DATA.cloud_sync);
    }
}
