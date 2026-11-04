//! 应用状态机与异步事件循环。
//!
//! 事件来源:专门线程阻塞读 crossterm 输入、shell 输出转发任务、
//! 连接任务回执。SFTP 操作在事件循环内联执行(v0.1 足够:
//! 操作期间的 shell 输出会先在通道中排队)。

use std::collections::VecDeque;
use std::future::Future;
use std::path::{Path, PathBuf};
use std::time::Duration;

use crossterm::event::{Event as CrosstermEvent, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use linop_cast::{Cast, CastWriter};
use linop_core::{Auth, Connection, PortForward, Registry};
use linop_db::{QueryResult, SqliteDb, StmtOutcome};
use linop_monitor::ResourceSample;
use linop_redis::{KeyInfo as RedisKeyInfo, KeyValue, RedisClient};
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
    ShellOutput(u64, Vec<u8>),
    ShellClosed(u64),
    Connected(Result<Box<Connected>, String>),
    /// 输入设备失效(非终端运行 / stdin 关闭):无法再交互,退出应用。
    InputClosed,
    /// 后台传输进度(会话 ID,累计字节)。
    TransferProgress(u64, u64),
    /// 后台传输结束(会话 ID;Ok 为累计字节,Err 中 "已取消" 表示用户取消)。
    TransferFinished(u64, Result<u64, String>),
    /// 自动重连:单次尝试完成(成功=新连接资源,失败=原因)。
    ReconnectDone(u64, Result<Box<Connected>, String>),
    /// 自动重连:退避等待结束,发起下一次尝试。
    ReconnectTick(u64),
    /// 监控采样到达(会话 ID;Err 为采样/解析错误)。
    MonitorSample(u64, Result<ResourceSample, String>),
    /// 回放:到点应用下一个事件。
    ReplayTick,
}

// --- 防挂死超时常量:任何网络 await 都有上限,坏连接不允许冻结事件循环 ---
/// 连接全过程(TCP + 握手 + 认证 + shell/SFTP + 转发)。
const CONNECT_TIMEOUT_SECS: u64 = 25;
/// 按键写入 / PTY 尺寸同步。
const WRITE_TIMEOUT_SECS: u64 = 5;
/// 目录浏览等元数据操作。
const SFTP_META_TIMEOUT_SECS: u64 = 15;
/// 上传 / 下载。
const SFTP_TRANSFER_TIMEOUT_SECS: u64 = 60;
/// 断开清理。
const TEARDOWN_TIMEOUT_SECS: u64 = 5;

/// 给网络操作加超时上限:超时返回 Err(提示连接可能已断开),
/// 业务错误转成消息字符串。事件循环因此永远不会被单个 await 挂死。
async fn capped<T, E: std::fmt::Display>(
    secs: u64,
    fut: impl Future<Output = std::result::Result<T, E>>,
) -> Result<T, String> {
    match tokio::time::timeout(Duration::from_secs(secs), fut).await {
        Ok(Ok(value)) => Ok(value),
        Ok(Err(err)) => Err(err.to_string()),
        Err(_) => Err("操作超时:连接可能已断开(F9 断开)".to_string()),
    }
}

/// 一次已建立的连接及其全部资源。
pub struct Connected {
    pub profile: Connection,
    pub ssh: Session,
    pub writer: ShellWriter,
    pub reader: ShellReader,
    pub sftp: Sftp,
    pub forwards: Vec<LocalForward>,
    pub remote_forwards: Vec<linop_ssh::RemoteForward>,
    pub dynamic_forwards: Vec<linop_ssh::DynamicForward>,
    /// 启动失败的转发及原因(不致命,进转发页与状态栏)。
    pub forward_errors: Vec<String>,
}

/// 模态对话框动作:决定 Enter 时执行什么、是否需要文本输入。
pub enum DialogAction {
    MkdirRemote,
    MkdirLocal,
    RenameRemote {
        old: String,
    },
    RenameLocal {
        old: String,
    },
    DeleteRemote {
        entry: Entry,
    },
    DeleteLocal {
        entry: Entry,
    },
    /// 在 picker 上就地输入密码后连接(仅本次会话生效,不落盘)。
    ConnectPassword(Box<Connection>),
    /// 在 picker 上就地输入私钥口令后连接(仅本次会话生效,不落盘)。
    ConnectPassphrase(Box<Connection>),
    /// 删除一个连接 profile(picker 上执行,带确认)。
    DeleteProfile(String),
    /// 从 ~/.ssh/config 导入连接(picker 上执行,带确认)。
    ImportProfiles(Vec<Connection>),
    /// 回放指定 .cast 文件(picker 上执行,输入路径)。
    OpenReplay,
    /// 在数据库浏览中执行 SQL(输入框,Enter 执行)。
    RunSql,
    /// 删除一个 Redis 键(浏览模式内,带确认)。
    DeleteRedisKey {
        name: String,
    },
    /// 删除一个数据库 profile(picker 上执行,带确认)。
    DeleteDbProfile(String),
}

impl DialogAction {
    /// 输入型对话框(新建/重命名)需要文本框;删除/导入只需确认。
    pub fn needs_input(&self) -> bool {
        !matches!(
            self,
            DialogAction::DeleteRemote { .. } | DialogAction::DeleteLocal { .. }
        ) && !matches!(
            self,
            DialogAction::ImportProfiles(_) | DialogAction::DeleteDbProfile(_)
        )
    }
}

/// 模态对话框:文本输入或删除确认,覆盖在文件页/连接列表上方。
pub struct Dialog {
    pub title: String,
    pub prompt: String,
    pub input: String,
    pub action: DialogAction,
    /// 校验/执行错误,显示在对话框底部。
    pub error: String,
    /// 执行成功后写入状态栏的消息(由 execute_dialog 填写)。
    pub status: String,
    /// 掩码输入(密码):界面显示 ●,不显示明文。
    pub secret: bool,
}

impl Dialog {
    fn status_message(&self) -> &str {
        &self.status
    }
}

/// 表单单个字段。
pub struct FormField {
    pub label: &'static str,
    pub value: String,
    /// 掩码输入(密码)。
    pub secret: bool,
}

/// 连接编辑表单:新建或修改 profile。字段固定顺序:
/// 名称/主机/端口/用户名/密码/密钥路径(密钥优先,密码可为空=连接时输入)。
pub struct ConnectionForm {
    /// 编辑目标的现有连接名;None 表示新建。
    pub original: Option<String>,
    pub fields: Vec<FormField>,
    pub focus: usize,
    /// 校验/保存错误。
    pub error: String,
}

/// 字段下标。
const F_NAME: usize = 0;
const F_HOST: usize = 1;
const F_PORT: usize = 2;
const F_USER: usize = 3;
const F_PASSWORD: usize = 4;
const F_KEY: usize = 5;

impl ConnectionForm {
    fn blank() -> Self {
        Self {
            original: None,
            fields: vec![
                FormField {
                    label: "名称   ",
                    value: String::new(),
                    secret: false,
                },
                FormField {
                    label: "主机   ",
                    value: String::new(),
                    secret: false,
                },
                FormField {
                    label: "端口   ",
                    value: "22".to_string(),
                    secret: false,
                },
                FormField {
                    label: "用户名 ",
                    value: String::new(),
                    secret: false,
                },
                FormField {
                    label: "密码   ",
                    value: String::new(),
                    secret: true,
                },
                FormField {
                    label: "密钥路径",
                    value: String::new(),
                    secret: false,
                },
            ],
            focus: 0,
            error: String::new(),
        }
    }

    fn edit(conn: &Connection) -> Self {
        let mut form = Self::blank();
        form.original = Some(conn.name.clone());
        form.fields[F_NAME].value = conn.name.clone();
        form.fields[F_HOST].value = conn.host.clone();
        form.fields[F_PORT].value = conn.port.to_string();
        form.fields[F_USER].value = conn.username.clone();
        match &conn.auth {
            Auth::Password { password } => form.fields[F_PASSWORD].value = password.clone(),
            Auth::Key { path, .. } => {
                form.fields[F_KEY].value = path.display().to_string();
            }
        }
        form
    }

    fn field(&self, idx: usize) -> &str {
        self.fields[idx].value.trim()
    }
}

/// 顶层模式。
#[derive(Debug, Clone, PartialEq)]
pub enum Mode {
    /// 连接列表。
    Picker,
    /// 连接建立中。
    Connecting { name: String },
    /// 正在查看某个会话(具体是哪个由 App.active 指示)。
    Session,
    /// 回放一段 .cast 录制(不属于任何会话)。
    Replay,
    /// 浏览一个数据库(SQLite):表列表 / 数据网格 / SQL 结果。
    DbBrowse,
    /// 浏览一个 Redis:键列表 / 值视图。
    RedisBrowse,
}

/// 会话内标签页。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tab {
    Shell,
    Files,
    Forwards,
    Monitor,
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

/// 监控页状态:样本环形缓冲 + 最近一次错误。
pub struct MonitorState {
    pub samples: VecDeque<ResourceSample>,
    pub error: String,
}

impl MonitorState {
    fn push(&mut self, sample: ResourceSample) {
        if self.samples.len() >= linop_monitor::MAX_SAMPLES {
            self.samples.pop_front();
        }
        self.samples.push_back(sample);
    }
}

/// 运行中的采样任务(每个样本以 AppEvent::MonitorSample 形式回流)。
pub struct MonitorParts {
    task: tokio::task::JoinHandle<()>,
}

impl MonitorParts {
    fn stop(self) {
        self.task.abort();
    }
}

/// 断线自动重连进度。
#[derive(Default)]
pub struct ReconnectState {
    pub attempts: u32,
}

/// 自动重连最大尝试次数。
pub const MAX_RECONNECT_ATTEMPTS: u32 = 5;

/// 第 n 次失败后的退避秒数:1,2,4,8,8…
fn reconnect_backoff(attempts: u32) -> Duration {
    Duration::from_secs(1u64.saturating_add(1u64 << (attempts.saturating_sub(1)).min(3)) - 1)
}

