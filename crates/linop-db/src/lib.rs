//! linop-db:数据库驱动层 —— 模式浏览、查询、执行。
//!
//! 对应 navop 的 `db` / `db_view` crate。当前内置 SQLite 驱动
//! (本地文件,纯进程内可测);驱动以独立模块划分,为后续
//! MySQL/PostgreSQL 等留位(navop 以扩展驱动形式支持更多数据库)。

pub mod sqlite;

pub use sqlite::{SqliteDb, StmtOutcome};

/// 一张表(或视图)的元信息。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TableInfo {
    pub name: String,
    /// 列名(顺序与建表一致)。
    pub columns: Vec<String>,
    /// 行数(COUNT(*);大表为估算也如实展示)。
    pub row_count: u64,
}

/// 查询结果:列名 + 行(单元格已格式化为文本,NULL 为 None)。
#[derive(Debug, Clone, Default, PartialEq)]
pub struct QueryResult {
    pub columns: Vec<String>,
    pub rows: Vec<Vec<Option<String>>>,
    /// 结果被行数上限截断时为 true。
    pub truncated: bool,
}

/// 非查询语句(INSERT/UPDATE/DDL)的执行结果。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ExecResult {
    pub rows_affected: u64,
}

#[derive(Debug, thiserror::Error)]
pub enum DbError {
    #[error("sqlite error: {0}")]
    Sqlite(#[from] rusqlite::Error),
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("database file not found: {0}")]
    NotFound(std::path::PathBuf),
    #[error("query error: {0}")]
    Query(String),
}

pub type Result<T> = std::result::Result<T, DbError>;

/// 查询结果行数上限(navop 数据网格同样分页;这里先截断到合理规模)。
pub const QUERY_ROW_LIMIT: usize = 500;

/// 把一个 SQLite 值格式化为展示文本。
fn format_value(value: rusqlite::types::ValueRef<'_>) -> Option<String> {
    use rusqlite::types::ValueRef;
    match value {
        ValueRef::Null => None,
        ValueRef::Integer(i) => Some(i.to_string()),
        ValueRef::Real(f) => Some(format!("{f}")),
        ValueRef::Text(t) => Some(String::from_utf8_lossy(t).into_owned()),
        ValueRef::Blob(b) => Some(format!(
            "x'{}'",
            b.iter().map(|x| format!("{x:02x}")).collect::<String>()
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn blob_and_null_formatting() {
        assert_eq!(format_value(rusqlite::types::ValueRef::Null), None);
        assert_eq!(
            format_value(rusqlite::types::ValueRef::Integer(-7)),
            Some("-7".into())
        );
        assert_eq!(
            format_value(rusqlite::types::ValueRef::Blob(&[0xde, 0xad])),
            Some("x'dead'".into())
        );
    }
}
