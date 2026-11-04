//! 测试用迷你 Redis 服务器:内存数据 + RESP2 应答。
//!
//! 只实现 linop-redis 客户端用到的命令子集,数据在启动时预置。

use std::collections::{BTreeMap, BTreeSet};
use std::io::{BufRead, BufReader, Read, Write};
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};

/// 预置数据(五种类型 + 可选 AUTH 口令)。
#[derive(Default)]
pub struct RedisData {
    pub strings: BTreeMap<String, String>,
    pub lists: BTreeMap<String, Vec<String>>,
    pub hashes: BTreeMap<String, Vec<(String, String)>>,
    pub sets: BTreeMap<String, BTreeSet<String>>,
    pub zsets: BTreeMap<String, Vec<(f64, String)>>,
    /// 设置后要求 AUTH(密码不匹配拒绝)。
    pub password: Option<String>,
}

impl RedisData {
    pub fn with_string(mut self, key: &str, value: &str) -> Self {
        self.strings.insert(key.to_string(), value.to_string());
        self
    }

    pub fn with_list(mut self, key: &str, items: &[&str]) -> Self {
        self.lists.insert(
            key.to_string(),
            items.iter().map(|s| s.to_string()).collect(),
        );
        self
    }

    pub fn with_hash(mut self, key: &str, items: &[(&str, &str)]) -> Self {
        self.hashes.insert(
            key.to_string(),
            items
                .iter()
                .map(|(k, v)| (k.to_string(), v.to_string()))
                .collect(),
        );
        self
    }

    pub fn with_set(mut self, key: &str, items: &[&str]) -> Self {
        self.sets.insert(
            key.to_string(),
            items.iter().map(|s| s.to_string()).collect(),
        );
        self
    }

    pub fn with_zset(mut self, key: &str, items: &[(f64, &str)]) -> Self {
        self.zsets.insert(
            key.to_string(),
            items
                .iter()
                .map(|(score, member)| (*score, member.to_string()))
                .collect(),
        );
        self
    }

    /// 全部键(排序)。
    fn all_keys(&self) -> Vec<String> {
        let mut keys: Vec<String> = self.strings.keys().cloned().collect();
        keys.extend(self.lists.keys().cloned());
        keys.extend(self.hashes.keys().cloned());
        keys.extend(self.sets.keys().cloned());
        keys.extend(self.zsets.keys().cloned());
        keys.sort();
        keys
    }

    fn key_type(&self, key: &str) -> Option<&'static str> {
        if self.strings.contains_key(key) {
            Some("string")
        } else if self.lists.contains_key(key) {
            Some("list")
        } else if self.hashes.contains_key(key) {
            Some("hash")
        } else if self.sets.contains_key(key) {
            Some("set")
        } else if self.zsets.contains_key(key) {
            Some("zset")
        } else {
            None
        }
    }

    fn delete(&mut self, key: &str) -> usize {
        let mut deleted = 0;
        for map in [
            &mut self.strings as &mut dyn Erasable,
            &mut self.lists,
            &mut self.hashes,
            &mut self.sets,
            &mut self.zsets,
        ] {
            deleted += map.erase(key);
        }
        deleted
    }
}

/// 各容器统一的按键删除(避免为五种类型写五遍)。
trait Erasable {
    fn erase(&mut self, key: &str) -> usize;
}

impl Erasable for BTreeMap<String, String> {
    fn erase(&mut self, key: &str) -> usize {
        self.remove(key).is_some() as usize
    }
}

impl Erasable for BTreeMap<String, Vec<String>> {
    fn erase(&mut self, key: &str) -> usize {
        self.remove(key).is_some() as usize
    }
}

impl Erasable for BTreeMap<String, Vec<(String, String)>> {
    fn erase(&mut self, key: &str) -> usize {
        self.remove(key).is_some() as usize
    }
}

impl Erasable for BTreeMap<String, BTreeSet<String>> {
    fn erase(&mut self, key: &str) -> usize {
        self.remove(key).is_some() as usize
    }
}

impl Erasable for BTreeMap<String, Vec<(f64, String)>> {
    fn erase(&mut self, key: &str) -> usize {
        self.remove(key).is_some() as usize
    }
}

/// 运行中的测试服务器(生命周期与测试进程一致,无需显式关闭)。
pub struct TestRedisServer {
    pub addr: SocketAddr,
}

impl TestRedisServer {
    /// 绑定随机端口并启动服务线程。同步 API:客户端可以是同步的
    /// (这正是与 SSH 测试服务器的差别 —— 后者挂在 tokio 运行时上)。
    pub fn start(data: RedisData) -> std::io::Result<Self> {
        let listener = std::net::TcpListener::bind(("127.0.0.1", 0))?;
        let addr = listener.local_addr()?;
        let shared = Arc::new(Mutex::new(data));
        std::thread::spawn(move || {
            for stream in listener.incoming() {
                let Ok(stream) = stream else { break };
                let shared = shared.clone();
                std::thread::spawn(move || {
                    let _ = serve_connection(stream, shared);
                });
            }
        });
        Ok(Self { addr })
    }
}