pub struct SessionState {
    /// 会话唯一 ID,事件路由用。
    pub id: u64,
    /// 断线重连状态;None = 在线。
    pub reconnect: Option<ReconnectState>,
    /// shell 输出转发任务句柄:重连/断开时主动终止,避免旧任务的
    /// ShellClosed 再次触发重连循环。
    pub reader_task: tokio::task::JoinHandle<()>,
    pub profile: Connection,
    pub ssh: Session,
    pub writer: ShellWriter,
    pub sftp: Sftp,
    pub forwards: Vec<LocalForward>,
    pub remote_forwards: Vec<linop_ssh::RemoteForward>,
    pub dynamic_forwards: Vec<linop_ssh::DynamicForward>,
    /// 启动失败的转发及原因。
    pub forward_failures: Vec<String>,
    /// 转发页选中项(本地+反向统一序)。
    pub forwards_sel: usize,
    pub term: Terminal,
    pub tab: Tab,
    pub files: FilesState,
    /// 监控页数据与采样任务(进入监控页时懒启动)。
    pub monitor: MonitorState,
    pub monitor_parts: Option<MonitorParts>,
    /// 进行中的会话录制(Shell 页按 v 开关)。
    pub recording: Option<Recording>,
    /// 进行中的后台传输(单会话同时只允许一个)。
    pub active_transfer: Option<ActiveTransfer>,
    /// 底部状态栏消息(最近一次操作的反馈)。
    pub status: String,
}

/// 数据库浏览状态:连接 + 当前视图(表列表或数据网格)。
pub struct DbBrowser {
    pub profile: linop_core::DbProfile,
    /// 连接共享给阻塞线程执行 SQL(Mutex:rusqlite Connection 非.Sync()。
    db: std::sync::Arc<std::sync::Mutex<SqliteDb>>,
    pub tables: Vec<linop_db::TableInfo>,
    pub view: DbView,
    /// 表列表选中行。
    pub sel: usize,
    /// 底部状态消息。
    pub status: String,
}

impl DbBrowser {
    /// 克隆共享句柄给阻塞线程。
    fn db_handle(&self) -> std::sync::Arc<std::sync::Mutex<SqliteDb>> {
        self.db.clone()
    }
}

/// 数据库当前展示的视图。
pub enum DbView {
    /// 表/视图列表。
    Tables,
    /// 数据网格:来源(表名或 SQL)与结果。
    Data {
        title: String,
        result: QueryResult,
        page: usize,
    },
}

/// Redis 浏览状态。
pub struct RedisBrowser {
    pub profile: linop_core::DbProfile,
    /// 同步客户端共享给阻塞线程(Mutex:命令即发即收)。
    client: std::sync::Arc<std::sync::Mutex<RedisClient>>,
    /// 经跳板机建立的隧道(直连时为 None);浏览期间必须存活。
    tunnel: Option<RedisTunnel>,
    pub keys: Vec<RedisKeyInfo>,
    pub view: RedisView,
    pub sel: usize,
    pub status: String,
}

/// Redis 当前展示的视图。
pub enum RedisView {
    /// 键列表。
    Keys,
    /// 某个键的值。
    Value {
        key: String,
        kind: String,
        value: KeyValue,
    },
}

/// 一条存活的数据库隧道:SSH 会话 + 本地转发。
/// 停止顺序:先转发后 SSH(转发依赖会话)。
pub struct RedisTunnel {
    ssh: Session,
    forward: LocalForward,
}

impl RedisTunnel {
    fn stop(self) {
        self.forward.stop();
        let ssh = self.ssh;
        tokio::spawn(async move {
            let _ = capped(TEARDOWN_TIMEOUT_SECS, ssh.disconnect()).await;
        });
    }
}

impl RedisBrowser {
    fn client(&self) -> std::sync::Arc<std::sync::Mutex<RedisClient>> {
        self.client.clone()
    }

    /// SCAN 占位用的名字列表。
    fn keys_names(&self) -> Vec<String> {
        self.keys.iter().map(|k| k.name.clone()).collect()
    }

    /// 选中键的 (名称, 类型)。
    fn picked(&self) -> Option<(String, String)> {
        self.keys
            .get(self.sel)
            .map(|k| (k.name.clone(), k.kind.clone()))
    }
}

/// 进行中的录制:写入器与目标文件。
pub struct Recording {
    writer: CastWriter,
    path: PathBuf,
}

/// 回放状态:cast 事件按原始时间间隔喂进独立终端。
pub struct ReplayState {
    pub path: PathBuf,
    pub cast: Cast,
    /// 下一个待应用事件的下标。
    pub cursor: usize,
    pub term: Terminal,
    pub playing: bool,
    /// 播放调度任务;暂停/回退/退出时取消。
    pub task: Option<tokio::task::JoinHandle<()>>,
}

/// 一次后台传输的进度状态。
pub struct ActiveTransfer {
    pub direction: &'static str,
    pub name: String,
    pub total: u64,
    pub transferred: u64,
    pub cancel: linop_sftp::CancelFlag,
}

impl ActiveTransfer {
    /// 状态栏进度文案,如 "下载 big.iso [██████░░░░] 45%(14 MiB/31 MiB) · c 取消"。
    fn label(&self) -> String {
        let done = linop_sftp::human_size(self.transferred);
        let size = linop_sftp::human_size(self.total);
        let percent = (self.transferred * 100)
            .checked_div(self.total)
            .map_or_else(String::new, |p| format!("{p:3}%"));
        let filled = (self.transferred * 10)
            .checked_div(self.total)
            .map_or(0, |v| v as usize)
            .min(10);
        let bar = format!("{}{}", "█".repeat(filled), "░".repeat(10 - filled));
        format!(
            "{} {} [{bar}] {}({}/{}) · c 取消",
            self.direction, self.name, percent, done, size
        )
    }
}

pub struct App {
    pub registry: Registry,
    /// 数据库 profile 注册表(databases.toml)。
    pub db_registry: linop_core::DbRegistry,
    pub picker_sel: usize,
    /// picker 页全局提示(错误等)。
    pub notice: String,
    pub mode: Mode,
    /// 并存会话列表;active 指示当前查看的会话。
    pub sessions: Vec<SessionState>,
    pub active: usize,
    next_sid: u64,
    /// 打开中的模态对话框(文件页操作)。
    pub dialog: Option<Dialog>,
    /// 打开中的连接编辑表单。
    pub form: Option<ConnectionForm>,
    /// draw 阶段记录的 shell 内容区尺寸;事件循环据此同步仿真器与 PTY。
    pub shell_area: Option<(u16, u16)>,
    /// 打开中的录制回放(与 Mode::Replay 对应)。
    pub replay: Option<ReplayState>,
    /// 打开中的数据库浏览(与 Mode::DbBrowse 对应)。
    pub db: Option<DbBrowser>,
    /// 打开中的 Redis 浏览(与 Mode::RedisBrowse 对应)。
    pub redis: Option<RedisBrowser>,
}

impl App {
    pub fn new(registry: Registry) -> Self {
        Self {
            registry,
            db_registry: linop_core::DbRegistry::new(),
            picker_sel: 0,
            notice: String::new(),
            mode: Mode::Picker,
            sessions: Vec::new(),
            active: 0,
            next_sid: 0,
            dialog: None,
            form: None,
            shell_area: None,
            replay: None,
            db: None,
            redis: None,
        }
    }

    pub fn connections(&self) -> &[Connection] {
        self.registry.connections()
    }

    /// picker 统一选择域总长:在线会话数 + 连接 profile 数。
    fn picker_len(&self) -> usize {
        self.sessions.len() + self.connections().len() + self.db_registry.profiles().len()
    }

    /// picker 光标当前落到的数据库 profile(仅第三段有效)。
    fn picked_db(&self) -> Option<&linop_core::DbProfile> {
        let idx = self
            .picker_sel
            .checked_sub(self.sessions.len() + self.connections().len())?;
        self.db_registry.profiles().get(idx)
    }

    fn move_picker(&mut self, delta: i32) {
        let len = self.picker_len();
        if len == 0 {
            return;
        }
        self.picker_sel = (self.picker_sel as i32 + delta).rem_euclid(len as i32) as usize;
    }

    /// 当前活动会话(可变)。
    fn session_mut(&mut self) -> Option<&mut SessionState> {
        self.sessions.get_mut(self.active)
    }

    /// 按会话 ID 查找;返回 (下标, 会话)。
    fn session_by_id(&mut self, id: u64) -> Option<(usize, &mut SessionState)> {
        self.sessions
            .iter_mut()
            .enumerate()
            .find(|(_, s)| s.id == id)
    }

    /// 断开并移除指定下标的会话;调整 active,会话清空时回到列表。
    async fn disconnect_at(&mut self, idx: usize) {
        if idx >= self.sessions.len() {
            return;
        }
        let session = self.sessions.remove(idx);
        if self.active >= self.sessions.len() {
            self.active = self.sessions.len().saturating_sub(1);
        }
        if self.sessions.is_empty() {
            self.mode = Mode::Picker;
        }
        teardown_session(session).await;
    }

