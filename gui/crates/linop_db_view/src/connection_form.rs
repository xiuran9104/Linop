//! 新建连接向导的数据库表单(kind = "database:<驱动>")。
//!
//! 连接配置归 Linop 的 databases.toml(DbProfile),不走 one-hub.db 的
//! StoredConnection;保存后注入共享 DbxRuntime(面板与 MCP 立即可用,
//! 带 via 跳板的连接此时建立隧道)。

use domain_api::{
    ConnectionFormProvider, ConnectionFormRequest, ConnectionKindDescriptor, ConnectionKindGroup,
};
use gpui::{AnyView, App, AppContext, Context, FocusHandle, Focusable, IntoElement, ParentElement,
    Render, SharedString, Styled, Window, div, prelude::FluentBuilder as _, px};
use gpui_component::{ActiveTheme, Disableable, Sizable, button::Button, h_flex, input::Input,
    input::InputState, v_flex};
use linop_core::{DbDriver, DbProfile, DbRegistry};
use one_core::gpui_tokio::Tokio;
use rust_i18n::t;

use crate::DbxRuntime;

/// 数据库连接种类 id 前缀(向导左栏「数据库」分组);后缀 = 驱动名。
pub(crate) const KIND_PREFIX: &str = "database:";

/// (驱动, i18n 标签键)按向导展示顺序。常规主机/文件连接型;
/// 云凭证型(dynamodb/cloudflare-d1/turso/hbase)字段语义不同,
/// 不上向导,可在 databases.toml 手写(driver 名可被 parse 识别)。
const KIND_ENTRIES: [(DbDriver, &'static str); 20] = [
    (DbDriver::Postgres, "postgres"),
    (DbDriver::MySql, "mysql"),
    (DbDriver::Sqlite, "sqlite"),
    (DbDriver::Redis, "redis"),
    (DbDriver::SqlServer, "sqlserver"),
    (DbDriver::Mongo, "mongodb"),
    (DbDriver::Elastic, "elasticsearch"),
    (DbDriver::DuckDb, "duckdb"),
    (DbDriver::ClickHouse, "clickhouse"),
    (DbDriver::Doris, "doris"),
    (DbDriver::StarRocks, "starrocks"),
    (DbDriver::ManticoreSearch, "manticoresearch"),
    (DbDriver::QuestDB, "questdb"),
    (DbDriver::InfluxDB, "influxdb"),
    (DbDriver::InfluxDB3, "influxdb3"),
    (DbDriver::VictoriaMetrics, "victoriametrics"),
    (DbDriver::Solr, "solr"),
    (DbDriver::Meilisearch, "meilisearch"),
    (DbDriver::EasySearch, "easysearch"),
    (DbDriver::CouchDB, "couchdb"),
];

fn kind_label_fn(key: &'static str) -> fn(&App) -> SharedString {
    match key {
        "postgres" => |_: &App| t!("ConnectionForm.kind.postgres").to_string().into(),
        "mysql" => |_: &App| t!("ConnectionForm.kind.mysql").to_string().into(),
        "sqlite" => |_: &App| t!("ConnectionForm.kind.sqlite").to_string().into(),
        "redis" => |_: &App| t!("ConnectionForm.kind.redis").to_string().into(),
        "sqlserver" => |_: &App| t!("ConnectionForm.kind.sqlserver").to_string().into(),
        "mongodb" => |_: &App| t!("ConnectionForm.kind.mongodb").to_string().into(),
        "elasticsearch" => |_: &App| t!("ConnectionForm.kind.elasticsearch").to_string().into(),
        "duckdb" => |_: &App| t!("ConnectionForm.kind.duckdb").to_string().into(),
        "clickhouse" => |_: &App| t!("ConnectionForm.kind.clickhouse").to_string().into(),
        "doris" => |_: &App| t!("ConnectionForm.kind.doris").to_string().into(),
        "starrocks" => |_: &App| t!("ConnectionForm.kind.starrocks").to_string().into(),
        "manticoresearch" => |_: &App| {
            t!("ConnectionForm.kind.manticoresearch").to_string().into()
        },
        "questdb" => |_: &App| t!("ConnectionForm.kind.questdb").to_string().into(),
        "influxdb" => |_: &App| t!("ConnectionForm.kind.influxdb").to_string().into(),
        "influxdb3" => |_: &App| t!("ConnectionForm.kind.influxdb3").to_string().into(),
        "victoriametrics" => |_: &App| {
            t!("ConnectionForm.kind.victoriametrics").to_string().into()
        },
        "solr" => |_: &App| t!("ConnectionForm.kind.solr").to_string().into(),
        "meilisearch" => |_: &App| t!("ConnectionForm.kind.meilisearch").to_string().into(),
        "easysearch" => |_: &App| t!("ConnectionForm.kind.easysearch").to_string().into(),
        _ => |_: &App| t!("ConnectionForm.kind.couchdb").to_string().into(),
    }
}

fn port_placeholder(driver: DbDriver) -> &'static str {
    match driver.default_port() {
        0 => "端口(文件型驱动留空)",
        _ => match driver {
            DbDriver::MySql => "端口(缺省 3306;MariaDB/Doris/StarRocks 同协议)",
            _ => "",
        },
    }
}

