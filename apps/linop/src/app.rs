//! 应用状态机与异步事件循环。
//!
//! 事件来源:专门线程阻塞读 crossterm 输入、shell 输出转发任务、
//! 连接任务回执。SFTP 操作在事件循环内联执行(v0.1 足够:
//! 操作期间的 shell 输出会先在通道中排队)。

use std::path::{Path, PathBuf};

use crossterm::event::{Event as CrosstermEvent, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use linop_core::{Auth, Connection, Registry};
use linop_sftp::{Entry, Sftp};
use linop_ssh::{KnownHosts, KnownHostsPolicy, LocalForward, Session, ShellReader, ShellWriter};
use linop_term::Terminal;
use tokio::sync::mpsc;

use crate::keys;

/// 事件循环事件。Resize 不带载荷:尺寸由下一次 draw + sync_size 自取。
pub enum AppEvent {
    Key(KeyEvent),
    Paste(String),
    Resize,
    ShellOutput(Vec<u8>),
    ShellClosed,
    Connected(Result<Box<Connected>, String>),
}

/// 一次已建立的连接及其全部资源。
pub struct Connected {
    pub profile: Connection,
    pub ssh: Session,
    pub writer: ShellWriter,
    pub reader: ShellReader,
    pub sftp: Sftp,
    pub forwards: Vec<LocalForward>,
    /// 启动失败的转发规则及原因(不致命,进转发页与状态栏)。
    pub forward_errors: Vec<(linop_core::PortForward, String)>,
}

/// 模态对话框动作:决定 Enter 时执行什么、是否需要文本输入。
pub enum DialogAction {
    MkdirRemote,
    MkdirLocal,
    RenameRemote { old: String },
    RenameLocal { old: String },
    DeleteRemote { entry: Entry },
    DeleteLocal { entry: Entry },
}

impl DialogAction {
    /// 输入型对话框(新建/重命名)需要文本框;删除只需确认。
    pub fn needs_input(&self) -> bool {
        !matches!(
            self,
            DialogAction::DeleteRemote { .. } | DialogAction::DeleteLocal { .. }
        )
    }
}

/// 模态对话框:文本输入或删除确认,覆盖在文件页上方。
pub struct Dialog {
    pub title: String,
    pub prompt: String,
    pub input: String,
    pub action: DialogAction,
    /// 校验/执行错误,显示在对话框底部。
    pub error: String,
    /// 执行成功后写入状态栏的消息(由 execute_dialog 填写)。
    pub status: String,
}

impl Dialog {
    fn status_message(&self) -> &str {
        &self.status
    }
}

/// 顶层模式。
pub enum Mode {
    /// 连接列表。
    Picker,
    /// 连接建立中。
    Connecting { name: String },
    /// 已连接会话。
    Session(Box<SessionState>),
}

/// 会话内标签页。
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Tab {
    Shell,
    Files,
    Forwards,
}

/// 文件页中当前操作的窗格。
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Pane {
    Remote,
    Local,
}

pub struct FilesState {
    pub local_cwd: PathBuf,
    pub local_entries: Vec<Entry>,
    pub remote_entries: Vec<Entry>,
    pub remote_sel: usize,
    pub local_sel: usize,
    pub active: Pane,
}

pub struct SessionState {
    pub profile: Connection,
    pub ssh: Session,
    pub writer: ShellWriter,
    pub sftp: Sftp,
    pub forwards: Vec<LocalForward>,
    /// 启动失败的转发规则及原因。
    pub forward_failures: Vec<(linop_core::PortForward, String)>,
    pub term: Terminal,
    pub tab: Tab,
    pub files: FilesState,
    /// 底部状态栏消息(最近一次操作的反馈)。
    pub status: String,
}

pub struct App {
    pub registry: Registry,
    pub picker_sel: usize,
    /// picker 页全局提示(错误等)。
    pub notice: String,
    pub mode: Mode,
    /// 打开中的模态对话框(文件页操作)。
    pub dialog: Option<Dialog>,
    /// draw 阶段记录的 shell 内容区尺寸;事件循环据此同步仿真器与 PTY。
    pub shell_area: Option<(u16, u16)>,
}

impl App {
    pub fn new(registry: Registry) -> Self {
        Self {
            registry,
            picker_sel: 0,
            notice: String::new(),
            mode: Mode::Picker,
            dialog: None,
            shell_area: None,
        }
    }

    pub fn connections(&self) -> &[Connection] {
        self.registry.connections()
    }

    fn move_picker(&mut self, delta: i32) {
        let len = self.connections().len();
        if len == 0 {
            return;
        }
        self.picker_sel = (self.picker_sel as i32 + delta).rem_euclid(len as i32) as usize;
    }

    /// 开始连接指定 profile。
    pub fn connect(&mut self, conn: &Connection, tx: &mpsc::UnboundedSender<AppEvent>) {
        let (cols, rows) = self.shell_area.unwrap_or((80, 24));
        self.mode = Mode::Connecting {
            name: conn.name.clone(),
        };
        self.notice.clear();
        spawn_connect(tx.clone(), conn.clone(), cols, rows);
    }

    /// 事件处理;返回 false 结束事件循环。
    pub async fn handle(&mut self, event: AppEvent, tx: &mpsc::UnboundedSender<AppEvent>) -> bool {
        match event {
            AppEvent::Connected(result) => self.on_connected(result, tx).await,
            AppEvent::Key(key) if key.kind == KeyEventKind::Press => {
                return self.on_key(key, tx).await;
            }
            AppEvent::Key(_) => {}
            AppEvent::Paste(text) => {
                if let Mode::Session(s) = &mut self.mode
                    && s.tab == Tab::Shell
                {
                    let bytes = keys::paste(&text, s.term.bracketed_paste());
                    let _ = s.writer.write(&bytes).await;
                    s.term.set_scrollback(0);
                }
            }
            AppEvent::Resize => {}
            AppEvent::ShellOutput(bytes) => {
                if let Mode::Session(s) = &mut self.mode {
                    s.term.feed(&bytes);
                }
            }
            AppEvent::ShellClosed => {
                if let Some(s) = self.mode_swap_to_picker() {
                    self.notice = format!("{} 的会话已结束", s.profile.name);
                    teardown_session(s).await;
                }
            }
        }
        true
    }

    async fn on_key(&mut self, key: KeyEvent, tx: &mpsc::UnboundedSender<AppEvent>) -> bool {
        // 模态对话框优先接管按键。
        if let Some(dialog) = self.dialog.take() {
            self.handle_dialog_key(key, dialog).await;
            return true;
        }
        match &mut self.mode {
            Mode::Picker => match key.code {
                KeyCode::Char('q') | KeyCode::Esc => return false,
                KeyCode::Up | KeyCode::Char('k') => self.move_picker(-1),
                KeyCode::Down | KeyCode::Char('j') => self.move_picker(1),
                KeyCode::Enter => {
                    if let Some(conn) = self.connections().get(self.picker_sel).cloned() {
                        self.connect(&conn, tx);
                    }
                }
                KeyCode::Char('r') => match Registry::load_default() {
                    Ok(reg) => {
                        self.registry = reg;
                        self.picker_sel = 0;
                        self.notice = "已重新加载连接列表".into();
                    }
                    Err(e) => self.notice = format!("重新加载失败:{e}"),
                },
                _ => {}
            },
            Mode::Connecting { .. } => {
                // 任意键取消连接:回到列表;迟到的结果由 on_connected 丢弃。
                self.mode = Mode::Picker;
            }
            Mode::Session(s) => {
                if key.modifiers.contains(KeyModifiers::CONTROL)
                    && matches!(key.code, KeyCode::Char('q') | KeyCode::Char('Q'))
                {
                    let session = self.mode_swap_to_picker();
                    if let Some(s) = session {
                        teardown_session(s).await;
                    }
                    return false;
                }
                match key.code {
                    KeyCode::F(1) => s.tab = Tab::Shell,
                    KeyCode::F(2) => s.tab = Tab::Files,
                    KeyCode::F(3) => s.tab = Tab::Forwards,
                    KeyCode::F(9) => {
                        let session = self.mode_swap_to_picker();
                        if let Some(s) = session {
                            self.notice = format!("已断开 {}", s.profile.name);
                            teardown_session(s).await;
                        }
                    }
                    _ => self.session_key(key).await,
                }
            }
        }
        true
    }

    /// 对话框按键:Esc 关闭、Enter 执行、退格/字符编辑输入。
    async fn handle_dialog_key(&mut self, key: KeyEvent, mut dialog: Dialog) {
        match key.code {
            KeyCode::Esc => {}
            KeyCode::Enter => {
                dialog.error.clear();
                match self.execute_dialog(&mut dialog).await {
                    Ok(()) => {
                        self.status_to_session(dialog.status_message());
                    }
                    Err(msg) => {
                        dialog.error = msg;
                        self.dialog = Some(dialog);
                    }
                }
            }
            KeyCode::Backspace => {
                dialog.input.pop();
                self.dialog = Some(dialog);
            }
            KeyCode::Char(c)
                if !key
                    .modifiers
                    .intersects(KeyModifiers::CONTROL.union(KeyModifiers::ALT)) =>
            {
                dialog.input.push(c);
                self.dialog = Some(dialog);
            }
            _ => self.dialog = Some(dialog),
        }
    }

    fn status_to_session(&mut self, message: &str) {
        if let Mode::Session(s) = &mut self.mode {
            s.status = message.to_string();
        }
    }

    /// 执行对话框动作;成功时已顺手刷新对应窗格。
    async fn execute_dialog(&mut self, dialog: &mut Dialog) -> Result<(), String> {
        let Mode::Session(s) = &mut self.mode else {
            return Err("会话已断开".to_string());
        };
        let name = dialog.input.trim();
        match &dialog.action {
            DialogAction::MkdirRemote => {
                if name.is_empty() {
                    return Err("目录名不能为空".into());
                }
                s.sftp.mkdir(name).await.map_err(|e| e.to_string())?;
                s.refresh_remote().await;
                dialog.status = format!("已创建远端目录 {name}");
            }
            DialogAction::MkdirLocal => {
                if name.is_empty() {
                    return Err("目录名不能为空".into());
                }
                std::fs::create_dir(s.files.local_cwd.join(name)).map_err(|e| e.to_string())?;
                s.refresh_local();
                dialog.status = format!("已创建本地目录 {name}");
            }
            DialogAction::RenameRemote { old } => {
                if name.is_empty() {
                    return Err("新名称不能为空".into());
                }
                s.sftp.rename(old, name).await.map_err(|e| e.to_string())?;
                s.refresh_remote().await;
                dialog.status = format!("远端 {old} 已改名为 {name}");
            }
            DialogAction::RenameLocal { old } => {
                if name.is_empty() {
                    return Err("新名称不能为空".into());
                }
                let from = s.files.local_cwd.join(old);
                let to = s.files.local_cwd.join(name);
                std::fs::rename(&from, &to).map_err(|e| e.to_string())?;
                s.refresh_local();
                dialog.status = format!("本地 {old} 已改名为 {name}");
            }
            DialogAction::DeleteRemote { entry } => {
                if entry.is_dir {
                    s.sftp
                        .remove_dir(&entry.name)
                        .await
                        .map_err(|e| e.to_string())?;
                } else {
                    s.sftp
                        .remove_file(&entry.name)
                        .await
                        .map_err(|e| e.to_string())?;
                }
                s.clamp_selection();
                s.refresh_remote().await;
                dialog.status = format!("已删除远端 {}", entry.name);
            }
            DialogAction::DeleteLocal { entry } => {
                let path = s.files.local_cwd.join(&entry.name);
                if entry.is_dir {
                    // 只删空目录,避免误删整棵子树。
                    std::fs::remove_dir(&path).map_err(|e| e.to_string())?;
                } else {
                    std::fs::remove_file(&path).map_err(|e| e.to_string())?;
                }
                s.clamp_selection();
                s.refresh_local();
                dialog.status = format!("已删除本地 {}", entry.name);
            }
        }
        Ok(())
    }

    async fn session_key(&mut self, key: KeyEvent) {
        let Mode::Session(s) = &mut self.mode else {
            unreachable!()
        };
        match s.tab {
            Tab::Shell => match key.code {
                KeyCode::PageUp => s.term.scroll(1),
                KeyCode::PageDown => s.term.scroll(-1),
                _ => {
                    if let Some(bytes) = keys::encode(key, s.term.application_cursor()) {
                        let _ = s.writer.write(&bytes).await;
                        s.term.set_scrollback(0);
                    }
                }
            },
            Tab::Files => match key.code {
                KeyCode::Tab | KeyCode::Left | KeyCode::Right => {
                    s.files.active = match s.files.active {
                        Pane::Remote => Pane::Local,
                        Pane::Local => Pane::Remote,
                    };
                }
                KeyCode::Up | KeyCode::Char('k') => s.move_selection(-1),
                KeyCode::Down | KeyCode::Char('j') => s.move_selection(1),
                KeyCode::Backspace => s.enter_parent().await,
                KeyCode::Enter => s.enter_selected().await,
                KeyCode::Char('d') => s.download_selected().await,
                KeyCode::Char('u') => s.upload_selected().await,
                KeyCode::Char('n') => {
                    let (title, action) = match s.files.active {
                        Pane::Remote => ("新建远端目录", DialogAction::MkdirRemote),
                        Pane::Local => ("新建本地目录", DialogAction::MkdirLocal),
                    };
                    self.dialog = Some(new_dialog(title, "目录名:", action));
                }
                KeyCode::Char('R') => {
                    let spec = match s.files.active {
                        Pane::Remote => s.files.remote_entries.get(s.files.remote_sel).map(|e| {
                            (
                                DialogAction::RenameRemote {
                                    old: e.name.clone(),
                                },
                                e.name.clone(),
                            )
                        }),
                        Pane::Local => s.files.local_entries.get(s.files.local_sel).map(|e| {
                            (
                                DialogAction::RenameLocal {
                                    old: e.name.clone(),
                                },
                                e.name.clone(),
                            )
                        }),
                    };
                    if let Some((action, old)) = spec {
                        self.dialog =
                            Some(new_dialog("重命名", &format!("{old} 的新名称:"), action));
                    }
                }
                KeyCode::Char('x') => {
                    let spec = match s.files.active {
                        Pane::Remote => s
                            .files
                            .remote_entries
                            .get(s.files.remote_sel)
                            .cloned()
                            .map(|entry| (DialogAction::DeleteRemote { entry }, "远端")),
                        Pane::Local => s
                            .files
                            .local_entries
                            .get(s.files.local_sel)
                            .cloned()
                            .map(|entry| (DialogAction::DeleteLocal { entry }, "本地")),
                    };
                    if let Some((action, side)) = spec {
                        let name = match &action {
                            DialogAction::DeleteRemote { entry }
                            | DialogAction::DeleteLocal { entry } => entry.name.clone(),
                            _ => unreachable!(),
                        };
                        self.dialog = Some(new_dialog(
                            "确认删除",
                            &format!("删除{side} {name}?(目录须为空) Enter 确认 / Esc 取消"),
                            action,
                        ));
                    }
                }
                _ => {}
            },
            Tab::Forwards => {}
        }
    }

    async fn on_connected(
        &mut self,
        result: Result<Box<Connected>, String>,
        tx: &mpsc::UnboundedSender<AppEvent>,
    ) {
        match result {
            Ok(boxed) => {
                if !matches!(self.mode, Mode::Connecting { .. }) {
                    // 用户已取消:迟到的连接结果直接回收。
                    let conn = *boxed;
                    teardown_parts(conn.profile, conn.ssh, conn.sftp, conn.forwards).await;
                    return;
                }
                let Connected {
                    profile,
                    ssh,
                    writer,
                    mut reader,
                    sftp,
                    forwards,
                    forward_errors,
                } = *boxed;
                let tx = tx.clone();
                tokio::spawn(async move {
                    while let Some(bytes) = reader.read_output().await {
                        if tx.send(AppEvent::ShellOutput(bytes)).is_err() {
                            break;
                        }
                    }
                    let _ = tx.send(AppEvent::ShellClosed);
                });

                let (remote_entries, warn) = match sftp.list_dir(".").await {
                    Ok(entries) => (entries, None),
                    Err(e) => (Vec::new(), Some(format!("SFTP 目录读取失败:{e}"))),
                };
                let local_cwd = dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));
                let local_entries = list_local(&local_cwd);
                let forward_note = (!forward_errors.is_empty()).then(|| {
                    forward_errors
                        .iter()
                        .map(|(_, err)| err.clone())
                        .collect::<Vec<_>>()
                        .join(";")
                });
                let status = warn.or(forward_note).unwrap_or_else(|| {
                    format!("已连接 {},自动转发 {} 条", profile.name, forwards.len())
                });
                let (cols, rows) = self.shell_area.unwrap_or((80, 24));
                self.mode = Mode::Session(Box::new(SessionState {
                    profile,
                    ssh,
                    writer,
                    sftp,
                    forwards,
                    forward_failures: forward_errors,
                    term: Terminal::new(cols, rows),
                    tab: Tab::Shell,
                    files: FilesState {
                        local_cwd,
                        local_entries,
                        remote_entries,
                        remote_sel: 0,
                        local_sel: 0,
                        active: Pane::Remote,
                    },
                    status,
                }));
            }
            Err(msg) => {
                if matches!(self.mode, Mode::Connecting { .. }) {
                    self.notice = msg;
                    self.mode = Mode::Picker;
                }
            }
        }
    }

    /// 把 Session 模式换回 Picker 并取走会话状态;非会话模式返回 None。
    fn mode_swap_to_picker(&mut self) -> Option<SessionState> {
        if matches!(self.mode, Mode::Session(_)) {
            match std::mem::replace(&mut self.mode, Mode::Picker) {
                Mode::Session(s) => Some(*s),
                _ => unreachable!(),
            }
        } else {
            None
        }
    }

    /// draw 之后调用:把仿真器与远端 PTY 同步到布局给出的内容区尺寸。
    pub async fn sync_size(&mut self) {
        let Some((cols, rows)) = self.shell_area else {
            return;
        };
        let Mode::Session(s) = &mut self.mode else {
            return;
        };
        if s.term.cols() != cols || s.term.rows() != rows {
            s.term.resize(cols, rows);
            let _ = s.writer.resize(cols, rows).await;
        }
    }
}

