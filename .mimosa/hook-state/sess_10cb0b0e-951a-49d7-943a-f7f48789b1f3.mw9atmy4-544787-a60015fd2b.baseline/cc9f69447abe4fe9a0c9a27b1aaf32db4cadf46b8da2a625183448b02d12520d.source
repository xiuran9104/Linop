//! 测试用 SFTP 服务端:把 SFTP 请求映射到本地临时目录上的 std::fs。
//!
//! 只实现 linop 客户端用到的子集(realpath/opendir/readdir/open/
//! read/write/close/stat/remove/mkdir/rmdir/rename)。

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

use russh_sftp::protocol::{
    Attrs, Data, File, FileAttributes, Handle, Name, OpenFlags, Status, StatusCode, Version,
};

/// 文件句柄表:handle 字符串 -> 已打开的路径。
pub struct FsSftpHandler {
    root: PathBuf,
    handles: HashMap<String, PathBuf>,
    /// 已一次性读完条目的目录句柄:客户端随后会再 readdir 一次,
    /// 按 SFTP v3 语义应答 EOF 而不是报错。
    done_dirs: HashSet<String>,
    next_handle: u64,
}

fn status(id: u32, code: StatusCode, msg: &str) -> Status {
    Status {
        id,
        status_code: code,
        error_message: msg.to_string(),
        language_tag: "en-US".to_string(),
    }
}

fn ok_status(id: u32) -> Status {
    status(id, StatusCode::Ok, "Ok")
}

fn io_status(_id: u32, err: &std::io::Error) -> StatusCode {
    match err.kind() {
        std::io::ErrorKind::NotFound => StatusCode::NoSuchFile,
        std::io::ErrorKind::AlreadyExists => StatusCode::Failure,
        _ => StatusCode::Failure,
    }
}

fn attrs_from_md(md: &std::fs::Metadata) -> FileAttributes {
    #[cfg(unix)]
    let permissions = Some(std::os::unix::fs::MetadataExt::mode(md));
    #[cfg(not(unix))]
    let permissions = None;
    FileAttributes {
        size: Some(md.len()),
        permissions,
        mtime: md
            .modified()
            .ok()
            .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
            .map(|d| d.as_secs() as u32),
        ..Default::default()
    }
}

impl FsSftpHandler {
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self {
            root: root.into(),
            handles: HashMap::new(),
            done_dirs: HashSet::new(),
            next_handle: 0,
        }
    }

    /// 把 SFTP 路径(以 '/' 开头)安全映射到 root 下的真实路径。
    fn resolve(&self, path: &str) -> PathBuf {
        let rel = path.trim_start_matches('/');
        let full = self.root.join(rel);
        // 防御性处理:不允许越出 root。
        let canonical = full
            .ancestors()
            .find(|p| p.exists())
            .and_then(|p| p.canonicalize().ok())
            .unwrap_or_else(|| self.root.clone());
        if canonical.starts_with(&self.root) {
            full
        } else {
            self.root.clone()
        }
    }
}

impl russh_sftp::server::Handler for FsSftpHandler {
    type Error = StatusCode;

    fn unimplemented(&self) -> Self::Error {
        StatusCode::OpUnsupported
    }

    async fn init(
        &mut self,
        _version: u32,
        _extensions: HashMap<String, String>,
    ) -> Result<Version, Self::Error> {
        Ok(Version::new())
    }

    async fn realpath(&mut self, id: u32, path: String) -> Result<Name, Self::Error> {
        let full = self.resolve(&path);
        let canonical = full.canonicalize().unwrap_or(full);
        let rel = canonical
            .strip_prefix(self.root.canonicalize().unwrap_or(self.root.clone()))
            .unwrap_or(Path::new(""));
        let shown = Path::new("/").join(rel);
        Ok(Name {
            id,
            files: vec![File::new(
                shown.display().to_string(),
                FileAttributes::default(),
            )],
        })
    }

    async fn opendir(&mut self, id: u32, path: String) -> Result<Handle, Self::Error> {
        let full = self.resolve(&path);
        if !full.is_dir() {
            return Err(StatusCode::NoSuchFile);
        }
        self.next_handle += 1;
        let handle = format!("dir-{}", self.next_handle);
        self.handles.insert(handle.clone(), full);
        Ok(Handle { id, handle })
    }

    async fn readdir(&mut self, id: u32, handle: String) -> Result<Name, Self::Error> {
        let path = match self.handles.get(&handle) {
            Some(p) => p.clone(),
            None => return Err(StatusCode::Failure),
        };
        if self.done_dirs.contains(&handle) {
            return Err(StatusCode::Eof);
        }
        let mut files = Vec::new();
        for entry in std::fs::read_dir(&path).map_err(|_| StatusCode::Failure)? {
            let entry = entry.map_err(|_| StatusCode::Failure)?;
            let md = entry.metadata().map_err(|_| StatusCode::Failure)?;
            files.push(File::new(
                entry.file_name().to_string_lossy().to_string(),
                attrs_from_md(&md),
            ));
        }
        // 一次性返回全部条目;句柄保留到 close,后续 readdir 以 EOF 应答。
        self.done_dirs.insert(handle);
        Ok(Name { id, files })
    }

