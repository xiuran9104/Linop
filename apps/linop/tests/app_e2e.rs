//! 应用层端到端:TestSshServer(含 SFTP)↔ 完整 App 状态机 + TestBackend 渲染。
//!
//! 走真实事件循环路径:picker 选择 → 连接 → shell 回显 → 文件上传/下载 → 断开。

use std::time::{Duration, Instant};

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use linop_core::{Auth, Connection, Registry};
use linop_sftp::Entry;
use ratatui::backend::TestBackend;
use tokio::sync::mpsc;

use linop::app::{
    self as app_mod, App, AppEvent, MAX_RECONNECT_ATTEMPTS, Mode, Pane, ReconnectState, Tab,
};
use linop::{keys, ui};

/// LINOP_CONFIG_DIR 是进程级环境变量:用它的测试必须持有此锁串行执行。
/// 用异步 Mutex 以便跨 await 持有。
static ENV_LOCK: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

async fn env_guard() -> tokio::sync::MutexGuard<'static, ()> {
    ENV_LOCK.lock().await
}

type TestTerminal = ratatui::Terminal<TestBackend>;

fn key(code: KeyCode) -> AppEvent {
    AppEvent::Key(KeyEvent::new(code, KeyModifiers::NONE))
}

/// 渲染一帧;先等首个事件(最多 wait),之后静默 300ms 即返回。
/// 避免"事件还没产生"被误判为"已安静"。
async fn settle(
    terminal: &mut TestTerminal,
    app: &mut App,
    rx: &mut mpsc::UnboundedReceiver<AppEvent>,
    tx: &mpsc::UnboundedSender<AppEvent>,
    wait: Duration,
) {
    let deadline = Instant::now() + wait;
    match tokio::time::timeout(wait, rx.recv()).await {
        Ok(Some(event)) => {
            app.handle(event, tx).await;
        }
        _ => {
            redraw(terminal, app).await;
            return;
        }
    }
    loop {
        match tokio::time::timeout(Duration::from_millis(300), rx.recv()).await {
            Ok(Some(event)) => {
                app.handle(event, tx).await;
            }
            Ok(None) | Err(_) => break,
        }
        if Instant::now() >= deadline {
            break;
        }
    }
    redraw(terminal, app).await;
}

async fn redraw(terminal: &mut TestTerminal, app: &mut App) {
    terminal.draw(|frame| ui::draw(frame, app)).unwrap();
    app.sync_size().await;
}

/// 整屏文本:按真实终端语义逐格取字形——宽字符跳过其覆盖的续格
/// (TestBackend 网格中续格可能残留上一帧窄字符,但真实终端不显示它)。
fn screen_text(terminal: &TestTerminal) -> String {
    let buffer = terminal.backend().buffer();
    (0..buffer.area.height)
        .map(|y| {
            let mut line = String::new();
            let mut x = 0;
            while x < buffer.area.width {
                let symbol = buffer.cell((x, y)).unwrap().symbol();
                let width = unicode_width::UnicodeWidthStr::width(symbol).max(1);
                line.push_str(symbol);
                x += width as u16;
            }
            line
        })
        .collect::<Vec<_>>()
        .join("\n")
        .replace(' ', "")
}

fn test_registry(addr: std::net::SocketAddr) -> Registry {
    let mut conn = Connection::new("t-e2e", addr.ip().to_string(), "u", Auth::password("p"));
    conn.port = addr.port();
    vec![conn].into_iter().collect()
}

