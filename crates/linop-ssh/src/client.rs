//! 连接与认证。

use std::sync::{Arc, Mutex};
use std::time::Duration;

use std::collections::HashMap;

use linop_core::{Auth, Connection, PortForward};
use russh::client::Handle;
use russh::keys::PublicKeyOrCertificate;

use crate::forward::LocalForward;
use crate::host_keys::{Decision, KnownHostsPolicy};
use crate::shell::ShellChannel;
use crate::{RemoteForward, SshError, TERM_TYPE, load_key};

/// 远端反向通道到达的一条连接:由驱动任务桥接到本地目标。
pub(crate) struct ForwardedConnection {
    pub target_host: String,
    pub target_port: u16,
    pub channel: russh::Channel<russh::client::Msg>,
}

/// 远端 bind 端口 → 本地目标 的映射,反向转发启停时维护。
pub(crate) type RemoteTargets = Arc<Mutex<HashMap<u16, (String, u16)>>>;

/// russh 客户端回调:校验主机指纹 + 记录认证 banner + 接管反向通道。
pub struct ClientHandler {
    host: String,
    port: u16,
    policy: Arc<Mutex<KnownHostsPolicy>>,
    /// 最近一次连接收到的 banner(若有)。
    pub banner: Arc<Mutex<Option<String>>>,
    /// 反向转发注册表与到达连接出口。
    remote_targets: RemoteTargets,
    forward_tx: tokio::sync::mpsc::UnboundedSender<ForwardedConnection>,
}

impl russh::client::Handler for ClientHandler {
    type Error = russh::Error;

    async fn check_server_key(
        &mut self,
        server_public_key: &PublicKeyOrCertificate,
    ) -> Result<bool, Self::Error> {
        let fingerprint = server_public_key
            .public_key()
            .fingerprint(Default::default())
            .to_string();
        let mut policy = self.policy.lock().unwrap();
        let decision = match &mut *policy {
            KnownHostsPolicy::AcceptAll => Decision::Trusted,
            KnownHostsPolicy::TrustOnFirstUse(hosts) => {
                let decision = hosts.decide(&self.host, self.port, &fingerprint);
                if !matches!(decision, Decision::Mismatch { .. }) {
                    let _ = hosts.save();
                }
                decision
            }
        };
        Ok(match decision {
            Decision::Trusted | Decision::NewlyTrusted => true,
            Decision::Mismatch { .. } => false,
        })
    }

    async fn auth_banner(
        &mut self,
        banner: &str,
        _session: &mut russh::client::Session,
    ) -> Result<(), Self::Error> {
        *self.banner.lock().unwrap() = Some(banner.to_string());
        Ok(())
    }

    /// 远端有人连到反向转发端口:接受通道,交给驱动任务桥接到本地目标。
    async fn server_channel_open_forwarded_tcpip(
        &mut self,
        channel: russh::Channel<russh::client::Msg>,
        _connected_address: &str,
        connected_port: u32,
        _originator_address: &str,
        _originator_port: u32,
        reply: russh::client::ChannelOpenHandle,
        _session: &mut russh::client::Session,
    ) -> Result<(), Self::Error> {
        let target = self
            .remote_targets
            .lock()
            .unwrap()
            .get(&(connected_port as u16))
            .cloned();
        match target {
            Some((target_host, target_port)) => {
                reply.accept().await;
                let _ = self.forward_tx.send(ForwardedConnection {
                    target_host,
                    target_port,
                    channel,
                });
            }
            None => {
                // 未注册的端口:丢弃 reply 即拒绝。
            }
        }
        Ok(())
    }
}

/// 已认证的 SSH 会话。可克隆(共享底层连接句柄),克隆方可各自开通道。
#[derive(Clone)]
pub struct Session {
    handle: std::sync::Arc<Handle<ClientHandler>>,
    banner: Arc<Mutex<Option<String>>>,
    remote_targets: RemoteTargets,
}

