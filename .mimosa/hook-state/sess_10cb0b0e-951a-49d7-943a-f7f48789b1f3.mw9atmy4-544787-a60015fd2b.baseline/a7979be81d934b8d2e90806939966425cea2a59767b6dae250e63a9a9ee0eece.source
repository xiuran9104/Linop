//! 交互式 shell 通道:写入按键字节流,读出终端输出。
//!
//! 通道可拆为 `ShellReader` / `ShellWriter` 两半:前端 TUI 持有写半边
//! 发送输入与窗口变化,读半边交给后台任务持续把远端输出送回事件循环。

use russh::client::Msg;
use russh::{Channel, ChannelMsg, ChannelReadHalf, ChannelWriteHalf};

use crate::Result;

/// 一个已请求 PTY + shell 的会话通道。
pub struct ShellChannel {
    read: ChannelReadHalf,
    write: ChannelWriteHalf<Msg>,
}

impl ShellChannel {
    pub(crate) fn new(channel: Channel<Msg>) -> Self {
        let (read, write) = channel.split();
        Self { read, write }
    }

    /// 拆分为读写两半,供并发使用(读半边 move 进输出转发任务)。
    pub fn into_parts(self) -> (ShellReader, ShellWriter) {
        (
            ShellReader { read: self.read },
            ShellWriter { write: self.write },
        )
    }

    /// 发送终端输入(按键编码后的字节)。
    pub async fn write(&self, data: &[u8]) -> Result<()> {
        self.write.data_bytes(data.to_vec()).await?;
        Ok(())
    }

    /// 读取一批输出(stdout 或 stderr 合并);通道关闭时返回 `None`。
    /// 忽略窗口变化等控制消息,持续等待到数据或关闭。
    pub async fn read_output(&mut self) -> Option<Vec<u8>> {
        read_output_from(&mut self.read).await
    }

    /// 通知远端终端尺寸变化。
    pub async fn resize(&self, cols: u16, rows: u16) -> Result<()> {
        self.write
            .window_change(cols as u32, rows as u32, 0, 0)
            .await?;
        Ok(())
    }

    /// 向远端发送信号(如 Ctrl+C 已被前端拦截时可用)。
    pub async fn signal(&self, sig: russh::Sig) -> Result<()> {
        self.write.signal(sig).await?;
        Ok(())
    }
}

/// 读取一批输出(stdout 或 stderr 合并);通道关闭时返回 `None`。
/// 忽略窗口变化等控制消息,持续等待到数据或关闭。
async fn read_output_from(read: &mut ChannelReadHalf) -> Option<Vec<u8>> {
    loop {
        match read.wait().await? {
            ChannelMsg::Data { data } => return Some(data.to_vec()),
            ChannelMsg::ExtendedData { data, .. } => return Some(data.to_vec()),
            ChannelMsg::Eof | ChannelMsg::Close => return None,
            _ => continue,
        }
    }
}

/// shell 读半边:后台任务读取远端输出。
pub struct ShellReader {
    read: ChannelReadHalf,
}

impl ShellReader {
    /// 读取一批输出(stdout 或 stderr 合并);通道关闭时返回 `None`。
    /// 忽略窗口变化等控制消息,持续等待到数据或关闭。
    pub async fn read_output(&mut self) -> Option<Vec<u8>> {
        read_output_from(&mut self.read).await
    }
}

/// shell 写半边:前端持有,发送终端输入与 PTY 控制。
pub struct ShellWriter {
    write: ChannelWriteHalf<Msg>,
}

impl ShellWriter {
    /// 发送终端输入(按键编码后的字节)。
    pub async fn write(&self, data: &[u8]) -> Result<()> {
        self.write.data_bytes(data.to_vec()).await?;
        Ok(())
    }

    /// 通知远端终端尺寸变化。
    pub async fn resize(&self, cols: u16, rows: u16) -> Result<()> {
        self.write
            .window_change(cols as u32, rows as u32, 0, 0)
            .await?;
        Ok(())
    }

    /// 向远端发送信号。
    pub async fn signal(&self, sig: russh::Sig) -> Result<()> {
        self.write.signal(sig).await?;
        Ok(())
    }
}