fn serve_connection(
    stream: std::net::TcpStream,
    shared: Arc<Mutex<RedisData>>,
) -> std::io::Result<()> {
    stream.set_nodelay(true)?;
    let clone = stream.try_clone()?;
    let mut reader = BufReader::new(stream);
    let mut writer = clone;
    // 未配置口令的服务器免认证;配置了则等 AUTH。
    let mut authed = shared.lock().map(|data| data.password.is_none()).unwrap_or(true);

    loop {
        let Some(cmd) = read_command(&mut reader)? else {
            return Ok(()); // 对端关闭
        };
        let reply = dispatch(&cmd, &shared, &mut authed);
        // dispatch 约定:长度 1 = 标量应答;更长 = 数组应答(SCAN/LRANGE…)。
        let mut iter = reply.into_iter();
        let value = match (iter.next(), iter.next()) {
            (Some(only), None) => only,
            (first, second) => WireValue::Array(first.into_iter().chain(second).collect()),
        };
        write_reply(&mut writer, &value)?;
        writer.flush()?;
    }
}

/// 读一个 RESP 命令数组;连接关闭返回 None。
fn read_command(
    reader: &mut BufReader<std::net::TcpStream>,
) -> std::io::Result<Option<Vec<String>>> {
    let mut line = String::new();
    if reader.read_line(&mut line)? == 0 {
        return Ok(None);
    }
    let line = line.trim_end_matches("\r\n");
    let Some(rest) = line.strip_prefix('*') else {
        return Ok(None); // 不支持 inline 命令
    };
    let count: usize = rest.parse().map_err(|_| bad("命令数组长度非法"))?;
    let mut args = Vec::with_capacity(count);
    for _ in 0..count {
        let mut header = String::new();
        reader.read_line(&mut header)?;
        let header = header.trim_end_matches("\r\n");
        let Some(len) = header.strip_prefix('$') else {
            return Err(bad("bulk 头非法"));
        };
        let len: usize = len.parse().map_err(|_| bad("bulk 长度非法"))?;
        let mut buf = vec![0u8; len + 2];
        reader.read_exact(&mut buf)?;
        buf.truncate(len);
        args.push(String::from_utf8_lossy(&buf).into_owned());
    }
    Ok(Some(args))
}

fn bad(msg: &str) -> std::io::Error {
    std::io::Error::other(msg.to_string())
}

type Wire = Vec<WireValue>;

enum WireValue {
    Simple(String),
    Error(String),
    Int(i64),
    Bulk(String),
    BulkNil,
    Array(Wire),
}

impl WireValue {
    fn bulk(value: impl Into<String>) -> Self {
        WireValue::Bulk(value.into())
    }
}

fn write_reply(writer: &mut impl Write, value: &WireValue) -> std::io::Result<()> {
    match value {
        WireValue::Simple(text) => write!(writer, "+{text}\r\n"),
        WireValue::Error(msg) => write!(writer, "-ERR {msg}\r\n"),
        WireValue::Int(n) => write!(writer, ":{n}\r\n"),
        WireValue::Bulk(text) => write!(writer, "${}\r\n{text}\r\n", text.len()),
        WireValue::BulkNil => write!(writer, "$-1\r\n"),
        WireValue::Array(items) => {
            write!(writer, "*{}\r\n", items.len())?;
            for item in items {
                write_reply(writer, item)?;
            }
            Ok(())
        }
    }
}

fn err_reply(msg: &str) -> Wire {
    vec![WireValue::Error(msg.to_string())]
}

