//! 测试用 SSH 服务端:russh server + run_stream。

use std::collections::{HashMap, HashSet};
use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use russh::keys::{PrivateKey, PublicKey};
use russh::server::{Auth, ChannelOpenHandle, Msg, Session};
use russh::{Channel, ChannelId};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};

use crate::sftp::FsSftpHandler;

/// 监控探针的固定输出:CPU 差分 ≈ 7.95%、内存 50%、磁盘 40%、负载 0.10/0.20/0.30。
pub const PROBE_FIXTURE: &str = concat!(
    "0.10 0.20 0.30 1/500 12345\n",
    "MemTotal:       16384000 kB\n",
    "MemAvailable:    8192000 kB\n",
    "cpu  100 20 30 850 10 0 0 0 0 0\n",
    "cpu  150 30 40 1650 20 0 0 0 0 0\n",
    "/dev/sda1 49996160 19998464 27465728 43% /\n",
);

/// 每连接的 SSH 会话处理器。
struct TestSession {
    /// 合法用户名/密码;`None` 表示接受公钥认证的任意已验证密钥。
    credentials: Option<(String, String)>,
    /// 已授权的公钥指纹;`None` 表示接受任意公钥。
    allowed_fingerprints: Option<Vec<String>>,
    /// channel_open_session 保存的通道,供 SFTP 子系统取用。
    channels: Arc<Mutex<HashMap<ChannelId, Channel<Msg>>>>,
    /// 已交给 SFTP 子系统的通道。russh 把通道数据同时投递给
    /// ChannelStream 和 `data` 回调,这些通道的数据属于 SFTP 协议,
    /// 回调里绝不能再回显,否则协议包会被弹回客户端导致会话崩溃。
    sftp_channels: Arc<Mutex<HashSet<ChannelId>>>,
    /// direct-tcpip 桥接通道:这些通道的数据由桥接任务转发,
    /// 回显逻辑必须跳过(否则隧道数据被原样弹回,回显与桥接竞速)。
    forward_channels: Arc<Mutex<HashSet<ChannelId>>>,
    /// SFTP 文件系统根目录;设置后启用 SFTP 子系统。
    sftp_root: Option<PathBuf>,
    /// 反向转发探针出口:客户端请求 tcpip_forward 时,服务端模拟一条
    /// 到达连接(写 "rf-probe"),把经客户端桥接回显的结果发到这里。
    forward_probe: Option<Arc<tokio::sync::watch::Sender<Vec<u8>>>>,
    /// 是否把会话通道收到的数据回显回去(shell echo 测试)。
    echo_session_data: bool,
}

impl russh::server::Handler for TestSession {
    type Error = russh::Error;

    async fn auth_password(&mut self, user: &str, password: &str) -> Result<Auth, Self::Error> {
        let ok = match &self.credentials {
            Some((u, p)) => u == user && p == password,
            None => true,
        };
        Ok(if ok { Auth::Accept } else { Auth::reject() })
    }

    async fn auth_publickey(
        &mut self,
        user: &str,
        key: russh::server::VerifiedPublicKey<'_>,
    ) -> Result<Auth, Self::Error> {
        let _ = user;
        match &self.allowed_fingerprints {
            None => Ok(Auth::Accept),
            Some(list) => {
                let fp = key.public_key.fingerprint(Default::default()).to_string();
                Ok(if list.contains(&fp) {
                    Auth::Accept
                } else {
                    Auth::reject()
                })
            }
        }
    }

    async fn channel_open_session(
        &mut self,
        channel: Channel<Msg>,
        reply: ChannelOpenHandle,
        _session: &mut Session,
    ) -> Result<(), Self::Error> {
        self.channels.lock().unwrap().insert(channel.id(), channel);
        reply.accept().await;
        Ok(())
    }

    async fn pty_request(
        &mut self,
        channel: ChannelId,
        _term: &str,
        _col_width: u32,
        _row_height: u32,
        _pix_width: u32,
        _pix_height: u32,
        _modes: &[(russh::Pty, u32)],
        session: &mut Session,
    ) -> Result<(), Self::Error> {
        session.channel_success(channel)?;
        Ok(())
    }

    async fn shell_request(
        &mut self,
        channel: ChannelId,
        session: &mut Session,
    ) -> Result<(), Self::Error> {
        session.channel_success(channel)?;
        Ok(())
    }

    async fn data(
        &mut self,
        channel: ChannelId,
        data: &[u8],
        session: &mut Session,
    ) -> Result<(), Self::Error> {
        let bridged = {
            let sftp = self.sftp_channels.lock().unwrap().contains(&channel);
            let forwarded = self.forward_channels.lock().unwrap().contains(&channel);
            sftp || forwarded
        };
        if self.echo_session_data && !bridged {
            session.data(channel, data.to_vec())?;
        }
        Ok(())
    }

