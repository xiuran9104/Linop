//! linop-redis:面向键浏览的 RESP2 Redis 客户端。
//!
//! 对应 navop 的 `redis-runtime` / `redis_view`:SCAN 浏览键、
//! 按类型取值(string/list/hash/set/zset)、TTL、删除。
//! 同步 API(std TcpStream),上层用 spawn_blocking 包裹。
//! 协议实现遵循 RESP2(Redis 2.x+ 默认),不依赖外部 crate。

use std::fmt::Write as _;
use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpStream;

/// 浏览单次 SCAN 聚合的键数量上限(防大库一次拉爆)。
pub const SCAN_BATCH: usize = 200;

#[derive(Debug, thiserror::Error)]
pub enum RedisError {
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("redis error reply: {0}")]
    Server(String),
    #[error("protocol error: {0}")]
    Protocol(String),
    #[error("unexpected reply: expect {expect}, got {got}")]
    Unexpected { expect: &'static str, got: String },
}

pub type Result<T> = std::result::Result<T, RedisError>;

/// 一个 RESP 应答(客户端视角)。
#[derive(Debug, Clone, PartialEq)]
pub enum Reply {
    Simple(String),
    Error(String),
    Int(i64),
    Bulk(Option<Vec<u8>>),
    Array(Option<Vec<Reply>>),
}

impl Reply {
    /// 期望简单字符串(如 PING → +PONG)。
    pub fn into_simple(self) -> Result<String> {
        match self {
            Reply::Simple(s) => Ok(s),
            other => Err(unexpected("simple string", &other)),
        }
    }

    /// 期望整数。
    pub fn into_int(self) -> Result<i64> {
        match self {
            Reply::Int(i) => Ok(i),
            other => Err(unexpected("integer", &other)),
        }
    }

    /// 期望 bulk 字符串(nil 归一为 None)。
    pub fn into_bulk(self) -> Result<Option<String>> {
        match self {
            Reply::Bulk(Some(bytes)) => Ok(Some(String::from_utf8_lossy(&bytes).into_owned())),
            Reply::Bulk(None) => Ok(None),
            other => Err(unexpected("bulk string", &other)),
        }
    }

    /// 期望数组(nil 归一为空)。
    pub fn into_array(self) -> Result<Vec<Reply>> {
        match self {
            Reply::Array(Some(items)) => Ok(items),
            Reply::Array(None) => Ok(Vec::new()),
            other => Err(unexpected("array", &other)),
        }
    }

    /// 期望 OK(简单字符串 "OK")。
    pub fn into_ok(self) -> Result<()> {
        let text = self.into_simple()?;
        if text.eq_ignore_ascii_case("OK") {
            Ok(())
        } else {
            Err(unexpected("OK", &Reply::Simple(text)))
        }
    }
}

fn unexpected(expect: &'static str, got: &Reply) -> RedisError {
    RedisError::Unexpected {
        expect,
        got: format!("{got:?}"),
    }
}

/// 键的概要信息(浏览列表用)。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KeyInfo {
    pub name: String,
    /// string/list/hash/set/zset/stream 等。
    pub kind: String,
    /// 长度语义按类型:字符数/元素个数;-1 表示未知。
    pub len: i64,
    /// 剩余 TTL 秒;-1 永久,-2 无键。
    pub ttl_secs: i64,
}

/// 键的完整值(按类型取回)。
#[derive(Debug, Clone, PartialEq)]
pub enum KeyValue {
    Str(String),
    List(Vec<String>),
    Hash(Vec<(String, String)>),
    Set(Vec<String>),
    ZSet(Vec<(String, String)>),
    /// 未知类型的原始 bulk 展示(stream 等)。
    Other {
        kind: String,
        raw: String,
    },
}

/// 连接句柄;命令即发即收(无 pipeline)。
pub struct RedisClient {
    reader: BufReader<TcpStream>,
    writer: TcpStream,
}