fn dispatch(cmd: &[String], shared: &Arc<Mutex<RedisData>>, authed: &mut bool) -> Wire {
    let name = cmd
        .first()
        .map(String::as_str)
        .unwrap_or_default()
        .to_ascii_uppercase();
    let args = &cmd[1..];

    match name.as_str() {
        "AUTH" => {
            let Ok(guard) = shared.lock() else {
                return err_reply("锁中毒");
            };
            let expected = guard.password.as_deref().unwrap_or_default();
            if args.first().map(String::as_str) == Some(expected) && !expected.is_empty()
                || expected.is_empty()
            {
                *authed = true;
                vec![WireValue::Simple("OK".into())]
            } else {
                err_reply("invalid password")
            }
        }
        "PING" if !*authed => err_reply("NOAUTH Authentication required"),
        "QUIT" => vec![WireValue::Simple("OK".into())],
        _ if !*authed => err_reply("NOAUTH Authentication required"),
        "PING" => vec![WireValue::Simple("PONG".into())],
        "DBSIZE" => {
            let guard = shared.lock().unwrap();
            vec![WireValue::Int(guard.all_keys().len() as i64)]
        }
        "SCAN" => {
            // SCAN cursor [MATCH ..] [COUNT n]:游标按全键序号的字符串实现。
            let Ok(mut cursor) = args
                .first()
                .map(String::as_str)
                .unwrap_or("0")
                .parse::<usize>()
            else {
                return err_reply("cursor 非法");
            };
            let mut count = 10usize;
            let mut iter = args.iter();
            while let Some(flag) = iter.next() {
                if flag.eq_ignore_ascii_case("COUNT")
                    && let Some(value) = iter.next()
                    && let Ok(parsed) = value.parse::<usize>()
                {
                    count = parsed.max(1);
                }
            }
            let guard = shared.lock().unwrap();
            let keys = guard.all_keys();
            let start = cursor.min(keys.len());
            let end = (start + count).min(keys.len());
            let batch: Vec<WireValue> = keys[start..end].iter().map(WireValue::bulk).collect();
            cursor = if end == keys.len() { 0 } else { end };
            vec![WireValue::bulk(cursor.to_string()), WireValue::Array(batch)]
        }
        "TYPE" => {
            let guard = shared.lock().unwrap();
            let key = args.first().map(String::as_str).unwrap_or_default();
            vec![WireValue::Simple(
                guard.key_type(key).unwrap_or("none").to_string(),
            )]
        }
        "STRLEN" => {
            let guard = shared.lock().unwrap();
            let len = args
                .first()
                .and_then(|k| guard.strings.get(k))
                .map(|v| v.len() as i64)
                .unwrap_or(0);
            vec![WireValue::Int(len)]
        }
        "LLEN" => {
            let guard = shared.lock().unwrap();
            let len = args
                .first()
                .and_then(|k| guard.lists.get(k))
                .map(|v| v.len() as i64)
                .unwrap_or(0);
            vec![WireValue::Int(len)]
        }
        "HLEN" => {
            let guard = shared.lock().unwrap();
            let len = args
                .first()
                .and_then(|k| guard.hashes.get(k))
                .map(|v| v.len() as i64)
                .unwrap_or(0);
            vec![WireValue::Int(len)]
        }
        "SCARD" => {
            let guard = shared.lock().unwrap();
            let len = args
                .first()
                .and_then(|k| guard.sets.get(k))
                .map(|v| v.len() as i64)
                .unwrap_or(0);
            vec![WireValue::Int(len)]
        }
        "ZCARD" => {
            let guard = shared.lock().unwrap();
            let len = args
                .first()
                .and_then(|k| guard.zsets.get(k))
                .map(|v| v.len() as i64)
                .unwrap_or(0);
            vec![WireValue::Int(len)]
        }
        "TTL" => vec![WireValue::Int(-1)],
        "GET" => {
            let guard = shared.lock().unwrap();
            match args.first().and_then(|k| guard.strings.get(k)) {
                Some(value) => vec![WireValue::bulk(value)],
                None => vec![WireValue::BulkNil],
            }
        }
        "LRANGE" => {
            let guard = shared.lock().unwrap();
            let items = args
                .first()
                .and_then(|k| guard.lists.get(k))
                .cloned()
                .unwrap_or_default();
            vec![WireValue::Array(
                items.into_iter().map(WireValue::bulk).collect(),
            )]
        }
        "HGETALL" => {
            let guard = shared.lock().unwrap();
            let items = args
                .first()
                .and_then(|k| guard.hashes.get(k))
                .cloned()
                .unwrap_or_default();
            let mut wire = Vec::new();
            for (field, value) in items {
                wire.push(WireValue::bulk(field));
                wire.push(WireValue::bulk(value));
            }
            vec![WireValue::Array(wire)]
        }
        "SMEMBERS" => {
            let guard = shared.lock().unwrap();
            let items = args
                .first()
                .and_then(|k| guard.sets.get(k))
                .cloned()
                .unwrap_or_default();
            vec![WireValue::Array(
                items.into_iter().map(WireValue::bulk).collect(),
            )]
        }
        "ZRANGE" => {
            let guard = shared.lock().unwrap();
            let items = args
                .first()
                .and_then(|k| guard.zsets.get(k))
                .cloned()
                .unwrap_or_default();
            let mut sorted = items;
            sorted.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));
            let with_scores = args.iter().any(|a| a.eq_ignore_ascii_case("WITHSCORES"));
            let mut wire = Vec::new();
            for (score, member) in sorted {
                wire.push(WireValue::bulk(member));
                if with_scores {
                    wire.push(WireValue::bulk(format!("{score}")));
                }
            }
            vec![WireValue::Array(wire)]
        }
        "DEL" => {
            let mut guard = shared.lock().unwrap();
            let deleted = args.first().map(|k| guard.delete(k)).unwrap_or(0);
            vec![WireValue::Int(deleted as i64)]
        }
        other => err_reply(&format!("unknown command '{other}'")),
    }
}