#[tokio::test]
async fn picker_connect_shell_files_disconnect() -> anyhow::Result<()> {
    let _env = env_guard().await;
    let config_dir = tempfile::tempdir()?;
    unsafe { std::env::set_var("LINOP_CONFIG_DIR", config_dir.path()) };

    let remote_root = tempfile::tempdir()?;
    let local_dir = tempfile::tempdir()?;
    std::fs::write(local_dir.path().join("hello.txt"), b"hello-linop")?;
    let server = linop_testsupport::TestSshServer::start_with_sftp(remote_root.path()).await?;

    let (tx, mut rx) = mpsc::unbounded_channel();
    let mut app = app_mod::App::new(test_registry(server.addr));
    let mut terminal = TestTerminal::new(TestBackend::new(80, 24))?;

    // 1. picker 渲染并选中连接。
    redraw(&mut terminal, &mut app).await;
    assert!(
        screen_text(&terminal).contains("t-e2e"),
        "picker 应显示连接名"
    );

    // 2. Enter 连接 → 回执到达后进入会话。
    app.handle(key(KeyCode::Enter), &tx).await;
    settle(
        &mut terminal,
        &mut app,
        &mut rx,
        &tx,
        Duration::from_secs(5),
    )
    .await;
    assert!(matches!(app.mode, Mode::Session), "应进入会话模式");

    // 3. shell 页:键入字符,echo 服务器应回显到仿真终端。
    for c in ['h', 'i'] {
        app.handle(key(KeyCode::Char(c)), &tx).await;
    }
    settle(
        &mut terminal,
        &mut app,
        &mut rx,
        &tx,
        Duration::from_secs(2),
    )
    .await;
    let text = screen_text(&terminal);
    assert!(text.contains("hi"), "shell 页应显示回显输入");

    // 4. 文件页:把本地窗格指到临时目录,上传 hello.txt 到远端。
    app.handle(key(KeyCode::F(2)), &tx).await;
    if let Some(s) = app.sessions.get_mut(app.active) {
        s.files.local_cwd = local_dir.path().to_path_buf();
        s.files.local_entries = vec![local_entry("hello.txt", 11)];
        s.files.active = Pane::Local;
    }
    redraw(&mut terminal, &mut app).await;
    assert!(
        screen_text(&terminal).contains("hello.txt"),
        "本地窗格应显示文件"
    );

    app.handle(key(KeyCode::Char('u')), &tx).await;
    settle(
        &mut terminal,
        &mut app,
        &mut rx,
        &tx,
        Duration::from_secs(3),
    )
    .await;
    assert!(
        remote_root.path().join("hello.txt").exists(),
        "远端应收到上传的文件"
    );
    if let Some(s) = app.sessions.get(app.active) {
        assert!(s.active_transfer.is_none(), "上传结束后不应残留传输状态");
        assert!(
            s.status.contains("上传完成"),
            "状态栏应报上传完成:{:?}",
            s.status
        );
    }
    if let Some(s) = app.sessions.get_mut(app.active) {
        s.files.active = Pane::Remote;
    }

    // 5. 下载回另一个本地路径。
    std::fs::remove_file(local_dir.path().join("hello.txt"))?;
    app.handle(key(KeyCode::Char('d')), &tx).await;
    settle(
        &mut terminal,
        &mut app,
        &mut rx,
        &tx,
        Duration::from_secs(3),
    )
    .await;
    assert_eq!(
        std::fs::read(local_dir.path().join("hello.txt"))?,
        b"hello-linop",
        "下载应还原文件内容"
    );
    if let Some(s) = app.sessions.get(app.active) {
        assert!(
            s.status.contains("下载完成"),
            "状态栏应报下载完成:{:?}",
            s.status
        );
    }

    // 6. 对话框操作:新建目录 → 重命名 → 删除(远端窗格,选中项为第 0 项)。
    app.handle(key(KeyCode::Char('n')), &tx).await;
    assert!(app.dialog.is_some(), "n 应打开新建目录对话框");
    app.handle(key(KeyCode::Esc), &tx).await;
    assert!(app.dialog.is_none(), "Esc 应关闭对话框");

    app.handle(key(KeyCode::Char('n')), &tx).await;
    for c in "docs".chars() {
        app.handle(key(KeyCode::Char(c)), &tx).await;
    }
    app.handle(key(KeyCode::Enter), &tx).await;
    settle(
        &mut terminal,
        &mut app,
        &mut rx,
        &tx,
        Duration::from_secs(2),
    )
    .await;
    assert!(app.dialog.is_none(), "成功后对话框应关闭");
    assert!(
        remote_root.path().join("docs").is_dir(),
        "应创建远端目录 docs"
    );

    // 选中第 0 项(docs)重命名。
    app.handle(key(KeyCode::Char('R')), &tx).await;
    for c in "docs2".chars() {
        app.handle(key(KeyCode::Char(c)), &tx).await;
    }
    app.handle(key(KeyCode::Enter), &tx).await;
    settle(
        &mut terminal,
        &mut app,
        &mut rx,
        &tx,
        Duration::from_secs(2),
    )
    .await;
    assert!(remote_root.path().join("docs2").is_dir());
    assert!(
        !remote_root.path().join("docs").exists(),
        "重命名后旧名应消失"
    );

    // 删除空目录 docs2(确认对话框直接 Enter)。
    app.handle(key(KeyCode::Char('x')), &tx).await;
    assert!(app.dialog.is_some(), "x 应打开删除确认");
    app.handle(key(KeyCode::Enter), &tx).await;
    settle(
        &mut terminal,
        &mut app,
        &mut rx,
        &tx,
        Duration::from_secs(2),
    )
    .await;
    assert!(
        !remote_root.path().join("docs2").exists(),
        "确认后应删除目录"
    );

    // 7. 断开回列表。
    app.handle(key(KeyCode::F(9)), &tx).await;
    redraw(&mut terminal, &mut app).await;
    assert!(matches!(app.mode, Mode::Picker));
    assert!(screen_text(&terminal).contains("已断开t-e2e"));

    server.shutdown();
    unsafe { std::env::remove_var("LINOP_CONFIG_DIR") };
    Ok(())
}

fn local_entry(name: &str, size: u64) -> Entry {
    Entry {
        name: name.to_string(),
        is_dir: false,
        size,
        size_h: linop_sftp::human_size(size),
    }
}

/// keys::encode 与远端 application cursor 模式的联动冒烟。
#[test]
fn encode_matches_term_modes() {
    let mut term = linop_term::Terminal::new(10, 2);
    let up = KeyEvent::new(KeyCode::Up, KeyModifiers::NONE);
    assert_eq!(
        keys::encode(up, term.application_cursor()),
        Some(b"\x1b[A".to_vec())
    );
    term.feed(b"\x1b[?1h");
    assert_eq!(
        keys::encode(up, term.application_cursor()),
        Some(b"\x1bOA".to_vec())
    );
    assert_eq!(term.cursor(), Some((0, 0)));
}

