//! 测试用迷你 Telnet 服务器(std 线程,NVT 回显)。
//!
//! 行为:连接后发送横幅与一段选项协商(WILL,客户端应回 DONT),
//! 之后把收到的每一行回显为 `> {line}\r\n`。用于 linop-telnet
//! 与 app 层的端到端测试。

use std::collections::HashMap;
use std::io::{BufReader, Read, Write};
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};

const IAC: u8 = 255;
const DONT: u8 = 254;
const DO: u8 = 253;
const WILL: u8 = 251;
const WONT: u8 = 252;

/// 运行中的测试服务器(生命周期与测试进程一致)。
pub struct TestTelnetServer {
    pub addr: SocketAddr,
    /// 各连接已回显的行(断言用)。
    pub echoed: Arc<Mutex<HashMap<SocketAddr, Vec<String>>>>,
}

impl TestTelnetServer {
    pub fn start() -> std::io::Result<Self> {
        let listener = std::net::TcpListener::bind(("127.0.0.1", 0))?;
        let addr = listener.local_addr()?;
        let echoed = Arc::new(Mutex::new(HashMap::new()));
        let shared = echoed.clone();
        std::thread::spawn(move || {
            for stream in listener.incoming() {
                let Ok(stream) = stream else { break };
                let peer = stream.peer_addr().unwrap_or(addr);
                let echoed = shared.clone();
                std::thread::spawn(move || {
                    let _ = serve_connection(stream, peer, echoed);
                });
            }
        });
        Ok(Self { addr, echoed })
    }
}

fn serve_connection(
    stream: std::net::TcpStream,
    peer: SocketAddr,
    echoed: Arc<Mutex<HashMap<SocketAddr, Vec<String>>>>,
) -> std::io::Result<()> {
    stream.set_nodelay(true)?;
    let clone = stream.try_clone()?;
    let mut reader = BufReader::new(stream);
    let mut writer = clone;

    // 横幅 + 选项协商(WILL 终端类型):客户端必须回 DONT 并继续数据流。
    writer.write_all(b"Welcome to linop-telnet-test\r\n")?;
    writer.write_all(&[IAC, WILL, 24])?;
    writer.flush()?;

    // 字节级读行:客户端协商应答(0xFF…)不是合法 UTF-8,不能用 read_line。
    let mut line = Vec::new();
    let mut buf = [0u8; 1];
    loop {
        line.clear();
        loop {
            match reader.read(&mut buf)? {
                0 => return Ok(()), // 对端关闭
                _ => {
                    line.push(buf[0]);
                    if buf[0] == b'\n' {
                        break;
                    }
                }
            }
        }
        let mut bytes = strip_iac(&line);
        while bytes.last() == Some(&b'\n') || bytes.last() == Some(&b'\r') {
            bytes.pop();
        }
        echoed
            .lock()
            .unwrap()
            .entry(peer)
            .or_default()
            .push(String::from_utf8_lossy(&bytes).into_owned());
        // RFC 854:发送方必须转义 0xFF(IAC IAC)。
        let mut reply = b"> ".to_vec();
        for &byte in &bytes {
            if byte == IAC {
                reply.extend_from_slice(&[IAC, IAC]);
            } else {
                reply.push(byte);
            }
        }
        reply.extend_from_slice(b"\r\n");
        writer.write_all(&reply)?;
        writer.flush()?;
        if bytes == b"quit" {
            return Ok(());
        }
    }
}

/// 剥离 IAC:IAC IAC = 字面 0xFF;IAC + 协商命令(WILL/WONT/DO/DONT)
/// 再吃一个选项字节;其余 IAC + 命令吃两字节。
fn strip_iac(data: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(data.len());
    let mut i = 0;
    while i < data.len() {
        if data[i] != IAC {
            out.push(data[i]);
            i += 1;
            continue;
        }
        let Some(&next) = data.get(i + 1) else { break };
        match next {
            IAC => {
                out.push(IAC);
                i += 2;
            }
            WILL | WONT | DONT | DO => i += 3,
            _ => i += 2,
        }
    }
    out
}
