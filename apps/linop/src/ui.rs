//! ratatui 渲染:连接选择器、连接中提示、会话三页(Shell/文件/转发)。

use ratatui::Frame;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, List, ListItem, ListState, Paragraph, Tabs, Wrap};

use crate::app::{App, Dialog, Mode, Pane, SessionState, Tab};
use linop_core::{Auth, endpoint};
use linop_sftp::Entry;

pub fn draw(frame: &mut Frame, app: &mut App) {
    match &mut app.mode {
        Mode::Picker => {
            app.shell_area = None;
            draw_picker(frame, app);
        }
        Mode::Connecting { name } => {
            app.shell_area = None;
            draw_connecting(frame, name);
        }
        Mode::Session(s) => {
            app.shell_area = Some(draw_session(frame, s));
        }
    }
    if let Some(dialog) = &app.dialog {
        draw_dialog(frame, dialog);
    }
}

fn draw_picker(frame: &mut Frame, app: &mut App) {
    let area = frame.area();
    let [body, footer] = Layout::vertical([Constraint::Min(0), Constraint::Length(2)]).areas(area);

    let conns = app.connections();
    let title = format!(" Linop · {} 个连接 ", conns.len());
    let block = Block::default()
        .title(title)
        .borders(Borders::ALL)
        .border_style(Style::new().fg(Color::Cyan));

    if conns.is_empty() {
        let hint = format!(
            "还没有连接配置。\n\n编辑 {} 添加连接,例如:\n\n[my-server]\nhost = \"192.168.1.10\"\nusername = \"root\"\npassword = \"...\"\n\n保存后按 r 重新加载。",
            linop_core::default_connections_path().display()
        );
        frame.render_widget(
            Paragraph::new(hint).wrap(Wrap { trim: false }).block(block),
            body,
        );
    } else {
        let items: Vec<ListItem> = conns
            .iter()
            .map(|c| {
                let auth_label = match &c.auth {
                    Auth::Key { .. } => "密钥",
                    Auth::Password { password } if password.is_empty() => "密码未配置",
                    Auth::Password { .. } => "密码",
                };
                let forwards = if c.forwards.is_empty() {
                    String::new()
                } else {
                    format!("  [{} 条转发]", c.forwards.len())
                };
                ListItem::new(Line::from(vec![
                    Span::styled(format!("{:<16}", c.name), Style::new().bold()),
                    Span::raw(format!(" {:<34}", endpoint(c))),
                    Span::styled(format!("[{auth_label}]"), Style::new().fg(Color::DarkGray)),
                    Span::styled(forwards, Style::new().fg(Color::DarkGray)),
                ]))
            })
            .collect();
        let mut state = ListState::default();
        state.select(Some(app.picker_sel));
        frame.render_stateful_widget(
            List::new(items)
                .block(block)
                .highlight_style(
                    Style::new()
                        .bg(Color::DarkGray)
                        .add_modifier(Modifier::BOLD),
                )
                .highlight_symbol("▶ "),
            body,
            &mut state,
        );
    }

    let mut footer_lines = vec![Line::from("↑/↓ 选择 · Enter 连接 · r 重载 · q 退出")];
    if !app.notice.is_empty() {
        footer_lines.push(Line::from(Span::styled(
            app.notice.clone(),
            Style::new().fg(Color::Yellow),
        )));
    }
    frame.render_widget(
        Paragraph::new(footer_lines).wrap(Wrap { trim: false }),
        footer,
    );
}

fn draw_connecting(frame: &mut Frame, name: &str) {
    let area = frame.area();
    let text = Paragraph::new(format!("正在连接 {name} …\n\n按任意键取消。"))
        .centered()
        .wrap(Wrap { trim: false });
    frame.render_widget(text.centered(), area);
}