impl SessionState {
    /// 刷新远端条目缓存(保持 cwd 不变)。
    async fn refresh_remote(&mut self) {
        if let Ok(entries) = self.sftp.list_dir(".").await {
            self.files.remote_entries = entries;
            self.clamp_selection();
        }
    }

    fn refresh_local(&mut self) {
        self.files.local_entries = list_local(&self.files.local_cwd);
        self.clamp_selection();
    }

    /// 条目变少后把两个选中游标拉回有效范围。
    fn clamp_selection(&mut self) {
        self.files.remote_sel = self
            .files
            .remote_sel
            .min(self.files.remote_entries.len().saturating_sub(1));
        self.files.local_sel = self
            .files
            .local_sel
            .min(self.files.local_entries.len().saturating_sub(1));
    }

    fn move_selection(&mut self, delta: i32) {
        let (sel, len) = match self.files.active {
            Pane::Remote => (&mut self.files.remote_sel, self.files.remote_entries.len()),
            Pane::Local => (&mut self.files.local_sel, self.files.local_entries.len()),
        };
        if len > 0 {
            *sel = (*sel as i32 + delta).clamp(0, len as i32 - 1) as usize;
        }
    }

    async fn enter_selected(&mut self) {
        match self.files.active {
            Pane::Remote => {
                let Some(entry) = self
                    .files
                    .remote_entries
                    .get(self.files.remote_sel)
                    .cloned()
                else {
                    return;
                };
                if !entry.is_dir {
                    self.status = format!("{} 不是目录", entry.name);
                    return;
                }
                match self.sftp.enter(&entry.name).await {
                    Ok(entries) => {
                        self.files.remote_entries = entries;
                        self.files.remote_sel = 0;
                    }
                    Err(e) => self.status = format!("进入目录失败:{e}"),
                }
            }
            Pane::Local => {
                let Some(entry) = self.files.local_entries.get(self.files.local_sel).cloned()
                else {
                    return;
                };
                if !entry.is_dir {
                    self.status = format!("{} 不是目录", entry.name);
                    return;
                }
                self.files.local_cwd.push(&entry.name);
                self.files.local_entries = list_local(&self.files.local_cwd);
                self.files.local_sel = 0;
            }
        }
    }

