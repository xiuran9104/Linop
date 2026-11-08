//! linop-dbx CLI:TUI/脚本与 DBX 后端的桥接子命令。
//!
//! 用法:
//!   linop-dbx connections                 列出 DBX 连接(JSON)
//!   linop-dbx tables <连接名>             列表(JSON)
//!   linop-dbx query <连接名> <SQL> [上限]  执行查询(JSON)
//!   linop-dbx launch-web <dbx-web 路径>   启动 DBX Web UI
//! DBX_DATA_DIR 环境变量重定向数据目录(与 DBX 桌面/CLI/MCP 一致)。

use linop_dbx::Dbx;

#[tokio::main]
async fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let usage = "usage: linop-dbx connections | tables <name> | query <name> <sql> [max] | launch-web <bin>";
    let Some(cmd) = args.first() else {
        eprintln!("{usage}");
        std::process::exit(2);
    };
    let code = match cmd.as_str() {
        "connections" => {
            run(async {
                let dbx = Dbx::open_default().await.map_err(|e| e.to_string())?;
                let list = summaries(&dbx).await?;
                serde_json::to_string_pretty(&list).map_err(|e| e.to_string())
            })
            .await
        }
        "tables" => {
            let Some(name) = args.get(1) else {
                err_exit(usage)
            };
            run(async {
                let dbx = Dbx::open_default().await.map_err(|e| e.to_string())?;
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
                let dbx = Dbx::open_default().await.map_err(|e| e.to_string())?;
                let result = dbx.query(name, sql, max).await.map_err(|e| e.to_string())?;
                serde_json::to_string_pretty(&result).map_err(|e| e.to_string())
            })
            .await
        }
        "launch-web" => {
            let Some(bin) = args.get(1) else {
                err_exit(usage)
            };
            match Dbx::launch_web(std::path::Path::new(bin)) {
                Ok(child) => {
                    eprintln!("dbx-web 已启动(pid {})", child.id());
                    0
                }
                Err(e) => {
                    eprintln!("启动失败:{e}");
                    1
                }
            }
        }
        _ => err_exit(usage),
    };
    std::process::exit(code);
}

async fn summaries(dbx: &Dbx) -> Result<Vec<linop_dbx::DbxConnectionSummary>, String> {
    dbx.connections().await.map_err(|e| e.to_string())
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

fn err_exit(usage: &str) -> ! {
    eprintln!("{usage}");
    std::process::exit(2);
}
