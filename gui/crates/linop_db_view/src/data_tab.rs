//! 数据页:DBX 同款数据网格 —— 分页、列头排序、WHERE 过滤、单元格内联编辑
//! (UPDATE)、删行(DELETE)、插行(INSERT)、导出 CSV。
//!
//! 取数/写回都走共享 DbxRuntime 的 `query`(DBX 执行路径,只读风控对
//! SELECT 生效;写语句在 DBX 的写通道执行)。

use std::collections::{BTreeMap, BTreeSet};
use std::sync::{Arc, Mutex};

use gpui::{
    AppContext, Context, Entity, Focusable, Hsla, IntoElement, ParentElement, Render, SharedString,
    Styled, Window, div, px,
};
use gpui_component::{ActiveTheme, Disableable, Sizable, button::Button, h_flex, input::Input,
    input::InputState, v_flex};
use linop_core::DbDriver;
use one_core::gpui_tokio::Tokio;
use one_ui::{
    CellEditor, Column, ColumnSort, EditTableDelegate, EditTableState,
};

use crate::DbxRuntime;

const PAGE_SIZE: u64 = 50;
/// 新增行在视图层用的哨兵行号(排在既有行之后)。
const NEW_ROW: usize = usize::MAX;

#[derive(Clone, Debug)]
pub struct ColMeta {
    pub name: String,
    pub data_type: String,
    pub is_primary_key: bool,
    pub is_nullable: bool,
}

/// 网格共享核心:委托(网格内交互)与 Tab(工具栏/分页/刷新)同读同写。
struct GridCore {
    driver: DbDriver,
    columns: Vec<ColMeta>,
    rows: Vec<Vec<serde_json::Value>>,
    /// 未保存的单元格修改:(行号或 NEW_ROW, 列号) → 新值(字符串)。
    modified: BTreeMap<(usize, usize), String>,
    /// 未保存的删除行。
    deleted: BTreeSet<usize>,
    /// 未保存的新增行(列值;None 表示未开始编辑)。
    new_row: Option<Vec<String>>,
    sort: Option<(usize, bool)>,
    filter: String,
    page: u64,
    /// COUNT(*) 结果;None = 未知(加载中/失败)。
    total: Option<u64>,
    loading: bool,
    status: String,
}

impl GridCore {
    fn has_pk(&self) -> bool {
        self.columns.iter().any(|c| c.is_primary_key)
    }

    fn pk_columns(&self) -> Vec<&ColMeta> {
        self.columns.iter().filter(|c| c.is_primary_key).collect()
    }

    fn view_rows_count(&self) -> usize {
        self.rows.len() + usize::from(self.new_row.is_some())
    }

    fn cell_value(&self, row_ix: usize, col_ix: usize) -> Option<&serde_json::Value> {
        if row_ix == NEW_ROW || self.new_row.is_some() && row_ix == self.rows.len() {
            return None;
        }
        self.rows.get(row_ix).and_then(|r| r.get(col_ix))
    }

    fn cell_string(&self, row_ix: usize, col_ix: usize) -> String {
        if let Some(v) = self.modified.get(&(row_ix, col_ix)) {
            return v.clone();
        }
        if row_ix == NEW_ROW || self.new_row.is_some() && row_ix == self.rows.len() {
            return self
                .new_row
                .as_ref()
                .and_then(|r| r.get(col_ix))
                .cloned()
                .unwrap_or_default();
        }
        self.cell_value(row_ix, col_ix).map(value_to_string).unwrap_or_default()
    }
}

fn value_to_string(v: &serde_json::Value) -> String {
    match v {
        serde_json::Value::Null => String::new(),
        serde_json::Value::String(s) => s.clone(),
        other => other.to_string(),
    }
}

