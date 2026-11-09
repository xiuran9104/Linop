//! 数据库面板视图:连接选择 + 表列表 + SQL 执行 + 结果网格(EditTable)。

use std::sync::Arc;

use gpui::{
    AppContext, Context, Entity, FocusHandle, Focusable, InteractiveElement, IntoElement,
    ParentElement, Render, SharedString, StatefulInteractiveElement, Styled, Window, div,
    prelude::FluentBuilder as _, px,
};
use gpui_component::{
    ActiveTheme, Disableable, Sizable, button::Button, h_flex, input::Input,
    input::InputState, v_flex,
};
use one_core::gpui_tokio::Tokio;
use one_ui::{Column, EditTableDelegate, EditTableState};
use one_core::tab_container::{TabContent, TabContentEvent};
use gpui::EventEmitter;

use crate::{
    DbxRuntime,
    console_tab::ConsoleTab,
    data_tab::DataTableTab,
    structure_tab::StructureTab,
};

pub struct DbxPanel {
    kind: &'static str,
    focus: FocusHandle,
    runtime: Option<Arc<DbxRuntime>>,
    profiles: Vec<linop_dbx::DbxConnectionSummary>,
    selected: Option<String>,
    tables: Vec<TableInfoRow>,
    selected_table: Option<String>,
    /// 查询结果网格(查询成功后重建;表未选时的自由查询结果)。
    result_table: Option<gpui::Entity<EditTableState<ResultGridDelegate>>>,
    /// 新建连接表单(点「新建」后 Some)。
    form: Option<ConnectionForm>,
    status: String,
    loading: bool,
    /// 右侧当前标签:0=数据 1=结构 2=SQL。
    tab: usize,
    /// 当前 (连接, 表) 的数据/结构页实体(切换表时重建)。
    data_tab: Option<Entity<DataTableTab>>,
    structure_tab: Option<Entity<StructureTab>>,
    console_tab: Option<ConsoleTabState>,
    /// 两击删除确认:待删除的连接名。
    pending_delete: Option<String>,
}

/// SQL 控制台状态(按连接惰性创建,切连接时重建)。
#[derive(Clone)]
struct ConsoleTabState {
    conn: String,
    entity: Entity<ConsoleTab>,
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
#[derive(Clone)]
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
    fn focus_handle(&self, _cx: &gpui::App) -> gpui::FocusHandle {
        self.focus.clone()
    }
}

