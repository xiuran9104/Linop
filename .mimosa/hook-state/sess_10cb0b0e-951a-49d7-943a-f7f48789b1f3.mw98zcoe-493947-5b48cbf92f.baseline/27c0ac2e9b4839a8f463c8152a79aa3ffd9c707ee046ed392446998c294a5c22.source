//! 结构页:列清单 + 索引清单 + DDL(DBX exact_metadata 同源)。

use std::sync::{Arc, Mutex};

use gpui::{AppContext, Context, Entity, Focusable, InteractiveElement, IntoElement,
    ParentElement, Render, SharedString, StatefulInteractiveElement, Styled, Window, div, px,
    prelude::FluentBuilder as _};
use gpui_component::{ActiveTheme, Disableable, Sizable, button::Button, h_flex, v_flex};
use one_core::gpui_tokio::Tokio;
use one_ui::{Column, EditTableDelegate, EditTableState};

use crate::DbxRuntime;

/// 结构页实体:列/索引/DDL 三段。
pub struct StructureTab {
    runtime: Arc<DbxRuntime>,
    conn: String,
    table: String,
    /// (列清单, 索引清单, DDL) 加载完成后一次性写入。
    loaded: Arc<Mutex<Option<StructureData>>>,
    status: Arc<Mutex<String>>,
    loading: bool,
    columns_grid: Entity<EditTableState<ColumnsDelegate>>,
    focus: gpui::FocusHandle,
}

#[derive(Clone, Default)]
struct StructureData {
    columns: Vec<Vec<String>>,
    indexes: Vec<Vec<String>>,
    ddl: String,
}

impl StructureTab {
    pub fn new(
        runtime: Arc<DbxRuntime>,
        conn: String,
        table: String,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let delegate = ColumnsDelegate {
            data: Arc::new(Mutex::new(Vec::new())),
        };
        let columns_grid = cx.new(|cx| EditTableState::new(delegate, _window, cx));
        let mut this = Self {
            runtime,
            conn,
            table,
            loaded: Arc::new(Mutex::new(None)),
            status: Arc::new(Mutex::new("加载中…".into())),
            loading: false,
            columns_grid,
            focus: cx.focus_handle(),
        };
        this.refetch(cx);
        this
    }

    /// (连接, 表) 键:面板用于判断是否需要重建。
    pub fn table_key(&self) -> (String, String) {
        (self.conn.clone(), self.table.clone())
    }

    pub fn refetch(&mut self, cx: &mut Context<Self>) {
        if self.loading {
            return;
        }
        self.loading = true;
        *self.status.lock().unwrap() = "加载中…".into();
        let runtime = self.runtime.clone();
        let conn = self.conn.clone();
        let table = self.table.clone();
        let loaded = self.loaded.clone();
        let status = self.status.clone();
        let task = Tokio::spawn(cx, async move {
            let columns = runtime.0.table_columns(&conn, &table).await;
            let indexes = runtime.0.table_indexes(&conn, &table).await;
            let ddl = runtime.0.table_ddl(&conn, &table).await;
            (columns, indexes, ddl)
        });
        let grid_data = {
            let delegate = self.columns_grid.read(cx).delegate();
            delegate.data.clone()
        };
        cx.spawn(async move |this, cx| {
            let outcome = task.await;
            let _ = this.update(cx, |this, cx| {
                this.loading = false;
                let mut data = StructureData::default();
                match outcome {
                    Ok((Ok(columns), Ok(indexes), Ok(ddl))) => {
                        data.columns = columns
                            .iter()
                            .map(|c| {
                                vec![
                                    c.name.clone(),
                                    c.data_type.clone(),
                                    if c.is_primary_key {
                                        "是".into()
                                    } else {
                                        String::new()
                                    },
                                    if c.is_nullable { "NULL".into() } else { "NOT NULL".into() },
                                    c.column_default.clone().unwrap_or_default(),
                                    c.comment.clone().unwrap_or_default(),
                                ]
                            })
                            .collect();
                        data.indexes = indexes
                            .iter()
                            .map(|i| {
                                vec![
                                    i.name.clone(),
                                    i.columns.join(", "),
                                    if i.is_unique { "唯一".into() } else { String::new() },
                                ]
                            })
                            .collect();
                        data.ddl = ddl;
                        *status.lock().unwrap() = format!(
                            "{} 列 / {} 索引",
                            data.columns.len(),
                            data.indexes.len()
                        );
                    }
                    Ok((Err(e), _, _)) | Ok((_, Err(e), _)) | Ok((_, _, Err(e))) => {
                        *status.lock().unwrap() = format!("加载失败:{e}");
                    }
                    Err(join) => {
                        *status.lock().unwrap() = format!("加载任务失败:{join}");
                    }
                }
                *loaded.lock().unwrap() = Some(data.clone());
                *grid_data.lock().unwrap() = data.columns;
                cx.notify();
            });
        })
        .detach();
    }
}