    /// exec:监控探针命令返回固定 /proc 形态输出(与 linop-monitor
    /// 的 PROBE_COMMAND 对应),其余命令回显"已执行"。不真正执行命令。
    async fn exec_request(
        &mut self,
        channel: ChannelId,
        command: &[u8],
        session: &mut Session,
    ) -> Result<(), Self::Error> {
        session.channel_success(channel)?;
        self.channels.lock().unwrap().remove(&channel);
        let command = String::from_utf8_lossy(command);
        let command = command.as_ref();
        let output = if command.contains("/proc/loadavg") {
            PROBE_FIXTURE.as_bytes().to_vec()
        } else {
            format!("ran: {command}\n").into_bytes()
        };
        session.data(channel, output)?;
        session.eof(channel)?;
        session.close(channel)?;
        Ok(())
    }

    async fn subsystem_request(
        &mut self,
        channel_id: ChannelId,
        name: &str,
        session: &mut Session,
    ) -> Result<(), Self::Error> {
        if name == "sftp" && self.sftp_root.is_some() {
            let channel = self.channels.lock().unwrap().remove(&channel_id).unwrap();
            session.channel_success(channel_id)?;
            self.sftp_channels.lock().unwrap().insert(channel_id);
            let handler = FsSftpHandler::new(self.sftp_root.clone().unwrap());
            // 独立 task 驱动 SFTP 协议循环,避免阻塞 SSH 连接事件循环。
            tokio::spawn(russh_sftp::server::run(channel.into_stream(), handler));
        } else {
            session.channel_failure(channel_id)?;
        }
        Ok(())
    }

    async fn tcpip_forward(
        &mut self,
        address: &str,
        port: &mut u32,
        session: &mut Session,
    ) -> Result<bool, Self::Error> {
        let Some(probe_tx) = self.forward_probe.clone() else {
            // 未启用探针:直接允许转发请求(测试服务器宽松策略)。
            return Ok(true);
        };
        // 允许转发,并在独立任务里模拟"有人连上了远端端口":向客户端发起
        // forwarded-tcpip 通道,写探针并等待客户端桥到本地目标后的回显。
        // 不能在回调内等待通道打开——事件循环正被本回调占用,会死锁。
        let handle = session.handle();
        let addr = address.to_string();
        let port = *port;
        tokio::spawn(async move {
            let channel = match handle
                .channel_open_forwarded_tcpip(&addr, port, "probe", 0)
                .await
            {
                Ok(c) => c,
                Err(_) => return,
            };
            let mut stream = channel.into_stream();
            let _ = stream.write_all(b"rf-probe").await;
            let mut buf = vec![0u8; 64];
            let echoed =
                match tokio::time::timeout(Duration::from_secs(3), stream.read(&mut buf)).await {
                    Ok(Ok(n)) if n > 0 => {
                        buf.truncate(n);
                        buf
                    }
                    _ => Vec::new(),
                };
            let _ = probe_tx.send(echoed);
        });
        Ok(true)
    }

    async fn channel_open_direct_tcpip(
        &mut self,
        channel: Channel<Msg>,
        host_to_connect: &str,
        port_to_connect: u32,
        _originator_address: &str,
        _originator_port: u32,
        reply: ChannelOpenHandle,
        _session: &mut Session,
    ) -> Result<(), Self::Error> {
        self.forward_channels.lock().unwrap().insert(channel.id());
        reply.accept().await;
        let target = format!("{host_to_connect}:{port_to_connect}");
        tokio::spawn(async move {
            let mut upstream = match TcpStream::connect(&target).await {
                Ok(s) => s,
                Err(_) => {
                    let mut stream = channel.into_stream();
                    let _ = stream.shutdown().await;
                    return;
                }
            };
            let mut stream = channel.into_stream();
            let _ = tokio::io::copy_bidirectional(&mut upstream, &mut stream).await;
        });
        Ok(())
    }
}

/// 进程内测试 SSH 服务器。
pub struct TestSshServer {
    pub addr: SocketAddr,
    /// 服务端私钥,可用于指纹校验测试。
    pub host_key: PrivateKey,
    pub host_fingerprint: String,
    shutdown: Arc<tokio::sync::Notify>,
}