impl Session {
    /// 建立连接并完成认证。
    pub async fn connect(
        host: &str,
        port: u16,
        username: &str,
        auth: &Auth,
        policy: KnownHostsPolicy,
    ) -> crate::Result<Self> {
        let config = Arc::new(russh::client::Config {
            keepalive_interval: Some(Duration::from_secs(30)),
            nodelay: true,
            ..Default::default()
        });
        let banner = Arc::new(Mutex::new(None));
        let (forward_tx, mut forward_rx) = tokio::sync::mpsc::unbounded_channel();
        let remote_targets: RemoteTargets = Arc::new(Mutex::new(HashMap::new()));
        let handler = ClientHandler {
            host: host.to_string(),
            port,
            policy: Arc::new(Mutex::new(policy)),
            banner: banner.clone(),
            remote_targets: remote_targets.clone(),
            forward_tx,
        };
        let mut handle = russh::client::connect(config, (host, port), handler).await?;

        let result = match auth {
            Auth::Password { password } => handle.authenticate_password(username, password).await?,
            Auth::Key { .. } => {
                let key = load_key(auth)?;
                handle.authenticate_publickey(username, key).await?
            }
        };
        if !result.success() {
            return Err(SshError::AuthFailed {
                user: username.to_string(),
                host: host.to_string(),
            });
        }
        // 反向转发驱动:到达的远端连接桥接到本地目标。
        tokio::spawn(async move {
            while let Some(conn) = forward_rx.recv().await {
                tokio::spawn(async move {
                    if let Ok(mut tcp) = tokio::net::TcpStream::connect((
                        conn.target_host.as_str(),
                        conn.target_port,
                    ))
                    .await
                    {
                        let mut stream = conn.channel.into_stream();
                        let _ = tokio::io::copy_bidirectional(&mut tcp, &mut stream).await;
                    }
                });
            }
        });
        Ok(Self {
            handle: std::sync::Arc::new(handle),
            banner,
            remote_targets,
        })
    }

    /// 从连接 profile 建立(方便上层直接传 Registry 里的配置)。
    pub async fn connect_profile(
        conn: &Connection,
        policy: KnownHostsPolicy,
    ) -> crate::Result<Self> {
        Self::connect(&conn.host, conn.port, &conn.username, &conn.auth, policy).await
    }

    /// 认证阶段服务器 banner(如 MOTD 提示)。
    pub fn banner(&self) -> Option<String> {
        self.banner.lock().unwrap().clone()
    }

    /// 打开一个交互式 shell(带 PTY)。
    pub async fn open_shell(&self, cols: u16, rows: u16) -> crate::Result<ShellChannel> {
        let channel = self.handle.channel_open_session().await?;
        channel
            .request_pty(true, TERM_TYPE, cols as u32, rows as u32, 0, 0, &[])
            .await?;
        channel.request_shell(true).await?;
        Ok(ShellChannel::new(channel))
    }

    /// 打开 SFTP 子系统通道流,交给 linop-sftp 使用。
    pub async fn open_sftp_stream(
        &self,
    ) -> crate::Result<russh::ChannelStream<russh::client::Msg>> {
        let channel = self.handle.channel_open_session().await?;
        channel.request_subsystem(true, "sftp").await?;
        Ok(channel.into_stream())
    }

    /// 启动一条本地端口转发。
    pub async fn open_local_forward(&self, fwd: &PortForward) -> crate::Result<LocalForward> {
        LocalForward::start(self.handle.clone(), fwd).await
    }

    /// 启动一条反向端口转发(-R)。
    pub async fn open_remote_forward(
        &self,
        rule: &linop_core::RemoteForward,
    ) -> crate::Result<RemoteForward> {
        RemoteForward::start(self.handle.clone(), self.remote_targets.clone(), rule).await
    }

    /// 断开连接。
    pub async fn disconnect(self) -> crate::Result<()> {
        self.handle
            .disconnect(russh::Disconnect::ByApplication, "bye", "en")
            .await?;
        Ok(())
    }
}