    async fn enter_parent(&mut self) {
        match self.files.active {
            Pane::Remote => match self.sftp.go_parent().await {
                Ok(entries) => {
                    self.files.remote_entries = entries;
                    self.files.remote_sel = 0;
                }
                Err(e) => self.status = format!("返回上级失败:{e}"),
            },
            Pane::Local => {
                self.files.local_cwd.pop();
                self.files.local_entries = list_local(&self.files.local_cwd);
                self.files.local_sel = 0;
            }
        }
    }

    /// 下载远端选中项到本地窗格当前目录。
    async fn download_selected(&mut self) {
        let Some(entry) = self
            .files
            .remote_entries
            .get(self.files.remote_sel)
            .cloned()
        else {
            return;
        };
        let target = self.files.local_cwd.join(&entry.name);
        match self.sftp.download(&entry.name, &target).await {
            Ok(()) => {
                self.files.local_entries = list_local(&self.files.local_cwd);
                self.status = format!("已下载 {} → {}", entry.name, target.display());
            }
            Err(e) => self.status = format!("下载失败:{e}"),
        }
    }

    /// 上传本地选中项到远端当前目录。
    async fn upload_selected(&mut self) {
        let Some(entry) = self.files.local_entries.get(self.files.local_sel).cloned() else {
            return;
        };
        let source = self.files.local_cwd.join(&entry.name);
        match self.sftp.upload(&source, &entry.name).await {
            Ok(()) => {
                if let Ok(entries) = self.sftp.list_dir(".").await {
                    self.files.remote_entries = entries;
                }
                self.status = format!(
                    "已上传 {} → {}:{}",
                    entry.name,
                    self.profile.host,
                    self.sftp.cwd()
                );
            }
            Err(e) => self.status = format!("上传失败:{e}"),
        }
    }
}

