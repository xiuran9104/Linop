#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

//! `linop`：Full。
//!
//! 应用 crate 只负责**装配**：选定产品身份、领域包与初始化流程；主页、设置、
//! 标签、AI、Notes、团队、扩展、凭据库、云同步、托盘、更新等能力全部来自共享
//! 外壳 crate `linop_app`。

use std::sync::Arc;

use domain_api::{AppIdentity, Domain, Product, ProductFeatures};

struct FullProduct;

impl Product for FullProduct {
    type Assets = linop_app::AppAssets;

    fn identity(&self) -> &'static AppIdentity {
        &AppIdentity::FULL
    }

    fn features(&self) -> ProductFeatures {
        ProductFeatures::FULL
    }

    fn assets(&self) -> Self::Assets {
        linop_app::AppAssets::new()
    }

    fn domains(&self) -> Vec<Arc<dyn Domain>> {
        vec![
            Arc::new(shell_domain::ShellDomain),
        ]
    }

    fn mcp_managed_by_app_kit(&self) -> bool {
        false
    }

    fn init(&self, cx: &mut gpui::App) -> anyhow::Result<()> {
        linop_app::init_app(cx)
    }
}

fn main() {
    linop_app::run_with(FullProduct, env!("CARGO_PKG_VERSION"));
}