impl Focusable for StructureTab {
    fn focus_handle(&self, _cx: &gpui::App) -> gpui::FocusHandle {
        self.focus.clone()
    }
}

impl Render for StructureTab {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme().clone();
        let data = self.loaded.lock().unwrap().clone().unwrap_or_default();
        let status = self.status.lock().unwrap().clone();

        let ddl_view: gpui::AnyElement = if data.ddl.is_empty() {
            div()
                .text_sm()
                .text_color(theme.colors.muted_foreground)
                .child("(DDL 不可用:当前驱动未提供 GetTableDdl)")
                .into_any_element()
        } else {
            div()
                .id("ddl-view")
                .font_family("monospace")
                .text_size(px(12.0))
                .overflow_y_scroll()
                .child(data.ddl.clone())
                .into_any_element()
        };

        v_flex()
            .size_full()
            .gap_2()
            .p_2()
            .child(
                h_flex()
                    .gap_2()
                    .items_center()
                    .child(
                        Button::new("struct-refresh")
                            .label("刷新")
                            .small()
                            .disabled(self.loading)
                            .on_click(cx.listener(|this, _ev, _window, cx| {
                                this.refetch(cx);
                            })),
                    )
                    .child(
                        div()
                            .text_sm()
                            .text_color(theme.colors.muted_foreground)
                            .child(status),
                    ),
            )
            .child(
                div()
                    .text_sm()
                    .child("列"),
            )
            .child(
                div()
                    .id("struct-columns")
                    .max_h_64()
                    .overflow_y_scroll()
                    .child(self.columns_grid.clone()),
            )
            .child(div().text_sm().child("索引"))
            .child(
                v_flex()
                    .gap_1()
                    .child(
                        h_flex()
                            .gap_4()
                            .text_xs()
                            .text_color(theme.colors.muted_foreground)
                            .child(div().w(px(200.0)).child("名称"))
                            .child(div().flex_1().child("列"))
                            .child(div().w(px(60.0)).child("唯一")),
                    )
                    .children(data.indexes.iter().map(|row| {
                        h_flex()
                            .gap_4()
                            .text_sm()
                            .child(div().w(px(200.0)).child(row.first().cloned().unwrap_or_default()))
                            .child(div().flex_1().child(row.get(1).cloned().unwrap_or_default()))
                            .child(div().w(px(60.0)).child(row.get(2).cloned().unwrap_or_default()))
                    }))
                    .when(data.indexes.is_empty(), |col| {
                        col.child(
                            div()
                                .text_sm()
                                .text_color(theme.colors.muted_foreground)
                                .child("(无索引或不可见)"),
                        )
                    }),
            )
            .child(div().text_sm().child("DDL"))
            .child(
                div()
                    .id("struct-ddl")
                    .flex_1()
                    .min_h_0()
                    .overflow_y_scroll()
                    .rounded_md()
                    .border_1()
                    .border_color(theme.colors.border)
                    .p_2()
                    .child(ddl_view),
            )
    }
}

/// 列清单只读网格委托。
struct ColumnsDelegate {
    data: Arc<Mutex<Vec<Vec<String>>>>,
}

const COLUMN_HEADERS: [&str; 6] = ["名称", "类型", "主键", "可空", "默认", "注释"];

impl EditTableDelegate for ColumnsDelegate {
    fn columns_count(&self, _cx: &gpui::App) -> usize {
        COLUMN_HEADERS.len()
    }

    fn rows_count(&self, _cx: &gpui::App) -> usize {
        self.data.lock().unwrap().len()
    }

    fn column(&self, col_ix: usize, _cx: &gpui::App) -> Column {
        Column::new(
            SharedString::from(COLUMN_HEADERS[col_ix]),
            SharedString::from(COLUMN_HEADERS[col_ix]),
        )
        .width(px(if col_ix == 5 { 220.0 } else { 120.0 }))
    }

    fn render_td(
        &mut self,
        row_ix: usize,
        col_ix: usize,
        _window: &mut Window,
        cx: &mut Context<EditTableState<Self>>,
    ) -> impl IntoElement {
        let theme = cx.theme().clone();
        let text = self
            .data
            .lock()
            .unwrap()
            .get(row_ix)
            .and_then(|r| r.get(col_ix))
            .cloned()
            .unwrap_or_default();
        div()
            .px_2()
            .text_ellipsis()
            .overflow_hidden()
            .text_color(theme.colors.foreground)
            .child(text)
    }
}