/// 构造一个空输入的对话框。
fn new_dialog(title: &str, prompt: &str, action: DialogAction) -> Dialog {
    Dialog {
        title: title.to_string(),
        prompt: prompt.to_string(),
        input: String::new(),
        action,
        error: String::new(),
        status: String::new(),
    }
}

/// 列出本地目录,排序规则与远端一致(目录优先,再按名称)。
pub fn list_local(path: &Path) -> Vec<Entry> {
    let Ok(read_dir) = std::fs::read_dir(path) else {
        return Vec::new();
    };
    let mut entries: Vec<Entry> = read_dir
        .filter_map(|e| e.ok())
        .map(|e| {
            let md = e.metadata().ok();
            let is_dir = md.as_ref().is_some_and(|m| m.is_dir());
            let size = md.as_ref().map_or(0, |m| m.len());
            Entry {
                name: e.file_name().to_string_lossy().to_string(),
                is_dir,
                size,
                size_h: linop_sftp::human_size(size),
            }
        })
        .filter(|e| e.name != "." && e.name != "..")
        .collect();
    entries.sort_by(|a, b| b.is_dir.cmp(&a.is_dir).then_with(|| a.name.cmp(&b.name)));
    entries
}

/// 会话资源回收:停转发、关 SFTP、断开连接。
async fn teardown_session(s: SessionState) {
    teardown_parts(s.profile, s.ssh, s.sftp, s.forwards).await;
}