/// 数值列:字面量不加引号(非数值文本会被驱动拒绝,天然防注入);
/// 其余:单引号转义;空值且可空 → NULL。
fn sql_literal(col: &ColMeta, raw: &str) -> String {
    if raw.is_empty() {
        if col.is_nullable {
            return "NULL".into();
        }
        return "''".into();
    }
    if is_numeric_type(&col.data_type) && raw.parse::<f64>().is_ok() {
        return raw.to_string();
    }
    format!("'{}'", raw.replace('\'', "''"))
}

fn is_numeric_type(data_type: &str) -> bool {
    let t = data_type.to_ascii_lowercase();
    ["int", "integer", "bigint", "smallint", "tinyint", "mediumint", "decimal",
     "numeric", "number", "float", "double", "real", "bit", "serial", "bigserial",
     "int4", "int8", "int2", "float4", "float8"]
        .iter()
        .any(|n| t.starts_with(n))
}

fn quote_ident(driver: DbDriver, name: &str) -> String {
    match driver {
        DbDriver::MySql => format!("`{}`", name.replace('`', "``")),
        _ => format!("\"{}\"", name.replace('"', "\"\"")),
    }
}

/// 数据页实体:工具栏 + 可编辑网格 + 分页。
pub struct DataTableTab {
    runtime: Arc<DbxRuntime>,
    conn: String,
    table: String,
    driver: DbDriver,
    core: Arc<Mutex<GridCore>>,
    grid: Entity<EditTableState<DataTableDelegate>>,
    filter_input: Entity<InputState>,
    focus: gpui::FocusHandle,
}

impl DataTableTab {
    pub fn new(
        runtime: Arc<DbxRuntime>,
        conn: String,
        table: String,
        driver: DbDriver,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let core = Arc::new(Mutex::new(GridCore {
            driver,
            columns: Vec::new(),
            rows: Vec::new(),
            modified: BTreeMap::new(),
            deleted: BTreeSet::new(),
            new_row: None,
            sort: None,
            filter: String::new(),
            page: 0,
            total: None,
            loading: true,
            status: "加载中…".into(),
        }));
        let delegate = DataTableDelegate {
            core: core.clone(),
            tab: cx.weak_entity(),
        };
        let grid = cx.new(|cx| EditTableState::new(delegate, window, cx));
        let filter_input =
            cx.new(|cx| InputState::new(window, cx).placeholder("WHERE 条件,如: id > 10 AND name LIKE 'a%'"));
        let this = Self {
            runtime,
            conn,
            table,
            driver,
            core,
            grid,
            filter_input,
            focus: cx.focus_handle(),
        };
        this.refetch(cx);
        this
    }

    /// (连接, 表) 键:面板用于判断是否需要重建。
    pub fn table_key(&self) -> (String, String) {
        (self.conn.clone(), self.table.clone())
    }