/// 服务器行为配置。
#[derive(Default)]
pub struct ServerOptions {
    pub username: Option<String>,
    pub password: Option<String>,
    /// 设置后启用 SFTP 子系统,文件落到该目录。
    pub sftp_root: Option<PathBuf>,
    /// 仅接受这些指纹的公钥;`None` 接受任意公钥。
    pub allowed_fingerprints: Option<Vec<String>>,
    pub echo_session_data: bool,
    /// 启用后:客户端的 tcpip_forward 请求被接受,并立即发起探针连接
    /// (见 [`TestSshServer::start_with_forward_probe`])。
    pub forward_probe: Option<Arc<tokio::sync::watch::Sender<Vec<u8>>>>,
}

impl TestSshServer {
    pub async fn start(opts: ServerOptions) -> anyhow::Result<Self> {
        let listener = TcpListener::bind(("127.0.0.1", 0)).await?;
        let addr = listener.local_addr()?;
        // 固定种子生成 Ed25519 宿主密钥:测试服务器无需密码学级随机性,
        // 且绕开 rand_core 版本分裂问题。
        let host_key = PrivateKey::from(russh::keys::ssh_key::private::Ed25519Keypair::from_seed(
            &[7u8; 32],
        ));
        let host_fingerprint = host_key
            .public_key()
            .fingerprint(Default::default())
            .to_string();

        let config = Arc::new(russh::server::Config {
            auth_rejection_time: Duration::from_millis(50),
            auth_rejection_time_initial: Some(Duration::from_millis(50)),
            keys: vec![host_key.clone()],
            ..Default::default()
        });

        let shutdown = Arc::new(tokio::sync::Notify::new());
        let shutdown_task = shutdown.clone();

        tokio::spawn(async move {
            let channels = Arc::new(Mutex::new(HashMap::new()));
            let sftp_channels = Arc::new(Mutex::new(HashSet::new()));
            let forward_channels = Arc::new(Mutex::new(HashSet::new()));
            loop {
                let accept = tokio::select! {
                    _ = shutdown_task.notified() => break,
                    r = listener.accept() => r,
                };
                let (stream, _) = match accept {
                    Ok(s) => s,
                    Err(_) => break,
                };
                let handler = TestSession {
                    credentials: opts
                        .username
                        .clone()
                        .map(|u| (u, opts.password.clone().unwrap_or_default())),
                    allowed_fingerprints: opts.allowed_fingerprints.clone(),
                    channels: channels.clone(),
                    sftp_channels: sftp_channels.clone(),
                    forward_channels: forward_channels.clone(),
                    sftp_root: opts.sftp_root.clone(),
                    echo_session_data: opts.echo_session_data,
                    forward_probe: opts.forward_probe.clone(),
                };
                if let Ok(running) =
                    russh::server::run_stream(config.clone(), stream, handler).await
                {
                    // 后台驱动该连接,直到客户端断开。
                    tokio::spawn(async move {
                        let _ = running.await;
                    });
                }
            }
        });

        Ok(Self {
            addr,
            host_key,
            host_fingerprint,
            shutdown,
        })
    }

    /// 最简服务器:任意密码 + shell 回显。
    pub async fn start_echo() -> anyhow::Result<Self> {
        Self::start(ServerOptions {
            echo_session_data: true,
            ..Default::default()
        })
        .await
    }

    /// 带固定账密的服务器。
    pub async fn start_with_password(user: &str, password: &str) -> anyhow::Result<Self> {
        Self::start(ServerOptions {
            username: Some(user.to_string()),
            password: Some(password.to_string()),
            echo_session_data: true,
            ..Default::default()
        })
        .await
    }

    /// 带真实文件系统 SFTP 子系统的服务器。
    pub async fn start_with_sftp(root: impl Into<PathBuf>) -> anyhow::Result<Self> {
        Self::start(ServerOptions {
            sftp_root: Some(root.into()),
            echo_session_data: true,
            ..Default::default()
        })
        .await
    }

    /// 带反向转发探针的服务器:客户端请求反向转发时,服务端模拟一条到达
    /// 连接并尝试回显;结果通过 watch 读取(空 Vec 表示探针失败)。
    /// `sftp_root` 可选:同时启用 SFTP 子系统。
    pub async fn start_with_forward_probe(
        sftp_root: Option<PathBuf>,
    ) -> anyhow::Result<(Self, tokio::sync::watch::Receiver<Vec<u8>>)> {
        let (tx, rx) = tokio::sync::watch::channel(Vec::new());
        let server = Self::start(ServerOptions {
            forward_probe: Some(Arc::new(tx)),
            sftp_root,
            ..Default::default()
        })
        .await?;
        Ok((server, rx))
    }

    pub fn public_key(&self) -> PublicKey {
        self.host_key.public_key().clone()
    }

    pub fn shutdown(&self) {
        self.shutdown.notify_waiters();
    }
}