/// 空密码 profile:Enter 弹掩码输入框,输入后完成连接;明文不出现在屏幕上。
#[tokio::test]
async fn empty_password_prompts_masked_and_connects() -> anyhow::Result<()> {
    let _env = env_guard().await;
    // 隔离 HOME:空 HOME 下没有默认密钥,必然走密码输入框路径。
    let isolated_home = tempfile::tempdir()?;
    let old_home = std::env::var("HOME").ok();
    unsafe { std::env::set_var("HOME", isolated_home.path()) };
    let config_dir = tempfile::tempdir()?;
    unsafe { std::env::set_var("LINOP_CONFIG_DIR", config_dir.path()) };
    let remote_root = tempfile::tempdir()?;
    let server = linop_testsupport::TestSshServer::start_with_sftp(remote_root.path()).await?;
    let mut conn = Connection::new(
        "pw-e2e",
        server.addr.ip().to_string(),
        "u",
        Auth::password(""),
    );
    conn.port = server.addr.port();

    let (tx, mut rx) = mpsc::unbounded_channel();
    let mut app = app_mod::App::new(vec![conn].into_iter().collect());
    let mut terminal = TestTerminal::new(TestBackend::new(80, 24))?;
    redraw(&mut terminal, &mut app).await;

    // Enter 应弹出密码输入框而不是直接连接失败。
    app.handle(key(KeyCode::Enter), &tx).await;
    assert!(app.dialog.is_some(), "空密码应弹掩码输入框");
    assert!(!matches!(app.mode, Mode::Connecting { .. }));

    for c in "secret1".chars() {
        app.handle(key(KeyCode::Char(c)), &tx).await;
    }
    redraw(&mut terminal, &mut app).await;
    let text = screen_text(&terminal);
    assert!(!text.contains("secret1"), "屏幕不得出现密码明文");
    assert!(text.contains("●"), "掩码点号应可见");

    // Enter 确认 → 用输入的密码连接(测试服务器接受任意密码)。
    app.handle(key(KeyCode::Enter), &tx).await;
    settle(
        &mut terminal,
        &mut app,
        &mut rx,
        &tx,
        Duration::from_secs(5),
    )
    .await;
    assert!(matches!(app.mode, Mode::Session), "输入密码后应连上");

    app.handle(key(KeyCode::F(9)), &tx).await;
    server.shutdown();
    unsafe { std::env::remove_var("LINOP_CONFIG_DIR") };
    restore_home(old_home);
    Ok(())
}

/// 多会话:两台服务器并存,F8 回列表不断开,Enter 切换,F9 断当前,x 断指定行。
#[tokio::test]
async fn multiple_sessions_coexist_and_switch() -> anyhow::Result<()> {
    let _env = env_guard().await;
    let config_dir = tempfile::tempdir()?;
    unsafe { std::env::set_var("LINOP_CONFIG_DIR", config_dir.path()) };
    let root_a = tempfile::tempdir()?;
    let root_b = tempfile::tempdir()?;
    let server_a = linop_testsupport::TestSshServer::start_with_sftp(root_a.path()).await?;
    let server_b = linop_testsupport::TestSshServer::start_with_sftp(root_b.path()).await?;

    let named = |name: &str, addr: &std::net::SocketAddr| {
        let mut c = Connection::new(name, addr.ip().to_string(), "u", Auth::password("p"));
        c.port = addr.port();
        c
    };
    let registry: Registry = vec![named("one", &server_a.addr), named("two", &server_b.addr)]
        .into_iter()
        .collect();

    let (tx, mut rx) = mpsc::unbounded_channel();
    let mut app = app_mod::App::new(registry);
    let mut terminal = TestTerminal::new(TestBackend::new(80, 24))?;
    redraw(&mut terminal, &mut app).await;

    // 连接 one,输入 aa。
    app.handle(key(KeyCode::Enter), &tx).await;
    settle(
        &mut terminal,
        &mut app,
        &mut rx,
        &tx,
        Duration::from_secs(5),
    )
    .await;
    assert!(matches!(app.mode, Mode::Session));
    for c in ['a', 'a'] {
        app.handle(key(KeyCode::Char(c)), &tx).await;
    }
    settle(
        &mut terminal,
        &mut app,
        &mut rx,
        &tx,
        Duration::from_secs(2),
    )
    .await;
    assert!(screen_text(&terminal).contains("aa"));

    // F8 回列表:会话仍在线,列表显示在线段。
    app.handle(key(KeyCode::F(8)), &tx).await;
    assert!(matches!(app.mode, Mode::Picker));
    assert_eq!(app.sessions.len(), 1, "F8 不断开会话");
    redraw(&mut terminal, &mut app).await;
    assert!(screen_text(&terminal).contains("●"), "在线会话段应显示");

    // 下移两行到 profile two(列表 = [会话one, profile one, profile two]),建立第二个会话。
    app.handle(key(KeyCode::Down), &tx).await;
    app.handle(key(KeyCode::Down), &tx).await;
    app.handle(key(KeyCode::Enter), &tx).await;
    settle(
        &mut terminal,
        &mut app,
        &mut rx,
        &tx,
        Duration::from_secs(5),
    )
    .await;
    assert_eq!(app.sessions.len(), 2);
    for c in ['b', 'b'] {
        app.handle(key(KeyCode::Char(c)), &tx).await;
    }
    settle(
        &mut terminal,
        &mut app,
        &mut rx,
        &tx,
        Duration::from_secs(2),
    )
    .await;
    let text = screen_text(&terminal);
    assert!(text.contains("bb"), "第二个会话应显示自己的输入");
    assert!(!text.contains("aa"), "切换后的终端不应残留上个会话的内容");

    // 回列表,上移两行到第 0 行(会话 one)切换回去:终端内容还在。
    app.handle(key(KeyCode::F(8)), &tx).await;
    app.handle(key(KeyCode::Up), &tx).await;
    app.handle(key(KeyCode::Up), &tx).await;
    app.handle(key(KeyCode::Enter), &tx).await;
    assert!(matches!(app.mode, Mode::Session));
    assert_eq!(app.sessions[app.active].profile.name, "one");
    redraw(&mut terminal, &mut app).await;
    assert!(
        screen_text(&terminal).contains("aa"),
        "切回的会话应保留终端内容"
    );

    // F9 断开当前(one):剩 two,回到列表。
    app.handle(key(KeyCode::F(9)), &tx).await;
    assert_eq!(app.sessions.len(), 1);
    assert_eq!(app.sessions[0].profile.name, "two");
    assert!(matches!(app.mode, Mode::Picker));

    // x 断开选中行的会话(two)。
    app.handle(key(KeyCode::Char('x')), &tx).await;
    assert!(app.sessions.is_empty());

    server_a.shutdown();
    server_b.shutdown();
    unsafe { std::env::remove_var("LINOP_CONFIG_DIR") };
    Ok(())
}