    /// 组装 SELECT / COUNT 并异步取数。
    pub fn refetch(&self, cx: &mut Context<Self>) {
        let snapshot = {
            let mut core = self.core.lock().unwrap();
            core.loading = true;
            core.status = "加载中…".into();
            (core.filter.clone(), core.sort.clone(), core.page)
        };
        let (filter, sort, page) = snapshot;
        let table = quote_ident(self.driver, &self.table);
        let where_clause = if filter.trim().is_empty() {
            String::new()
        } else {
            format!(" WHERE {}", filter.trim())
        };
        let order_clause = match sort {
            Some((ix, asc)) => {
                let col = self.core.lock().unwrap().columns.get(ix).map(|c| c.name.clone());
                match col {
                    Some(name) => format!(
                        " ORDER BY {} {}",
                        quote_ident(self.driver, &name),
                        if asc { "ASC" } else { "DESC" }
                    ),
                    None => String::new(),
                }
            }
            None => String::new(),
        };
        let offset = page.saturating_mul(PAGE_SIZE);
        let count_sql = format!("SELECT COUNT(*) FROM {table}{where_clause}");
        let data_sql = format!(
            "SELECT * FROM {table}{where_clause}{order_clause} LIMIT {PAGE_SIZE} OFFSET {offset}"
        );
        let runtime = self.runtime.clone();
        let conn = self.conn.clone();
        let core = self.core.clone();
        let task = Tokio::spawn(cx, async move {
            let count = runtime.0.query(&conn, &count_sql).await;
            let data = runtime.0.query(&conn, &data_sql).await;
            (count, data)
        });
        let core_for_task = core.clone();
        cx.spawn(async move |this, cx| {
            let outcome = task.await;
            let _ = this.update(cx, |_this, cx| {
                let mut core = core_for_task.lock().unwrap();
                core.loading = false;
                match outcome {
                    Ok((Ok(count), Ok(mut data))) => {
                        core.total = count
                            .rows
                            .first()
                            .and_then(|r| r.first())
                            .and_then(|v| v.as_u64());
                        core.rows = std::mem::take(&mut data.rows);
                        core.status = format!(
                            "第 {} 页 / 共 {} 行{}",
                            core.page + 1,
                            core.total.unwrap_or(core.rows.len() as u64),
                            if core.modified.is_empty()
                                && core.deleted.is_empty()
                                && core.new_row.is_none()
                            {
                                String::new()
                            } else {
                                " · 有未保存修改".to_string()
                            },
                        );
                    }
                    Ok((_, Err(e))) | Ok((Err(e), _)) => {
                        core.rows.clear();
                        core.status = format!("查询失败:{e}");
                    }
                    Err(join) => {
                        core.status = format!("查询任务失败:{join}");
                    }
                }
                // 网格与分页栏都由本实体渲染,统一 notify。
                cx.notify();
            });
        })
        .detach();
    }

    fn apply_filter(&mut self, cx: &mut Context<Self>) {
        let filter = self.filter_input.read(cx).value().trim().to_string();
        {
            let mut core = self.core.lock().unwrap();
            core.filter = filter;
            core.page = 0;
        }
        self.refetch(cx);
    }

    fn set_page(&mut self, page: u64, cx: &mut Context<Self>) {
        self.core.lock().unwrap().page = page;
        self.refetch(cx);
    }

    /// 保存未落库的修改/删除/新增。
    fn save_changes(&mut self, cx: &mut Context<Self>) {
        let (statements, count) = {
            let core = self.core.lock().unwrap();
            let table = quote_ident(self.driver, &self.table);
            let mut statements: Vec<String> = Vec::new();

            // 删除行(按行号倒序,避免语义混乱)。
            for &row_ix in core.deleted.iter().rev() {
                let Some(pk_set) = Self::pk_where(&core, row_ix) else {
                    continue;
                };
                statements.push(format!("DELETE FROM {table} WHERE {pk_set}"));
            }
            // 既有行修改:逐单元格 UPDATE(以主键定位)。
            for ((row_ix, col_ix), value) in core.modified.iter() {
                if *row_ix == NEW_ROW || core.new_row.is_some() && *row_ix == core.rows.len() {
                    continue;
                }
                let Some(col) = core.columns.get(*col_ix) else {
                    continue;
                };
                let Some(pk_set) = Self::pk_where(&core, *row_ix) else {
                    continue;
                };
                statements.push(format!(
                    "UPDATE {table} SET {} = {} WHERE {}",
                    quote_ident(self.driver, &col.name),
                    sql_literal(col, value),
                    pk_set
                ));
            }
            // 新增行。
            if let Some(new_row) = &core.new_row {
                let mut cols = Vec::new();
                let mut lits = Vec::new();
                for (ix, col) in core.columns.iter().enumerate() {
                    let raw = new_row.get(ix).cloned().unwrap_or_default();
                    if raw.is_empty() {
                        continue;
                    }
                    cols.push(quote_ident(self.driver, &col.name));
                    lits.push(sql_literal(col, &raw));
                }
                if !cols.is_empty() {
                    statements.push(format!(
                        "INSERT INTO {table} ({}) VALUES ({})",
                        cols.join(", "),
                        lits.join(", ")
                    ));
                }
            }
            let n = statements.len();
            (statements, n)
        };
        if count == 0 {
            self.core.lock().unwrap().status = "没有需要保存的修改".into();
            cx.notify();
            return;
        }
        let runtime = self.runtime.clone();
        let conn = self.conn.clone();
        let core = self.core.clone();
        let task = Tokio::spawn(cx, async move {
            let mut errors = Vec::new();
            for sql in &statements {
                if let Err(e) = runtime.0.query(&conn, sql).await {
                    errors.push(format!("{sql} → {e}"));
                }
            }
            errors
        });
        cx.spawn(async move |this, cx| {
            let errors = task.await.unwrap_or_else(|e| vec![format!("保存任务失败:{e}")]);
            let _ = this.update(cx, |this, cx| {
                {
                    let mut core = core.lock().unwrap();
                    if errors.is_empty() {
                        core.modified.clear();
                        core.deleted.clear();
                        core.new_row = None;
                        core.status = "已保存".into();
                    } else {
                        core.status = format!("保存失败:{} 条语句出错", errors.len());
                    }
                }
                if errors.is_empty() {
                    this.refetch(cx);
                } else {
                    cx.notify();
                }
            });
        })
        .detach();
    }

