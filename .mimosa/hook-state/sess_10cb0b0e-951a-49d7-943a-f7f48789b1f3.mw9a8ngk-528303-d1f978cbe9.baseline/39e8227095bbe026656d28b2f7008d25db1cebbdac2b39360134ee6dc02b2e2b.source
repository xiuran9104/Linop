//! 端口转发隧道回归:裸 RESP → LocalForward → 测试 SSH 服务器 → 迷你 Redis。
//! 防止桥接通道再次被会话回显污染(曾把隧道数据原样弹回)。

use linop_core::{Auth, Connection, PortForward};
use linop_ssh::{KnownHostsPolicy, Session};
use linop_testsupport::{RedisData, TestRedisServer, TestSshServer};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

#[tokio::test]
async fn tunnel_forwards_resp_without_echo_pollution() -> anyhow::Result<()> {
    let redis = TestRedisServer::start(RedisData::default().with_string("k", "v"))?;
    let ssh = TestSshServer::start_echo().await?;

    let mut jump = Connection::new("j", ssh.addr.ip().to_string(), "u", Auth::password("p"));
    jump.port = ssh.addr.port();
    let session = Session::connect_profile(&jump, KnownHostsPolicy::AcceptAll).await?;

    let rule = PortForward::new(0, "127.0.0.1", redis.addr.port());
    let forward = session.open_local_forward(&rule).await?;
    tokio::time::sleep(std::time::Duration::from_millis(50)).await;

    let mut sock = tokio::net::TcpStream::connect(forward.local_addr).await?;
    sock.write_all(b"*1\r\n$4\r\nPING\r\n").await?;
    let reply = read_line(&mut sock).await;
    assert_eq!(reply, "+PONG", "隧道应转发到 Redis 而不是回显命令");

    // 取值也走隧道。
    sock.write_all(b"*3\r\n$3\r\nGET\r\n$1\r\nk\r\n$0\r\n\r\n")
        .await?;
    let header = read_line(&mut sock).await;
    assert_eq!(header, "$1", "GET 应返回 bulk 头");
    let mut value = [0u8; 3];
    sock.read_exact(&mut value).await?;
    assert_eq!(&value, b"v\r\n");
    Ok(())
}

async fn read_line(sock: &mut tokio::net::TcpStream) -> String {
    let mut buf = Vec::new();
    let mut byte = [0u8; 1];
    loop {
        sock.read_exact(&mut byte).await.expect("连接不应提前关闭");
        buf.push(byte[0]);
        if buf.ends_with(b"\r\n") {
            buf.truncate(buf.len() - 2);
            return String::from_utf8_lossy(&buf).into_owned();
        }
    }
}