    /// 断开全部会话(退出前清理)。
    async fn disconnect_all(&mut self) {
        for session in std::mem::take(&mut self.sessions) {
            teardown_session(session).await;
        }
        self.active = 0;
        self.mode = Mode::Picker;
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
                if let Some(s) = self.session_mut()
                    && s.tab == Tab::Shell
                {
                    let bytes = keys::paste(&text, s.term.bracketed_paste());
                    if let Err(msg) = capped(WRITE_TIMEOUT_SECS, s.writer.write(&bytes)).await {
                        s.status = msg;
                    } else {
                        s.term.set_scrollback(0);
                    }
                }
            }
            AppEvent::Resize => {}
            AppEvent::ShellOutput(id, bytes) => {
                if let Some((_, s)) = self.session_by_id(id) {
                    s.term.feed(&bytes);
                    // 录制中:同步写盘;失败自动结束录制(文件已逐条 flush,不丢已录内容)。
                    if let Some(rec) = &mut s.recording
                        && rec.writer.write_output(&bytes).is_err()
                        && let Some(rec) = s.recording.take()
                    {
                        let path = rec.path.display().to_string();
                        let _ = rec.writer.finish();
                        s.status = format!("录制写入失败已停止:{path}");
                    }
                }
            }
            AppEvent::ShellClosed(id) => {
                // 意外断线:保留会话与终端内容,进入自动重连周期;
                // 重连周期中 reader 的收尾事件忽略(可能重复到达)。
                if let Some((_, s)) = self.session_by_id(id)
                    && s.reconnect.is_none()
                {
                    let profile = s.profile.clone();
                    let cols = s.term.cols();
                    let rows = s.term.rows();
                    // 旧连接已死:停监控采样与转发,清掉传输状态。
                    if let Some(parts) = s.monitor_parts.take() {
                        parts.stop();
                    }
                    for forward in s.forwards.drain(..) {
                        forward.stop();
                    }
                    for remote in s.remote_forwards.drain(..) {
                        let _ = tokio::time::timeout(Duration::from_secs(2), remote.stop()).await;
                    }
                    for dynamic in s.dynamic_forwards.drain(..) {
                        dynamic.stop();
                    }
                    s.active_transfer = None;
                    s.reconnect = Some(ReconnectState::default());
                    s.status = format!("{} 已断开,自动重连中…", profile.name);
                    s.term
                        .feed("\r\n\x1b[33m—— 连接断开,正在重连 ——\x1b[0m\r\n".as_bytes());
                    spawn_reconnect_attempt(tx.clone(), id, profile, cols, rows);
                }
            }
            AppEvent::ReconnectTick(id) => {
                if let Some((_, s)) = self.session_by_id(id)
                    && s.reconnect.is_some()
                {
                    let profile = s.profile.clone();
                    let (cols, rows) = (s.term.cols(), s.term.rows());
                    spawn_reconnect_attempt(tx.clone(), id, profile, cols, rows);
                }
            }
            AppEvent::ReconnectDone(id, result) => {
                self.on_reconnect_done(id, result, tx).await;
            }
            AppEvent::MonitorSample(id, sample) => {
                if let Some((_, s)) = self.session_by_id(id) {
                    match sample {
                        Ok(sample) => {
                            s.monitor.push(sample);
                            s.monitor.error.clear();
                        }
                        Err(msg) => s.monitor.error = msg,
                    }
                }
            }
            AppEvent::ReplayTick => {
                let mut next_delta = None;
                let mut done = false;
                if let Some(rp) = self.replay.as_mut()
                    && rp.playing
                {
                    match rp.cast.events.get(rp.cursor).cloned() {
                        Some(event) => {
                            rp.term.feed(event.data.as_bytes());
                            rp.cursor += 1;
                            next_delta = rp
                                .cast
                                .events
                                .get(rp.cursor)
                                .map(|next| (next.elapsed - event.elapsed).max(0.0));
                        }
                        None => {
                            rp.playing = false;
                            done = true;
                        }
                    }
                }
                if done {
                    self.cancel_replay_task();
                } else if let Some(delta) = next_delta {
                    self.schedule_replay_tick(delta, tx);
                }
            }
            AppEvent::InputClosed => {
                self.notice = "输入设备已关闭,退出".to_string();
                return false;
            }
            AppEvent::TransferProgress(id, bytes) => {
                if let Some((_, s)) = self.session_by_id(id)
                    && let Some(transfer) = &mut s.active_transfer
                {
                    transfer.transferred = bytes;
                    s.status = transfer.label();
                }
            }
            AppEvent::TransferFinished(id, result) => {
                if let Some((_, s)) = self.session_by_id(id)
                    && let Some(transfer) = s.active_transfer.take()
                {
                    s.status = match result {
                        Ok(bytes) => format!(
                            "{}完成:{}({})",
                            transfer.direction,
                            transfer.name,
                            linop_sftp::human_size(bytes)
                        ),
                        Err(msg) => {
                            format!("{}失败:{} · {}", transfer.direction, transfer.name, msg)
                        }
                    };
                    s.refresh_local();
                    s.refresh_remote().await;
                }
            }
        }
        true
    }

    async fn on_key(&mut self, key: KeyEvent, tx: &mpsc::UnboundedSender<AppEvent>) -> bool {
        // 模态对话框与表单优先接管按键。
        if let Some(dialog) = self.dialog.take() {
            self.handle_dialog_key(key, dialog, tx).await;
            return true;
        }
        if let Some(form) = self.form.take() {
            self.handle_form_key(key, form);
            return true;
        }
        match &mut self.mode {
            Mode::Picker => match key.code {
                KeyCode::Char('q') | KeyCode::Esc => {
                    self.disconnect_all().await;
                    return false;
                }
                KeyCode::Up | KeyCode::Char('k') => self.move_picker(-1),
                KeyCode::Down | KeyCode::Char('j') => self.move_picker(1),
                KeyCode::Char('x') => {
                    if self.picker_sel < self.sessions.len() {
                        // 会话行:断开。
                        let idx = self.picker_sel;
                        self.disconnect_at(idx).await;
                        self.picker_sel = self.picker_sel.min(self.picker_len().saturating_sub(1));
                    } else if self.picker_sel < self.sessions.len() + self.connections().len() {
                        if let Some(conn) = self
                            .connections()
                            .get(self.picker_sel - self.sessions.len())
                            .cloned()
                        {
                            // 连接行:删除配置(确认)。
                            self.dialog = Some(new_dialog(
                                "删除连接",
                                &format!("删除连接配置 {}?Enter 确认 / Esc 取消", conn.name),
                                DialogAction::DeleteProfile(conn.name.clone()),
                            ));
                        }
                    } else if let Some(db) = self.picked_db().cloned() {
                        // 数据库行:删除 profile(确认,不动数据库文件)。
                        self.dialog = Some(new_dialog(
                            "删除数据库配置",
                            &format!(
                                "移除数据库配置 {}?(仅删条目,不动文件)Enter 确认 / Esc 取消",
                                db.name
                            ),
                            DialogAction::DeleteDbProfile(db.name.clone()),
                        ));
                    }
                }
                // 从 ~/.ssh/config 导入。
                KeyCode::Char('i') => {
                    let path = linop_core::ssh_config::default_ssh_config_path();
                    let text = match std::fs::read_to_string(&path) {
                        Ok(text) => text,
                        Err(_) => {
                            self.notice = format!("未找到 ssh config:{}", path.display());
                            return true;
                        }
                    };
                    let parsed = linop_core::ssh_config::parse_ssh_config(&text);
                    if parsed.is_empty() {
                        self.notice = "ssh config 中没有可导入的主机".to_string();
                        return true;
                    }
                    let skipped = parsed
                        .iter()
                        .filter(|c| self.registry.get(&c.name).is_some())
                        .count();
                    let fresh: Vec<Connection> = parsed
                        .into_iter()
                        .filter(|c| self.registry.get(&c.name).is_none())
                        .collect();
                    if fresh.is_empty() {
                        self.notice = format!("{skipped} 个主机全部同名,无需导入");
                        return true;
                    }
                    self.dialog = Some(new_dialog(
                        "导入 ssh config",
                        &format!(
                            "从 {} 导入 {} 条(跳过 {} 个同名)?Enter 确认 / Esc 取消",
                            path.display(),
                            fresh.len(),
                            skipped
                        ),
                        DialogAction::ImportProfiles(fresh),
                    ));
                }
                // 新建/编辑连接配置。
                KeyCode::Char('n') => self.form = Some(ConnectionForm::blank()),
                KeyCode::Char('e') => {
                    let idx = self.picker_sel.saturating_sub(self.sessions.len());
                    if let Some(conn) = self.connections().get(idx).cloned() {
                        self.form = Some(ConnectionForm::edit(&conn));
                    }
                }
                KeyCode::Enter => {
                    // 在线会话行:切换过去;连接行:发起新连接。
                    if self.picker_sel < self.sessions.len() {
                        self.active = self.picker_sel;
                        self.mode = Mode::Session;
                        return true;
                    }
                    // 数据库行:打开浏览模式。
                    if self.picker_sel >= self.sessions.len() + self.connections().len() {
                        if let Some(db) = self.picked_db().cloned() {
                            self.open_db(db).await;
                        }
                        return true;
                    }
                    let profile_idx = self.picker_sel - self.sessions.len();
                    let Some(conn) = self.connections().get(profile_idx).cloned() else {
                        return true;
                    };
                    {
                        if let Auth::Password { password } = &conn.auth
                            && password.is_empty()
                        {
                            // 存在默认密钥且上次不是密钥失败:先试默认密钥直连;
                            // 失败(错误文案标记)后再 Enter 才弹密码框。
                            let has_default = linop_ssh::default_identity_paths()
                                .iter()
                                .any(|p| p.exists());
                            let default_failed = self.notice.contains("默认密钥认证失败");
                            if has_default && !default_failed {
                                self.connect(&conn, tx);
                            } else {
                                self.dialog = Some(Dialog {
                                    title: format!("连接 {}", conn.name),
                                    prompt: format!("输入 {} 的密码:", linop_core::endpoint(&conn)),
                                    input: String::new(),
                                    action: DialogAction::ConnectPassword(Box::new(conn)),
                                    error: String::new(),
                                    status: String::new(),
                                    secret: true,
                                });
                            }
                        } else if linop_ssh::key_needs_passphrase(&conn.auth) {
                            // 加密私钥未配置口令:就地弹掩码输入框。
                            self.dialog = Some(Dialog {
                                title: format!("连接 {}", conn.name),
                                prompt: "私钥需要口令:".to_string(),
                                input: String::new(),
                                action: DialogAction::ConnectPassphrase(Box::new(conn)),
                                error: String::new(),
                                status: String::new(),
                                secret: true,
                            });
                        } else {
                            self.connect(&conn, tx);
                        }
                    }
                }
                // 回放录制:默认取最新 .cast,可在输入框改路径。
                KeyCode::Char('p') => {
                    let default = linop_core::newest_recording().map(|p| p.display().to_string());
                    match default {
                        Some(default) => {
                            self.dialog = Some(Dialog {
                                title: "回放录制".to_string(),
                                prompt: "cast 文件路径:".to_string(),
                                input: default,
                                action: DialogAction::OpenReplay,
                                error: String::new(),
                                status: String::new(),
                                secret: false,
                            });
                        }
                        None => {
                            self.notice =
                                "暂无录制:进入会话后在 Shell 页按 v 开始/结束录制".to_string();
                        }
                    }
                }
                KeyCode::Char('r') => {
                    let db = linop_core::DbRegistry::load_default();
                    match (Registry::load_default(), db) {
                        (Ok(reg), Ok(dbs)) => {
                            self.registry = reg;
                            self.db_registry = dbs;
                            self.picker_sel = 0;
                            self.notice = "已重新加载连接与数据库列表".into();
                        }
                        (Err(e), _) | (_, Err(e)) => {
                            self.notice = format!("重新加载失败:{e}");
                        }
                    }
                }
                _ => {}
            },
            Mode::Connecting { .. } => {
                // 任意键取消连接:回到列表;迟到的结果由 on_connected 丢弃。
                self.mode = Mode::Picker;
            }
            Mode::Replay => self.replay_key(key, tx).await,
            Mode::DbBrowse => self.db_key(key, tx).await,
            Mode::RedisBrowse => self.redis_key(key, tx).await,
            Mode::Session => {
                if key.modifiers.contains(KeyModifiers::CONTROL)
                    && matches!(key.code, KeyCode::Char('q') | KeyCode::Char('Q'))
                {
                    self.disconnect_all().await;
                    return false;
                }
                let Some(s) = self.session_mut() else {
                    self.mode = Mode::Picker;
                    return true;
                };
                match key.code {
                    KeyCode::F(1) => s.tab = Tab::Shell,
                    KeyCode::F(2) => s.tab = Tab::Files,
                    KeyCode::F(3) => s.tab = Tab::Forwards,
                    KeyCode::F(4) => s.tab = Tab::Monitor,
                    // 回到列表,会话保持在线。
                    KeyCode::F(8) => self.mode = Mode::Picker,
                    // 断开当前会话并回到列表。
                    KeyCode::F(9) => {
                        let idx = self.active;
                        let name = self.sessions[idx].profile.name.clone();
                        self.mode = Mode::Picker;
                        self.picker_sel = idx.min(self.picker_len().saturating_sub(1));
                        self.disconnect_at(idx).await;
                        self.notice = format!("已断开 {name}");
                    }
                    _ => self.session_key(key, tx).await,
                }
                // 监控页需要采样器:懒启动(重连中/已停止则跳过)。
                self.ensure_monitor(tx);
            }
        }
        true
    }

    /// 当前会话在监控页且尚无采样器时启动之;对端拒绝 exec 也不致命,
    /// 错误会以 MonitorSample(Err) 形式显示在监控页。
    fn ensure_monitor(&mut self, tx: &mpsc::UnboundedSender<AppEvent>) {
        let Some(s) = self.sessions.get_mut(self.active) else {
            return;
        };
        if s.tab != Tab::Monitor || s.monitor_parts.is_some() || s.reconnect.is_some() {
            return;
        }
        let sid = s.id;
        let ssh = s.ssh.clone();
        let pump_tx = tx.clone();
        let task = linop_monitor::Monitor::spawn_with(ssh, move |sample| {
            let _ = pump_tx.send(AppEvent::MonitorSample(sid, sample));
        });
        s.monitor_parts = Some(MonitorParts { task });
    }

    /// 连接表单按键:↑/↓/Tab 切字段,Enter 保存,Esc 取消,退格/字符编辑。
    fn handle_form_key(&mut self, key: KeyEvent, mut form: ConnectionForm) {
        let field_count = form.fields.len();
        match key.code {
            KeyCode::Esc => {}
            KeyCode::Up | KeyCode::BackTab => {
                form.focus = (form.focus + field_count - 1) % field_count;
                self.form = Some(form);
            }
            KeyCode::Down | KeyCode::Tab => {
                form.focus = (form.focus + 1) % field_count;
                self.form = Some(form);
            }
            KeyCode::Enter => match self.save_form(&form) {
                Ok(name) => {
                    self.notice = format!("已保存连接 {name}");
                    // 光标跳到该连接所在行。
                    if let Some(idx) = self.sessions.len().checked_add(
                        self.registry
                            .connections()
                            .iter()
                            .position(|c| c.name == name)
                            .unwrap_or(0),
                    ) {
                        self.picker_sel = idx;
                    }
                }
                Err(msg) => {
                    form.error = msg;
                    self.form = Some(form);
                }
            },
            KeyCode::Backspace => {
                form.fields[form.focus].value.pop();
                self.form = Some(form);
            }
            KeyCode::Char(c)
                if !key
                    .modifiers
                    .intersects(KeyModifiers::CONTROL.union(KeyModifiers::ALT)) =>
            {
                form.fields[form.focus].value.push(c);
                self.form = Some(form);
            }
            _ => self.form = Some(form),
        }
    }

    /// 校验并保存表单;返回保存的连接名。
    fn save_form(&mut self, form: &ConnectionForm) -> Result<String, String> {
        let name = form.field(F_NAME);
        let host = form.field(F_HOST);
        let username = form.field(F_USER);
        if name.is_empty() {
            return Err("名称不能为空".into());
        }
        if host.is_empty() {
            return Err("主机不能为空".into());
        }
        if username.is_empty() {
            return Err("用户名不能为空".into());
        }
        let port = if form.field(F_PORT).is_empty() {
            22
        } else {
            form.field(F_PORT)
                .parse::<u16>()
                .map_err(|_| "端口必须是 1-65535 的数字".to_string())?
        };
        // 重名检查:改名(或新建)时不得与其他连接冲突。
        if form.original.as_deref() != Some(name) && self.registry.get(name).is_some() {
            return Err(format!("连接名 {name} 已存在"));
        }
        // 认证:密钥路径优先;密钥未变时沿用原 passphrase(表单不编辑它)。
        let auth = if !form.field(F_KEY).is_empty() {
            let passphrase = form
                .original
                .as_deref()
                .and_then(|orig| self.registry.get(orig))
                .and_then(|old| match &old.auth {
                    Auth::Key { path, passphrase }
                        if path.display().to_string() == form.field(F_KEY) =>
                    {
                        passphrase.clone()
                    }
                    _ => None,
                });
            Auth::key_with_passphrase(form.field(F_KEY), passphrase.unwrap_or_default())
        } else {
            // 密码留空:连接时弹掩码输入框。
            Auth::password(form.field(F_PASSWORD))
        };

        let mut conn = Connection::new(name, host, username, auth);
        conn.port = port;
        // 表单不编辑 notes/forwards:编辑时保留原值。
        if let Some(orig) = &form.original
            && let Some(old) = self.registry.get(orig)
        {
            conn.notes = old.notes.clone();
            conn.forwards = old.forwards.clone();
        }
        self.registry.upsert(conn);
        self.registry
            .save(linop_core::default_connections_path())
            .map_err(|e| format!("写入配置失败:{e}"))?;
        Ok(name.to_string())
    }

    /// 对话框按键:Esc 关闭、Enter 执行、退格/字符编辑输入。
    async fn handle_dialog_key(
        &mut self,
        key: KeyEvent,
        mut dialog: Dialog,
        tx: &mpsc::UnboundedSender<AppEvent>,
    ) {
        // 导入 ssh config 确认:作用于 picker,先行处理。
        if let (KeyCode::Enter, DialogAction::ImportProfiles(profiles)) =
            (&key.code, &dialog.action)
        {
            let count = profiles.len();
            for conn in profiles {
                self.registry.upsert(conn.clone());
            }
            match self.registry.save(linop_core::default_connections_path()) {
                Ok(()) => self.notice = format!("已从 ssh config 导入 {count} 条"),
                Err(e) => self.notice = format!("导入失败(未保存):{e}"),
            }
            return;
        }
        // 删除连接确认同样作用于 picker,先行处理。
        if let (KeyCode::Enter, DialogAction::DeleteProfile(name)) = (&key.code, &dialog.action) {
            let name = name.clone();
            match self.registry.remove(&name) {
                Ok(_) => {
                    if let Err(e) = self.registry.save(linop_core::default_connections_path()) {
                        self.notice = format!("删除失败(未保存):{e}");
                    } else {
                        self.notice = format!("已删除连接 {name}");
                    }
                    self.picker_sel = self.picker_sel.min(self.picker_len().saturating_sub(1));
                }
                Err(e) => self.notice = format!("删除失败:{e}"),
            }
            return;
        }
        // 密码/私钥口令连接对话框不经过 execute_dialog(它作用于会话,而此时在 picker)。
        if let (KeyCode::Enter, DialogAction::ConnectPassword(conn)) = (key.code, &dialog.action) {
            if dialog.input.is_empty() {
                dialog.error = "密码不能为空,Esc 取消".into();
                self.dialog = Some(dialog);
            } else {
                let mut conn = (**conn).clone();
                conn.auth = Auth::password(dialog.input.clone());
                self.connect(&conn, tx);
            }
            return;
        }
        if let (KeyCode::Enter, DialogAction::ConnectPassphrase(conn)) = (key.code, &dialog.action)
        {
            if dialog.input.is_empty() {
                dialog.error = "口令不能为空,Esc 取消".into();
                self.dialog = Some(dialog);
            } else {
                let mut conn = (**conn).clone();
                if let Auth::Key { path, .. } = &conn.auth {
                    conn.auth = Auth::key_with_passphrase(path.clone(), dialog.input.clone());
                }
                self.connect(&conn, tx);
            }
            return;
        }
        // 删除数据库 profile:作用于 picker。
        if let (KeyCode::Enter, DialogAction::DeleteDbProfile(name)) = (&key.code, &dialog.action) {
            let name = name.clone();
            match self.db_registry.remove(&name) {
                Ok(_) => {
                    match self.db_registry.save(linop_core::default_databases_path()) {
                        Ok(()) => self.notice = format!("已移除数据库配置 {name}(文件未动)"),
                        Err(e) => self.notice = format!("移除失败(未保存):{e}"),
                    }
                    self.picker_sel = self.picker_sel.min(self.picker_len().saturating_sub(1));
                }
                Err(e) => self.notice = format!("移除失败:{e}"),
            }
            return;
        }
        // 删除 Redis 键:在 Redis 浏览模式内执行。
        if let (KeyCode::Enter, DialogAction::DeleteRedisKey { name }) = (&key.code, &dialog.action)
        {
            let name = name.clone();
            if self.mode == Mode::RedisBrowse && self.redis.is_some() {
                let target = name.clone();
                let deleted = self.redis_call(move |client| client.del(&target)).await;
                match deleted {
                    Ok(true) => {
                        if let Some(browser) = self.redis.as_mut() {
                            let mut names = browser.keys_names();
                            names.retain(|k| k != &name);
                            let client = browser.client();
                            match refresh_redis_keys(client, names).await {
                                Ok(infos) => {
                                    browser.keys = infos;
                                    browser.sel =
                                        browser.sel.min(browser.keys.len().saturating_sub(1));
                                    browser.status = format!("已删除 {name}");
                                }
                                Err(msg) => browser.status = msg,
                            }
                        }
                    }
                    Ok(false) => {
                        if let Some(browser) = self.redis.as_mut() {
                            browser.status = format!("{name} 已不存在");
                        }
                    }
                    Err(msg) => {
                        dialog.error = msg;
                        self.dialog = Some(dialog);
                        return;
                    }
                }
            }
            return;
        }
        // SQL 对话框:在数据库浏览模式内执行。
        if let (KeyCode::Enter, DialogAction::RunSql) = (&key.code, &dialog.action) {
            let sql = dialog.input.trim().to_string();
            if sql.is_empty() {
                dialog.error = "SQL 不能为空,Esc 取消".into();
                self.dialog = Some(dialog);
            } else if self.mode == Mode::DbBrowse && self.db.is_some() {
                let outcome = self.run_sql_blocking(&sql).await;
                // 错误保留对话框就地显示;成功关闭对话框并进入结果网格。
                match &outcome {
                    Err(msg) => {
                        dialog.error = msg.clone();
                        self.dialog = Some(dialog);
                    }
                    Ok(_) => apply_query(self, sql, outcome),
                }
            } else {
                dialog.error = "数据库未打开(内部状态异常)".into();
                self.dialog = Some(dialog);
            }
            return;
        }
        // 回放对话框:读取并解析 cast,成功即进入回放模式。
        if let (KeyCode::Enter, DialogAction::OpenReplay) = (&key.code, &dialog.action) {
            let path = PathBuf::from(dialog.input.trim());
            let parsed = std::fs::read_to_string(&path)
                .map_err(|e| e.to_string())
                .and_then(|text| linop_cast::parse(&text).map_err(|e| e.to_string()));
            match parsed {
                Ok(cast) => {
                    let term = Terminal::new(cast.width.max(1), cast.height.max(1));
                    self.replay = Some(ReplayState {
                        path: path.clone(),
                        cast,
                        cursor: 0,
                        term,
                        playing: true,
                        task: None,
                    });
                    self.mode = Mode::Replay;
                    self.schedule_replay_tick(0.0, tx);
                }
                Err(msg) => {
                    dialog.error = format!("无法回放:{msg}");
                    self.dialog = Some(dialog);
                }
            }
            return;
        }
        match key.code {
            KeyCode::Esc => {}
            KeyCode::Enter => {
                dialog.error.clear();
                match capped(SFTP_TRANSFER_TIMEOUT_SECS, self.execute_dialog(&mut dialog)).await {
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
        if let Some(s) = self.session_mut() {
            s.status = message.to_string();
        }
    }

    /// 执行对话框动作;成功时已顺手刷新对应窗格。
    async fn execute_dialog(&mut self, dialog: &mut Dialog) -> Result<(), String> {
        let Some(s) = self.session_mut() else {
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
            DialogAction::ConnectPassword(_)
            | DialogAction::ConnectPassphrase(_)
            | DialogAction::DeleteProfile(_)
            | DialogAction::ImportProfiles(_)
            | DialogAction::OpenReplay
            | DialogAction::RunSql
            | DialogAction::DeleteDbProfile(_)
            | DialogAction::DeleteRedisKey { .. } => {
                // 已在 handle_dialog_key 里先行处理,不会走到这里。
                return Err("内部错误:picker 对话框不应进入会话执行".to_string());
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

    async fn session_key(&mut self, key: KeyEvent, tx: &mpsc::UnboundedSender<AppEvent>) {
        let Some(s) = self.session_mut() else {
            return;
        };
        match s.tab {
            Tab::Shell => match key.code {
                KeyCode::Char('v') => self.toggle_recording(),
                KeyCode::PageUp => s.term.scroll(1),
                KeyCode::PageDown => s.term.scroll(-1),
                _ if s.reconnect.is_some() => {
                    // 重连期间旧写半边已死:输入会失败/超时,直接暂停。
                    s.status = "重连中,输入已暂停…".to_string();
                }
                _ => {
                    if let Some(bytes) = keys::encode(key, s.term.application_cursor()) {
                        match capped(WRITE_TIMEOUT_SECS, s.writer.write(&bytes)).await {
                            Ok(()) => s.term.set_scrollback(0),
                            Err(msg) => s.status = msg,
                        }
                    }
                }
            },
            Tab::Files => match key.code {
                // 取消进行中的后台传输。
                KeyCode::Char('c') => {
                    if let Some(transfer) = &s.active_transfer {
                        transfer.cancel.cancel();
                        s.status = format!("正在取消 {} {}…", transfer.direction, transfer.name);
                    }
                }
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
                KeyCode::Char('d') => s.download_selected(tx),
                KeyCode::Char('u') => s.upload_selected(tx),
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
            Tab::Monitor => {}
            Tab::Forwards => {
                let total = s.forwards.len() + s.remote_forwards.len() + s.dynamic_forwards.len();
                match key.code {
                    KeyCode::Up | KeyCode::Char('k') if total > 0 => {
                        s.forwards_sel = (s.forwards_sel + total - 1) % total;
                    }
                    KeyCode::Down | KeyCode::Char('j') if total > 0 => {
                        s.forwards_sel = (s.forwards_sel + 1) % total;
                    }
                    // 停止选中的转发(本地或反向)。
                    KeyCode::Char('x') => {
                        let idx = s.forwards_sel;
                        if idx < s.forwards.len() {
                            let forward = s.forwards.remove(idx);
                            let label = forward.to_string();
                            forward.stop();
                            s.status = format!("已停止本地转发 {label}");
                        } else if idx < s.forwards.len() + s.remote_forwards.len() {
                            let remote_idx = idx - s.forwards.len();
                            let remote = s.remote_forwards.remove(remote_idx);
                            let label = remote.to_string();
                            match tokio::time::timeout(
                                Duration::from_secs(WRITE_TIMEOUT_SECS),
                                remote.stop(),
                            )
                            .await
                            {
                                Ok(()) => s.status = format!("已停止反向转发 {label}"),
                                Err(_) => s.status = format!("停止反向转发超时:{label}"),
                            }
                        } else if idx < total {
                            let dynamic_idx = idx - s.forwards.len() - s.remote_forwards.len();
                            let dynamic = s.dynamic_forwards.remove(dynamic_idx);
                            let label = dynamic.to_string();
                            dynamic.stop();
                            s.status = format!("已停止 SOCKS 转发 {label}");
                        }
                        let total = s.forwards.len() + s.remote_forwards.len();
                        s.forwards_sel = s.forwards_sel.min(total.saturating_sub(1));
                    }
                    _ => {}
                }
            }
        }
    }

    /// 回放模式按键:Space 播放/暂停,→ 逐事件前进,← 回退重放,q 退出。
    async fn replay_key(&mut self, key: KeyEvent, tx: &mpsc::UnboundedSender<AppEvent>) {
        if self.replay.is_none() {
            self.mode = Mode::Picker;
            return;
        }
        match key.code {
            KeyCode::Char('q') | KeyCode::Esc => {
                self.cancel_replay_task();
                self.replay = None;
                self.mode = Mode::Picker;
            }
            KeyCode::Char(' ') => {
                let resume = self.replay.as_mut().is_some_and(|rp| {
                    rp.playing = !rp.playing;
                    rp.playing
                });
                if resume {
                    // schedule 内部先取消旧调度再按播放态重建。
                    self.schedule_replay_tick(0.0, tx);
                } else {
                    self.cancel_replay_task();
                }
            }
            // 逐事件前进(自动暂停,便于细看输出)。
            KeyCode::Right => {
                if let Some(rp) = self.replay.as_mut() {
                    rp.playing = false;
                    if let Some(event) = rp.cast.events.get(rp.cursor).cloned() {
                        rp.term.feed(event.data.as_bytes());
                        rp.cursor += 1;
                    }
                }
                self.cancel_replay_task();
            }
            // 回退:重建终端重放到前一个事件(事件量小,重放开销可忽略)。
            KeyCode::Left => {
                if let Some(rp) = self.replay.as_mut() {
                    rp.playing = false;
                    rp.cursor = rp.cursor.saturating_sub(1);
                    let mut term = Terminal::new(rp.cast.width.max(1), rp.cast.height.max(1));
                    for event in &rp.cast.events[..rp.cursor] {
                        term.feed(event.data.as_bytes());
                    }
                    rp.term = term;
                }
                self.cancel_replay_task();
            }
            _ => {}
        }
    }

    /// 调度下一个回放事件:先取消旧调度,再按事件间隔睡到点。
    fn schedule_replay_tick(&mut self, secs: f64, tx: &mpsc::UnboundedSender<AppEvent>) {
        self.cancel_replay_task();
        let Some(rp) = self.replay.as_mut() else {
            return;
        };
        if !rp.playing {
            return;
        }
        let tx = tx.clone();
        let delay = std::time::Duration::from_secs_f64(secs.clamp(0.0, 3600.0));
        rp.task = Some(tokio::spawn(async move {
            if !delay.is_zero() {
                tokio::time::sleep(delay).await;
            }
            let _ = tx.send(AppEvent::ReplayTick);
        }));
    }

    fn cancel_replay_task(&mut self) {
        if let Some(rp) = self.replay.as_mut()
            && let Some(task) = rp.task.take()
        {
            task.abort();
        }
    }

    /// Shell 页按 v:开始/结束会话录制。
    fn toggle_recording(&mut self) {
        let Some(s) = self.session_mut() else {
            return;
        };
        match s.recording.take() {
            Some(rec) => {
                let path = rec.path.display().to_string();
                match rec.writer.finish() {
                    Ok(()) => s.status = format!("录制已保存:{path}"),
                    Err(e) => s.status = format!("结束录制失败({path}):{e}"),
                }
            }
            None => {
                let dir = linop_core::recordings_dir();
                let safe: String = s
                    .profile
                    .name
                    .chars()
                    .map(|c| {
                        if c.is_ascii_alphanumeric() || c == '-' || c == '_' {
                            c
                        } else {
                            '_'
                        }
                    })
                    .collect();
                let secs = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map(|d| d.as_secs())
                    .unwrap_or(0);
                let path = dir.join(format!("{safe}-{secs}.cast"));
                let (cols, rows) = (s.term.cols().max(1), s.term.rows().max(1));
                match CastWriter::create(&path, cols, rows) {
                    Ok(writer) => {
                        s.status = format!("⏺ 录制中 → {}(按 v 结束)", path.display());
                        s.recording = Some(Recording { writer, path });
                    }
                    Err(e) => s.status = format!("开始录制失败:{e}"),
                }
            }
        }
    }

    /// 打开数据库浏览:按驱动分派(SQLite / Redis)。
    async fn open_db(&mut self, profile: linop_core::DbProfile) {
        match profile.driver {
            linop_core::DbDriver::Sqlite => self.open_sqlite(profile).await,
            linop_core::DbDriver::Redis => self.open_redis(profile).await,
        }
    }

    /// 打开 SQLite 浏览:加载数据库并列出表;失败停留在列表并提示。
    async fn open_sqlite(&mut self, profile: linop_core::DbProfile) {
        let path = profile.path.clone();
        let task = tokio::task::spawn_blocking(move || {
            let db = SqliteDb::open(&path)?;
            let tables = db.tables()?;
            Ok::<_, linop_db::DbError>((db, tables))
        });
        let outcome =
            match tokio::time::timeout(Duration::from_secs(SFTP_META_TIMEOUT_SECS), task).await {
                Ok(Ok(work)) => work,
                Ok(Err(join)) => Err(linop_db::DbError::Query(format!("后台任务失败:{join}"))),
                Err(_) => Err(linop_db::DbError::Query("打开数据库超时".into())),
            };
        match outcome {
            Ok((db, tables)) => {
                let name = profile.name.clone();
                self.db = Some(DbBrowser {
                    profile,
                    db: std::sync::Arc::new(std::sync::Mutex::new(db)),
                    tables,
                    view: DbView::Tables,
                    sel: 0,
                    status: format!("已打开 {name}(Enter 看数据 · s 执行 SQL)"),
                });
                self.mode = Mode::DbBrowse;
            }
            Err(e) => self.notice = format!("打开数据库失败:{e}"),
        }
    }

    /// 数据库浏览按键:表列表与数据网格两个视图分别处理。
    async fn db_key(&mut self, key: KeyEvent, _tx: &mpsc::UnboundedSender<AppEvent>) {
        if self.db.is_none() {
            self.mode = Mode::Picker;
            return;
        }
        let in_tables = matches!(self.db.as_ref().unwrap().view, DbView::Tables);
        if in_tables {
            match key.code {
                KeyCode::Char('q') | KeyCode::Esc => {
                    self.db = None;
                    self.mode = Mode::Picker;
                }
                KeyCode::Up | KeyCode::Char('k') => {
                    if let Some(db) = self.db.as_mut()
                        && !db.tables.is_empty()
                    {
                        let len = db.tables.len();
                        db.sel = (db.sel + len - 1) % len;
                    }
                }
                KeyCode::Down | KeyCode::Char('j') => {
                    if let Some(db) = self.db.as_mut()
                        && !db.tables.is_empty()
                    {
                        let len = db.tables.len();
                        db.sel = (db.sel + 1) % len;
                    }
                }
                // 刷新表列表(DDL 后结构可能变化)。
                KeyCode::Char('r') => {
                    if let Some(db) = self.db.as_mut() {
                        let status = refresh_tables(db);
                        db.status = status;
                    }
                }
                // 进入表数据网格(SELECT * 该表)。
                KeyCode::Enter => {
                    let sql = self.db.as_ref().and_then(|db| {
                        db.tables
                            .get(db.sel)
                            .map(|t| format!("SELECT * FROM \"{}\"", t.name.replace('"', "\"\"")))
                    });
                    let Some(sql) = sql else { return };
                    let outcome = self.run_sql_blocking(&sql).await;
                    apply_query(self, sql, outcome);
                }
                // 打开 SQL 输入框。
                KeyCode::Char('s') => {
                    self.dialog = Some(Dialog {
                        title: "执行 SQL".to_string(),
                        prompt: "SQL(单条语句,Enter 执行):".to_string(),
                        input: String::new(),
                        action: DialogAction::RunSql,
                        error: String::new(),
                        status: String::new(),
                        secret: false,
                    });
                }
                _ => {}
            }
        } else {
            match key.code {
                // 返回表列表。
                KeyCode::Backspace | KeyCode::Esc | KeyCode::Char('q') => {
                    if let Some(db) = self.db.as_mut() {
                        db.view = DbView::Tables;
                    }
                }
                // 重新执行当前视图来源(title 即原 SQL)。
                KeyCode::Char('r') => {
                    let sql = match self.db.as_ref().unwrap().view {
                        DbView::Data { ref title, .. } => title.clone(),
                        DbView::Tables => return,
                    };
                    let outcome = self.run_sql_blocking(&sql).await;
                    apply_query(self, sql, outcome);
                }
                // 翻页。
                KeyCode::Left | KeyCode::PageUp | KeyCode::Right | KeyCode::PageDown => {
                    if let Some(db) = self.db.as_mut()
                        && let DbView::Data { result, page, .. } = &mut db.view
                    {
                        let page_size = 20usize;
                        let total_pages = result.rows.len().div_ceil(page_size).max(1);
                        match key.code {
                            KeyCode::Left | KeyCode::PageUp => {
                                *page = page.saturating_sub(1);
                            }
                            _ => {
                                if *page + 1 < total_pages {
                                    *page += 1;
                                }
                            }
                        }
                    }
                }
                _ => {}
            }
        }
    }

    /// 在阻塞线程上执行 SQL(超时上限防大查询冻结事件循环)。
    async fn run_sql_blocking(&mut self, sql: &str) -> std::result::Result<StmtOutcome, String> {
        let handle = self.db.as_ref().expect("db_key 保证已打开").db_handle();
        let sql = sql.to_string();
        let task = tokio::task::spawn_blocking(move || {
            handle
                .lock()
                .map_err(|_| linop_db::DbError::Query("数据库连接锁中毒".into()))
                .and_then(|db| db.run(&sql))
        });
        match tokio::time::timeout(Duration::from_secs(SFTP_META_TIMEOUT_SECS), task).await {
            Ok(Ok(outcome)) => outcome.map_err(|e| e.to_string()),
            Ok(Err(join)) => Err(format!("后台任务失败:{join}")),
            Err(_) => Err("SQL 执行超时".to_string()),
        }
    }

    /// 关闭 Redis 浏览:先拆隧道(转发 → SSH),再清状态。
    fn close_redis(&mut self) {
        if let Some(browser) = self.redis.take()
            && let Some(tunnel) = browser.tunnel
        {
            tunnel.stop();
        }
        self.mode = Mode::Picker;
    }

    /// 按 profile.via 建隧道:返回 (Redis 端点, 隧道)。
    /// 直连时端点即 profile 本身;隧道为 None。
    async fn open_db_tunnel(
        &mut self,
        profile: &linop_core::DbProfile,
    ) -> Result<(String, Option<RedisTunnel>), String> {
        let Some(via) = profile.via.clone() else {
            return Ok((profile.redis_endpoint(), None));
        };
        let Some(jump) = self.registry.get(&via).cloned() else {
            return Err(format!("跳板连接 {via} 不存在(检查 connections.toml)"));
        };
        let policy = KnownHostsPolicy::TrustOnFirstUse(KnownHosts::load_default());
        let ssh = capped(
            CONNECT_TIMEOUT_SECS,
            Session::connect_profile(&jump, policy),
        )
        .await
        .map_err(|msg| format!("连接跳板 {via} 失败:{msg}"))?;
        let rule = PortForward::new(
            0,
            profile.host.clone().unwrap_or_default(),
            profile.port.unwrap_or(6379),
        );
        let forward = ssh
            .open_local_forward(&rule)
            .await
            .map_err(|e| format!("跳板 {via} 上建立转发失败:{e}"))?;
        let endpoint = forward.local_addr.to_string();
        Ok((endpoint, Some(RedisTunnel { ssh, forward })))
    }

    /// 打开 Redis 浏览:连接 + AUTH + 首轮 SCAN。
    /// profile 指定 `via` 时先经该 SSH 跳板建立本地隧道再连。
    async fn open_redis(&mut self, profile: linop_core::DbProfile) {
        let (endpoint, tunnel) = match self.open_db_tunnel(&profile).await {
            Ok(pair) => pair,
            Err(msg) => {
                self.notice = msg;
                return;
            }
        };
        let password = profile.password.clone();
        let task = tokio::task::spawn_blocking(move || {
            let mut client = RedisClient::connect(&endpoint, password.as_deref())?;
            let keys = client.scan_keys(linop_redis::SCAN_BATCH)?;
            Ok::<_, linop_redis::RedisError>((client, keys))
        });
        let outcome =
            match tokio::time::timeout(Duration::from_secs(SFTP_META_TIMEOUT_SECS), task).await {
                Ok(Ok(work)) => work,
                Ok(Err(join)) => Err(linop_redis::RedisError::Io(std::io::Error::other(format!(
                    "后台任务失败:{join}"
                )))),
                Err(_) => Err(linop_redis::RedisError::Io(std::io::Error::other(
                    "连接超时",
                ))),
            };
        match outcome {
            Ok((client, keys)) => {
                let mut browser = RedisBrowser {
                    profile,
                    client: std::sync::Arc::new(std::sync::Mutex::new(client)),
                    tunnel,
                    keys: keys
                        .into_iter()
                        .map(|name| RedisKeyInfo {
                            name,
                            kind: String::new(),
                            len: -1,
                            ttl_secs: -1,
                        })
                        .collect(),
                    view: RedisView::Keys,
                    sel: 0,
                    status: String::new(),
                };
                // SCAN 到的键可能已过期:补齐概要。
                let client = browser.client();
                let names = browser.keys_names();
                match refresh_redis_keys(client, names).await {
                    Ok(infos) => {
                        browser.keys = infos;
                        browser.sel = 0;
                        browser.status =
                            format!("{} 个键 · Enter 看值 · x 删除 · r 重扫", browser.keys.len());
                        self.redis = Some(browser);
                        self.mode = Mode::RedisBrowse;
                    }
                    Err(msg) => self.notice = format!("Redis 键信息读取失败:{msg}"),
                }
            }
            Err(e) => self.notice = format!("连接 Redis 失败:{e}"),
        }
    }

    /// 在阻塞线程上跑一个 Redis 命令(超时上限防慢服务器冻结事件循环)。
    async fn redis_call<T, F>(&self, f: F) -> Result<T, String>
    where
        T: Send + 'static,
        F: FnOnce(&mut RedisClient) -> Result<T, linop_redis::RedisError> + Send + 'static,
    {
        let client = self.redis.as_ref().expect("redis_key 保证已打开").client();
        let task = tokio::task::spawn_blocking(move || {
            let mut client = client.lock().map_err(|_| "连接锁中毒".to_string())?;
            f(&mut client).map_err(|e| e.to_string())
        });
        match tokio::time::timeout(Duration::from_secs(SFTP_META_TIMEOUT_SECS), task).await {
            Ok(Ok(value)) => value,
            Ok(Err(join)) => Err(format!("后台任务失败:{join}")),
            Err(_) => Err("Redis 命令超时".to_string()),
        }
    }

    /// Redis 浏览按键:键列表与值视图。
    async fn redis_key(&mut self, key: KeyEvent, _tx: &mpsc::UnboundedSender<AppEvent>) {
        if self.redis.is_none() {
            self.mode = Mode::Picker;
            return;
        }
        let in_keys = matches!(self.redis.as_ref().unwrap().view, RedisView::Keys);
        if in_keys {
            match key.code {
                KeyCode::Char('q') | KeyCode::Esc => {
                    self.close_redis();
                }
                KeyCode::Up | KeyCode::Char('k') => {
                    if let Some(browser) = self.redis.as_mut()
                        && !browser.keys.is_empty()
                    {
                        let len = browser.keys.len();
                        browser.sel = (browser.sel + len - 1) % len;
                    }
                }
                KeyCode::Down | KeyCode::Char('j') => {
                    if let Some(browser) = self.redis.as_mut()
                        && !browser.keys.is_empty()
                    {
                        let len = browser.keys.len();
                        browser.sel = (browser.sel + 1) % len;
                    }
                }
                // 重扫键空间。
                KeyCode::Char('r') => {
                    let names = self
                        .redis_call(|client| client.scan_keys(linop_redis::SCAN_BATCH))
                        .await;
                    match names {
                        Ok(names) => {
                            if let Some(browser) = self.redis.as_mut() {
                                browser.keys = names
                                    .into_iter()
                                    .map(|name| RedisKeyInfo {
                                        name,
                                        kind: String::new(),
                                        len: -1,
                                        ttl_secs: -1,
                                    })
                                    .collect();
                                browser.sel = 0;
                                let client = browser.client();
                                let names = browser.keys_names();
                                match refresh_redis_keys(client, names).await {
                                    Ok(infos) => {
                                        browser.status = format!("已重扫:{} 个键", infos.len());
                                        browser.keys = infos;
                                    }
                                    Err(msg) => browser.status = msg,
                                }
                            }
                        }
                        Err(msg) => {
                            if let Some(browser) = self.redis.as_mut() {
                                browser.status = format!("重扫失败:{msg}");
                            }
                        }
                    }
                }
                // 查看选中键的值。
                KeyCode::Enter => {
                    let picked = self.redis.as_ref().unwrap().picked();
                    let Some((name, kind)) = picked else { return };
                    let key = name.clone();
                    let value_kind = kind.clone();
                    let value = self
                        .redis_call(move |client| client.key_value(&name, &value_kind))
                        .await;
                    match value {
                        Ok(value) => {
                            if let Some(browser) = self.redis.as_mut() {
                                browser.view = RedisView::Value { key, kind, value };
                            }
                        }
                        Err(msg) => {
                            if let Some(browser) = self.redis.as_mut() {
                                browser.status = format!("读取键失败:{msg}");
                            }
                        }
                    }
                }
                // 删除选中键(确认)。
                KeyCode::Char('x') => {
                    let picked = self.redis.as_ref().unwrap().picked();
                    if let Some((name, _)) = picked {
                        self.dialog = Some(new_dialog(
                            "删除键",
                            &format!("删除键 {name}?Enter 确认 / Esc 取消"),
                            DialogAction::DeleteRedisKey { name },
                        ));
                    }
                }
                _ => {}
            }
        } else {
            match key.code {
                // 返回键列表。
                KeyCode::Backspace | KeyCode::Esc | KeyCode::Char('q') => {
                    if let Some(browser) = self.redis.as_mut() {
                        browser.view = RedisView::Keys;
                    }
                }
                _ => {}
            }
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
                    for remote in conn.remote_forwards {
                        let _ = tokio::time::timeout(Duration::from_secs(2), remote.stop()).await;
                    }
                    teardown_parts(conn.ssh, conn.sftp, conn.forwards).await;
                    return;
                }
                let Connected {
                    profile,
                    ssh,
                    writer,
                    reader,
                    sftp,
                    forwards,
                    remote_forwards,
                    dynamic_forwards,
                    forward_errors,
                } = *boxed;
                self.next_sid += 1;
                let sid = self.next_sid;
                let reader_task = spawn_shell_reader(tx.clone(), sid, reader);

                let (remote_entries, warn) =
                    match capped(SFTP_META_TIMEOUT_SECS, sftp.list_dir(".")).await {
                        Ok(entries) => (entries, None),
                        Err(e) => (Vec::new(), Some(format!("SFTP 目录读取失败:{e}"))),
                    };
                let local_cwd = dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));
                let local_entries = list_local(&local_cwd);
                let forward_note = (!forward_errors.is_empty()).then(|| forward_errors.join(";"));
                let status = warn.or(forward_note).unwrap_or_else(|| {
                    let total = forwards.len() + remote_forwards.len() + dynamic_forwards.len();
                    format!("已连接 {},转发 {total} 条", profile.name)
                });
                let (cols, rows) = self.shell_area.unwrap_or((80, 24));
                self.sessions.push(SessionState {
                    id: sid,
                    reconnect: None,
                    reader_task,
                    profile,
                    ssh,
                    writer,
                    sftp,
                    forwards,
                    remote_forwards,
                    dynamic_forwards,
                    forward_failures: forward_errors,
                    forwards_sel: 0,
                    term: Terminal::new(cols, rows),
                    tab: Tab::Shell,
                    recording: None,
                    monitor: MonitorState {
                        samples: VecDeque::new(),
                        error: String::new(),
                    },
                    monitor_parts: None,
                    active_transfer: None,
                    files: FilesState {
                        local_cwd,
                        local_entries,
                        remote_entries,
                        remote_sel: 0,
                        local_sel: 0,
                        active: Pane::Remote,
                    },
                    status,
                });
                self.active = self.sessions.len() - 1;
                self.mode = Mode::Session;
            }
            Err(msg) => {
                if matches!(self.mode, Mode::Connecting { .. }) {
                    self.notice = msg;
                    self.mode = Mode::Picker;
                }
            }
        }
    }

    /// 重连尝试回执:成功则无缝换上新连接资源;失败退避重试,耗尽则移除会话。
    async fn on_reconnect_done(
        &mut self,
        id: u64,
        result: Result<Box<Connected>, String>,
        tx: &mpsc::UnboundedSender<AppEvent>,
    ) {
        let conn = match result {
            Ok(conn) => conn,
            Err(msg) => {
                let Some((idx, s)) = self.session_by_id(id) else {
                    return;
                };
                let name = s.profile.name.clone();
                let Some(state) = s.reconnect.as_mut() else {
                    return;
                };
                state.attempts += 1;
                if state.attempts >= MAX_RECONNECT_ATTEMPTS {
                    self.notice = format!(
                        "{name} 重连失败(已尝试 {} 次),已移除",
                        MAX_RECONNECT_ATTEMPTS
                    );
                    self.disconnect_at(idx).await;
                } else {
                    let backoff = reconnect_backoff(state.attempts);
                    s.status = format!(
                        "重连失败({msg}),{}s 后第 {}/{} 次尝试…",
                        backoff.as_secs(),
                        state.attempts + 1,
                        MAX_RECONNECT_ATTEMPTS
                    );
                    spawn_reconnect_delay(tx.clone(), id, backoff);
                }
                return;
            }
        };
        let Connected {
            ssh,
            writer,
            reader,
            sftp,
            forwards,
            remote_forwards,
            dynamic_forwards,
            forward_errors,
            ..
        } = *conn;
        // 会话已被用户移除(F9):迟到结果直接回收。
        let Some((_, s)) = self.session_by_id(id) else {
            teardown_parts(ssh, sftp, forwards).await;
            return;
        };
        let remote_cwd = s.sftp.cwd().to_string();
        // 旧采样器若仍在(断线时未及清理),先停;稍后由 ensure_monitor 按需重启。
        if let Some(parts) = s.monitor_parts.take() {
            parts.stop();
        }
        s.reader_task.abort();
        s.reader_task = spawn_shell_reader(tx.clone(), s.id, reader);
        s.ssh = ssh;
        s.writer = writer;
        s.sftp = sftp;
        s.forwards = forwards;
        s.remote_forwards = remote_forwards;
        s.dynamic_forwards = dynamic_forwards;
        s.reconnect = None;
        s.term
            .feed("\r\n\x1b[32m—— 已重连 ——\x1b[0m\r\n".as_bytes());
        // 尽力恢复远端目录(新 SFTP 会话默认在主目录)。
        if let Ok(entries) = capped(SFTP_META_TIMEOUT_SECS, s.sftp.enter(&remote_cwd)).await {
            s.files.remote_entries = entries;
        }
        s.status = if forward_errors.is_empty() {
            "已重连".to_string()
        } else {
            format!("已重连;{} 条转发启动失败", forward_errors.len())
        };
        // 停留在监控页时立即恢复采样。
        self.ensure_monitor(tx);
    }

    /// draw 之后调用:把仿真器与远端 PTY 同步到布局给出的内容区尺寸。
    pub async fn sync_size(&mut self) {
        let Some((cols, rows)) = self.shell_area else {
            return;
        };
        let Some(s) = self.session_mut() else {
            return;
        };
        if s.term.cols() != cols || s.term.rows() != rows {
            s.term.resize(cols, rows);
            if let Err(msg) = capped(WRITE_TIMEOUT_SECS, s.writer.resize(cols, rows)).await {
                s.status = msg;
            }
        }
    }
}

impl SessionState {
    /// 刷新远端条目缓存(保持 cwd 不变)。
    async fn refresh_remote(&mut self) {
        if let Ok(entries) = capped(SFTP_META_TIMEOUT_SECS, self.sftp.list_dir(".")).await {
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
                match capped(SFTP_META_TIMEOUT_SECS, self.sftp.enter(&entry.name)).await {
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
            Pane::Remote => match capped(SFTP_META_TIMEOUT_SECS, self.sftp.go_parent()).await {
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

    /// 启动后台传输;同一时间只允许一个,进行中再按 d/u 会提示。
    fn spawn_transfer(
        &mut self,
        direction: &'static str,
        name: String,
        remote_abs: String,
        local_path: PathBuf,
        upload: bool,
        tx: &mpsc::UnboundedSender<AppEvent>,
    ) {
        if self.active_transfer.is_some() {
            self.status = "已有传输进行中,先按 c 取消".to_string();
            return;
        }
        let ssh = self.ssh.clone();
        let sid = self.id;
        let cancel = linop_sftp::CancelFlag::new();
        self.active_transfer = Some(ActiveTransfer {
            direction,
            name: name.clone(),
            total: 0,
            transferred: 0,
            cancel: cancel.clone(),
        });
        self.status = format!("{direction} {name} …");
        let tx = tx.clone();
        tokio::spawn(async move {
            let result =
                transfer_job(sid, ssh, &remote_abs, &local_path, upload, &cancel, &tx).await;
            let _ = tx.send(AppEvent::TransferFinished(sid, result));
        });
    }

    /// 下载远端选中项到本地窗格当前目录(后台流式)。
    fn download_selected(&mut self, tx: &mpsc::UnboundedSender<AppEvent>) {
        let Some(entry) = self
            .files
            .remote_entries
            .get(self.files.remote_sel)
            .cloned()
        else {
            return;
        };
        let target = self.files.local_cwd.join(&entry.name);
        let remote_abs = remote_absolute(self.sftp.cwd(), &entry.name);
        self.spawn_transfer("下载", entry.name, remote_abs, target, false, tx);
    }

    /// 上传本地选中项到远端当前目录(后台流式)。
    fn upload_selected(&mut self, tx: &mpsc::UnboundedSender<AppEvent>) {
        let Some(entry) = self.files.local_entries.get(self.files.local_sel).cloned() else {
            return;
        };
        let source = self.files.local_cwd.join(&entry.name);
        let remote_abs = remote_absolute(self.sftp.cwd(), &entry.name);
        self.spawn_transfer("上传", entry.name, remote_abs, source, true, tx);
    }
}

/// 拉取键概要(重扫与删除后刷新共用)。
/// 必须走 spawn_blocking:隧道数据面(转发 accept 循环、SSH 会话)挂在
/// 事件循环的 runtime 上,主线程同步等待会饿死它们 → 死锁。
async fn refresh_redis_keys(
    client: std::sync::Arc<std::sync::Mutex<RedisClient>>,
    names: Vec<String>,
) -> Result<Vec<RedisKeyInfo>, String> {
    let task = tokio::task::spawn_blocking(move || {
        let mut client = client.lock().map_err(|_| "连接锁中毒".to_string())?;
        client.key_infos(&names).map_err(|e| e.to_string())
    });
    match tokio::time::timeout(Duration::from_secs(SFTP_META_TIMEOUT_SECS), task).await {
        Ok(Ok(Ok(infos))) => Ok(infos),
        Ok(Ok(Err(msg))) => Err(msg),
        Ok(Err(join)) => Err(format!("后台任务失败:{join}")),
        Err(_) => Err("Redis 命令超时".to_string()),
    }
}

/// 刷新数据库表列表(DDL 后)。
fn refresh_tables(db: &mut DbBrowser) -> String {
    let handle = db.db_handle();
    let tables = handle
        .lock()
        .map_err(|e| e.to_string())
        .and_then(|db| db.tables().map_err(|e| e.to_string()));
    match tables {
        Ok(tables) => {
            let count = tables.len();
            db.tables = tables;
            db.sel = db.sel.min(count.saturating_sub(1));
            format!("已刷新:{count} 张表/视图")
        }
        Err(e) => format!("刷新失败:{e}"),
    }
}

/// 应用 SQL 结果:Rows → 数据网格;Exec → 状态栏消息。
fn apply_query(app: &mut App, title: String, outcome: std::result::Result<StmtOutcome, String>) {
    let Some(db) = app.db.as_mut() else { return };
    match outcome {
        Ok(StmtOutcome::Rows(result)) => {
            let rows = result.rows.len();
            let truncated = result.truncated;
            db.view = DbView::Data {
                title,
                result,
                page: 0,
            };
            db.status = if truncated {
                format!("{rows}+ 行(已达行数上限)")
            } else {
                format!("{rows} 行")
            };
        }
        Ok(StmtOutcome::Exec(exec)) => {
            db.status = format!("影响 {} 行", exec.rows_affected);
            refresh_tables(db);
        }
        Err(msg) => {
            db.status = format!("SQL 失败:{msg}");
        }
    }
}

/// 以 cwd 为基准拼出远端绝对路径(传输任务用独立 SFTP 会话,必须绝对路径)。
fn remote_absolute(cwd: &str, name: &str) -> String {
    if cwd == "/" {
        format!("/{name}")
    } else {
        format!("{cwd}/{name}")
    }
}

/// 后台传输体:独立打开 SFTP 通道,流式拷贝,节流上报进度。
/// 返回最后一次上报的字节数;取消映射为 "已取消"。
async fn transfer_job(
    sid: u64,
    ssh: Session,
    remote: &str,
    local: &Path,
    upload: bool,
    cancel: &linop_sftp::CancelFlag,
    tx: &mpsc::UnboundedSender<AppEvent>,
) -> Result<u64, String> {
    let sftp = Sftp::open(&ssh)
        .await
        .map_err(|e| format!("打开传输通道失败:{e}"))?;
    let mut last_reported: u64 = 0;
    let mut report = |done: u64, total: u64| {
        if done == total || done.saturating_sub(last_reported) >= step_of(total) {
            last_reported = done;
            let _ = tx.send(AppEvent::TransferProgress(sid, done));
        }
    };
    let result = if upload {
        sftp.upload_streaming(
            local,
            remote,
            &mut |_, done, total| report(done, total),
            cancel,
        )
        .await
    } else {
        sftp.download_streaming(
            remote,
            local,
            &mut |_, done, total| report(done, total),
            cancel,
        )
        .await
    };
    match result {
        Ok(()) => Ok(last_reported),
        Err(linop_sftp::SftpError::Cancelled) => Err("已取消".to_string()),
        Err(e) => Err(e.to_string()),
    }
}

/// 进度上报步长:总量的 1%(至少一个块 32 KiB),避免事件洪泛。
fn step_of(total: u64) -> u64 {
    (total / 100).max(32 * 1024)
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
        secret: false,
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
    s.reader_task.abort();
    // 结束录制:逐条 flush 已落盘,这里只补尾并关闭文件。
    if let Some(rec) = s.recording {
        let _ = rec.writer.finish();
    }
    if let Some(parts) = s.monitor_parts {
        parts.stop();
    }
    for remote in s.remote_forwards {
        let _ =
            tokio::time::timeout(Duration::from_secs(TEARDOWN_TIMEOUT_SECS), remote.stop()).await;
    }
    teardown_parts(s.ssh, s.sftp, s.forwards).await;
}

async fn teardown_parts(ssh: Session, sftp: Sftp, forwards: Vec<LocalForward>) {
    for forward in forwards {
        forward.stop();
    }
    // 清理同样带上限:对端已消失时不能卡住事件循环。
    let _ = capped(TEARDOWN_TIMEOUT_SECS, sftp.close()).await;
    let _ = capped(TEARDOWN_TIMEOUT_SECS, ssh.disconnect()).await;
}

/// shell 输出转发任务:通道关闭时发出 ShellClosed(意外断线→自动重连入口)。
fn spawn_shell_reader(
    tx: mpsc::UnboundedSender<AppEvent>,
    sid: u64,
    mut reader: ShellReader,
) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        while let Some(bytes) = reader.read_output().await {
            if tx.send(AppEvent::ShellOutput(sid, bytes)).is_err() {
                break;
            }
        }
        let _ = tx.send(AppEvent::ShellClosed(sid));
    })
}

/// 单次重连尝试任务。
fn spawn_reconnect_attempt(
    tx: mpsc::UnboundedSender<AppEvent>,
    sid: u64,
    profile: Connection,
    cols: u16,
    rows: u16,
) {
    tokio::spawn(async move {
        let result = capped(CONNECT_TIMEOUT_SECS, connect(profile, cols, rows))
            .await
            .map(Box::new);
        let _ = tx.send(AppEvent::ReconnectDone(sid, result));
    });
}

/// 重连退避任务:等待后触发下一次尝试。
fn spawn_reconnect_delay(tx: mpsc::UnboundedSender<AppEvent>, sid: u64, secs: Duration) {
    tokio::spawn(async move {
        tokio::time::sleep(secs).await;
        let _ = tx.send(AppEvent::ReconnectTick(sid));
    });
}

/// 阻塞读 crossterm 事件并转发到事件循环;输入设备失效时通知退出。
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
                Err(_) => {
                    // 非 tty 运行或 stdin 关闭:再也无法收到按键,让应用退出。
                    let _ = tx.send(AppEvent::InputClosed);
                    break;
                }
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
        // 整个连接过程(TCP + 认证 + shell/SFTP + 转发)受统一超时约束,
        // 不可达的主机不会让"正在连接"界面无限停留。
        let result = capped(CONNECT_TIMEOUT_SECS, connect(profile, cols, rows))
            .await
            .map(Box::new);
        let _ = tx.send(AppEvent::Connected(result));
    });
}

