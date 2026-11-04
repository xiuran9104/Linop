//! 进程内 SSH + SFTP 测试服务器。
//!
//! 在测试中直接 `TestSshServer::start()` 起一个真实的 russh 服务端
//! (密码/公钥认证、PTY shell 回显、direct-tcpip 转发、SFTP 子系统),
//! 无需系统 sshd。

mod redis;
mod sftp;
mod ssh;

pub use redis::{RedisData, TestRedisServer};
pub use sftp::FsSftpHandler;
pub use ssh::{ServerOptions, TestSshServer};

/// 读取 OpenSSH 公钥文件(测试用)。
pub fn load_public_key(
    path: &std::path::Path,
) -> Result<russh::keys::PublicKey, russh::keys::Error> {
    russh::keys::load_public_key(path)
}