impl RedisClient {
    /// 连接并完成可选认证 + PING 探活。
    pub fn connect(addr: &str, password: Option<&str>) -> Result<Self> {
        let stream = TcpStream::connect(addr)?;
        let mut client = Self {
            reader: BufReader::new(stream.try_clone()?),
            writer: stream,
        };
        if let Some(password) = password {
            client.cmd(&["AUTH", password])?.into_ok()?;
        }
        if client.cmd(&["PING"])?.into_simple()? != "PONG" {
            return Err(RedisError::Protocol("PING 未返回 PONG".into()));
        }
        Ok(client)
    }

    /// 发送命令并读取应答;服务器错误(-ERR)转 Err。
    pub fn cmd(&mut self, args: &[&str]) -> Result<Reply> {
        let mut buf = String::new();
        buf.push_str(&format!("*{}\r\n", args.len()));
        for arg in args {
            write!(buf, "${}\r\n{}\r\n", arg.len(), arg)
                .map_err(|e| RedisError::Io(std::io::Error::other(e)))?;
        }
        self.writer.write_all(buf.as_bytes())?;
        self.writer.flush()?;
        let reply = read_reply(&mut self.reader)?;
        if let Reply::Error(msg) = &reply {
            return Err(RedisError::Server(msg.clone()));
        }
        Ok(reply)
    }

    /// SCAN 全库(带 MATCH 无),返回按字典序排序的键;`limit` 截断防大库。
    pub fn scan_keys(&mut self, limit: usize) -> Result<Vec<String>> {
        let mut keys = Vec::new();
        let mut cursor: u64 = 0;
        loop {
            let reply = self.cmd(&[
                "SCAN",
                &cursor.to_string(),
                "COUNT",
                &SCAN_BATCH.to_string(),
            ])?;
            let pair = reply.into_array()?;
            if pair.len() != 2 {
                return Err(RedisError::Protocol(format!(
                    "SCAN 应答字段数异常:{pair:?}"
                )));
            }
            cursor = pair[0]
                .clone()
                .into_bulk()?
                .ok_or_else(|| RedisError::Protocol("SCAN cursor 为 nil".into()))?
                .parse::<u64>()
                .map_err(|_| RedisError::Protocol("SCAN cursor 非数字".into()))?;
            for item in pair[1].clone().into_array()? {
                if let Some(key) = item.into_bulk()? {
                    keys.push(key);
                }
            }
            if cursor == 0 || keys.len() >= limit {
                break;
            }
        }
        keys.sort();
        keys.truncate(limit);
        Ok(keys)
    }

    /// 取一个键的概要(TYPE + 长度 + TTL;TYPE 为 none 时返回 None)。
    pub fn key_info(&mut self, name: &str) -> Result<Option<KeyInfo>> {
        let kind = match self.cmd(&["TYPE", name])?.into_simple()? {
            k if k == "none" => return Ok(None),
            k => k,
        };
        let len = match kind.as_str() {
            "string" => self.cmd(&["STRLEN", name])?.into_int()?,
            "list" => self.cmd(&["LLEN", name])?.into_int()?,
            "hash" => self.cmd(&["HLEN", name])?.into_int()?,
            "set" => self.cmd(&["SCARD", name])?.into_int()?,
            "zset" => self.cmd(&["ZCARD", name])?.into_int()?,
            _ => -1,
        };
        let ttl_secs = self.cmd(&["TTL", name])?.into_int()?;
        Ok(Some(KeyInfo {
            name: name.to_string(),
            kind,
            len,
            ttl_secs,
        }))
    }

    /// 批量取概要(键可能在中途过期,过期的跳过)。
    pub fn key_infos(&mut self, names: &[String]) -> Result<Vec<KeyInfo>> {
        let mut infos = Vec::with_capacity(names.len());
        for name in names {
            if let Some(info) = self.key_info(name)? {
                infos.push(info);
            }
        }
        Ok(infos)
    }