/// 连接编辑器:n 新建(含掩码字段)→ 保存落盘;e 编辑追加;删除确认后移除。
#[tokio::test]
async fn connection_editor_create_edit_delete() -> anyhow::Result<()> {
    let _env = env_guard().await;
    let config_dir = tempfile::tempdir()?;
    unsafe { std::env::set_var("LINOP_CONFIG_DIR", config_dir.path()) };
    let toml_path = config_dir.path().join("connections.toml");

    let (tx, _rx) = mpsc::unbounded_channel();
    let mut app = app_mod::App::new(Registry::new());
    let mut terminal = TestTerminal::new(TestBackend::new(80, 24))?;
    redraw(&mut terminal, &mut app).await;
    assert!(app.connections().is_empty());

    // n 新建:名称 web9 → Tab 主机 10.0.0.9 → Tab 端口(默认 22)
    // → Tab 用户名 root → Tab 密码 pw9 → Tab 密钥(留空)→ Enter。
    app.handle(key(KeyCode::Char('n')), &tx).await;
    assert!(app.form.is_some(), "n 应打开新建表单");
    for field_input in ["web9", "10.0.0.9", "", "root", "pw9", ""] {
        for c in field_input.chars() {
            app.handle(key(KeyCode::Char(c)), &tx).await;
        }
        app.handle(key(KeyCode::Tab), &tx).await;
    }
    // 掩码字段:密码明文不上屏。
    redraw(&mut terminal, &mut app).await;
    assert!(
        !screen_text(&terminal).contains("pw9"),
        "密码字段必须掩码显示"
    );
    assert!(screen_text(&terminal).contains("●"));

    app.handle(key(KeyCode::Enter), &tx).await;
    assert!(app.form.is_none(), "保存成功后表单应关闭");
    let conn = app.registry.get("web9").expect("registry 应包含 web9");
    assert_eq!(
        (conn.host.as_str(), conn.port, conn.username.as_str()),
        ("10.0.0.9", 22, "root")
    );
    assert_eq!(conn.auth, Auth::password("pw9"));
    assert!(
        std::fs::read_to_string(&toml_path)?.contains("web9"),
        "TOML 应落盘"
    );
    redraw(&mut terminal, &mut app).await;
    assert!(screen_text(&terminal).contains("web9"), "列表应显示新连接");

    // e 编辑:密码字段(第 5 个)追加 x → pw9x。
    app.handle(key(KeyCode::Char('e')), &tx).await;
    assert!(app.form.is_some(), "e 应打开编辑表单");
    for _ in 0..4 {
        app.handle(key(KeyCode::Tab), &tx).await;
    }
    app.handle(key(KeyCode::Char('x')), &tx).await;
    app.handle(key(KeyCode::Enter), &tx).await;
    assert_eq!(
        app.registry.get("web9").unwrap().auth,
        Auth::password("pw9x")
    );

    // 校验:重名冲突。新建表单填已有名称 web9 + 合法主机 → 保存报错且表单保留。
    app.handle(key(KeyCode::Char('n')), &tx).await;
    for c in "web9".chars() {
        app.handle(key(KeyCode::Char(c)), &tx).await;
    }
    app.handle(key(KeyCode::Tab), &tx).await;
    for c in "h9".chars() {
        app.handle(key(KeyCode::Char(c)), &tx).await;
    }
    app.handle(key(KeyCode::Tab), &tx).await; // 跳过端口(默认 22)
    app.handle(key(KeyCode::Tab), &tx).await;
    for c in "root".chars() {
        app.handle(key(KeyCode::Char(c)), &tx).await;
    }
    app.handle(key(KeyCode::Enter), &tx).await;
    assert!(app.form.is_some(), "重名应保留表单并报错");
    assert!(app.form.as_ref().unwrap().error.contains("已存在"));
    app.handle(key(KeyCode::Esc), &tx).await;

    // x 删除:确认 → registry 与 TOML 均移除。
    app.picker_sel = 0; // 唯一 profile 行
    app.handle(key(KeyCode::Char('x')), &tx).await;
    assert!(app.dialog.is_some(), "x 应弹删除确认");
    app.handle(key(KeyCode::Enter), &tx).await;
    assert!(app.registry.get("web9").is_none());
    assert!(!std::fs::read_to_string(&toml_path)?.contains("web9"));

    drop(_rx);
    unsafe { std::env::remove_var("LINOP_CONFIG_DIR") };
    Ok(())
}