/// 数据库表单提供者:向「新建连接」逐条声明各驱动种类(DBX 同款列举)。
pub struct DbxConnectionFormProvider;

impl ConnectionFormProvider for DbxConnectionFormProvider {
    fn kinds(&self, _cx: &App) -> Vec<ConnectionKindDescriptor> {
        KIND_ENTRIES
            .iter()
            .enumerate()
            .map(|(order, (driver, label_key))| {
                ConnectionKindDescriptor::new(
                    format!("{KIND_PREFIX}{}", driver.as_str()),
                    kind_label_fn(label_key),
                    ConnectionKindGroup::Database,
                    order as i32,
                )
            })
            .collect()
    }

    fn build(
        &self,
        request: ConnectionFormRequest,
        window: &mut Window,
        cx: &mut App,
    ) -> Option<AnyView> {
        let driver = request
            .kind_id
            .as_ref()
            .strip_prefix(KIND_PREFIX)
            .and_then(DbDriver::parse)?;
        Some(cx.new(|cx| DbxConnectionForm::new(driver, window, cx)).into())
    }
}

struct DbxConnectionForm {
    focus: FocusHandle,
    name: gpui::Entity<InputState>,
    /// 由向导条目决定的固定驱动(表单内不可切换)。
    driver: DbDriver,
    host: gpui::Entity<InputState>,
    port: gpui::Entity<InputState>,
    /// SQLite 数据库文件路径(sqlite 驱动必填;其余驱动忽略)。
    path: gpui::Entity<InputState>,
    user: gpui::Entity<InputState>,
    password: gpui::Entity<InputState>,
    database: gpui::Entity<InputState>,
    /// 经由哪条 SSH 跳板(connections.toml 的连接名);None = 直连。
    via: Option<String>,
    via_candidates: Vec<String>,
    /// 出站代理 URL(http/socks5);None = 直连。
    proxy: gpui::Entity<InputState>,
    /// 连接测试进行中。
    testing: bool,
    status: Option<String>,
    saving: bool,
}