impl DbxPanel {
    /// 打开面板:后台加载运行时与连接清单。
    pub fn new(_window: &mut Window, cx: &mut Context<Self>) -> Self {
        let this = Self {
            kind: "sql",
            focus: cx.focus_handle(),
            runtime: None,
            profiles: Vec::new(),
            selected: None,
            tables: Vec::new(),
            selected_table: None,
            result_table: None,
            form: None,
            status: "正在打开数据库运行时…".into(),
            loading: true,
            tab: 0,
            data_tab: None,
            structure_tab: None,
            console_tab: None,
            pending_delete: None,
        };
        let task = Tokio::spawn(cx, load_runtime());
        cx.spawn(async move |this, cx| {
            let loaded = task.await.unwrap_or_else(|e| Err(anyhow::anyhow!("{e}")));
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
        self.data_tab = None;
        self.structure_tab = None;
        self.console_tab = None;
        self.selected_table = None;
        self.result_table = None;
        self.refresh_tables(cx);
    }

    /// 打开 (连接, 表) 对应的右侧页(数据/结构惰性创建;SQL 按连接复用)。
    fn open_table_tabs(
        &mut self,
        name: &str,
        table: &str,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(runtime) = self.runtime.clone() else {
            return;
        };
        let driver = self
            .profiles
            .iter()
            .find(|p| p.name == name)
            .and_then(|p| linop_core::DbDriver::parse(&p.driver))
            .unwrap_or(linop_core::DbDriver::Postgres);
        if self.data_tab.as_ref().map(|t| t.read(cx).table_key())
            != Some((name.to_string(), table.to_string()))
        {
            self.data_tab = Some(cx.new(|cx| {
                DataTableTab::new(runtime.clone(), name.to_string(), table.to_string(), driver, window, cx)
            }));
        }
        if self
            .structure_tab
            .as_ref()
            .map(|t| t.read(cx).table_key())
            != Some((name.to_string(), table.to_string()))
        {
            self.structure_tab = Some(cx.new(|cx| {
                StructureTab::new(runtime.clone(), name.to_string(), table.to_string(), window, cx)
            }));
        }
        let reuse_console = self
            .console_tab
            .as_ref()
            .is_some_and(|c| c.conn == name);
        if !reuse_console {
            let entity = cx.new(|cx| {
                ConsoleTab::new(runtime.clone(), name.to_string(), window, cx)
            });
            self.console_tab = Some(ConsoleTabState {
                conn: name.to_string(),
                entity,
            });
        }
        cx.notify();
    }

    /// 删除连接:databases.toml 移除 + 会话内下线(第二击确认)。
    fn delete_profile(&mut self, name: &str, cx: &mut Context<Self>) {
        if self.pending_delete.as_deref() != Some(name) {
            self.pending_delete = Some(name.to_string());
            self.status = format!("再点一次「删除」确认移除 {name}");
            cx.notify();
            return;
        }
        self.pending_delete = None;
        let mut registry = linop_core::DbRegistry::load_default().unwrap_or_default();
        if let Err(e) = registry.remove(name) {
            self.status = format!("删除失败:{e}");
            cx.notify();
            return;
        }
        if let Err(e) = registry.save(linop_core::default_databases_path()) {
            self.status = format!("删除失败(写盘):{e}");
            cx.notify();
            return;
        }
        if let Some(runtime) = self.runtime.clone() {
            runtime.0.remove_profile_blocking(name);
        }
        self.profiles.retain(|p| p.name != name);
        if self.selected.as_deref() == Some(name) {
            self.selected = self.profiles.first().map(|p| p.name.clone());
            self.tables.clear();
            self.selected_table = None;
            self.data_tab = None;
            self.structure_tab = None;
            self.console_tab = None;
        }
        self.status = format!("已删除 {name}");
        cx.notify();
    }

    fn select_table(&mut self, name: &str, window: &mut Window, cx: &mut Context<Self>) {
        self.selected_table = Some(name.to_string());
        let Some(conn) = self.selected.clone() else {
            return;
        };
        self.tab = 0;
        self.open_table_tabs(&conn, name, window, cx);
    }

    /// 保存连接表单:写 databases.toml + 会话内注入。
    fn save_form(&mut self, _window: &mut Window, cx: &mut Context<Self>) {
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
        let profile2 = profile.clone();
        let task = Tokio::spawn(cx, async move {
            runtime
                .add_profile(profile2)
                .await
                .map_err(|e| e.to_string())?;
            Ok::<(), String>(())
        });
        cx.spawn(async move |this, cx| {
            let outcome = task.await;
            let _ = this.update(cx, |panel, _cx| match outcome {
                Ok(Ok(())) => {
                    panel.selected = Some(name);
                    panel.profiles.push(linop_dbx::DbxConnectionSummary {
                        name: profile.name.clone(),
                        driver: profile.driver.as_str().to_string(),
                        host: profile.host.clone().unwrap_or_default(),
                        port: profile.port.unwrap_or_default(),
                        database: profile.database.clone(),
                    });
                    panel.status = "已保存并注入运行时".into();
                }
                Ok(Err(e)) => panel.status = format!("注入失败:{e}"),
                Err(e) => panel.status = format!("注入失败:{e}"),
            });
        })
        .detach();
    }
}

/// 后台加载共享运行时与连接清单(gpui_tokio 的 Tokio runtime 上执行)。
async fn load_runtime() -> anyhow::Result<(Arc<DbxRuntime>, Vec<linop_dbx::DbxConnectionSummary>)> {
    let runtime = DbxRuntime::shared().await?;
    let connections = runtime.0.connections().await;
    Ok((runtime, connections))
}

impl Render for DbxPanel {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl gpui::IntoElement {
        let theme = cx.theme().clone();
        // cx 将被 render_right 可变借用,先克隆渲染所需的局部快照。
        let profiles = self.profiles.clone();
        let tables = self.tables.clone();
        let form_state = self.form.clone();
        let loading = self.loading;
        let status_text = self.status.clone();
        let selected_name = self.selected.clone();
        let selected_table = self.selected_table.clone();
        let pending_delete = self.pending_delete.clone();

        let connection_row: Vec<_> = profiles
            .iter()
            .map(|p| {
                let name = p.name.clone();
                let is_selected = selected_name.as_deref() == Some(name.as_str());
                let confirming = pending_delete.as_deref() == Some(name.as_str());
                h_flex()
                    .gap_0p5()
                    .items_center()
                    .child(
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
                            .when(!is_selected, |style| {
                                style.hover(|s| s.bg(theme.colors.selection))
                            })
                            .child(format!("{} ({})", p.name, p.driver))
                            .on_click(cx.listener({
                                let name = name.clone();
                                move |panel, _ev, _window, cx| {
                                    panel.select_profile(&name, cx);
                                }
                            })),
                    )
                    .child(
                        Button::new(SharedString::from(format!("del-{name}")))
                            .label(if confirming { "确认删除" } else { "删除" })
                            .small()
                            .when(confirming, |b| {
                                b.bg(theme.colors.danger)
                                    .text_color(theme.colors.danger_foreground)
                            })
                            .on_click(cx.listener({
                                let name = name.clone();
                                move |panel, _ev, _window, cx| {
                                    panel.delete_profile(&name, cx);
                                }
                            })),
                    )
            })
            .collect();

        let table_rows: Vec<_> = tables.iter().map(|t| {
            let name = t.name.clone();
            let is_selected = selected_table.as_deref() == Some(name.as_str());
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
        })
        .collect();

        let form = form_state.as_ref().map(|form| {
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

        // 右侧三标签内容(先克隆,避免与 cx 双借用)。
        let right = render_right(
            cx,
            selected_table,
            self.result_table.clone(),
            self.tab,
            self.data_tab.clone(),
            self.structure_tab.clone(),
            self.console_tab.clone(),
        );

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
                                    .child(
                                        Button::new("refresh")
                                            .label("刷新表")
                                            .small()
                                            .disabled(loading)
                                            .on_click(cx.listener(|panel, _ev, _window, cx| {
                                                panel.refresh_tables(cx);
                                            })),
                                    ),
                            )
                            .child(div().text_color(theme.colors.muted_foreground).child(status_text.clone()))
                            .child(
                                div().flex_1().min_h_0().child(right),
                            ),
                    ),
            )
    }
}

#[allow(clippy::too_many_arguments)]
fn render_right(
    cx: &mut Context<DbxPanel>,
    selected_table: Option<String>,
    free_result: Option<gpui::Entity<EditTableState<ResultGridDelegate>>>,
    tab_ix: usize,
    data: Option<gpui::Entity<DataTableTab>>,
    structure: Option<gpui::Entity<StructureTab>>,
    console: Option<ConsoleTabState>,
) -> gpui::AnyElement {
    let theme = cx.theme().clone();
    let placeholder = |text: &'static str| {
        div()
            .size_full()
            .flex()
            .items_center()
            .justify_center()
            .text_color(theme.colors.muted_foreground)
            .child(text)
            .into_any_element()
    };
    let Some(table) = selected_table else {
        return match free_result {
            Some(t) => div().size_full().child(t).into_any_element(),
            None => placeholder("选择连接与表浏览数据;自由 SQL 用「SQL」标签(选中表后可用)"),
        };
    };