/// 绘制会话页,返回 shell 内容区尺寸 (cols, rows)。
fn draw_session(frame: &mut Frame, s: &mut SessionState) -> (u16, u16) {
    let area = frame.area();
    let [tab_bar, body, status_bar] = Layout::vertical([
        Constraint::Length(1),
        Constraint::Min(0),
        Constraint::Length(1),
    ])
    .areas(area);

    let tabs = Tabs::new(vec![" Shell (F1) ", " 文件 (F2) ", " 转发 (F3) "])
        .select(match s.tab {
            Tab::Shell => 0,
            Tab::Files => 1,
            Tab::Forwards => 2,
        })
        .highlight_style(Style::new().bold().fg(Color::Cyan).underlined());
    frame.render_widget(tabs, tab_bar);

    let mut shell_inner = None;
    match s.tab {
        Tab::Shell => {
            let block = Block::default()
                .title(format!(" {} ", endpoint(&s.profile)))
                .borders(Borders::ALL);
            let inner = block.inner(body);
            frame.render_widget(block, body);
            frame.render_widget(&s.term, inner);
            draw_cursor(frame, s, inner);
            shell_inner = Some(inner);
        }
        Tab::Files => draw_files(frame, body, s),
        Tab::Forwards => draw_forwards(frame, body, s),
    }

    let tab_hint = match s.tab {
        Tab::Shell => "PgUp/PgDn 回滚 · F9 断开",
        Tab::Files => "d 下载 · u 上传 · n 新建 · R 改名 · x 删除 · Tab 换窗格",
        Tab::Forwards => "F9 断开",
    };
    let mut status_spans = vec![
        Span::styled(
            format!(" {} ", s.profile.name),
            Style::new().bg(Color::Cyan).fg(Color::Black),
        ),
        Span::raw(" "),
        Span::raw(s.status.clone()),
        Span::styled(format!("  {tab_hint}"), Style::new().fg(Color::DarkGray)),
    ];
    if s.term.scrollback_offset() > 0 {
        let scroll = Span::styled(
            format!(
                " 回滚 {}/{} ",
                s.term.scrollback_offset(),
                s.term.scrollback_len()
            ),
            Style::new().bg(Color::Yellow).fg(Color::Black),
        );
        status_spans.insert(0, scroll);
    }
    frame.render_widget(Paragraph::new(Line::from(status_spans)), status_bar);

    let inner = shell_inner.unwrap_or(body);
    (inner.width, inner.height)
}

/// 在 shell 内容区叠加远端光标(反色);回滚浏览时不显示。
fn draw_cursor(frame: &mut Frame, s: &SessionState, inner: Rect) {
    if s.term.scrollback_offset() > 0 {
        return;
    }
    let Some((col, row)) = s.term.cursor() else {
        return;
    };
    let (x, y) = (
        inner.x + col.min(inner.width.saturating_sub(1)),
        inner.y + row.min(inner.height.saturating_sub(1)),
    );
    if let Some(cell) = frame.buffer_mut().cell_mut((x, y)) {
        cell.set_style(cell.style().add_modifier(Modifier::REVERSED));
    }
}

fn draw_files(frame: &mut Frame, body: Rect, s: &mut SessionState) {
    let [remote_area, local_area] =
        Layout::horizontal([Constraint::Percentage(50), Constraint::Percentage(50)]).areas(body);

    let remote_title = format!(" 远程 {}:{} ", s.profile.host, s.sftp.cwd());
    render_pane(
        frame,
        remote_area,
        &remote_title,
        &s.files.remote_entries,
        s.files.remote_sel,
        s.files.active == Pane::Remote,
    );

    let local_title = format!(" 本地 {} ", s.files.local_cwd.display());
    render_pane(
        frame,
        local_area,
        &local_title,
        &s.files.local_entries,
        s.files.local_sel,
        s.files.active == Pane::Local,
    );
}