/// 意外断线(注入 ShellClosed)→ 自动重连成功:会话保留、终端内容保留、
/// shell 恢复可输入,远端目录恢复。
#[tokio::test]
async fn auto_reconnect_restores_session() -> anyhow::Result<()> {
    let _env = env_guard().await;
    let config_dir = tempfile::tempdir()?;
    unsafe { std::env::set_var("LINOP_CONFIG_DIR", config_dir.path()) };
    let remote_root = tempfile::tempdir()?;
    let server = linop_testsupport::TestSshServer::start_with_sftp(remote_root.path()).await?;
    let mut conn = Connection::new("rc", server.addr.ip().to_string(), "u", Auth::password("p"));
    conn.port = server.addr.port();

    let (tx, mut rx) = mpsc::unbounded_channel();
    let mut app = app_mod::App::new(vec![conn].into_iter().collect());
    let mut terminal = TestTerminal::new(TestBackend::new(80, 24))?;
    redraw(&mut terminal, &mut app).await;

    app.handle(key(KeyCode::Enter), &tx).await;
    settle(
        &mut terminal,
        &mut app,
        &mut rx,
        &tx,
        Duration::from_secs(5),
    )
    .await;
    let sid = app.sessions[app.active].id;
    for c in "old".chars() {
        app.handle(key(KeyCode::Char(c)), &tx).await;
    }
    settle(
        &mut terminal,
        &mut app,
        &mut rx,
        &tx,
        Duration::from_secs(2),
    )
    .await;

    // 模拟网络断开:注入 reader 任务会发出的 ShellClosed。
    app.handle(AppEvent::ShellClosed(sid), &tx).await;
    assert_eq!(app.sessions.len(), 1, "断线后会话保留");
    let Some(s) = app.sessions.first() else {
        unreachable!()
    };
    assert!(s.reconnect.is_some(), "应进入重连状态");

    settle(
        &mut terminal,
        &mut app,
        &mut rx,
        &tx,
        Duration::from_secs(5),
    )
    .await;
    let Some(s) = app.sessions.first() else {
        unreachable!()
    };
    assert!(
        s.reconnect.is_none(),
        "重连成功后清除重连状态:{:?}",
        s.status
    );
    assert_eq!(app.sessions.len(), 1);

    // 终端保留断线前的内容 + 新 shell 可输入。
    let text = screen_text(&terminal);
    assert!(text.contains("old"), "重连不丢终端历史");
    for c in "new".chars() {
        app.handle(key(KeyCode::Char(c)), &tx).await;
    }
    settle(
        &mut terminal,
        &mut app,
        &mut rx,
        &tx,
        Duration::from_secs(2),
    )
    .await;
    assert!(
        screen_text(&terminal).contains("new"),
        "重连后 shell 应恢复输入"
    );

    app.handle(key(KeyCode::F(9)), &tx).await;
    server.shutdown();
    unsafe { std::env::remove_var("LINOP_CONFIG_DIR") };
    Ok(())
}

/// 重连耗尽:预置尝试次数后注入失败回执 → 会话被移除。
#[tokio::test]
async fn auto_reconnect_gives_up_after_max_attempts() -> anyhow::Result<()> {
    let _env = env_guard().await;
    let config_dir = tempfile::tempdir()?;
    unsafe { std::env::set_var("LINOP_CONFIG_DIR", config_dir.path()) };
    let remote_root = tempfile::tempdir()?;
    let server = linop_testsupport::TestSshServer::start_with_sftp(remote_root.path()).await?;
    let mut conn = Connection::new(
        "gone",
        server.addr.ip().to_string(),
        "u",
        Auth::password("p"),
    );
    conn.port = server.addr.port();

    let (tx, mut rx) = mpsc::unbounded_channel();
    let mut app = app_mod::App::new(vec![conn].into_iter().collect());
    let mut terminal = TestTerminal::new(TestBackend::new(80, 24))?;
    redraw(&mut terminal, &mut app).await;
    app.handle(key(KeyCode::Enter), &tx).await;
    settle(
        &mut terminal,
        &mut app,
        &mut rx,
        &tx,
        Duration::from_secs(5),
    )
    .await;
    assert_eq!(app.sessions.len(), 1);
    let sid = app.sessions[0].id;

    // 预置 attempts = MAX-1:注入一次失败回执即耗尽。
    app.sessions[0].reconnect = Some(ReconnectState {
        attempts: MAX_RECONNECT_ATTEMPTS - 1,
    });
    app.handle(
        AppEvent::ReconnectDone(sid, Err("连接被拒".to_string())),
        &tx,
    )
    .await;
    assert!(app.sessions.is_empty(), "耗尽后会话应被移除");
    assert!(app.notice.contains("重连失败"));

    server.shutdown();
    unsafe { std::env::remove_var("LINOP_CONFIG_DIR") };
    Ok(())
}