    let strip = h_flex()
        .gap_1()
        .border_b_1()
        .border_color(theme.colors.border)
        .pb_1()
        .children(["数据", "结构", "SQL"].iter().enumerate().map(|(ix, label)| {
            let active = tab_ix == ix;
            Button::new(SharedString::from(format!("tab-{ix}")))
                .label(*label)
                .small()
                .when(active, |b| {
                    b.bg(theme.colors.accent)
                        .text_color(theme.colors.accent_foreground)
                })
                .on_click(cx.listener(move |panel, _ev, _window, cx| {
                    panel.tab = ix;
                    cx.notify();
                }))
        }));
    let content = match tab_ix {
        0 => match &data {
            Some(tab) => div().size_full().child(tab.clone()).into_any_element(),
            None => placeholder("数据页加载中…"),
        },
        1 => match &structure {
            Some(tab) => div().size_full().child(tab.clone()).into_any_element(),
            None => placeholder("结构页加载中…"),
        },
        _ => match &console {
            Some(state) => div().size_full().child(state.entity.clone()).into_any_element(),
            None => placeholder("选择连接后可用"),
        },
    };
    let _ = &table;
    v_flex()
        .size_full()
        .gap_2()
        .child(strip)
        .child(div().flex_1().min_h_0().overflow_hidden().child(content))
        .into_any_element()
}