impl DbxConnectionForm {
    fn new(driver: DbDriver, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let mk = |placeholder: &'static str, window: &mut Window, cx: &mut Context<Self>| {
            cx.new(|cx| InputState::new(window, cx).placeholder(placeholder))
        };
        let via_candidates = linop_core::Registry::load_default()
            .map(|reg| {
                reg.connections()
                    .iter()
                    .map(|c| c.name.clone())
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        Self {
            focus: cx.focus_handle(),
            name: mk("名称(必填)", window, cx),
            driver,
            host: mk("主机(如 10.0.0.5)", window, cx),
            port: mk(port_placeholder(driver), window, cx),
            path: mk("/path/to/app.db(文件型驱动必填)", window, cx),
            user: mk("用户名", window, cx),
            password: mk("密码", window, cx),
            database: mk("库名", window, cx),
            via: None,
            via_candidates,
            proxy: mk("http://host:port 或 socks5://host:port(留空直连)", window, cx),
            status: None,
            saving: false,
            testing: false,
        }
    }

    /// 从表单读出 profile;校验失败给出提示。
    fn build_profile(
        &self,
        cx: &Context<Self>,
    ) -> Result<DbProfile, &'static str> {
        let read = |e: &gpui::Entity<InputState>| e.read(cx).value().trim().to_string();
        let name = read(&self.name);
        if name.is_empty() {
            return Err("名称必填");
        }
        let host = read(&self.host);
        let path = read(&self.path);
        if self.driver.is_file_based() && path.is_empty() {
            return Err("文件型驱动需要数据库文件路径");
        }
        if !self.driver.is_file_based() && host.is_empty() {
            return Err("主机必填");
        }
        Ok(DbProfile {
            name: name.clone(),
            driver: self.driver,
            path: path.into(),
            host: (!host.is_empty()).then_some(host),
            port: read(&self.port).parse().ok(),
            password: {
                let p = read(&self.password);
                (!p.is_empty()).then_some(p)
            },
            user: {
                let u = read(&self.user);
                (!u.is_empty()).then_some(u)
            },
            database: {
                let d = read(&self.database);
                (!d.is_empty()).then_some(d)
            },
            via: self.via.clone(),
            proxy: {
                let p = read(&self.proxy);
                (!p.is_empty()).then_some(p)
            },
        })
    }

    fn test_connection(&mut self, cx: &mut Context<Self>) {
        let profile = match self.build_profile(cx) {
            Ok(p) => p,
            Err(msg) => {
                self.status = Some(msg.to_string());
                cx.notify();
                return;
            }
        };
        self.testing = true;
        self.status = Some("测试中…".into());
        cx.notify();
        let task = Tokio::spawn(cx, async move {
            let conns = linop_core::Registry::load_default().unwrap_or_default();
            linop_dbx::Dbx::test_profile(profile, &conns).await
        });
        cx.spawn(async move |this, cx| {
            let outcome = task.await;
            let _ = this.update(cx, |this, cx| {
                this.testing = false;
                this.status = Some(match outcome {
                    Ok(Ok(msg)) => format!("✓ {msg}"),
                    Ok(Err(e)) => format!("✗ {e}"),
                    Err(join) => format!("✗ 测试任务失败:{join}"),
                });
                cx.notify();
            });
        })
        .detach();
    }

    fn save(&mut self, _window: &mut Window, cx: &mut Context<Self>) {
        let profile = match self.build_profile(cx) {
            Ok(p) => p,
            Err(msg) => {
                self.status = Some(msg.to_string());
                cx.notify();
                return;
            }
        };
        let name = profile.name.clone();

        // 持久化到 databases.toml。
        let mut registry = DbRegistry::load_default().unwrap_or_default();
        registry.upsert(profile.clone());
        if let Err(e) = registry.save(linop_core::default_databases_path()) {
            self.status = Some(format!("保存失败:{e}"));
            cx.notify();
            return;
        }

        // 注入共享运行时(via 跳板此时建立隧道)。
        self.saving = true;
        self.status = Some("保存中…".into());
        cx.notify();
        let task = Tokio::spawn(cx, async move {
            let runtime = DbxRuntime::shared().await?;
            runtime.add_profile(profile).await?;
            Ok::<(), anyhow::Error>(())
        });
        cx.spawn(async move |this, cx| {
            let outcome = task.await;
            let _ = this.update_in(cx, |form, window, cx| {
                form.saving = false;
                match outcome {
                    Ok(Ok(())) => {
                        let _ =
                            one_core::window_close::close_window_after_save(window, cx);
                    }
                    Ok(Err(e)) => {
                        form.status = Some(format!("已写入 databases.toml,注入运行时失败:{e}"));
                        cx.notify();
                    }
                    Err(join) => {
                        form.status = Some(format!("注入任务失败:{join}"));
                        cx.notify();
                    }
                }
            });
        })
        .detach();
    }

}

