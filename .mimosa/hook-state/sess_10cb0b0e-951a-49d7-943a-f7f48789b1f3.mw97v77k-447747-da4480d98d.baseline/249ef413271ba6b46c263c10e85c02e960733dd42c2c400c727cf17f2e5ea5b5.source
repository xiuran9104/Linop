//! linop-telnet:面向终端工作的最小 Telnet(RFC 854)客户端。
//!
//! 对应 navop 的 Telnet 支持:"哑终端"策略 —— 对服务端的一切选项
//! 协商回 WONT/DONT,数据字节原样透传给上层终端仿真。
//! 同步 API(std TcpStream + 读超时),上层用阻塞任务轮询。

use std::io::{Read, Write};
use std::net::{TcpStream, ToSocketAddrs};
use std::time::Duration;

/// 单次 poll 的读超时:到期返回 `TelnetEvent::Idle`,调用方决定继续或停止。
pub const POLL_TIMEOUT: Duration = Duration::from_millis(250);

/// 一次 poll 的读取块大小。
const READ_CHUNK: usize = 4096;

#[derive(Debug, thiserror::Error)]
pub enum TelnetError {
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("connect failed: {0}")]
    Connect(String),
}

pub type Result<T> = std::result::Result<T, TelnetError>;

/// 一次 poll 的结果。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TelnetEvent {
    /// 解码后的数据字节(协商序列已剥离并自动应答)。
    Data(Vec<u8>),
    /// 读超时,连接仍存活。
    Idle,
}

/// IAC 命令字节。
const IAC: u8 = 255;
const DONT: u8 = 254;
const DO: u8 = 253;
const WILL: u8 = 251;
const WONT: u8 = 252;
const SB: u8 = 250;
const SE: u8 = 240;

/// 跨 poll 携带的解析状态。
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
enum IacState {
    /// 普通数据。
    #[default]
    Data,
    /// 刚收到 IAC,等命令字节。
    GotIac,
    /// 协商命令(DO/DONT/WILL/WONT),等选项字节。
    Negotiate(u8),
    /// 子协商 SB 中(吃 到 IAC SE 为止;IAC IAC 表示字面 0xFF)。
    Subneg { seen_iac: bool },
}

/// 把一段原始字节解码为数据字节并生成协商应答。
/// 纯函数,便于跨块边界单元测试。
fn decode(state: &mut IacState, input: &[u8], out: &mut Vec<u8>, replies: &mut Vec<u8>) {
    for &byte in input {
        match *state {
            IacState::Data => {
                if byte == IAC {
                    *state = IacState::GotIac;
                } else {
                    out.push(byte);
                }
            }
            IacState::GotIac => match byte {
                IAC => {
                    // 字面 0xFF。
                    out.push(IAC);
                    *state = IacState::Data;
                }
                DO | DONT | WILL | WONT => *state = IacState::Negotiate(byte),
                SB => *state = IacState::Subneg { seen_iac: false },
                // 其余单字节命令(NOP/GO AHEAD/…)直接忽略。
                _ => *state = IacState::Data,
            },
            IacState::Negotiate(cmd) => {
                // 哑终端:拒绝一切选项;仍按 RFC 回应,保持会话推进。
                let reply = match cmd {
                    DO => Some(WONT),
                    WILL => Some(DONT),
                    // WONT/DONT 是对端的让步,无需回应。
                    _ => None,
                };
                if let Some(reply) = reply {
                    replies.extend_from_slice(&[IAC, reply, byte]);
                }
                *state = IacState::Data;
            }
            IacState::Subneg { seen_iac } => match (seen_iac, byte) {
                (true, SE) => *state = IacState::Data,
                (true, IAC) => {
                    // SB 内的 IAC IAC = 字面 0xFF(丢弃,不透传)。
                    *state = IacState::Subneg { seen_iac: false };
                }
                (true, _) => *state = IacState::Subneg { seen_iac: false },
                (false, IAC) => *state = IacState::Subneg { seen_iac: true },
                (false, _) => {}
            },
        }
    }
}

/// 对写入字节做 IAC 转义(0xFF → 0xFF 0xFF)。
pub fn escape_for_wire(data: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(data.len() + 8);
    for &byte in data {
        if byte == IAC {
            out.extend_from_slice(&[IAC, IAC]);
        } else {
            out.push(byte);
        }
    }
    out
}

/// 一个 Telnet 连接(读写两个独立句柄)。
/// `poll` 读超时返回 Idle;其余错误视为连接死亡。
/// 读写分离:读侧长轮询不会饿死写侧(此前共用一把锁导致写饿死超时)。
pub struct TelnetClient {
    reader: TelnetReader,
    writer: TelnetWriter,
}

/// 读半边:poll 数据 + 自动协商应答。
pub struct TelnetReader {
    sock: TcpStream,
    /// 协商应答走独立写句柄,不与数据写竞争。
    reply_sock: TcpStream,
    state: IacState,
}

/// 写半边:IAC 转义后的数据写入。
pub struct TelnetWriter {
    sock: TcpStream,
}

