//! SQL 控制台页:多行源码编辑器(EditorState)+ 执行 + 会话历史。

use std::sync::{Arc, Mutex};

use gpui::{AppContext, Context, Entity, Focusable, InteractiveElement, IntoElement,
    ParentElement, Render, SharedString, StatefulInteractiveElement, Styled, Window, div,
    prelude::FluentBuilder as _};
use gpui_component::{
    ActiveTheme, Disableable, Sizable, button::Button, h_flex, input::Editor,
    input::EditorState, v_flex,
};
use one_core::gpui_tokio::Tokio;

use crate::DbxRuntime;

/// SQL 控制台实体。
pub struct ConsoleTab {
    runtime: Arc<DbxRuntime>,
    conn: String,
    editor: Entity<EditorState>,
    history: Arc<Mutex<Vec<String>>>,
    running: bool,
    status: String,
    /// 最近一次查询结果(简单文本预览;完整网格走数据页)。
    result_preview: Arc<Mutex<Option<String>>>,
    focus: gpui::FocusHandle,
}

impl ConsoleTab {
    pub fn new(
        runtime: Arc<DbxRuntime>,
        conn: String,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let editor = cx.new(|cx| {
            EditorState::new(window, cx)
                .line_number(true)
                .soft_wrap(true)
                .placeholder("SQL(Enter 换行,Ctrl+Enter 或点「执行」运行)")
        });
        Self {
            runtime,
            conn,
            editor,
            history: Arc::new(Mutex::new(Vec::new())),
            running: false,
            status: "就绪".into(),
            result_preview: Arc::new(Mutex::new(None)),
            focus: cx.focus_handle(),
        }
    }

    fn execute(&mut self, cx: &mut Context<Self>) {
        let sql = self.editor.read(cx).value().trim().to_string();
        if sql.is_empty() || self.running {
            return;
        }
        self.running = true;
        self.status = "执行中…".into();
        cx.notify();
        {
            let mut history = self.history.lock().unwrap();
            history.retain(|h| h != &sql);
            history.insert(0, sql.clone());
            history.truncate(20);
        }
        let runtime = self.runtime.clone();
        let conn = self.conn.clone();
        let history = self.history.clone();
        let preview = self.result_preview.clone();
        let task = Tokio::spawn(cx, async move { runtime.0.query(&conn, &sql).await });
        cx.spawn(async move |this, cx| {
            let outcome = task.await;
            let _ = this.update(cx, |this, cx| {
                this.running = false;
                match outcome {
                    Ok(Ok(result)) => {
                        let rows: Vec<String> = result
                            .rows
                            .iter()
                            .take(50)
                            .map(|row| {
                                row.iter()
                                    .map(|cell| match cell {
                                        serde_json::Value::Null => "NULL".into(),
                                        serde_json::Value::String(s) => s.clone(),
                                        other => other.to_string(),
                                    })
                                    .collect::<Vec<_>>()
                                    .join(" | ")
                            })
                            .collect();
                        let n = result.rows.len();
                        let columns = result.columns.join(" | ");
                        let text = if n == 0 {
                            "OK(无结果集)".to_string()
                        } else {
                            format!(
                                "{}{}\n{}\n{}",
                                columns,
                                if columns.is_empty() { "" } else { "\n" },
                                "-".repeat(columns.len().min(80)),
                                rows.join("\n")
                            )
                        };
                        *preview.lock().unwrap() = Some(text.clone());
                        this.status = format!(
                            "完成:{n} 行{}",
                            if result.truncated { "(截断)" } else { "" }
                        );
                    }
                    Ok(Err(e)) => {
                        this.status = format!("执行失败:{e}");
                    }
                    Err(join) => {
                        this.status = format!("执行任务失败:{join}");
                    }
                }
                let _ = history;
                let _ = preview;
                cx.notify();
            });
        })
        .detach();
    }

    fn load_history(&mut self, sql: String, window: &mut Window, cx: &mut Context<Self>) {
        self.editor
            .update(cx, |state, cx| state.set_value(sql, window, cx));
        cx.notify();
    }
}

impl Focusable for ConsoleTab {
    fn focus_handle(&self, _cx: &gpui::App) -> gpui::FocusHandle {
        self.focus.clone()
    }
}

impl Render for ConsoleTab {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme().clone();
        let history = self.history.lock().unwrap().clone();
        let preview = self.result_preview.lock().unwrap().clone();

        let history_list = history
            .iter()
            .enumerate()
            .map(|(ix, sql)| {
                let sql = sql.clone();
                div()
                    .id(SharedString::from(format!("history-{ix}")))
                    .cursor_pointer()
                    .px_2()
                    .py_0p5()
                    .rounded_sm()
                    .text_ellipsis()
                    .overflow_hidden()
                    .when(ix == 0, |style| {
                        style.text_color(theme.colors.muted_foreground)
                    })
                    .child(sql.replace('\n', " "))
                    .on_click(cx.listener(move |this, _ev, window, cx| {
                        this.load_history(sql.clone(), window, cx);
                    }))
            })
            .collect::<Vec<_>>();

        v_flex()
            .size_full()
            .gap_2()
            .p_2()
            .child(
                h_flex()
                    .gap_2()
                    .items_center()
                    .child(
                        Button::new("run-sql")
                            .label("执行")
                            .small()
                            .disabled(self.running)
                            .on_click(cx.listener(|this, _ev, _window, cx| {
                                this.execute(cx);
                            })),
                    )
                    .child(
                        Button::new("clear-history")
                            .label("清历史")
                            .small()
                            .disabled(history.is_empty())
                            .on_click(cx.listener(|this, _ev, _window, cx| {
                                this.history.lock().unwrap().clear();
                                cx.notify();
                            })),
                    )
                    .child(
                        div()
                            .text_sm()
                            .text_color(theme.colors.muted_foreground)
                            .child(self.status.clone()),
                    ),
            )
            .child(
                div()
                    .h_1_2()
                    .min_h_0()
                    .rounded_md()
                    .border_1()
                    .border_color(theme.colors.border)
                    .p_1()
                    .child(Editor::new(&self.editor).size_full()),
            )
            .child(
                div()
                    .text_sm()
                    .child(format!("历史(点回填,最多 20 条)")),
            )
            .child(
                v_flex()
                    .id("sql-history")
                    .max_h_32()
                    .overflow_y_scroll()
                    .gap_0p5()
                    .children(history_list)
                    .when(history.is_empty(), |col| {
                        col.child(
                            div()
                                .text_sm()
                                .text_color(theme.colors.muted_foreground)
                                .child("(暂无)"),
                        )
                    }),
            )
            .children(preview.map(|text| {
                div()
                    .id("sql-preview")
                    .flex_1()
                    .min_h_0()
                    .overflow_y_scroll()
                    .rounded_md()
                    .border_1()
                    .border_color(theme.colors.border)
                    .p_2()
                    .font_family("monospace")
                    .text_size(gpui::px(12.0))
                    .whitespace_nowrap()
                    .child(text)
            }))
    }
}
