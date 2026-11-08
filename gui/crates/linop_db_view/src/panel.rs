//! 数据库面板视图:连接选择 + 表列表 + SQL 执行 + 结果网格(EditTable)。

use std::sync::Arc;

use gpui::{
    AppContext, Context, FocusHandle, Focusable, InteractiveElement, IntoElement, ParentElement,
    Render, SharedString, StatefulInteractiveElement, Styled, Window, div,
    prelude::FluentBuilder as _, px,
};
use gpui_component::InteractiveElementExt as _;
use gpui_component::{
    ActiveTheme, Disableable, Sizable, StyledExt as _, button::Button, h_flex, input::Input,
    input::InputState, v_flex,
};
use linop_core::DbProfile;
use one_core::gpui_tokio::Tokio;
use one_ui::{Column, EditTableDelegate, EditTableState};
use one_core::tab_container::{TabContent, TabContentEvent, TabOpenMode};
use gpui::EventEmitter;

use crate::{DbxRuntime, default_store_path};

pub struct DbxPanel {
    kind: &'static str,
    focus: FocusHandle,
    runtime: Option<Arc<DbxRuntime>>,
    profiles: Vec<DbProfile>,
    selected: Option<String>,
    tables: Vec<TableInfoRow>,
    selected_table: Option<String>,
    sql_input: gpui::Entity<InputState>,
    /// 查询结果网格(查询成功后重建)。
    result_table: Option<gpui::Entity<EditTableState<ResultGridDelegate>>>,
    /// 新建连接表单(点「新建」后 Some)。
    form: Option<ConnectionForm>,
    status: String,
    loading: bool,
}

#[derive(Clone)]
pub struct TableInfoRow {
    pub name: String,
}

/// 查询结果网格的委托:只读展示。
pub struct ResultGridDelegate {
    columns: Vec<String>,
    rows: Arc<Vec<Vec<String>>>,
}

impl EditTableDelegate for ResultGridDelegate {
    fn columns_count(&self, _cx: &gpui::App) -> usize {
        self.columns.len()
    }

    fn rows_count(&self, _cx: &gpui::App) -> usize {
        self.rows.len()
    }

    fn column(&self, col_ix: usize, _cx: &gpui::App) -> Column {
        let name = self
            .columns
            .get(col_ix)
            .cloned()
            .unwrap_or_else(|| format!("col{col_ix}"));
        Column::new(name.clone(), name)
    }

    fn render_td(
        &mut self,
        row_ix: usize,
        col_ix: usize,
        _window: &mut Window,
        _cx: &mut gpui::Context<EditTableState<Self>>,
    ) -> impl gpui::IntoElement {
        let text = self
            .rows
            .get(row_ix)
            .and_then(|row| row.get(col_ix))
            .cloned()
            .unwrap_or_default();
        div()
            .px_2()
            .text_ellipsis()
            .overflow_hidden()
            .child(text)
    }
}

/// 新建连接表单的字段集(v1:文本输入,driver 用 postgres/mysql/sqlite/redis)。
pub struct ConnectionForm {
    pub name: gpui::Entity<InputState>,
    pub driver: gpui::Entity<InputState>,
    pub host: gpui::Entity<InputState>,
    pub port: gpui::Entity<InputState>,
    pub user: gpui::Entity<InputState>,
    pub password: gpui::Entity<InputState>,
    pub database: gpui::Entity<InputState>,
}

impl ConnectionForm {
    fn new(window: &mut Window, cx: &mut Context<DbxPanel>) -> Self {
        let mk = |placeholder: &'static str, window: &mut Window, cx: &mut Context<DbxPanel>| {
            cx.new(|cx| InputState::new(window, cx).placeholder(placeholder))
        };
        Self {
            name: mk("名称", window, cx),
            driver: mk("驱动(postgres/mysql/sqlite/redis)", window, cx),
            host: mk("主机", window, cx),
            port: mk("端口(缺省见驱动)", window, cx),
            user: mk("用户名", window, cx),
            password: mk("密码", window, cx),
            database: mk("库名", window, cx),
        }
    }
}

impl EventEmitter<TabContentEvent> for DbxPanel {}

impl TabContent for DbxPanel {
    fn content_key(&self) -> &'static str {
        "DbxPanel"
    }

    fn title(&self, _cx: &gpui::App) -> SharedString {
        match self.kind {
            "redis" => "数据库 · Redis(DBX)".into(),
            _ => "数据库 · SQL(DBX)".into(),
        }
    }

    fn icon(&self, _cx: &gpui::App) -> Option<gpui_component::Icon> {
        None
    }
}