    fn pk_where(core: &GridCore, row_ix: usize) -> Option<String> {
        let row = core.rows.get(row_ix)?;
        let pks = core.pk_columns();
        if pks.is_empty() {
            return None;
        }
        let parts = pks
            .iter()
            .filter_map(|pk| {
                let ix = core.columns.iter().position(|c| c.name == pk.name)?;
                let raw = value_to_string(row.get(ix)?);
                Some(format!(
                    "{} = {}",
                    quote_ident(core.driver, &pk.name),
                    sql_literal(pk, &raw)
                ))
            })
            .collect::<Vec<_>>();
        (!parts.is_empty()).then(|| parts.join(" AND "))
    }

    /// 导出当前页为 CSV(写到 $HOME/linop-exports)。
    fn export_csv(&self, cx: &mut Context<Self>) {
        let (columns, rows) = {
            let core = self.core.lock().unwrap();
            let columns: Vec<String> =
                core.columns.iter().map(|c| c.name.clone()).collect();
            let mut rows = core
                .rows
                .iter()
                .map(|row| {
                    columns
                        .iter()
                        .enumerate()
                        .map(|(ix, _)| {
                            row.get(ix).map(value_to_string).unwrap_or_default()
                        })
                        .collect::<Vec<_>>()
                })
                .collect::<Vec<_>>();
            if let Some(new_row) = &core.new_row {
                rows.push(new_row.clone());
            }
            (columns, rows)
        };
        let path = std::env::var_os("HOME")
            .map(|h| std::path::PathBuf::from(h).join("linop-exports"))
            .unwrap_or_else(|| std::path::PathBuf::from("linop-exports"));
        let _ = std::fs::create_dir_all(&path);
        let file = path.join(format!(
            "{}-page{}.csv",
            self.table.replace(['/', '\\'], "_"),
            self.core.lock().unwrap().page + 1
        ));
        let mut csv = String::new();
        csv.push_str(&columns
            .iter()
            .map(|c| format!("\"{}\"", c.replace('"', "\"\"")))
            .collect::<Vec<_>>()
            .join(","));
        csv.push('\n');
        for row in rows {
            csv.push_str(&row
                .iter()
                .map(|v| format!("\"{}\"", v.replace('"', "\"\"")))
                .collect::<Vec<_>>()
                .join(","));
            csv.push('\n');
        }
        match std::fs::write(&file, csv) {
            Ok(()) => {
                self.core.lock().unwrap().status =
                    format!("已导出 {}", file.display());
            }
            Err(e) => {
                self.core.lock().unwrap().status = format!("导出失败:{e}");
            }
        }
        cx.notify();
    }

    fn discard_changes(&mut self, cx: &mut Context<Self>) {
        {
            let mut core = self.core.lock().unwrap();
            core.modified.clear();
            core.deleted.clear();
            core.new_row = None;
            core.status = "已放弃未保存修改".into();
        }
        cx.notify();
    }
}

