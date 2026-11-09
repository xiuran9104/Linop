//! 本地端口转发(对应 navop 的 `port_forwarding` crate):
//! 本地 listener 上的 TCP 连接经 SSH 隧道直达远端目标。

use std::net::SocketAddr;

use linop_core::PortForward;
use russh::client::Handle;
use tokio::io::AsyncWriteExt;

use crate::client::ClientHandler;
use crate::{Result, SshError};

/// 一条运行中的本地转发。
pub struct LocalForward {
    pub local_addr: SocketAddr,
    pub remote: PortForward,
    task: tokio::task::JoinHandle<()>,
}

impl LocalForward {
    /// 监听 `127.0.0.1:{local_port}`,把连接转发到远端 `remote_host:remote_port`。
    pub async fn start(
        ssh: std::sync::Arc<Handle<ClientHandler>>,
        fwd: &PortForward,
    ) -> Result<Self> {
        let listener = tokio::net::TcpListener::bind(("127.0.0.1", fwd.local_port))
            .await
            .map_err(SshError::Listen)?;
        let local_addr = listener.local_addr()?;
        let remote = fwd.clone();
        let remote_task = fwd.clone();

        let task = tokio::spawn(async move {
            loop {
                let Ok((mut tcp, _)) = listener.accept().await else {
                    break;
                };
                let channel = match ssh
                    .channel_open_direct_tcpip(
                        remote_task.remote_host.clone(),
                        remote_task.remote_port as u32,
                        "127.0.0.1",
                        0,
                    )
                    .await
                {
                    Ok(c) => c,
                    Err(_) => continue,
                };
                let mut stream = channel.into_stream();
                tokio::spawn(async move {
                    let _ = tokio::io::copy_bidirectional(&mut tcp, &mut stream).await;
                    let _ = tcp.shutdown().await;
                });
            }
        });

        Ok(Self {
            local_addr,
            remote,
            task,
        })
    }

    /// 停止转发(断开 listener;已建立的连接自行收尾)。
    pub fn stop(self) {
        self.task.abort();
    }
}

impl std::fmt::Display for LocalForward {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{} -> {}:{}",
            self.local_addr, self.remote.remote_host, self.remote.remote_port
        )
    }
}

/// 一条运行中的反向端口转发(-R):远端 bind 端口上的连接
/// 经隧道回到本机 target。drop 不取消,需显式 [`RemoteForward::stop`]。
pub struct RemoteForward {
    pub bind_host: String,
    pub bind_port: u16,
    pub target_host: String,
    pub target_port: u16,
    ssh: std::sync::Arc<Handle<crate::client::ClientHandler>>,
    targets: crate::client::RemoteTargets,
}

impl RemoteForward {
    /// 请求远端监听 `bind_host:bind_port`,并把到达连接映射到本地 target。
    /// `bind_port == 0` 时由远端分配,实际端口见返回值的 `bind_port`。
    pub async fn start(
        ssh: std::sync::Arc<Handle<crate::client::ClientHandler>>,
        targets: crate::client::RemoteTargets,
        rule: &linop_core::RemoteForward,
    ) -> crate::Result<Self> {
        let assigned = ssh
            .tcpip_forward(rule.bind_host.clone(), rule.bind_port as u32)
            .await?;
        let bind_port = if rule.bind_port == 0 {
            u16::try_from(assigned)
                .map_err(|_| SshError::Listen(std::io::Error::other("远端分配的端口非法")))?
        } else {
            rule.bind_port
        };
        targets
            .lock()
            .unwrap()
            .insert(bind_port, (rule.target_host.clone(), rule.target_port));
        Ok(Self {
            bind_host: rule.bind_host.clone(),
            bind_port,
            target_host: rule.target_host.clone(),
            target_port: rule.target_port,
            ssh,
            targets,
        })
    }

    /// 停止:注销映射并请求远端关闭监听。
    pub async fn stop(self) {
        self.targets.lock().unwrap().remove(&self.bind_port);
        let _ = self
            .ssh
            .cancel_tcpip_forward(&self.bind_host, self.bind_port as u32)
            .await;
    }
}

impl std::fmt::Display for RemoteForward {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}:{} → 本机 {}:{}",
            self.bind_host, self.bind_port, self.target_host, self.target_port
        )
    }
}
