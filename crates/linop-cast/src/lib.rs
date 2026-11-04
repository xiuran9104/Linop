//! linop-cast:会话录制 —— asciinema cast v2 格式的写入与解析。
//!
//! 对应 navop 的"会话录制回放":shell 输出按相对时间戳逐事件落盘,
//! 产生标准 `.cast` 文件,可用 `asciinema play` 及任何 cast 播放器回放,
//! Linop 自身也在 TUI 内提供回放(见 apps/linop 的回放模式)。

use std::io::Write as _;
use std::path::Path;
use std::time::{Instant, SystemTime, UNIX_EPOCH};

#[derive(Debug, thiserror::Error)]
pub enum CastError {
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("invalid cast: {0}")]
    Invalid(String),
}

pub type Result<T> = std::result::Result<T, CastError>;

/// 一条回放事件:`[elapsed, "o"|"i", data]`。
/// `o` = 终端输出,`i` = 键盘输入,`m`/`r` = 标记/调整大小(忽略亦可)。
#[derive(Debug, Clone, PartialEq)]
pub struct CastEvent {
    /// 相对录制开始的秒数。
    pub elapsed: f64,
    pub stream: char,
    pub data: String,
}

/// 解析后的 cast 文件。
#[derive(Debug, Clone, PartialEq)]
pub struct Cast {
    pub width: u16,
    pub height: u16,
    pub events: Vec<CastEvent>,
}

impl Cast {
    /// 全部输出事件拼接(快速预览/全文检索用)。
    pub fn output_text(&self) -> String {
        self.events
            .iter()
            .filter(|e| e.stream == 'o')
            .map(|e| e.data.as_str())
            .collect()
    }

    /// 录制总时长(最后一个事件的相对秒数;无事件为 0)。
    pub fn duration_secs(&self) -> f64 {
        self.events.last().map_or(0.0, |e| e.elapsed)
    }
}

/// 录制写入器:事件即时落盘(逐条 flush),崩溃也不丢已录内容。
///
/// SSH 输出是原始字节流,可能把一个 UTF-8 多字节字符拆在两次包里;
/// 写入器缓存不完整前缀,凑齐后才作为一条事件写出。
pub struct CastWriter {
    file: std::fs::File,
    start: Instant,
    carry: Vec<u8>,
}

impl CastWriter {
    pub fn create(path: impl AsRef<Path>, width: u16, height: u16) -> Result<Self> {
        let path = path.as_ref();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        let header = serde_json::json!({
            "version": 2,
            "width": width,
            "height": height,
            "timestamp": timestamp,
            "env": { "TERM": "xterm-256color" },
        });
        let mut file = std::fs::File::create(path)?;
        writeln!(file, "{header}")?;
        file.flush()?;
        Ok(Self {
            file,
            start: Instant::now(),
            carry: Vec::new(),
        })
    }

    /// 追加一段终端输出;返回本条写出的字符串长度(0 = 缓存中等待凑齐)。
    pub fn write_output(&mut self, bytes: &[u8]) -> Result<usize> {
        self.carry.extend_from_slice(bytes);
        let valid = match std::str::from_utf8(&self.carry) {
            Ok(_) => self.carry.len(),
            Err(err) => err.valid_up_to(),
        };
        if valid == 0 {
            // 连一个完整字符都没有:继续等后续包。
            return Ok(0);
        }
        let text = String::from_utf8_lossy(&self.carry[..valid]).into_owned();
        self.emit('o', &text)?;
        self.carry.drain(..valid);
        Ok(text.len())
    }

    fn emit(&mut self, stream: char, data: &str) -> Result<()> {
        let elapsed = self.start.elapsed().as_secs_f64();
        let data = serde_json::to_string(data)?;
        writeln!(self.file, "[{elapsed:.3},\"{stream}\",{data}]")?;
        self.file.flush()?;
        Ok(())
    }

    /// 结束录制:残缺字节(如被截断的宽字符)以替换符补齐写出后关闭。
    pub fn finish(mut self) -> Result<()> {
        if !self.carry.is_empty() {
            let text = String::from_utf8_lossy(&self.carry).into_owned();
            self.emit('o', &text)?;
        }
        self.file.flush()?;
        Ok(())
    }
}

