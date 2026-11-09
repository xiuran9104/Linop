//! Telnet 客户端端到端:linop-telnet ↔ testsupport 迷你服务器。

use linop_telnet::{TelnetClient, TelnetEvent};
use linop_testsupport::TestTelnetServer;

/// 轮询直到输出包含目标字节(总时限 3s)。
fn wait_for(client: &mut TelnetClient, needle: &[u8], acc: &mut Vec<u8>) {
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(3);
    while !acc.windows(needle.len()).any(|w| w == needle) {
        assert!(
            std::time::Instant::now() < deadline,
            "等待 {needle:?} 超时,已有 {:?} 字节",
            acc.len()
        );
        match client.poll().expect("连接不应死亡") {
            TelnetEvent::Data(bytes) => acc.extend_from_slice(&bytes),
            TelnetEvent::Idle => continue,
        }
    }
}

#[test]
fn banner_negotiation_and_echo() -> anyhow::Result<()> {
    let server = TestTelnetServer::start()?;
    let mut client = TelnetClient::connect(&server.addr.ip().to_string(), server.addr.port())?;

    // 横幅(协商 WILL 已被剥离并自动回 DONT)。
    let mut acc = Vec::new();
    wait_for(&mut client, b"Welcome to linop-telnet-test", &mut acc);
    assert!(!acc.contains(&255), "协商字节不应透传:{acc:?}");

    // 回显往返。
    client.write(b"hello linop\r\n")?;
    wait_for(&mut client, b"> hello linop", &mut acc);

    // 二进制安全:0xFF 需转义后原样返回。
    client.write(&[0xFF, 0x01, 0x0D, 0x0A])?;
    wait_for(&mut client, &[0xFF, 0x01], &mut acc);
    Ok(())
}