impl Focusable for DataTableTab {
    fn focus_handle(&self, _cx: &gpui::App) -> gpui::FocusHandle {
        self.focus.clone()
    }
}

impl Render for DataTableTab {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme().clone();
        let (loading, status, page, total, has_pk, dirty) = {
            let core = self.core.lock().unwrap();
            (
                core.loading,
                core.status.clone(),
                core.page,
                core.total,
                core.has_pk(),
                !core.modified.is_empty()
                    || !core.deleted.is_empty()
                    || core.new_row.is_some(),
            )
        };
        let pages = total.map(|t| t.div_ceil(PAGE_SIZE)).unwrap_or(1).max(1);

        let toolbar = h_flex()
            .gap_2()
            .items_center()
            .child(Input::new(&self.filter_input).small().flex_1())
            .child(
                Button::new("apply-filter")
                    .label("过滤")
                    .small()
                    .disabled(loading)
                    .on_click(cx.listener(|this, _ev, _window, cx| {
                        this.apply_filter(cx);
                    })),
            )
            .child(
                Button::new("refresh")
                    .label("刷新")
                    .small()
                    .disabled(loading)
                    .on_click(cx.listener(|this, _ev, _window, cx| {
                        this.refetch(cx);
                    })),
            )
            .child(
                Button::new("insert-row")
                    .label("插入行")
                    .small()
                    .disabled(loading || !has_pk)
                    .on_click(cx.listener(|this, _ev, _window, cx| {
                        {
                            let mut core = this.core.lock().unwrap();
                            if core.new_row.is_none() {
                                core.new_row =
                                    Some(vec![String::new(); core.columns.len()]);
                            }
                        }
                        cx.notify();
                    })),
            )
            .child(
                Button::new("save-changes")
                    .label("保存修改")
                    .small()
                    .disabled(loading || !dirty || !has_pk)
                    .on_click(cx.listener(|this, _ev, _window, cx| {
                        this.save_changes(cx);
                    })),
            )
            .child(
                Button::new("discard-changes")
                    .label("放弃")
                    .small()
                    .disabled(loading || !dirty)
                    .on_click(cx.listener(|this, _ev, _window, cx| {
                        this.discard_changes(cx);
                    })),
            )
            .child(
                Button::new("export-csv")
                    .label("导出 CSV")
                    .small()
                    .disabled(loading)
                    .on_click(cx.listener(|this, _ev, _window, cx| {
                        this.export_csv(cx);
                    })),
            );

        let pager = h_flex()
            .gap_2()
            .items_center()
            .child(
                Button::new("prev-page")
                    .label("上一页")
                    .small()
                    .disabled(page == 0 || loading)
                    .on_click(cx.listener(|this, _ev, _window, cx| {
                        let page = this.core.lock().unwrap().page;
                        if page > 0 {
                            this.set_page(page - 1, cx);
                        }
                    })),
            )
            .child(
                div()
                    .text_sm()
                    .text_color(theme.colors.muted_foreground)
                    .child(format!(
                        "第 {} / {} 页{}",
                        page + 1,
                        pages,
                        total.map(|t| format!("(共 {t} 行)")).unwrap_or_default()
                    )),
            )
            .child(
                Button::new("next-page")
                    .label("下一页")
                    .small()
                    .disabled(page + 1 >= pages || loading)
                    .on_click(cx.listener(|this, _ev, _window, cx| {
                        let page = this.core.lock().unwrap().page;
                        this.set_page(page + 1, cx);
                    })),
            );

        let banner = if !has_pk {
            Some(
                div()
                    .text_sm()
                    .text_color(theme.colors.danger_foreground)
                    .child("该表没有主键:网格只读(排序/过滤/导出可用),无法编辑或分页写回。"),
            )
        } else {
            None
        };