impl TelnetClient {
    /// 连接 `host:port` 并开启读超时轮询。
    pub fn connect(host: &str, port: u16) -> Result<Self> {
        let addr = (host, port)
            .to_socket_addrs()
            .map_err(|e| TelnetError::Connect(e.to_string()))?
            .next()
            .ok_or_else(|| TelnetError::Connect(format!("无法解析 {host}")))?;
        let sock = TcpStream::connect(addr)?;
        sock.set_read_timeout(Some(POLL_TIMEOUT))?;
        sock.set_nodelay(true)?;
        let reply_sock = sock.try_clone()?;
        let writer_sock = sock.try_clone()?;
        Ok(Self {
            reader: TelnetReader {
                sock,
                reply_sock,
                state: IacState::default(),
            },
            writer: TelnetWriter { sock: writer_sock },
        })
    }

    /// 拆出读/写半边(读线程与写线程各持其一,互不阻塞)。
    pub fn into_parts(self) -> (TelnetReader, TelnetWriter) {
        (self.reader, self.writer)
    }

    /// 便捷:单连接场景直接 poll(见 `TelnetReader::poll`)。
    pub fn poll(&mut self) -> Result<TelnetEvent> {
        self.reader.poll()
    }

    /// 便捷:单连接场景直接写(见 `TelnetWriter::write`)。
    pub fn write(&mut self, data: &[u8]) -> Result<()> {
        self.writer.write(data)
    }
}

impl TelnetReader {
    /// 读一批数据(含协商处理);超时返回 Idle。
    /// 协商应答(WONT/DONT)在内部直接写回。
    pub fn poll(&mut self) -> Result<TelnetEvent> {
        let mut raw = vec![0u8; READ_CHUNK];
        match self.sock.read(&mut raw) {
            Ok(0) => {
                Err(std::io::Error::new(std::io::ErrorKind::UnexpectedEof, "连接被对端关闭").into())
            }
            Ok(n) => {
                let mut out = Vec::new();
                let mut replies = Vec::new();
                decode(&mut self.state, &raw[..n], &mut out, &mut replies);
                if !replies.is_empty() {
                    self.reply_sock.write_all(&replies)?;
                    self.reply_sock.flush()?;
                }
                Ok(TelnetEvent::Data(out))
            }
            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => Ok(TelnetEvent::Idle),
            Err(e) => Err(e.into()),
        }
    }
}

impl TelnetWriter {
    /// 写入数据(自动 IAC 转义)。
    pub fn write(&mut self, data: &[u8]) -> Result<()> {
        self.sock.write_all(&escape_for_wire(data))?;
        self.sock.flush()?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn decode_all(chunks: &[&[u8]]) -> (Vec<u8>, Vec<u8>) {
        let mut state = IacState::default();
        let (mut out, mut replies) = (Vec::new(), Vec::new());
        for chunk in chunks {
            decode(&mut state, chunk, &mut out, &mut replies);
        }
        (out, replies)
    }

    #[test]
    fn plain_data_passes_through() {
        let (out, replies) = decode_all(&[b"hello\r\n"]);
        assert_eq!(out, b"hello\r\n");
        assert!(replies.is_empty());
    }

    #[test]
    fn negotiation_is_stripped_and_refused() {
        // DO ECHO(1) → WONT;WILL SGA(3) → DONT。
        let (out, replies) = decode_all(&[&[b'a', IAC, DO, 1, b'b', IAC, WILL, 3, b'c']]);
        assert_eq!(out, b"abc");
        assert_eq!(replies, [IAC, WONT, 1, IAC, DONT, 3]);
    }

    #[test]
    fn literal_iac_is_doubled_on_decode() {
        let (out, _) = decode_all(&[&[IAC, IAC, b'x']]);
        assert_eq!(out, [IAC, b'x']);
    }

    #[test]
    fn subnegotiation_is_consumed_across_chunks() {
        let sb = [IAC, SB, 24, b'p', b'q', IAC, IAC, b'z', IAC, SE];
        let (out, replies) = decode_all(&[&sb[..4], &sb[4..]]);
        assert!(out.is_empty(), "SB 内容不透传:{out:?}");
        assert!(replies.is_empty());
    }

    #[test]
    fn subneg_then_data_continues() {
        let mut state = IacState::default();
        let (mut out, mut replies) = (Vec::new(), Vec::new());
        decode(
            &mut state,
            &[IAC, SB, 24, 1, 2, IAC, SE, b'o', b'k'],
            &mut out,
            &mut replies,
        );
        assert_eq!(out, b"ok");
    }

    #[test]
    fn partial_iac_across_chunk_boundary() {
        let (out, replies) = decode_all(&[&[b'a', IAC], &[DO, 1, b'b']]);
        assert_eq!(out, b"ab");
        assert_eq!(replies, [IAC, WONT, 1]);
    }

    #[test]
    fn escape_writes_doubles_ff() {
        assert_eq!(escape_for_wire(&[b'a', IAC, b'b']), [b'a', IAC, IAC, b'b']);
    }

    #[test]
    fn unknown_iac_command_is_skipped() {
        // IAC + 未知命令(如 NOP=241)只吃命令字节,不影响后续数据。
        let (out, _) = decode_all(&[&[IAC, 241, b'x']]);
        assert_eq!(out, b"x");
    }
}
