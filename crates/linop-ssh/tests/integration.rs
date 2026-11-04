//! 进程内端到端测试:真实 russh 客户端 ↔ linop-testsupport 服务端。

use linop_core::{Auth, Connection, PortForward};
use linop_ssh::{KnownHosts, KnownHostsPolicy, Session};
use linop_testsupport::TestSshServer;

fn conn_to(addr: &std::net::SocketAddr, auth: Auth) -> Connection {
    let mut c = Connection::new("test", addr.ip().to_string(), "user", auth);
    c.port = addr.port();
    c
}

#[tokio::test]
async fn password_auth_success_and_failure() -> anyhow::Result<()> {
    let server = TestSshServer::start_with_password("user", "right-pass").await?;

    let ok = conn_to(&server.addr, Auth::password("right-pass"));
    let session = Session::connect_profile(&ok, KnownHostsPolicy::AcceptAll).await;
    assert!(session.is_ok(), "correct password should authenticate");

    let bad = conn_to(&server.addr, Auth::password("wrong-pass"));
    assert!(
        Session::connect_profile(&bad, KnownHostsPolicy::AcceptAll)
            .await
            .is_err(),
        "wrong password must fail"
    );
    Ok(())
}

#[tokio::test]
async fn publickey_auth_with_generated_key() -> anyhow::Result<()> {
    use russh::keys::ssh_key::private::Ed25519Keypair;

    let dir = tempfile::tempdir()?;
    let key_path = dir.path().join("id_ed25519");
    let keypair = Ed25519Keypair::from_seed(&[42u8; 32]);
    let private = russh::keys::PrivateKey::from(keypair);
    std::fs::write(
        &key_path,
        private
            .to_openssh(russh::keys::ssh_key::LineEnding::LF)?
            .as_str(),
    )?;

    let fingerprint = private
        .public_key()
        .fingerprint(Default::default())
        .to_string();
    let server = linop_testsupport::TestSshServer::start(linop_testsupport::ServerOptions {
        allowed_fingerprints: Some(vec![fingerprint]),
        ..Default::default()
    })
    .await?;

    let profile = conn_to(&server.addr, Auth::key(&key_path));
    assert!(
        Session::connect_profile(&profile, KnownHostsPolicy::AcceptAll)
            .await
            .is_ok(),
        "publickey auth should succeed"
    );
    Ok(())
}

#[tokio::test]
async fn shell_echo_roundtrip() -> anyhow::Result<()> {
    let server = TestSshServer::start_echo().await?;
    let profile = conn_to(&server.addr, Auth::password("x"));
    let session = Session::connect_profile(&profile, KnownHostsPolicy::AcceptAll).await?;

    let mut shell = session.open_shell(80, 24).await?;
    shell.write(b"ping\n").await?;
    let echoed = tokio::time::timeout(std::time::Duration::from_secs(3), shell.read_output())
        .await
        .expect("echo should arrive in time");
    assert_eq!(echoed, Some(b"ping\n".to_vec()));

    shell.resize(100, 30).await?;
    session.disconnect().await?;
    Ok(())
}

#[tokio::test]
async fn local_forward_to_echo_server() -> anyhow::Result<()> {
    // 起一个真实 TCP echo 服务器作为转发目标。
    let echo_listener = tokio::net::TcpListener::bind(("127.0.0.1", 0)).await?;
    let echo_addr = echo_listener.local_addr()?;
    tokio::spawn(async move {
        loop {
            let Ok((mut sock, _)) = echo_listener.accept().await else {
                break;
            };
            tokio::spawn(async move {
                let (mut r, mut w) = sock.split();
                let _ = tokio::io::copy(&mut r, &mut w).await;
            });
        }
    });

    let server = TestSshServer::start_echo().await?;
    let profile = conn_to(&server.addr, Auth::password("x"));
    let session = Session::connect_profile(&profile, KnownHostsPolicy::AcceptAll).await?;

    // 本地端口选 0,由内核分配,避免测试环境端口冲突。
    let fwd = PortForward::new(0, echo_addr.ip().to_string(), echo_addr.port());
    let forward = session.open_local_forward(&fwd).await?;
    assert_eq!(forward.local_addr.ip().to_string(), "127.0.0.1");

    let mut local = tokio::net::TcpStream::connect(forward.local_addr).await?;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    local.write_all(b"hello forward").await?;
    let mut buf = vec![0u8; 64];
    let n = tokio::time::timeout(std::time::Duration::from_secs(3), local.read(&mut buf)).await??;
    assert_eq!(&buf[..n], b"hello forward");

    forward.stop();
    Ok(())
}

#[tokio::test]
async fn tofu_known_hosts_detects_mismatch() -> anyhow::Result<()> {
    let server = TestSshServer::start_echo().await?;
    let dir = tempfile::tempdir()?;
    let mut hosts = KnownHosts::with_path(dir.path().join("known_hosts"));
    // 预置一个错误指纹,模拟中间人。
    hosts.decide("127.0.0.1", server.addr.port(), "SHA256:WRONG");

    let profile = conn_to(&server.addr, Auth::password("x"));
    let err = Session::connect_profile(&profile, KnownHostsPolicy::TrustOnFirstUse(hosts)).await;
    assert!(err.is_err(), "host key mismatch must fail the connection");
    Ok(())
}

#[tokio::test]
async fn remote_forward_tunnel_reaches_local_target() -> anyhow::Result<()> {
    // 本地 echo 目标:反向通道的流量应经客户端桥回这里。
    let echo_listener = tokio::net::TcpListener::bind(("127.0.0.1", 0)).await?;
    let echo_addr = echo_listener.local_addr()?;
    tokio::spawn(async move {
        loop {
            let Ok((mut sock, _)) = echo_listener.accept().await else {
                break;
            };
            tokio::spawn(async move {
                let (mut r, mut w) = sock.split();
                let _ = tokio::io::copy(&mut r, &mut w).await;
            });
        }
    });

    let (server, mut probe) =
        linop_testsupport::TestSshServer::start_with_forward_probe(None).await?;
    let profile = conn_to(&server.addr, Auth::password("x"));
    let ssh = Session::connect_profile(&profile, KnownHostsPolicy::AcceptAll).await?;

    // 请求远端监听 23456;探针应把 "rf-probe" 桥到本地 echo 再回来。
    let rule = linop_core::RemoteForward::new(23456, echo_addr.ip().to_string(), echo_addr.port());
    let fwd = ssh.open_remote_forward(&rule).await?;
    assert_eq!(fwd.bind_port, 23456);
    assert_eq!(
        fwd.to_string(),
        format!("127.0.0.1:23456 → 本机 127.0.0.1:{}", echo_addr.port())
    );

    let mut echoed = Vec::new();
    for _ in 0..20 {
        if probe.changed().await.is_ok() && !probe.borrow().is_empty() {
            echoed = probe.borrow().clone();
            break;
        }
    }
    assert_eq!(
        echoed,
        b"rf-probe".to_vec(),
        "探针应经客户端桥接到本地目标并回显"
    );

    fwd.stop().await;
    ssh.disconnect().await?;
    Ok(())
}