impl Focusable for DbxPanel {
    fn focus_handle(&self, cx: &gpui::App) -> gpui::FocusHandle {
        self.focus.clone()
    }
}

impl DbxPanel {
    /// 打开面板:后台加载运行时与连接清单。
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let sql_input =
            cx.new(|cx| InputState::new(window, cx).placeholder("SQL(Enter 执行)"));
        let mut this = Self {
            kind: "sql",
            focus: cx.focus_handle(),
            runtime: None,
            profiles: Vec::new(),
            selected: None,
            tables: Vec::new(),
            selected_table: None,
            sql_input,
            result_table: None,
            form: None,
            status: "正在打开数据库运行时…".into(),
            loading: true,
        };
        cx.spawn(async move |this, cx| {
            let loaded = load_runtime().await;
            let _ = this.update(cx, |panel, cx| match loaded {
                Ok((runtime, profiles)) => {
                    panel.loading = false;
                    panel.status = if profiles.is_empty() {
                        "databases.toml 中没有连接配置,点「新建连接」添加".into()
                    } else {
                        format!("已就绪:{} 个连接", profiles.len())
                    };
                    panel.selected = profiles.first().map(|p| p.name.clone());
                    panel.profiles = profiles;
                    panel.runtime = Some(runtime);
                    cx.notify();
                }
                Err(e) => {
                    panel.loading = false;
                    panel.status = format!("初始化失败:{e}");
                    cx.notify();
                }
            });
        })
        .detach();
        this
    }

    fn refresh_tables(&mut self, cx: &mut Context<Self>) {
        let Some(runtime) = self.runtime.clone() else {
            return;
        };
        let Some(name) = self.selected.clone() else {
            self.status = "先选择一个连接".into();
            cx.notify();
            return;
        };
        self.loading = true;
        self.status = "刷新表列表…".into();
        cx.notify();
        let task = Tokio::spawn(cx, async move { runtime.0.list_tables(&name).await });
        cx.spawn(async move |this, cx| {
            let outcome = task.await;
            let _ = this.update(cx, |panel, cx| {
                panel.loading = false;
                match outcome {
                    Ok(Ok(mut tables)) => {
                        tables.sort_by(|a, b| a.name.cmp(&b.name));
                        panel.status = format!("{} 张表/视图", tables.len());
                        panel.tables = tables
                            .into_iter()
                            .map(|t| TableInfoRow { name: t.name })
                            .collect();
                    }
                    Ok(Err(e)) => panel.status = format!("刷新失败:{e}"),
                    Err(e) => panel.status = format!("刷新失败:{e}"),
                }
                cx.notify();
            });
        })
        .detach();
    }

    fn select_profile(&mut self, name: &str, cx: &mut Context<Self>) {
        self.selected = Some(name.to_string());
        self.refresh_tables(cx);
    }

    fn select_table(&mut self, name: &str, window: &mut Window, cx: &mut Context<Self>) {
        self.selected_table = Some(name.to_string());
        let sql = format!("SELECT * FROM {}", quote_table(name));
        self.sql_input.update(cx, |input, cx| input.set_value(sql, window, cx));
        self.run_sql(window, cx);
    }

    fn run_sql(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(runtime) = self.runtime.clone() else { return };
        let Some(name) = self.selected.clone() else { return };
        let sql = self.sql_input.read(cx).value().to_string();
        if sql.trim().is_empty() {
            return;
        }
        self.loading = true;
        self.status = "执行中…".into();
        cx.notify();
        let task = Tokio::spawn(cx, async move { runtime.0.query(&name, &sql).await });
        cx.spawn(async move |this, cx| {
            let outcome = task.await;
            let _ = this.update_in(cx, |panel, window, cx| {
                panel.loading = false;
                match outcome {
                    Ok(Ok(result)) => {
                        let rows: Vec<Vec<String>> = result
                            .rows
                            .iter()
                            .map(|row| {
                                row.iter()
                                    .map(|cell| match cell {
                                        serde_json::Value::Null => "NULL".into(),
                                        serde_json::Value::String(s) => s.clone(),
                                        other => other.to_string(),
                                    })
                                    .collect()
                            })
                            .collect();
                        let n = rows.len();
                        panel.status = if result.truncated {
                            format!("{n}+ 行(截断)")
                        } else {
                            format!("{n} 行")
                        };
                        panel.result_table = Some(cx.new(|cx| {
                            EditTableState::new(
                                ResultGridDelegate {
                                    columns: result.columns.clone(),
                                    rows: Arc::new(rows),
                                },
                                window,
                                cx,
                            )
                        }));
                    }
                    Ok(Err(e)) => panel.status = format!("查询失败:{e}"),
                    Err(e) => panel.status = format!("查询失败:{e}"),
                }
                cx.notify();
            });
        })
        .detach();
    }

    /// 保存连接表单:写 databases.toml + 会话内注入。
    fn save_form(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(form) = &self.form else { return };
        let read = |e: &gpui::Entity<InputState>| e.read(cx).value().trim().to_string();
        let (name, driver, host, port, user, password, database) = (
            read(&form.name),
            read(&form.driver),
            read(&form.host),
            read(&form.port),
            read(&form.user),
            read(&form.password),
            read(&form.database),
        );
        if name.is_empty() || driver.is_empty() {
            self.status = "名称与驱动必填".into();
            cx.notify();
            return;
        }
        let Some(driver) = linop_core::DbDriver::parse(&driver) else {
            self.status = "驱动仅支持 postgres / mysql / sqlite / redis".into();
            cx.notify();
            return;
        };
        let profile = linop_core::DbProfile {
            name: name.clone(),
            driver,
            path: Default::default(),
            host: (!host.is_empty()).then_some(host),
            port: port.parse().ok(),
            password: (!password.is_empty()).then_some(password),
            user: (!user.is_empty()).then_some(user),
            database: (!database.is_empty()).then_some(database),
            via: None,
        };

        // 持久化到 databases.toml。
        let mut registry = linop_core::DbRegistry::load_default().unwrap_or_default();
        registry.upsert(profile.clone());
        let store_path = linop_core::default_databases_path();
        if let Err(e) = registry.save(&store_path) {
            self.status = format!("保存失败:{e}");
            cx.notify();
            return;
        }

        // 注入运行时(共享 Arc,内部锁保证一致)。
        self.form = None;
        self.status = format!("已保存 {name}");
        let Some(runtime) = self.runtime.clone() else { return };
        let name2 = name.clone();
        let profile2 = profile.clone();
        let task = Tokio::spawn(cx, async move {
            runtime
                .0
                .add_profile(profile2)
                .await
                .map_err(|e| e.to_string())?;
            Ok::<(), String>(())
        });
        cx.spawn(async move |this, cx| {
            let outcome = task.await;
            let _ = this.update(cx, |panel, cx| match outcome {
                Ok(Ok(())) => {
                    panel.selected = Some(name);
                    panel.profiles.push(profile);
                    panel.status = "已保存并注入运行时".into();
                }
                Ok(Err(e)) => panel.status = format!("注入失败:{e}"),
                Err(e) => panel.status = format!("注入失败:{e}"),
            });
        })
        .detach();
    }
}

