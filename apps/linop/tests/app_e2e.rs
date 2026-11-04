//! 应用层端到端:TestSshServer(含 SFTP)↔ 完整 App 状态机 + TestBackend 渲染。
//!
//! 走真实事件循环路径:picker 选择 → 连接 → shell 回显 → 文件上传/下载 → 断开。

use std::time::{Duration, Instant};

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use linop_core::{Auth, Connection, Registry};
use linop_sftp::Entry;
use ratatui::backend::TestBackend;
use tokio::sync::mpsc;

use linop::app::{self as app_mod, App, AppEvent, Mode, Pane};
use linop::{keys, ui};

type TestTerminal = ratatui::Terminal<TestBackend>;

fn key(code: KeyCode) -> AppEvent {
    AppEvent::Key(KeyEvent::new(code, KeyModifiers::NONE))
}

/// 渲染一帧并把事件处理到静默(300ms 无新事件)或超时为止。
async fn settle(
    terminal: &mut TestTerminal,
    app: &mut App,
    rx: &mut mpsc::UnboundedReceiver<AppEvent>,
    tx: &mpsc::UnboundedSender<AppEvent>,
    wait: Duration,
) {
    let deadline = Instant::now() + wait;
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
    Ok(())
}

/// 多会话:两台服务器并存,F8 回列表不断开,Enter 切换,F9 断当前,x 断指定行。
#[tokio::test]
async fn multiple_sessions_coexist_and_switch() -> anyhow::Result<()> {
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