/// 解析 cast v2 文本(首行 header JSON,后续每行一个事件数组)。
pub fn parse(text: &str) -> Result<Cast> {
    let mut lines = text.lines().filter(|l| !l.trim().is_empty());
    let header_line = lines
        .next()
        .ok_or_else(|| CastError::Invalid("空文件".into()))?;
    let header: serde_json::Value = serde_json::from_str(header_line)
        .map_err(|e| CastError::Invalid(format!("header 非法:{e}")))?;
    if header.get("version").and_then(|v| v.as_u64()) != Some(2) {
        return Err(CastError::Invalid("仅支持 cast v2".into()));
    }
    let width = header
        .get("width")
        .and_then(|v| v.as_u64())
        .and_then(|v| u16::try_from(v).ok())
        .ok_or_else(|| CastError::Invalid("header 缺少 width".into()))?;
    let height = header
        .get("height")
        .and_then(|v| v.as_u64())
        .and_then(|v| u16::try_from(v).ok())
        .ok_or_else(|| CastError::Invalid("header 缺少 height".into()))?;

    let mut events = Vec::new();
    for (idx, line) in lines.enumerate() {
        let value: serde_json::Value = serde_json::from_str(line)
            .map_err(|e| CastError::Invalid(format!("第 {} 个事件非法:{e}", idx + 1)))?;
        let arr = value
            .as_array()
            .ok_or_else(|| CastError::Invalid(format!("第 {} 个事件不是数组", idx + 1)))?;
        if arr.len() < 3 {
            return Err(CastError::Invalid(format!("第 {} 个事件字段不足", idx + 1)));
        }
        let elapsed = arr[0].as_f64().unwrap_or(0.0);
        let stream = arr[1]
            .as_str()
            .and_then(|s| s.chars().next())
            .ok_or_else(|| CastError::Invalid(format!("第 {} 个事件流标识非法", idx + 1)))?;
        let data = arr[2].as_str().unwrap_or("").to_string();
        events.push(CastEvent {
            elapsed,
            stream,
            data,
        });
    }
    Ok(Cast {
        width,
        height,
        events,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn write_then_parse_roundtrip() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("r.cast");
        let mut writer = CastWriter::create(&path, 100, 30).unwrap();
        writer.write_output(b"hello ").unwrap();
        std::thread::sleep(std::time::Duration::from_millis(20));
        writer.write_output(b"world\r\n").unwrap();
        writer.finish().unwrap();

        let cast = parse(&std::fs::read_to_string(&path).unwrap()).unwrap();
        assert_eq!(cast.width, 100);
        assert_eq!(cast.height, 30);
        assert_eq!(cast.events.len(), 2);
        assert!(cast.events[0].elapsed < cast.events[1].elapsed);
        assert_eq!(cast.output_text(), "hello world\r\n");
        assert!(cast.duration_secs() >= 0.02);
    }

    #[test]
    fn utf8_split_across_chunks_is_reassembled() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("u.cast");
        let mut writer = CastWriter::create(&path, 80, 24).unwrap();
        // "中" 是 3 字节,拆成 2+1 两段写入。
        let bytes = "中".as_bytes();
        assert_eq!(
            writer.write_output(&bytes[..2]).unwrap(),
            0,
            "不完整字符应缓存等待"
        );
        assert_eq!(writer.write_output(&bytes[2..]).unwrap(), "中".len());
        writer.finish().unwrap();

        let cast = parse(&std::fs::read_to_string(&path).unwrap()).unwrap();
        assert_eq!(cast.output_text(), "中");
    }

    #[test]
    fn control_sequences_are_escaped_in_json() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("esc.cast");
        let mut writer = CastWriter::create(&path, 80, 24).unwrap();
        writer
            .write_output(b"\x1b[31mred\x1b[0m \"quoted\"\n")
            .unwrap();
        writer.finish().unwrap();

        let text = std::fs::read_to_string(&path).unwrap();
        let cast = parse(&text).unwrap();
        assert_eq!(cast.output_text(), "\x1b[31mred\x1b[0m \"quoted\"\n");
    }

    #[test]
    fn parser_rejects_bad_headers_and_events() {
        assert!(parse("").is_err());
        assert!(parse("{\"version\":1,\"width\":80,\"height\":24}").is_err());
        assert!(parse("{\"version\":2,\"width\":80}").is_err());
        let valid_header = "{\"version\":2,\"width\":80,\"height\":24}";
        assert!(parse(&format!("{valid_header}\nnot json")).is_err());
        assert!(parse(&format!("{valid_header}\n[1,\"o\"]")).is_err());
        // 空行被忽略,不影响解析。
        let ok = parse(&format!("{valid_header}\n\n[0.5,\"o\",\"hi\"]\n\n")).unwrap();
        assert_eq!(ok.events.len(), 1);
        assert_eq!(ok.duration_secs(), 0.5);
    }
}