    async fn open(
        &mut self,
        id: u32,
        filename: String,
        pflags: OpenFlags,
        _attrs: FileAttributes,
    ) -> Result<Handle, Self::Error> {
        let full = self.resolve(&filename);
        if pflags.contains(OpenFlags::READ) && !full.exists() {
            return Err(StatusCode::NoSuchFile);
        }
        if pflags.contains(OpenFlags::CREATE) {
            if let Some(parent) = full.parent() {
                std::fs::create_dir_all(parent).map_err(|_| StatusCode::Failure)?;
            }
            let mut opts = std::fs::OpenOptions::new();
            opts.write(true).read(true);
            if pflags.contains(OpenFlags::TRUNCATE) {
                opts.truncate(true);
            }
            opts.create(true)
                .open(&full)
                .map_err(|_| StatusCode::Failure)?;
        }
        self.next_handle += 1;
        let handle = format!("file-{}", self.next_handle);
        self.handles.insert(handle.clone(), full);
        Ok(Handle { id, handle })
    }

    async fn close(&mut self, id: u32, handle: String) -> Result<Status, Self::Error> {
        self.handles.remove(&handle);
        self.done_dirs.remove(&handle);
        Ok(ok_status(id))
    }

    async fn read(
        &mut self,
        id: u32,
        handle: String,
        offset: u64,
        len: u32,
    ) -> Result<Data, Self::Error> {
        use std::io::{Read, Seek, SeekFrom};
        let path = match self.handles.get(&handle) {
            Some(p) => p.clone(),
            None => return Err(StatusCode::Failure),
        };
        let mut file = std::fs::File::open(&path).map_err(|_| StatusCode::NoSuchFile)?;
        file.seek(SeekFrom::Start(offset))
            .map_err(|_| StatusCode::Failure)?;
        let mut buf = vec![0u8; len as usize];
        let n = file.read(&mut buf).map_err(|_| StatusCode::Failure)?;
        buf.truncate(n);
        if n == 0 {
            // SFTP v3:读尽文件以 EOF 状态回复。
            return Err(StatusCode::Eof);
        }
        Ok(Data { id, data: buf })
    }

    async fn write(
        &mut self,
        id: u32,
        handle: String,
        offset: u64,
        data: Vec<u8>,
    ) -> Result<Status, Self::Error> {
        use std::io::{Seek, SeekFrom, Write};
        let path = match self.handles.get(&handle) {
            Some(p) => p.clone(),
            None => return Err(StatusCode::Failure),
        };
        let mut file = std::fs::OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(false)
            .open(&path)
            .map_err(|e| io_status(id, &e))?;
        file.seek(SeekFrom::Start(offset))
            .map_err(|_| StatusCode::Failure)?;
        file.write_all(&data).map_err(|e| io_status(id, &e))?;
        Ok(ok_status(id))
    }

    async fn stat(&mut self, id: u32, path: String) -> Result<Attrs, Self::Error> {
        let full = self.resolve(&path);
        let md = std::fs::metadata(&full).map_err(|_| StatusCode::NoSuchFile)?;
        Ok(Attrs {
            id,
            attrs: attrs_from_md(&md),
        })
    }

    async fn lstat(&mut self, id: u32, path: String) -> Result<Attrs, Self::Error> {
        let full = self.resolve(&path);
        let md = std::fs::symlink_metadata(&full).map_err(|_| StatusCode::NoSuchFile)?;
        Ok(Attrs {
            id,
            attrs: attrs_from_md(&md),
        })
    }

    async fn remove(&mut self, id: u32, filename: String) -> Result<Status, Self::Error> {
        std::fs::remove_file(self.resolve(&filename)).map_err(|e| io_status(id, &e))?;
        Ok(ok_status(id))
    }

    async fn mkdir(
        &mut self,
        id: u32,
        path: String,
        _attrs: FileAttributes,
    ) -> Result<Status, Self::Error> {
        std::fs::create_dir(self.resolve(&path)).map_err(|e| io_status(id, &e))?;
        Ok(ok_status(id))
    }

    async fn rmdir(&mut self, id: u32, path: String) -> Result<Status, Self::Error> {
        std::fs::remove_dir(self.resolve(&path)).map_err(|e| io_status(id, &e))?;
        Ok(ok_status(id))
    }

    async fn rename(
        &mut self,
        id: u32,
        oldpath: String,
        newpath: String,
    ) -> Result<Status, Self::Error> {
        std::fs::rename(self.resolve(&oldpath), self.resolve(&newpath))
            .map_err(|e| io_status(id, &e))?;
        Ok(ok_status(id))
    }
}