async fn teardown_parts(
    _profile: Connection,
    ssh: Session,
    sftp: Sftp,
    forwards: Vec<LocalForward>,
) {
    for forward in forwards {
        forward.stop();
    }
    let _ = sftp.close().await;
    let _ = ssh.disconnect().await;
}

/// 阻塞读 crossterm 事件并转发到事件循环。
pub fn spawn_input_thread(tx: mpsc::UnboundedSender<AppEvent>) {
    std::thread::spawn(move || {
        loop {
            match crossterm::event::read() {
                Ok(CrosstermEvent::Key(key)) => {
                    if tx.send(AppEvent::Key(key)).is_err() {
                        break;
                    }
                }
                Ok(CrosstermEvent::Paste(text)) => {
                    if tx.send(AppEvent::Paste(text)).is_err() {
                        break;
                    }
                }
                Ok(CrosstermEvent::Resize(..)) => {
                    if tx.send(AppEvent::Resize).is_err() {
                        break;
                    }
                }
                Ok(_) => {}
                Err(_) => break,
            }
        }
    });
}

/// 主事件循环:draw → 同步尺寸 → 等事件。
pub async fn run(
    terminal: &mut ratatui::DefaultTerminal,
    mut app: App,
    mut rx: mpsc::UnboundedReceiver<AppEvent>,
    tx: mpsc::UnboundedSender<AppEvent>,
) -> anyhow::Result<()> {
    loop {
        terminal.draw(|frame| crate::ui::draw(frame, &mut app))?;
        app.sync_size().await;
        let Some(event) = rx.recv().await else { break };
        if !app.handle(event, &tx).await {
            break;
        }
    }
    Ok(())
}