        v_flex()
            .size_full()
            .gap_2()
            .p_2()
            .child(toolbar)
            .children(banner)
            .child(
                div()
                    .flex_1()
                    .min_h_0()
                    .overflow_hidden()
                    .child(self.grid.clone()),
            )
            .child(
                h_flex()
                    .justify_between()
                    .items_center()
                    .child(pager)
                    .child(
                        div()
                            .text_sm()
                            .text_color(theme.colors.muted_foreground)
                            .child(status),
                    ),
            )
    }
}

/// 网格委托:读共享 GridCore,支持内联编辑/排序/增删行。
struct DataTableDelegate {
    core: Arc<Mutex<GridCore>>,
    tab: gpui::WeakEntity<DataTableTab>,
}

fn modified_highlight(base: Hsla) -> Hsla {
    // 未保存修改:淡黄底(与 DBX 网格的脏单元格标记一致)。
    let mut hsla = base;
    hsla.a *= 0.18;
    hsla
}

impl DataTableDelegate {
    fn is_new_row(&self, row_ix: usize) -> bool {
        let core = self.core.lock().unwrap();
        row_ix == NEW_ROW || core.new_row.is_some() && row_ix == core.rows.len()
    }
}

impl EditTableDelegate for DataTableDelegate {
    fn cell_edit_enabled(&self, _cx: &gpui::App) -> bool {
        let core = self.core.lock().unwrap();
        core.has_pk() && !core.loading
    }

    fn single_click_to_edit(&self, _cx: &gpui::App) -> bool {
        true
    }

    fn row_number_enabled(&self, _cx: &gpui::App) -> bool {
        true
    }

    fn columns_count(&self, _cx: &gpui::App) -> usize {
        self.core.lock().unwrap().columns.len()
    }

    fn rows_count(&self, _cx: &gpui::App) -> usize {
        self.core.lock().unwrap().view_rows_count()
    }

    fn column(&self, col_ix: usize, _cx: &gpui::App) -> Column {
        let core = self.core.lock().unwrap();
        let meta = core.columns.get(col_ix);
        let sort = core.sort.filter(|(ix, _)| *ix == col_ix).map(|(_, asc)| {
            if asc {
                ColumnSort::Ascending
            } else {
                ColumnSort::Descending
            }
        });
        let mut name = meta.map(|m| m.name.clone()).unwrap_or_default();
        if let Some(m) = meta {
            if m.is_primary_key {
                name.push_str(" 🔑");
            }
        }
        Column::new(SharedString::from(name.clone()), SharedString::from(name))
            .sortable()
            .sort(sort.unwrap_or(ColumnSort::Default))
            .width(px(180.0))
    }

    fn perform_sort(
        &mut self,
        col_ix: usize,
        _sort: ColumnSort,
        _window: &mut Window,
        cx: &mut Context<EditTableState<Self>>,
    ) {
        let next = {
            let mut core = self.core.lock().unwrap();
            core.sort = match core.sort {
                Some((ix, true)) if ix == col_ix => Some((ix, false)),
                Some((ix, false)) if ix == col_ix => None,
                _ => Some((col_ix, true)),
            };
            core.page = 0;
            core.sort.clone()
        };
        let _ = next;
        if let Some(tab) = self.tab.upgrade() {
            let _ = tab.update(cx, |tab, cx| tab.refetch(cx));
        }
        cx.notify();
    }