fn render_pane(
    frame: &mut Frame,
    area: Rect,
    title: &str,
    entries: &[Entry],
    selected: usize,
    focused: bool,
) {
    let border_style = if focused {
        Style::new().fg(Color::Cyan)
    } else {
        Style::new().fg(Color::DarkGray)
    };
    let block = Block::default()
        .title(title)
        .borders(Borders::ALL)
        .border_style(border_style);
    let inner = block.inner(area);
    frame.render_widget(block, area);

    // 内容列宽:名称占满,大小右对齐在右侧固定区。
    let name_width = inner.width.saturating_sub(12) as usize;
    let items: Vec<ListItem> = entries
        .iter()
        .map(|e| {
            let mut name = e.name.clone();
            if e.is_dir {
                name.push('/');
            }
            let name = truncate(&name, name_width);
            let pad = name_width.saturating_sub(name.chars().count());
            let line = if e.is_dir {
                Line::from(format!("{name}{}{}", " ".repeat(pad), ""))
            } else {
                Line::from(format!("{name}{}{:>10}", " ".repeat(pad), e.size_h))
            };
            ListItem::new(line.style(if e.is_dir {
                Style::new().fg(Color::Blue)
            } else {
                Style::new()
            }))
        })
        .collect();
    let mut state = ListState::default();
    if !entries.is_empty() {
        state.select(Some(selected));
    }
    frame.render_stateful_widget(
        List::new(items).highlight_style(
            Style::new()
                .bg(Color::DarkGray)
                .add_modifier(Modifier::BOLD),
        ),
        inner,
        &mut state,
    );
}

fn draw_forwards(frame: &mut Frame, body: Rect, s: &mut SessionState) {
    let block = Block::default()
        .title(" 本地端口转发 ")
        .borders(Borders::ALL);
    let inner = block.inner(body);
    frame.render_widget(block, body);

    let mut items: Vec<ListItem> = s
        .forwards
        .iter()
        .map(|f| {
            ListItem::new(Line::from(vec![
                Span::styled("● 运行中  ", Style::new().fg(Color::Green)),
                Span::styled(f.to_string(), Style::new().bold()),
            ]))
        })
        .collect();
    items.extend(s.forward_failures.iter().map(|(fwd, err)| {
        ListItem::new(Line::from(vec![
            Span::styled("✗ 失败    ", Style::new().fg(Color::Red)),
            Span::styled(
                format!(
                    "127.0.0.1:{} → {}:{}",
                    fwd.local_port, fwd.remote_host, fwd.remote_port
                ),
                Style::new().bold(),
            ),
            Span::styled(format!("  {err}"), Style::new().fg(Color::DarkGray)),
        ]))
    }));
    if items.is_empty() {
        items.push(ListItem::new(
            "没有自动启动的转发。在 connections.toml 中为连接配置 [[连接.forwards]]。",
        ));
    }
    frame.render_widget(List::new(items), inner);
}

/// 居中模态对话框:标题 + 说明 + 输入行(█ 作光标)/确认行 + 错误行。
fn draw_dialog(frame: &mut Frame, dialog: &Dialog) {
    let area = frame.area();
    let width = 56.min(area.width.saturating_sub(4));
    let height = 5u16;
    let rect = Rect {
        x: area.x + (area.width.saturating_sub(width)) / 2,
        y: area.y + (area.height.saturating_sub(height)) / 2,
        width,
        height,
    };
    frame.render_widget(Clear, rect);
    let block = Block::default()
        .title(format!(" {} ", dialog.title))
        .borders(Borders::ALL)
        .border_style(Style::new().fg(Color::Yellow));
    let inner = block.inner(rect);
    frame.render_widget(block, rect);

    let mut lines = vec![Line::from(dialog.prompt.clone())];
    if dialog.action.needs_input() {
        lines.push(Line::from(format!("{}█", dialog.input)));
        lines.push(Line::from(Span::styled(
            "Enter 确认 · Esc 取消",
            Style::new().fg(Color::DarkGray),
        )));
    } else {
        lines.push(Line::from(""));
    }
    if !dialog.error.is_empty() {
        lines.push(Line::from(Span::styled(
            dialog.error.clone(),
            Style::new().fg(Color::Red),
        )));
    }
    frame.render_widget(Paragraph::new(lines).wrap(Wrap { trim: false }), inner);
}

/// 按显示宽度截断,保留前缀。
fn truncate(s: &str, width: usize) -> String {
    if s.chars().count() <= width {
        return s.to_string();
    }
    s.chars().take(width.saturating_sub(1)).collect::<String>() + "…"
}
