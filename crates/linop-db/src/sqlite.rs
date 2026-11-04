//! SQLite 驱动:本地文件,同步 API(上层用 spawn_blocking 包裹,
//! 事件循环永不阻塞)。

use std::path::{Path, PathBuf};

use rusqlite::Connection;

use super::{DbError, ExecResult, QUERY_ROW_LIMIT, QueryResult, Result, TableInfo, format_value};

/// 单条语句的执行结果:要么是结果集,要么是影响行数。
#[derive(Debug, Clone, PartialEq)]
pub enum StmtOutcome {
    Rows(QueryResult),
    Exec(ExecResult),
}

/// 一个打开的 SQLite 数据库。
pub struct SqliteDb {
    conn: Connection,
    path: PathBuf,
}

/// 标识符转义:表名进 SQL 前用双引号包裹并转义内部双引号。
fn quote_ident(name: &str) -> String {
    format!("\"{}\"", name.replace('"', "\"\""))
}

impl SqliteDb {
    /// 打开本地数据库文件;不存在时报错(避免误建空库)。
    pub fn open(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref();
        if !path.exists() {
            return Err(DbError::NotFound(path.to_path_buf()));
        }
        let conn = Connection::open(path)?;
        Ok(Self {
            conn,
            path: path.to_path_buf(),
        })
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    /// 列出用户表与视图(sqlite 内部表排除),按名称排序。
    pub fn tables(&self) -> Result<Vec<TableInfo>> {
        let mut stmt = self.conn.prepare(
            "SELECT name FROM sqlite_master \
             WHERE type IN ('table','view') AND name NOT LIKE 'sqlite_%' \
             ORDER BY name",
        )?;
        let names: Vec<String> = stmt
            .query_map([], |row| row.get::<_, String>(0))?
            .collect::<std::result::Result<_, _>>()?;
        names
            .into_iter()
            .map(|name| {
                let columns = self.columns(&name)?;
                let count: i64 = self.conn.query_row(
                    &format!("SELECT COUNT(*) FROM {}", quote_ident(&name)),
                    [],
                    |row| row.get(0),
                )?;
                Ok(TableInfo {
                    name,
                    columns,
                    row_count: count.max(0) as u64,
                })
            })
            .collect()
    }

    fn columns(&self, table: &str) -> Result<Vec<String>> {
        let mut stmt = self
            .conn
            .prepare(&format!("PRAGMA table_info({})", quote_ident(table)))?;
        let names = stmt
            .query_map([], |row| row.get::<_, String>(1))?
            .collect::<std::result::Result<Vec<_>, _>>()?;
        Ok(names)
    }

    /// 执行一条语句:有结果集则返回行(受默认行数上限截断),否则返回影响行数。
    pub fn run(&self, sql: &str) -> Result<StmtOutcome> {
        self.run_limited(sql, QUERY_ROW_LIMIT)
    }

    /// 同 `run`,但显式指定行数上限(测试截断逻辑用)。
    pub fn run_limited(&self, sql: &str, limit: usize) -> Result<StmtOutcome> {
        let mut stmt = self
            .conn
            .prepare(sql)
            .map_err(|e| DbError::Query(format!("{e}(仅支持单条语句)")))?;
        if stmt.column_count() == 0 {
            let affected = stmt.execute([])?;
            return Ok(StmtOutcome::Exec(ExecResult {
                rows_affected: affected as u64,
            }));
        }
        let columns: Vec<String> = stmt.column_names().map(str::to_string).collect();
        let mut rows_iter = stmt.query([])?;
        let mut rows = Vec::new();
        let mut truncated = false;
        while let Some(row) = rows_iter.next()? {
            if rows.len() >= limit {
                truncated = true;
                break;
            }
            let mut cells = Vec::with_capacity(columns.len());
            for i in 0..columns.len() {
                cells.push(format_value(row.get_ref(i)?));
            }
            rows.push(cells);
        }
        Ok(StmtOutcome::Rows(QueryResult {
            columns,
            rows,
            truncated,
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_db() -> (tempfile::TempDir, SqliteDb) {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("t.db");
        let conn = Connection::open(&path).unwrap();
        conn.execute_batch(
            "CREATE TABLE users (id INTEGER PRIMARY KEY, name TEXT, score REAL);
             CREATE TABLE logs (msg TEXT);
             CREATE VIEW v_users AS SELECT id, name FROM users;
             INSERT INTO users (name, score) VALUES ('alice', 9.5), ('bob', NULL);
             INSERT INTO logs (msg) VALUES ('boot'), ('ready');",
        )
        .unwrap();
        drop(conn);
        let db = SqliteDb::open(&path).unwrap();
        (dir, db)
    }

    #[test]
    fn lists_tables_and_views_with_columns_and_counts() {
        let (_dir, db) = temp_db();
        let tables = db.tables().unwrap();
        let names: Vec<&str> = tables.iter().map(|t| t.name.as_str()).collect();
        assert_eq!(names, vec!["logs", "users", "v_users"]);
        let users = &tables[1];
        assert_eq!(users.columns, vec!["id", "name", "score"]);
        assert_eq!(users.row_count, 2);
        let view = &tables[2];
        assert_eq!(view.columns, vec!["id", "name"]);
    }

    #[test]
    fn query_formats_cells() {
        let (_dir, db) = temp_db();
        let StmtOutcome::Rows(result) =
            db.run("SELECT name, score FROM users ORDER BY id").unwrap()
        else {
            panic!("SELECT 应返回结果集");
        };
        assert_eq!(result.columns, vec!["name", "score"]);
        assert_eq!(
            result.rows[0],
            vec![Some("alice".into()), Some("9.5".into())]
        );
        assert_eq!(
            result.rows[1],
            vec![Some("bob".into()), None],
            "NULL 应为 None"
        );
        assert!(!result.truncated);
    }

    #[test]
    fn execute_reports_affected_rows() {
        let (_dir, db) = temp_db();
        let StmtOutcome::Exec(exec) = db.run("DELETE FROM logs WHERE msg = 'boot'").unwrap() else {
            panic!("DELETE 应返回影响行数");
        };
        assert_eq!(exec.rows_affected, 1);
        let StmtOutcome::Rows(result) = db.run("SELECT COUNT(*) AS n FROM logs").unwrap() else {
            panic!("SELECT 应返回结果集");
        };
        assert_eq!(result.rows[0][0], Some("1".into()));
    }

    #[test]
    fn row_limit_truncates() {
        let (_dir, db) = temp_db();
        db.run("CREATE TABLE seq (n INTEGER)").unwrap();
        for i in 0..10 {
            db.run(&format!("INSERT INTO seq (n) VALUES ({i})"))
                .unwrap();
        }
        let StmtOutcome::Rows(result) = db.run_limited("SELECT n FROM seq", 4).unwrap() else {
            panic!("SELECT 应返回结果集");
        };
        assert_eq!(result.rows.len(), 4);
        assert!(result.truncated, "超出上限应标记 truncated");
    }

    #[test]
    fn missing_file_is_reported() {
        assert!(
            matches!(
                SqliteDb::open("/nonexistent/nope.db"),
                Err(DbError::NotFound(_))
            ),
            "缺文件应报 NotFound"
        );
    }

    #[test]
    fn bad_sql_is_query_error() {
        let (_dir, db) = temp_db();
        assert!(matches!(db.run("SELEKT 1"), Err(DbError::Query(_))));
    }
}