fn quote_table(name: &str) -> String {
    name.split('.')
        .map(|part| format!("\"{}\"", part.replace('"', "\"\"")))
        .collect::<Vec<_>>()
        .join(".")
}

/// 后台加载运行时与连接清单。
async fn load_runtime() -> anyhow::Result<(Arc<DbxRuntime>, Vec<DbProfile>)> {
    let profiles = linop_core::DbRegistry::load_default()
        .map_err(|e| anyhow::anyhow!("{}", e))?
        .profiles()
        .to_vec();
    let runtime = DbxRuntime::open_with(default_store_path(), profiles.clone()).await?;
    Ok((Arc::new(runtime), profiles))
}

impl Render for DbxPanel {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl gpui::IntoElement {
        let theme = cx.theme().clone();

        let connection_row = self.profiles.iter().map(|p| {
            let name = p.name.clone();
            let is_selected = self.selected.as_deref() == Some(name.as_str());
            div()
                .id(SharedString::from(format!("conn-{name}")))
                .cursor_pointer()
                .px_2()
                .py_1()
                .rounded_sm()
                .text_size(px(13.0))
                .when(is_selected, |style| {
                    style.bg(theme.colors.accent).text_color(theme.colors.accent_foreground)
                })
                .when(!is_selected, |style| style.hover(|s| s.bg(theme.colors.selection)))
                .child(format!("{} ({})", p.name, p.driver.as_str()))
                .on_click(cx.listener({
                    let name = name.clone();
                    move |panel, _ev, _window, cx| {
                        panel.select_profile(&name, cx);
                    }
                }))
        });