fn form_row(label: &str, child: impl IntoElement) -> gpui::Div {
    h_flex()
        .gap_3()
        .items_center()
        .child(
            div()
                .w(px(110.0))
                .text_sm()
                .text_right()
                .child(label.to_string()),
        )
        .child(div().flex_1().child(child))
}


impl Focusable for DbxConnectionForm {
    fn focus_handle(&self, _cx: &App) -> FocusHandle {
        self.focus.clone()
    }
}

impl Render for DbxConnectionForm {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme().clone();
        let field = |input: &gpui::Entity<InputState>| Input::new(input).small().flex_1();

        let driver_label = self.driver.as_str();

        let mut via_row = h_flex().flex_1().flex_wrap().gap_1();
        via_row = via_row.child(
            Button::new("via-direct")
                .label("直连")
                .small()
                .when(self.via.is_none(), |b| {
                    b.bg(theme.colors.accent)
                        .text_color(theme.colors.accent_foreground)
                })
                .on_click(cx.listener(|form, _ev, _window, cx| {
                    form.via = None;
                    cx.notify();
                })),
        );
        for candidate in self.via_candidates.clone() {
            let selected = self.via.as_deref() == Some(candidate.as_str());
            via_row = via_row.child(
                Button::new(SharedString::from(format!("via-{candidate}")))
                    .label(candidate.clone())
                    .small()
                    .when(selected, |b| {
                        b.bg(theme.colors.accent)
                            .text_color(theme.colors.accent_foreground)
                    })
                    .on_click(cx.listener(move |form, _ev, _window, cx| {
                        form.via = Some(candidate.clone());
                        cx.notify();
                    })),
            );
        }

        v_flex()
            .size_full()
            .p_4()
            .gap_3()
            .child(form_row("名称", field(&self.name)))
            .child(form_row("驱动", div().text_sm().child(driver_label)))
            .when(self.driver.is_mysql_family(), |col| {
                col.child(form_row(
                    "",
                    div()
                        .text_xs()
                        .text_color(theme.colors.muted_foreground)
                        .child(t!("DbxFormHint.mysql_family").to_string()),
                ))
            })
            .child(
                form_row(
                    "SQLite 文件",
                    h_flex()
                        .flex_1()
                        .items_center()
                        .gap_2()
                        .child(field(&self.path))
                        .child(
                            div()
                                .text_xs()
                                .text_color(theme.colors.muted_foreground)
                                .child("文件型驱动(sqlite/duckdb)必填"),
                        ),
                ),
            )
            .child(form_row("主机", field(&self.host)))
            .child(form_row("端口", field(&self.port)))
            .child(form_row("用户名", field(&self.user)))
            .child(form_row("密码", field(&self.password)))
            .child(form_row("库名", field(&self.database)))
            .child(form_row("代理", field(&self.proxy)))
            .child(form_row("SSH 跳板(via)", via_row))
            .when(self.status.is_some(), |col| {
                col.child(
                    div()
                        .text_sm()
                        .text_color(theme.colors.danger_foreground)
                        .child(self.status.clone().unwrap_or_default()),
                )
            })
            .child(
                h_flex()
                    .justify_end()
                    .gap_2()
                    .child(
                        Button::new("dbx-form-cancel")
                            .label(t!("DbxForm.cancel").to_string())
                            .small()
                            .on_click(cx.listener(|_, _ev, window, cx| {
                                let _ = one_core::window_close::close_window_for_reuse(window, cx);
                            })),
                    )
                    .child(
                        Button::new("dbx-form-test")
                            .label(if self.testing { "测试中…" } else { "测试连接" })
                            .small()
                            .disabled(self.testing || self.saving)
                            .on_click(cx.listener(|form, _ev, _window, cx| {
                                form.test_connection(cx);
                            })),
                    )
                    .child(
                        Button::new("dbx-form-save")
                            .label(t!("DbxForm.save").to_string())
                            .small()
                            .disabled(self.saving)
                            .on_click(cx.listener(|form, _ev, window, cx| {
                                form.save(window, cx);
                            })),
                    ),
            )
    }
}
