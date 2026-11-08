//! 数据库面板视图:连接选择 + 表列表 + SQL 执行 + 结果展示。

use std::sync::Arc;

use gpui::{
    AppContext, Context, FocusHandle, Focusable, IntoElement, InteractiveElement, ParentElement,
    Render, SharedString, Styled, Window, div, prelude::*, px,
};
use gpui_component::{
    ActiveTheme, Disableable, Sizable, StyledExt as _, button::Button, h_flex, input::Input,
    input::InputState, v_flex,
};
use linop_core::DbProfile;
use one_core::gpui_tokio::Tokio;
use gpui::EventEmitter;
use one_core::tab_container::{TabContent, TabContentEvent, TabOpenMode};

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
    result_lines: Vec<SharedString>,
    status: String,
    loading: bool,
}

#[derive(Clone)]
pub struct TableInfoRow {
    pub name: String,
    pub row_count: u64,
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
    pub fn new(kind: &'static str, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let sql_input =
            cx.new(|cx| InputState::new(window, cx).placeholder("SQL(Enter 执行)"));
        let mut this = Self {
            kind,
            focus: cx.focus_handle(),
            runtime: None,
            profiles: Vec::new(),
            selected: None,
            tables: Vec::new(),
            selected_table: None,
            sql_input,
            result_lines: Vec::new(),
            status: "正在打开数据库运行时…".into(),
            loading: true,
        };
        cx.spawn(async move |this, cx| {
            let loaded = load_runtime().await;
            let _ = this.update(cx, |panel, cx| match loaded {
                Ok((runtime, profiles)) => {
                    panel.loading = false;
                    panel.status = if profiles.is_empty() {
                        "databases.toml 中没有连接配置".into()
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
                            .map(|t| TableInfoRow { name: t.name, row_count: 0 })
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
            let _ = this.update(cx, |panel, cx| {
                panel.loading = false;
                match outcome {
                    Ok(Ok(result)) => {
                        let mut lines = vec![SharedString::from(result.columns.join("  |  "))];
                        for row in &result.rows {
                            let rendered: Vec<String> = row
                                .iter()
                                .map(|cell| match cell {
                                    serde_json::Value::Null => "NULL".into(),
                                    serde_json::Value::String(s) => s.clone(),
                                    other => other.to_string(),
                                })
                                .collect();
                            lines.push(SharedString::from(rendered.join("  |  ")));
                        }
                        let n = result.rows.len();
                        panel.status = if result.truncated {
                            format!("{n}+ 行(截断)")
                        } else {
                            format!("{n} 行")
                        };
                        panel.result_lines = lines;
                    }
                    Ok(Err(e)) => panel.status = format!("查询失败:{e}"),
                    Err(e) => panel.status = format!("查询失败:{e}"),
                }
                cx.notify();
            });
        })
        .detach();
    }
}

impl Render for DbxPanel {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl gpui::IntoElement {
        let theme = cx.theme().clone();

        let connection_row = self.profiles.iter().map(|p| {
            let name = p.name.clone();
            let is_selected = self.selected.as_deref() == Some(name.as_str());
            let label = format!("{} ({})", p.name, p.driver.as_str());
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
                .child(label)
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

        let mut result_items: Vec<gpui::AnyElement> = Vec::new();
        if self.result_lines.is_empty() {
            if !self.loading {
                result_items.push(
                    div()
                        .text_color(theme.colors.muted_foreground)
                        .child("在上方输入 SQL 后点击执行,或点击左侧表名预览数据".to_string())
                        .into_any_element(),
                );
            }
        } else {
            for (i, line) in self.result_lines.iter().enumerate() {
                let mut line_el = div().child(line.clone()).px_1();
                if i == 0 {
                    line_el = line_el
                        .font_weight(gpui::FontWeight::BOLD)
                        .border_b_1()
                        .border_color(theme.colors.border);
                }
                result_items.push(line_el.into_any_element());
            }
        }

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
                    .children(connection_row),
            )
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
                                div()
                                    .flex_1()
                                    .overflow_hidden()
                                    .child(v_flex().gap_1().children(result_items)),
                            ),
                    ),
            )
    }
}

/// 表名加引号(`schema.table` 拆段)。
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

/// `Styled::when` 由 gpui-component 提供;这里显式引入以便条件样式。
mod styling {
    pub use gpui_component::StyledExt;
}
pub use styling::*;