/// 建立连接:认证 → shell → SFTP → 启动 profile 里的全部转发。
async fn connect(profile: Connection, cols: u16, rows: u16) -> Result<Connected, String> {
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
    let mut remote_forwards = Vec::new();
    let mut forward_errors = Vec::new();
    for fwd in &profile.forwards {
        match ssh.open_local_forward(fwd).await {
            Ok(running) => forwards.push(running),
            Err(e) => forward_errors.push(format!(
                "转发 127.0.0.1:{} → {}:{} 启动失败:{e}",
                fwd.local_port, fwd.remote_host, fwd.remote_port
            )),
        }
    }
    for rule in &profile.remote_forwards {
        match ssh.open_remote_forward(rule).await {
            Ok(running) => remote_forwards.push(running),
            Err(e) => forward_errors.push(format!(
                "反向转发 {}:{} → 本机 {}:{} 启动失败:{e}",
                rule.bind_host, rule.bind_port, rule.target_host, rule.target_port
            )),
        }
    }
    let mut dynamic_forwards = Vec::new();
    for port in &profile.dynamic_ports {
        match ssh.open_dynamic_forward(*port).await {
            Ok(running) => dynamic_forwards.push(running),
            Err(e) => forward_errors.push(format!("SOCKS 转发 127.0.0.1:{port} 启动失败:{e}")),
        }
    }
    Ok(Connected {
        profile,
        ssh,
        writer,
        reader,
        sftp,
        forwards,
        remote_forwards,
        dynamic_forwards,
        forward_errors,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn capped_passes_value_and_maps_errors() {
        assert_eq!(capped(5, async { Ok::<_, &str>(7) }).await.unwrap(), 7);
        assert_eq!(
            capped(5, async { Err::<i32, _>("boom") })
                .await
                .unwrap_err(),
            "boom"
        );
    }

    #[tokio::test]
    async fn capped_times_out_pending_future() {
        let err = capped(1, std::future::pending::<Result<(), String>>())
            .await
            .unwrap_err();
        assert!(err.contains("超时"), "挂起的未来应被超时截断:{err}");
    }

    /// 黑洞服务器:接受 TCP 但从不完成 SSH 握手;连接必须被截断而非挂死。
    #[tokio::test]
    async fn blackhole_connect_is_capped() {
        let listener = tokio::net::TcpListener::bind(("127.0.0.1", 0))
            .await
            .unwrap();
        let addr = listener.local_addr().unwrap();
        tokio::spawn(async move {
            if let Ok((socket, _)) = listener.accept().await {
                // 持有连接但不读不写,模拟网络黑洞。
                let _socket = socket;
                std::future::pending::<()>().await;
            }
        });
        let mut conn =
            Connection::new("blackhole", addr.ip().to_string(), "u", Auth::password("p"));
        conn.port = addr.port();
        let err = match capped(
            1,
            Session::connect_profile(&conn, KnownHostsPolicy::AcceptAll),
        )
        .await
        {
            Ok(_) => panic!("黑洞服务器上的连接不应成功"),
            Err(msg) => msg,
        };
        assert!(err.contains("超时"), "黑洞应触发超时:{err}");
    }

    #[tokio::test]
    async fn input_closed_quits_app() {
        let (tx, _rx) = mpsc::unbounded_channel();
        let mut app = App::new(Registry::new());
        assert!(
            !app.handle(AppEvent::InputClosed, &tx).await,
            "输入设备失效必须退出而不是冻结"
        );
    }
}
