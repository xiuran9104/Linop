//! Linop 入口:SSH / SFTP / 端口转发一体式终端工作区。
//!
//! 用法:`linop [profile]`。不带参数进入连接选择器;
//! 带连接名则直接连接(connections.toml 中的表名)。

use anyhow::Context;

fn main() -> anyhow::Result<()> {
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()?;
    runtime.block_on(run())
}

async fn run() -> anyhow::Result<()> {
    let profile_arg = std::env::args().nth(1);
    let registry = linop_core::Registry::load_default().with_context(|| {
        format!(
            "加载 {} 失败",
            linop_core::default_connections_path().display()
        )
    })?;

    let mut terminal = ratatui::init::init();
    let _ = crossterm::execute!(std::io::stdout(), crossterm::event::EnableBracketedPaste);

    let result = async {
        let (tx, rx) = tokio::sync::mpsc::unbounded_channel();
        linop::app::spawn_input_thread(tx.clone());

        let mut application = linop::app::App::new(registry);
        if let Some(name) = profile_arg {
            if let Some(conn) = application.registry.get(&name).cloned() {
                application.connect(&conn, &tx);
            } else {
                application.notice = format!("找不到连接 {name:?},按 r 重载或直接选择列表");
            }
        }
        linop::app::run(&mut terminal, application, rx, tx).await
    }
    .await;

    let _ = crossterm::execute!(std::io::stdout(), crossterm::event::DisableBracketedPaste);
    ratatui::init::restore();
    result
}
