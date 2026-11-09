//! 新建连接向导的数据库表单(kind = "database")。
//!
//! 连接配置归 Linop 的 databases.toml(DbProfile),不走 one-hub.db 的
//! StoredConnection;保存后注入共享 DbxRuntime(面板与 MCP 立即可用,
//! 带 via 跳板的连接此时建立隧道)。

use std::sync::Arc;

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

/// 数据库连接种类 id(向导左栏「数据库」分组)。
pub(crate) const KIND_DATABASE: &str = "database";

fn database_kind_label(_cx: &App) -> SharedString {
    t!("ConnectionForm.kind.database").to_string().into()
}

/// 数据库表单提供者:向「新建连接」声明数据库种类并构造表单。
pub struct DbxConnectionFormProvider;

impl ConnectionFormProvider for DbxConnectionFormProvider {
    fn kinds(&self, _cx: &App) -> Vec<ConnectionKindDescriptor> {
        vec![ConnectionKindDescriptor::new(
            KIND_DATABASE,
            database_kind_label,
            ConnectionKindGroup::Database,
            0,
        )]
    }

    fn build(
        &self,
        request: ConnectionFormRequest,
        window: &mut Window,
        cx: &mut App,
    ) -> Option<AnyView> {
        if request.kind_id.as_ref() != KIND_DATABASE {
            return None;
        }
        Some(cx.new(|cx| DbxConnectionForm::new(window, cx)).into())
    }
}

struct DbxConnectionForm {
    focus: FocusHandle,
    name: gpui::Entity<InputState>,
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
    status: Option<String>,
    saving: bool,
}

const DRIVERS: [DbDriver; 4] = [
    DbDriver::Postgres,
    DbDriver::MySql,
    DbDriver::Sqlite,
    DbDriver::Redis,
];

impl DbxConnectionForm {
    fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
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
            driver: DbDriver::Postgres,
            host: mk("主机(如 10.0.0.5)", window, cx),
            port: mk("端口(缺省 5432/3306)", window, cx),
            path: mk("/root/data/app.db(sqlite 必填)", window, cx),
            user: mk("用户名", window, cx),
            password: mk("密码", window, cx),
            database: mk("库名", window, cx),
            via: None,
            via_candidates,
            status: None,
            saving: false,
        }
    }

    fn save(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let read = |e: &gpui::Entity<InputState>| e.read(cx).value().trim().to_string();
        let name = read(&self.name);
        if name.is_empty() {
            self.status = Some("名称必填".into());
            cx.notify();
            return;
        }
        let host = read(&self.host);
        let path = read(&self.path);
        if self.driver == DbDriver::Sqlite && path.is_empty() {
            self.status = Some("SQLite 需要数据库文件路径".into());
            cx.notify();
            return;
        }
        if self.driver != DbDriver::Sqlite && host.is_empty() {
            self.status = Some("主机必填".into());
            cx.notify();
            return;
        }
        let profile = DbProfile {
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
        };

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

        let driver_buttons = DRIVERS.iter().map(|driver| {
            let selected = self.driver == *driver;
            let label = driver.as_str();
            let driver = *driver;
            Button::new(SharedString::from(format!("driver-{label}")))
                .label(label)
                .small()
                .when(selected, |b| {
                    b.bg(theme.colors.accent)
                        .text_color(theme.colors.accent_foreground)
                })
                .on_click(cx.listener(move |form, _ev, _window, cx| {
                    form.driver = driver;
                    cx.notify();
                }))
        });

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
            .child(
                form_row(
                    "驱动",
                    h_flex().flex_1().gap_1().children(driver_buttons),
                ),
            )
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
                                .child("仅 sqlite;其余驱动忽略"),
                        ),
                ),
            )
            .child(form_row("主机", field(&self.host)))
            .child(form_row("端口", field(&self.port)))
            .child(form_row("用户名", field(&self.user)))
            .child(form_row("密码", field(&self.password)))
            .child(form_row("库名", field(&self.database)))
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