fn spawn_connect(tx: mpsc::UnboundedSender<AppEvent>, profile: Connection, cols: u16, rows: u16) {
    tokio::spawn(async move {
        let _ = tx.send(AppEvent::Connected(
            connect(profile, cols, rows).await.map(Box::new),
        ));
    });
}

/// 建立连接:认证 → shell → SFTP → 启动 profile 里的全部转发。
async fn connect(profile: Connection, cols: u16, rows: u16) -> Result<Connected, String> {
    if let Auth::Password { password } = &profile.auth
        && password.is_empty()
    {
        return Err(format!(
            "连接 {} 未配置密码,请在 {} 的对应表里补全 password 或改用 key_path",
            profile.name,
            linop_core::default_connections_path().display()
        ));
    }
    let policy = KnownHostsPolicy::TrustOnFirstUse(KnownHosts::load_default());
    let ssh = Session::connect_profile(&profile, policy)
        .await
        .map_err(|e| e.to_string())?;
    let shell = ssh
        .open_shell(cols, rows)
        .await
        .map_err(|e| format!("打开 shell 失败:{e}"))?;
    let (reader, writer) = shell.into_parts();
    let sftp = Sftp::open(&ssh)
        .await
        .map_err(|e| format!("打开 SFTP 失败:{e}"))?;
    let mut forwards = Vec::new();
    let mut forward_errors = Vec::new();
    for fwd in &profile.forwards {
        match ssh.open_local_forward(fwd).await {
            Ok(running) => forwards.push(running),
            Err(e) => forward_errors.push((
                fwd.clone(),
                format!(
                    "转发 127.0.0.1:{} → {}:{} 启动失败:{e}",
                    fwd.local_port, fwd.remote_host, fwd.remote_port
                ),
            )),
        }
    }
    Ok(Connected {
        profile,
        ssh,
        writer,
        reader,
        sftp,
        forwards,
        forward_errors,
    })
}