        let table_rows = self.tables.iter().map(|t| {
            let name = t.name.clone();
            let is_selected = self.selected_table.as_deref() == Some(name.as_str());
            div()
                .id(SharedString::from(format!("table-{name}")))
                .cursor_pointer()
                .px_2()
                .py_1()
                .rounded_sm()
                .text_size(px(13.0))
                .when(is_selected, |style| {
                    style.bg(theme.colors.accent).text_color(theme.colors.accent_foreground)
                })
                .when(!is_selected, |style| style.hover(|s| s.bg(theme.colors.selection)))
                .child(t.name.clone())
                .on_click(cx.listener({
                    let name = name.clone();
                    move |panel, _ev, window, cx| {
                        panel.select_table(&name, window, cx);
                    }
                }))
        });

        let form = self.form.as_ref().map(|form| {
            let field = |label: &'static str, input: &gpui::Entity<InputState>| {
                h_flex()
                    .gap_2()
                    .items_center()
                    .child(
                        div()
                            .w_24()
                            .text_size(px(13.0))
                            .text_color(theme.colors.muted_foreground)
                            .child(label),
                    )
                    .child(Input::new(input).small().flex_1())
            };
            v_flex()
                .gap_2()
                .p_2()
                .border_1()
                .border_color(theme.colors.border)
                .rounded_md()
                .child(field("名称", &form.name))
                .child(field("驱动", &form.driver))
                .child(field("主机", &form.host))
                .child(field("端口", &form.port))
                .child(field("用户名", &form.user))
                .child(field("密码", &form.password))
                .child(field("库名", &form.database))
                .child(
                    h_flex().gap_2().child(
                        Button::new("save-conn")
                            .label("保存连接")
                            .small()
                            .on_click(cx.listener(|panel, _ev, window, cx| {
                                panel.save_form(window, cx);
                            })),
                    ),
                )
        });

        let result_area = self
            .result_table
            .clone()
            .map(|table| div().size_full().child(table.clone()).into_any_element())
            .unwrap_or_else(|| {
                div()
                    .text_color(theme.colors.muted_foreground)
                    .child("在上方输入 SQL 后点击执行,或点击左侧表名预览数据".to_string())
                    .into_any_element()
            });

        v_flex()
            .size_full()
            .gap_2()
            .p_2()
            .child(
                h_flex()
                    .flex_wrap()
                    .gap_1()
                    .items_center()
                    .child(div().text_color(theme.colors.muted_foreground).child("连接:"))
                    .children(connection_row)
                    .child(
                        Button::new("new-conn")
                            .label("新建连接")
                            .small()
                            .on_click(cx.listener(|panel, _ev, window, cx| {
                                panel.form = Some(ConnectionForm::new(window, cx));
                                cx.notify();
                            })),
                    ),
            )
            .children(form)
            .child(
                h_flex()
                    .gap_2()
                    .flex_1()
                    .min_h_0()
                    .child(
                        div()
                            .w_1_4()
                            .h_full()
                            .overflow_hidden()
                            .child(v_flex().gap_1().children(table_rows)),
                    )
                    .child(
                        v_flex()
                            .flex_1()
                            .h_full()
                            .gap_2()
                            .child(
                                h_flex()
                                    .gap_2()
                                    .items_center()
                                    .child(Input::new(&self.sql_input).small().flex_1())
                                    .child(
                                        Button::new("run")
                                            .label("执行")
                                            .small()
                                            .disabled(self.loading)
                                            .on_click(cx.listener(|panel, _ev, window, cx| {
                                                panel.run_sql(window, cx);
                                            })),
                                    )
                                    .child(
                                        Button::new("refresh")
                                            .label("刷新表")
                                            .small()
                                            .disabled(self.loading)
                                            .on_click(cx.listener(|panel, _ev, _window, cx| {
                                                panel.refresh_tables(cx);
                                            })),
                                    ),
                            )
                            .child(div().text_color(theme.colors.muted_foreground).child(self.status.clone()))
                            .child(
                                div().flex_1().min_h_0().child(
                                    match &self.result_table {
                                        Some(table) => div().size_full().child(table.clone()).into_any_element(),
                                        None => result_area,
                                    },
                                ),
                            ),
                    ),
            )
    }
}