    fn render_td(
        &mut self,
        row_ix: usize,
        col_ix: usize,
        _window: &mut Window,
        cx: &mut Context<EditTableState<Self>>,
    ) -> impl IntoElement {
        let theme = cx.theme().clone();
        let core = self.core.lock().unwrap();
        let new_row_active = row_ix == NEW_ROW
            || core.new_row.is_some() && row_ix == core.rows.len();
        let deleted = core.deleted.contains(&row_ix);
        let modified = core.modified.contains_key(&(row_ix, col_ix))
            || (new_row_active
                && core
                    .new_row
                    .as_ref()
                    .and_then(|r| r.get(col_ix))
                    .is_some_and(|v| !v.is_empty()));
        let text = core.cell_string(row_ix, col_ix);
        let is_null = !modified
            && !new_row_active
            && core
                .cell_value(row_ix, col_ix)
                .is_some_and(serde_json::Value::is_null);
        drop(core);

        let mut cell = div()
            .px_2()
            .text_ellipsis()
            .overflow_hidden()
            .text_color(theme.colors.foreground);
        if is_null {
            cell = cell
                .text_color(theme.colors.muted_foreground)
                .child("NULL".to_string());
        } else {
            cell = cell.child(text);
        }
        if modified {
            cell = cell.bg(modified_highlight(theme.colors.background));
        }
        if deleted {
            cell = cell
                .text_color(theme.colors.muted_foreground)
                .line_through()
                .bg(modified_highlight(theme.colors.danger));
        }
        cell
    }

    fn build_input(
        &self,
        row_ix: usize,
        col_ix: usize,
        window: &mut Window,
        cx: &mut Context<EditTableState<Self>>,
    ) -> Option<(CellEditor, Vec<gpui::Subscription>)> {
        if self.core.lock().unwrap().deleted.contains(&row_ix) {
            return None;
        }
        let current = self.core.lock().unwrap().cell_string(row_ix, col_ix);
        let input = cx.new(|cx| {
            gpui_component::input::InputState::new(window, cx).placeholder("NULL")
        });
        input.update(cx, |state, cx| state.set_value(current, window, cx));
        Some((CellEditor::Input(input), Vec::new()))
    }

    fn on_cell_edited(
        &mut self,
        row_ix: usize,
        col_ix: usize,
        new_value: String,
        _window: &mut Window,
        cx: &mut Context<EditTableState<Self>>,
    ) -> bool {
        {
            let mut core = self.core.lock().unwrap();
            if row_ix == NEW_ROW || core.new_row.is_some() && row_ix == core.rows.len() {
                if let Some(new_row) = core.new_row.as_mut() {
                    if let Some(slot) = new_row.get_mut(col_ix) {
                        *slot = new_value;
                    }
                }
            } else {
                core.modified.insert((row_ix, col_ix), new_value);
            }
        }
        if let Some(tab) = self.tab.upgrade() {
            let _ = tab.update(cx, |_, cx| cx.notify());
        }
        cx.notify();
        true
    }

    fn is_cell_modified(&self, row_ix: usize, col_ix: usize, _cx: &gpui::App) -> bool {
        let core = self.core.lock().unwrap();
        core.modified.contains_key(&(row_ix, col_ix))
    }

    fn is_row_deleted(&self, row_ix: usize, _cx: &gpui::App) -> bool {
        self.core.lock().unwrap().deleted.contains(&row_ix)
    }

    fn is_row_added(&self, row_ix: usize, _cx: &gpui::App) -> bool {
        self.is_new_row(row_ix)
    }

    fn on_row_added(
        &mut self,
        _window: &mut Window,
        cx: &mut Context<EditTableState<Self>>,
    ) -> usize {
        let view_ix = {
            let mut core = self.core.lock().unwrap();
            if core.new_row.is_none() {
                core.new_row = Some(vec![String::new(); core.columns.len()]);
            }
            core.rows.len()
        };
        if let Some(tab) = self.tab.upgrade() {
            let _ = tab.update(cx, |_, cx| cx.notify());
        }
        cx.notify();
        view_ix
    }

    fn on_row_deleted(
        &mut self,
        row_ix: usize,
        _window: &mut Window,
        cx: &mut Context<EditTableState<Self>>,
    ) {
        {
            let mut core = self.core.lock().unwrap();
            if row_ix == NEW_ROW || core.new_row.is_some() && row_ix == core.rows.len() {
                core.new_row = None;
            } else {
                core.deleted.insert(row_ix);
            }
        }
        if let Some(tab) = self.tab.upgrade() {
            let _ = tab.update(cx, |_, cx| cx.notify());
        }
        cx.notify();
    }
}