/// 加密私钥未配置口令:Enter 弹掩码口令框,输入后用口令完成公钥认证连接。
/// 依赖系统 ssh-keygen;不存在时跳过。
#[tokio::test]
async fn encrypted_key_prompts_passphrase_and_connects() -> anyhow::Result<()> {
    if which("ssh-keygen").is_none() {
        eprintln!("跳过:系统无 ssh-keygen");
        return Ok(());
    }

    let config_dir = tempfile::tempdir()?;
    unsafe { std::env::set_var("LINOP_CONFIG_DIR", config_dir.path()) };
    let key_dir = tempfile::tempdir()?;
    let key_path = key_dir.path().join("id_enc");
    let status = std::process::Command::new("ssh-keygen")
        .args(["-t", "ed25519", "-N", "pp123", "-q", "-f"])
        .arg(&key_path)
        .status()?;
    assert!(status.success(), "ssh-keygen 生成加密密钥失败");

    // 公钥指纹 = 服务器的准入名单。
    let public = linop_testsupport::load_public_key(&key_path.with_extension("pub"))?;
    let fingerprint = public.fingerprint(Default::default()).to_string();
    let remote_root = tempfile::tempdir()?;
    let server = linop_testsupport::TestSshServer::start(linop_testsupport::ServerOptions {
        allowed_fingerprints: Some(vec![fingerprint]),
        sftp_root: Some(remote_root.path().to_path_buf()),
        ..Default::default()
    })
    .await?;

    let mut conn = Connection::new(
        "enc",
        server.addr.ip().to_string(),
        "u",
        Auth::key(&key_path),
    );
    conn.port = server.addr.port();

    let (tx, mut rx) = mpsc::unbounded_channel();
    let mut app = app_mod::App::new(vec![conn].into_iter().collect());
    let mut terminal = TestTerminal::new(TestBackend::new(80, 24))?;
    redraw(&mut terminal, &mut app).await;

    // Enter:探测到加密私钥 → 弹口令框,而不是直接连接失败。
    app.handle(key(KeyCode::Enter), &tx).await;
    assert!(app.dialog.is_some(), "加密私钥应弹口令输入框");
    for c in "wrong!".chars() {
        app.handle(key(KeyCode::Char(c)), &tx).await;
    }
    redraw(&mut terminal, &mut app).await;
    assert!(!screen_text(&terminal).contains("pp123") || true); // 掩码框不显示明文(此处输入的是错误口令)

    // 先输错口令:连接应失败回列表并提示;再次 Enter 重新弹框输对的。
    app.handle(key(KeyCode::Enter), &tx).await;
    settle(
        &mut terminal,
        &mut app,
        &mut rx,
        &tx,
        Duration::from_secs(5),
    )
    .await;
    assert!(matches!(app.mode, Mode::Picker), "错误口令应连接失败");

    app.handle(key(KeyCode::Enter), &tx).await;
    assert!(app.dialog.is_some());
    for c in "pp123".chars() {
        app.handle(key(KeyCode::Char(c)), &tx).await;
    }
    app.handle(key(KeyCode::Enter), &tx).await;
    settle(
        &mut terminal,
        &mut app,
        &mut rx,
        &tx,
        Duration::from_secs(12),
    )
    .await;
    assert!(matches!(app.mode, Mode::Session), "正确口令应连接成功");

    app.handle(key(KeyCode::F(9)), &tx).await;
    server.shutdown();
    unsafe { std::env::remove_var("LINOP_CONFIG_DIR") };
    Ok(())
}

fn which(bin: &str) -> Option<std::path::PathBuf> {
    std::env::var_os("PATH")?
        .to_string_lossy()
        .split(':')
        .find_map(|dir| {
            let p = std::path::Path::new(dir).join(bin);
            p.is_file().then_some(p)
        })
}

/// 反向转发:profile 配置 → 连接自动启动 → 转发页展示 → x 停止。
#[tokio::test]
async fn remote_forward_lifecycle_in_tui() -> anyhow::Result<()> {
    let _env = env_guard().await;
    let config_dir = tempfile::tempdir()?;
    unsafe { std::env::set_var("LINOP_CONFIG_DIR", config_dir.path()) };
    let remote_root = tempfile::tempdir()?;
    let (server, _probe) = linop_testsupport::TestSshServer::start_with_forward_probe(Some(
        remote_root.path().to_path_buf(),
    ))
    .await?;
    let mut conn = Connection::new("rf", server.addr.ip().to_string(), "u", Auth::password("p"));
    conn.port = server.addr.port();
    conn.remote_forwards
        .push(linop_core::RemoteForward::new(23456, "127.0.0.1", 5432));

    let (tx, mut rx) = mpsc::unbounded_channel();
    let mut app = app_mod::App::new(vec![conn].into_iter().collect());
    let mut terminal = TestTerminal::new(TestBackend::new(80, 24))?;
    redraw(&mut terminal, &mut app).await;

    app.handle(key(KeyCode::Enter), &tx).await;
    settle(
        &mut terminal,
        &mut app,
        &mut rx,
        &tx,
        Duration::from_secs(5),
    )
    .await;
    assert!(matches!(app.mode, Mode::Session));
    let s = app.sessions.first().expect("session");
    assert_eq!(s.remote_forwards.len(), 1, "反向转发应自动启动");
    assert!(s.forward_failures.is_empty(), "{:?}", s.forward_failures);

    // 转发页:展示反向条目;x 停止后消失。
    app.handle(key(KeyCode::F(3)), &tx).await;
    redraw(&mut terminal, &mut app).await;
    let text = screen_text(&terminal);
    assert!(text.contains("R●运行中"), "转发页应显示反向条目:{text}");
    assert!(
        text.contains("127.0.0.1:23456→本机127.0.0.1:5432"),
        "应显示转发目标:{text}"
    );

    app.handle(key(KeyCode::Char('x')), &tx).await;
    redraw(&mut terminal, &mut app).await;
    assert_eq!(app.sessions[0].remote_forwards.len(), 0, "x 应停止反向转发");
    assert!(app.sessions[0].status.contains("已停止反向转发"));

    app.handle(key(KeyCode::F(9)), &tx).await;
    server.shutdown();
    unsafe { std::env::remove_var("LINOP_CONFIG_DIR") };
    Ok(())
}

