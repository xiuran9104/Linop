//! linop-dbx CLI:TUI/脚本与 DBX 内核的桥接子命令。
//!
//! 用法:
//!   linop-dbx connections                 列出连接(JSON)
//!   linop-dbx tables <连接名>             表列表(JSON)
//!   linop-dbx query <连接名> <SQL> [上限]  执行查询(JSON)
//!
//! 连接配置来自 `~/.linop-dev/databases.toml`(linop-core::DbRegistry),
//! 数据库执行走 DBX 内核(dbx-core)。

use linop_dbx::Dbx;

#[tokio::main]
async fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let usage = "usage: linop-dbx connections | tables <name> | query <name> <sql> [max-rows]";
    let Some(cmd) = args.first() else {
        eprintln!("{usage}");
        std::process::exit(2);
    };
    let code = match cmd.as_str() {
        "connections" => {
            run(async {
                let dbx = open().await?;
                let list = dbx.connections();
                serde_json::to_string_pretty(&list).map_err(|e| e.to_string())
            })
            .await
        }
        "tables" => {
            let Some(name) = args.get(1) else {
                err_exit(usage)
            };
            run(async {
                let dbx = open().await?;
                let tables = dbx.list_tables(name).await.map_err(|e| e.to_string())?;
                serde_json::to_string_pretty(&tables).map_err(|e| e.to_string())
            })
            .await
        }
        "query" => {
            let (Some(name), Some(sql)) = (args.get(1), args.get(2)) else {
                err_exit(usage)
            };
            let max: usize = args.get(3).and_then(|v| v.parse().ok()).unwrap_or(500);
            run(async {
                let dbx = open().await?;
                let mut result = dbx.query(name, sql).await.map_err(|e| e.to_string())?;
                // QueryResult 只有 Deserialize(上游面向内部传输),CLI 在此
                // 投影为精简 JSON:列名 + 字符串化的行 + 元信息。
                if result.rows.len() > max {
                    result.rows.truncate(max);
                }
                let rows: Vec<Vec<serde_json::Value>> = result
                    .rows
                    .iter()
                    .map(|row| row.iter().map(cell_to_json).collect())
                    .collect();
                serde_json::to_string_pretty(&serde_json::json!({
                    "columns": result.columns,
                    "rows": rows,
                    "truncated": result.truncated,
                    "affected_rows": result.affected_rows,
                    "execution_time_ms": result.execution_time_ms,
                }))
                .map_err(|e| e.to_string())
            })
            .await
        }
        _ => err_exit(usage),
    };
    std::process::exit(code);
}

async fn open() -> Result<Dbx, String> {
    let profiles = linop_core::DbRegistry::load_default()
        .map_err(|e| e.to_string())?
        .profiles()
        .to_vec();
    let store = linop_dbx::default_store_path().map_err(|e| e.to_string())?;
    Dbx::open(&store, profiles).await.map_err(|e| e.to_string())
}

async fn run(f: impl std::future::Future<Output = Result<String, String>>) -> i32 {
    match f.await {
        Ok(json) => {
            println!("{json}");
            0
        }
        Err(e) => {
            eprintln!("错误:{e}");
            1
        }
    }
}

/// 单元格 → JSON(serde_json::Value 原样;其余字符串化)。
fn cell_to_json(value: &serde_json::Value) -> serde_json::Value {
    value.clone()
}

fn err_exit(usage: &str) -> ! {
    eprintln!("{usage}");
    std::process::exit(2);
}