    /// 按类型取键值。list/hash/set/zset 各取前 100 条(浏览足够)。
    pub fn key_value(&mut self, name: &str, kind: &str) -> Result<KeyValue> {
        let value = match kind {
            "string" => KeyValue::Str(self.cmd(&["GET", name])?.into_bulk()?.unwrap_or_default()),
            "list" => KeyValue::List(bulks_to_strings(
                self.cmd(&["LRANGE", name, "0", "99"])?.into_array()?,
            )?),
            "hash" => KeyValue::Hash(pairs(&self.cmd(&["HGETALL", name])?.into_array()?, "hash")?),
            "set" => {
                let mut members = bulks_to_strings(self.cmd(&["SMEMBERS", name])?.into_array()?)?;
                members.sort();
                KeyValue::Set(members)
            }
            "zset" => {
                let flat = bulks_to_strings(
                    self.cmd(&["ZRANGE", name, "0", "99", "WITHSCORES"])?
                        .into_array()?,
                )?;
                let mut pairs = Vec::new();
                let mut iter = flat.into_iter();
                while let (Some(member), Some(score)) = (iter.next(), iter.next()) {
                    pairs.push((member, score));
                }
                KeyValue::ZSet(pairs)
            }
            other => KeyValue::Other {
                kind: other.to_string(),
                raw: format!("(不支持浏览的键类型:{other})"),
            },
        };
        Ok(value)
    }

    /// 删除键,返回是否删除了(0 = 键不存在)。
    pub fn del(&mut self, name: &str) -> Result<bool> {
        Ok(self.cmd(&["DEL", name])?.into_int()? > 0)
    }

    /// 键总数(DBSIZE)。
    pub fn dbsize(&mut self) -> Result<i64> {
        self.cmd(&["DBSIZE"])?.into_int()
    }
}

fn bulks_to_strings(items: Vec<Reply>) -> Result<Vec<String>> {
    items
        .into_iter()
        .map(|item| Ok(item.into_bulk()?.unwrap_or_default()))
        .collect()
}

/// HGETALL 的扁平数组 → (field, value) 对。
fn pairs(items: &[Reply], what: &str) -> Result<Vec<(String, String)>> {
    let flat = bulks_to_strings(items.to_vec())?;
    if flat.len() % 2 != 0 {
        return Err(RedisError::Protocol(format!("{what} 应答不是偶数个字段")));
    }
    let (chunks, _) = flat.as_chunks::<2>();
    Ok(chunks
        .iter()
        .map(|[field, value]| (field.clone(), value.clone()))
        .collect())
}

/// 读取一个 RESP 应答。
fn read_reply(reader: &mut BufReader<TcpStream>) -> Result<Reply> {
    let mut line = String::new();
    let n = reader.read_line(&mut line)?;
    if n == 0 {
        return Err(RedisError::Protocol("连接被对端关闭".into()));
    }
    let line = line.trim_end_matches("\r\n");
    let (tag, rest) = line.split_at(1);
    match tag {
        "+" => Ok(Reply::Simple(rest.to_string())),
        "-" => Ok(Reply::Error(rest.to_string())),
        ":" => rest
            .parse::<i64>()
            .map(Reply::Int)
            .map_err(|_| RedisError::Protocol(format!("整数应答非法:{line:?}"))),
        "$" => {
            let len: i64 = rest
                .parse()
                .map_err(|_| RedisError::Protocol(format!("bulk 长度非法:{line:?}")))?;
            if len < 0 {
                return Ok(Reply::Bulk(None));
            }
            let mut buf = vec![0u8; len as usize + 2]; // 含 \r\n
            reader.read_exact(&mut buf)?;
            buf.truncate(len as usize);
            Ok(Reply::Bulk(Some(buf)))
        }
        "*" => {
            let len: i64 = rest
                .parse()
                .map_err(|_| RedisError::Protocol(format!("数组长度非法:{line:?}")))?;
            if len < 0 {
                return Ok(Reply::Array(None));
            }
            let mut items = Vec::with_capacity(len as usize);
            for _ in 0..len {
                items.push(read_reply(reader)?);
            }
            Ok(Reply::Array(Some(items)))
        }
        other => Err(RedisError::Protocol(format!("未知应答类型:{other:?}"))),
    }
}