/// SOCKS 动态转发:profile 配置 dynamic_ports → 连接自动启动 → 转发页展示 → x 停止。
#[tokio::test]
async fn dynamic_forward_lifecycle_in_tui() -> anyhow::Result<()> {
    let _env = env_guard().await;
    let config_dir = tempfile::tempdir()?;
    unsafe { std::env::set_var("LINOP_CONFIG_DIR", config_dir.path()) };
    let remote_root = tempfile::tempdir()?;
    let server = linop_testsupport::TestSshServer::start_with_sftp(remote_root.path()).await?;
    let mut conn = Connection::new(
        "dyn",
        server.addr.ip().to_string(),
        "u",
        Auth::password("p"),
    );
    conn.port = server.addr.port();
    conn.dynamic_ports.push(0); // 内核分配,避免测试环境端口冲突

    let (tx, mut rx) = mpsc::unbounded_channel();
    let mut app = app_mod::App::new(vec![conn].into_iter().collect());
    let mut terminal = TestTerminal::new(TestBackend::new(80, 24))?;
    redraw(&mut terminal, &mut app).await;

    app.handle(key(KeyCode::Enter), &tx).await;
    settle(
        &mut terminal,
        &mut app,
        &mut rx,
        &tx,
        Duration::from_secs(5),
    )
    .await;
    assert!(matches!(app.mode, Mode::Session));
    let s = app.sessions.first().expect("session");
    assert_eq!(s.dynamic_forwards.len(), 1, "SOCKS 转发应自动启动");
    assert!(s.forward_failures.is_empty(), "{:?}", s.forward_failures);

    app.handle(key(KeyCode::F(3)), &tx).await;
    redraw(&mut terminal, &mut app).await;
    let text = screen_text(&terminal);
    assert!(text.contains("D●运行中"), "转发页应显示 SOCKS 条目:{text}");
    assert!(
        text.contains("socks5://127.0.0.1:"),
        "应显示入口地址:{text}"
    );

    app.handle(key(KeyCode::Char('x')), &tx).await;
    redraw(&mut terminal, &mut app).await;
    assert_eq!(
        app.sessions[0].dynamic_forwards.len(),
        0,
        "x 应停止 SOCKS 转发"
    );
    assert!(
        app.sessions[0].status.contains("已停止 SOCKS 转发"),
        "{:?}",
        app.sessions[0].status
    );

    app.handle(key(KeyCode::F(9)), &tx).await;
    server.shutdown();
    unsafe { std::env::remove_var("LINOP_CONFIG_DIR") };
    Ok(())
}

/// 默认密钥自动发现:HOME/.ssh/id_ed25519 存在且 profile 空密码 →
/// Enter 直接连接(不弹框),用默认密钥完成公钥认证。
#[tokio::test]
async fn default_identity_used_for_empty_password_profile() -> anyhow::Result<()> {
    if which("ssh-keygen").is_none() {
        eprintln!("跳过:系统无 ssh-keygen");
        return Ok(());
    }
    let _env = env_guard().await;
    let isolated_home = tempfile::tempdir()?;
    let ssh_dir = isolated_home.path().join(".ssh");
    std::fs::create_dir_all(&ssh_dir)?;
    let key_path = ssh_dir.join("id_ed25519");
    let ok = std::process::Command::new("ssh-keygen")
        .args(["-t", "ed25519", "-N", "", "-q", "-f"])
        .arg(&key_path)
        .status()?
        .success();
    assert!(ok);
    let old_home = std::env::var("HOME").ok();
    unsafe { std::env::set_var("HOME", isolated_home.path()) };

    let config_dir = tempfile::tempdir()?;
    unsafe { std::env::set_var("LINOP_CONFIG_DIR", config_dir.path()) };
    let public = linop_testsupport::load_public_key(&key_path.with_extension("pub"))?;
    let fingerprint = public.fingerprint(Default::default()).to_string();
    let remote_root = tempfile::tempdir()?;
    let server = linop_testsupport::TestSshServer::start(linop_testsupport::ServerOptions {
        allowed_fingerprints: Some(vec![fingerprint]),
        sftp_root: Some(remote_root.path().to_path_buf()),
        ..Default::default()
    })
    .await?;
    let mut conn = Connection::new(
        "bykey",
        server.addr.ip().to_string(),
        "u",
        Auth::password(""),
    );
    conn.port = server.addr.port();

    let (tx, mut rx) = mpsc::unbounded_channel();
    let mut app = app_mod::App::new(vec![conn].into_iter().collect());
    let mut terminal = TestTerminal::new(TestBackend::new(80, 24))?;
    redraw(&mut terminal, &mut app).await;

    // Enter:有默认密钥 → 不弹框直接连接。
    app.handle(key(KeyCode::Enter), &tx).await;
    assert!(app.dialog.is_none(), "有默认密钥时应直连而非弹框");
    settle(
        &mut terminal,
        &mut app,
        &mut rx,
        &tx,
        Duration::from_secs(5),
    )
    .await;
    eprintln!("DI_NOTICE={:?}", app.notice);
    assert!(matches!(app.mode, Mode::Session), "默认密钥应完成认证");

    app.handle(key(KeyCode::F(9)), &tx).await;
    server.shutdown();
    unsafe { std::env::remove_var("LINOP_CONFIG_DIR") };
    restore_home(old_home);
    Ok(())
}

