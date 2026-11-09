//! 监控采样端到端:Monitor(真实 SSH exec)↔ TestSshServer exec 固定探针输出。

use linop_core::{Auth, Connection};
use linop_monitor::Monitor;
use linop_ssh::{KnownHostsPolicy, Session};
use linop_testsupport::TestSshServer;

#[tokio::test]
async fn monitor_samples_from_exec_probe() -> anyhow::Result<()> {
    let server = TestSshServer::start_echo().await?;
    let mut conn = Connection::new("m", server.addr.ip().to_string(), "u", Auth::password("p"));
    conn.port = server.addr.port();
    let ssh = Session::connect_profile(&conn, KnownHostsPolicy::AcceptAll).await?;

    // exec 基础能力:非探针命令回显回执。
    let plain = ssh.exec("echo hi").await?;
    assert_eq!(plain, b"ran: echo hi\n");

    let mut monitor = Monitor::spawn(ssh);
    let sample = match tokio::time::timeout(std::time::Duration::from_secs(5), monitor.next()).await
    {
        Ok(Some(Ok(sample))) => sample,
        other => panic!("首个采样失败或超时: {other:?}"),
    };
    // 期望值与 PROBE_FIXTURE 一致。
    assert!((sample.load1 - 0.10).abs() < 1e-9);
    assert!((sample.cpu_percent - 100.0 * 70.0 / 880.0).abs() < 1e-6);
    assert_eq!(sample.mem_percent(), 50.0);
    assert!((sample.disk_percent() - 40.0).abs() < 0.01);
    monitor.stop();
    Ok(())
}

#[tokio::test]
async fn session_exec_against_blackhole_times_out() -> anyhow::Result<()> {
    // Session::exec 必须可被超时截断:坏连接不允许挂死调用方。
    let listener = tokio::net::TcpListener::bind(("127.0.0.1", 0)).await?;
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        if let Ok((socket, _)) = listener.accept().await {
            let _socket = socket;
            std::future::pending::<()>().await;
        }
    });
    let mut conn = Connection::new("bh", addr.ip().to_string(), "u", Auth::password("p"));
    conn.port = addr.port();
    let err = tokio::time::timeout(
        std::time::Duration::from_secs(2),
        Session::connect_profile(&conn, KnownHostsPolicy::AcceptAll),
    )
    .await;
    assert!(err.is_err(), "黑洞服务器应触发超时");
    Ok(())
}
