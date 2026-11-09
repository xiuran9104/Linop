//! SOCKS5 动态转发(`ssh -D` 等价):本地监听端口作为 SOCKS5 代理,
//! CONNECT 请求的目标经 SSH `direct-tcpip` 通道连出。
//!
//! 只实现无鉴权 + CONNECT 命令(浏览器/常用客户端的路径),不支持
//! BIND 与 UDP ASSOCIATE。

use std::net::SocketAddr;

use russh::client::Handle;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};

use crate::client::ClientHandler;
use crate::{Result, SshError};

/// 一条运行中的 SOCKS5 动态转发。drop 不停止,需显式 [`DynamicForward::stop`]。
pub struct DynamicForward {
    pub local_addr: SocketAddr,
    task: tokio::task::JoinHandle<()>,
}

/// SOCKS5 应答:成功 / 一般失败。
const REPLY_OK: [u8; 10] = [5, 0, 0, 1, 0, 0, 0, 0, 0, 0];
const REPLY_FAIL: [u8; 10] = [5, 1, 0, 1, 0, 0, 0, 0, 0, 0];

impl DynamicForward {
    /// 监听 `127.0.0.1:{local_port}`(0 由内核分配)作为 SOCKS5 入口。
    pub async fn start(
        ssh: std::sync::Arc<Handle<ClientHandler>>,
        local_port: u16,
    ) -> Result<Self> {
        let listener = TcpListener::bind(("127.0.0.1", local_port))
            .await
            .map_err(SshError::Listen)?;
        let local_addr = listener.local_addr()?;
        let task = tokio::spawn(async move {
            loop {
                let Ok((mut sock, _)) = listener.accept().await else {
                    break;
                };
                let ssh = ssh.clone();
                tokio::spawn(async move {
                    let Some((host, port)) = socks_handshake(&mut sock).await else {
                        return;
                    };
                    let channel = ssh
                        .channel_open_direct_tcpip(host.clone(), port as u32, "127.0.0.1", 0)
                        .await;
                    match channel {
                        Ok(channel) => {
                            let _ = sock.write_all(&REPLY_OK).await;
                            let mut stream = channel.into_stream();
                            let _ = tokio::io::copy_bidirectional(&mut sock, &mut stream).await;
                        }
                        Err(_) => {
                            let _ = sock.write_all(&REPLY_FAIL).await;
                        }
                    }
                });
            }
        });
        Ok(Self { local_addr, task })
    }

    pub fn stop(self) {
        self.task.abort();
    }
}

impl std::fmt::Display for DynamicForward {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "socks5://{}", self.local_addr)
    }
}

/// SOCKS5 握手:方法协商(仅无鉴权)+ 解析 CONNECT 目标。
async fn socks_handshake(sock: &mut TcpStream) -> Option<(String, u16)> {
    // 方法协商:[VER, NMETHODS, METHODS..]
    let mut head = [0u8; 2];
    sock.read_exact(&mut head).await.ok()?;
    if head[0] != 5 {
        return None;
    }
    let mut methods = vec![0u8; head[1] as usize];
    sock.read_exact(&mut methods).await.ok()?;
    if !methods.contains(&0x00) {
        let _ = sock.write_all(&[5, 0xFF]).await;
        return None;
    }
    sock.write_all(&[5, 0x00]).await.ok()?;

    // 请求:[VER, CMD=CONNECT, RSV, ATYP, ADDR, PORT]
    let mut req = [0u8; 4];
    sock.read_exact(&mut req).await.ok()?;
    if req[0] != 5 || req[1] != 1 {
        return None; // 仅支持 CONNECT
    }
    let host = match req[3] {
        1 => {
            let mut octets = [0u8; 4];
            sock.read_exact(&mut octets).await.ok()?;
            std::net::Ipv4Addr::from(octets).to_string()
        }
        3 => {
            let mut len = [0u8; 1];
            sock.read_exact(&mut len).await.ok()?;
            let mut name = vec![0u8; len[0] as usize];
            sock.read_exact(&mut name).await.ok()?;
            String::from_utf8(name).ok()?
        }
        4 => {
            let mut octets = [0u8; 16];
            sock.read_exact(&mut octets).await.ok()?;
            std::net::Ipv6Addr::from(octets).to_string()
        }
        _ => return None,
    };
    let mut port = [0u8; 2];
    sock.read_exact(&mut port).await.ok()?;
    Some((host, u16::from_be_bytes(port)))
}