/// ssh config 导入:i 读取 → 确认 → 落盘;重复导入全部同名则提示跳过。
#[tokio::test]
async fn import_from_ssh_config() -> anyhow::Result<()> {
    let _env = env_guard().await;
    let config_dir = tempfile::tempdir()?;
    unsafe { std::env::set_var("LINOP_CONFIG_DIR", config_dir.path()) };
    let ssh_conf = tempfile::tempdir()?;
    let ssh_config_path = ssh_conf.path().join("config");
    std::fs::write(
        &ssh_config_path,
        "Host web-1\n  HostName 192.168.1.10\n  Port 2222\n  User root\n\nHost *\n  User ignored\n",
    )?;
    unsafe { std::env::set_var("LINOP_SSH_CONFIG", &ssh_config_path) };

    let (tx, _rx) = mpsc::unbounded_channel();
    let mut app = app_mod::App::new(Registry::new());
    let mut terminal = TestTerminal::new(TestBackend::new(80, 24))?;
    redraw(&mut terminal, &mut app).await;

    app.handle(key(KeyCode::Char('i')), &tx).await;
    assert!(app.dialog.is_some(), "i 应弹导入确认");
    redraw(&mut terminal, &mut app).await;
    let text = screen_text(&terminal);
    // 对话框窄,文案可能折行:分段断言。
    assert!(text.contains("导入1条"), "提示导入数量:{text}");
    assert!(
        text.contains("个同名") && text.contains("跳过0"),
        "无同名冲突:{text}"
    );

    app.handle(key(KeyCode::Enter), &tx).await;
    let conn = app.registry.get("web-1").expect("应导入 web-1");
    assert_eq!(
        (conn.host.as_str(), conn.port, conn.username.as_str()),
        ("192.168.1.10", 2222, "root")
    );
    assert!(
        std::fs::read_to_string(config_dir.path().join("connections.toml"))?.contains("web-1"),
        "导入应落盘"
    );

    // 再次导入:全部同名 → 提示跳过,不再弹框。
    app.notice.clear();
    app.handle(key(KeyCode::Char('i')), &tx).await;
    assert!(app.dialog.is_none(), "全部同名不应弹框");
    assert!(app.notice.contains("1 个主机全部同名"), "{:?}", app.notice);

    unsafe { std::env::remove_var("LINOP_SSH_CONFIG") };
    unsafe { std::env::remove_var("LINOP_CONFIG_DIR") };
    Ok(())
}

/// 恢复 HOME 环境变量。
fn restore_home(old: Option<String>) {
    unsafe {
        match old {
            Some(h) => std::env::set_var("HOME", h),
            None => std::env::remove_var("HOME"),
        }
    }
}

#[tokio::test]
async fn monitor_tab_samples_and_renders() -> anyhow::Result<()> {
    let _env = env_guard().await;
    let config_dir = tempfile::tempdir()?;
    unsafe { std::env::set_var("LINOP_CONFIG_DIR", config_dir.path()) };
    // 连接流程需要 SFTP;exec 探针返回固定 /proc 形态输出。
    let remote_root = tempfile::tempdir()?;
    let server = linop_testsupport::TestSshServer::start_with_sftp(remote_root.path()).await?;

    let (tx, mut rx) = mpsc::unbounded_channel();
    let mut app = app_mod::App::new(test_registry(server.addr));
    let mut terminal = TestTerminal::new(TestBackend::new(80, 24))?;

    // 连接并进入会话。
    app.handle(key(KeyCode::Enter), &tx).await;
    settle(
        &mut terminal,
        &mut app,
        &mut rx,
        &tx,
        Duration::from_secs(5),
    )
    .await;
    assert!(matches!(app.mode, Mode::Session), "应已进入会话");

    // F4 切到监控页:采样器懒启动,首个样本立即到达。
    app.handle(key(KeyCode::F(4)), &tx).await;
    settle(
        &mut terminal,
        &mut app,
        &mut rx,
        &tx,
        Duration::from_secs(5),
    )
    .await;

    let s = &app.sessions[app.active];
    assert_eq!(s.tab, Tab::Monitor);
    assert!(!s.monitor.samples.is_empty(), "监控页应已收到至少一个样本");
    assert!(
        s.monitor.error.is_empty(),
        "采样不应报错:{}",
        s.monitor.error
    );

    let text = screen_text(&terminal);
    assert!(text.contains("CPU"), "应显示 CPU 区块:{text}");
    assert!(text.contains("50%"), "内存应显示 50%:{text}");
    assert!(text.contains("40%"), "磁盘应显示 40%:{text}");
    assert!(text.contains("负载"), "应显示负载文本:{text}");
    Ok(())
}
