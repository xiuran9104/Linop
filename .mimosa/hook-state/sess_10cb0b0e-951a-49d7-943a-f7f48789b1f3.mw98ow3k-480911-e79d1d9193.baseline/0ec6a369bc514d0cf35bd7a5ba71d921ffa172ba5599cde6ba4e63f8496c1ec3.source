//! Postgres 真实端到端:docker postgres:18 → Linop TOML → AppState 注入 → 查询。

use linop_core::{DbProfile, DbRegistry};
use linop_dbx::Dbx;

/// RAII 的测试容器(随机宿主端口)。
struct PgContainer {
    name: String,
}

impl Drop for PgContainer {
    fn drop(&mut self) {
        let _ = std::process::Command::new("docker")
            .args(["rm", "-f", &self.name])
            .output();
    }
}

fn start_postgres() -> Option<(u16, PgContainer)> {
    let ok = std::process::Command::new("docker")
        .args(["version", "--format", "{{.Server.Version}}"])
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false);
    if !ok {
        eprintln!("docker 不可用,跳过 postgres e2e");
        return None;
    }
    let name = format!("linop-linop-db-pg-{}", std::process::id());
    let _ = std::process::Command::new("docker")
        .args(["rm", "-f", &name])
        .output();
    let out = std::process::Command::new("docker")
        .args([
            "run",
            "-d",
            "--name",
            &name,
            "-e",
            "POSTGRES_PASSWORD=linop",
            "-p",
            "127.0.0.1::5432",
            "postgres:18",
        ])
        .output()
        .expect("docker run");
    assert!(
        out.status.success(),
        "容器启动失败:{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let guard = PgContainer { name: name.clone() };

    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(20);
    loop {
        let ready = std::process::Command::new("docker")
            .args(["exec", &name, "pg_isready", "-U", "postgres"])
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false);
        if ready {
            break;
        }
        assert!(std::time::Instant::now() < deadline, "postgres 20s 未就绪");
        std::thread::sleep(std::time::Duration::from_millis(300));
    }

    let out = std::process::Command::new("docker")
        .args(["port", &name, "5432/tcp"])
        .output()
        .expect("docker port");
    let text = String::from_utf8_lossy(&out.stdout).into_owned();
    let port: u16 = text
        .lines()
        .find_map(|l| l.rsplit(':').next()?.parse().ok())?;
    Some((port, guard))
}

fn psql(container: &PgContainer, sql: &str) {
    let out = std::process::Command::new("docker")
        .args([
            "exec",
            container.name.as_str(),
            "psql",
            "-U",
            "postgres",
            "-v",
            "ON_ERROR_STOP=1",
            "-c",
            sql,
        ])
        .output()
        .expect("psql");
    assert!(
        out.status.success(),
        "psql 失败({sql}):{}",
        String::from_utf8_lossy(&out.stderr)
    );
}

#[tokio::test]
async fn query_via_linop_db_core_state_injection() {
    let Some((port, container)) = start_postgres() else {
        return;
    };
    psql(
        &container,
        "CREATE TABLE items (id serial PRIMARY KEY, name text); INSERT INTO items (name) VALUES ('alpha'), ('beta');",
    );

    let dir = tempfile::tempdir().unwrap();
    let store = dir.path().join("dbx.db");
    let profiles = vec![DbProfile::postgres(
        "pg",
        "127.0.0.1",
        port,
        "postgres",
        Some("linop".into()),
        None,
    )];
    // TOML 往返:确认注册表与注入链一致。
    let mut reg = DbRegistry::new();
    for p in &profiles {
        reg.upsert(p.clone());
    }
    let toml_path = dir.path().join("databases.toml");
    reg.save(&toml_path).unwrap();
    let loaded = DbRegistry::load(&toml_path).unwrap();
    assert_eq!(loaded.profiles(), &profiles);

    let conns_reg = linop_core::Registry::new();
    let dbx = Dbx::open(&store, loaded.profiles().to_vec(), &conns_reg).await.unwrap();

    // 表元数据。
    let tables = dbx.list_tables("pg").await.unwrap();
    assert!(
        tables.iter().any(|t| t.name.contains("items")),
        "应列出 items: {:?}",
        tables.iter().map(|t| &t.name).collect::<Vec<_>>()
    );

    // 查询。
    let result = dbx
        .query("pg", "SELECT name FROM items ORDER BY id")
        .await
        .unwrap();
    assert_eq!(result.columns, vec!["name"]);
    assert_eq!(result.rows.len(), 2);
    assert_eq!(result.rows[0][0], serde_json::json!("alpha"));
    assert_eq!(result.rows[1][0], serde_json::json!("beta"));

    // 非法 SQL:错误回传字符串。
    assert!(dbx.query("pg", "SELEKT 1").await.is_err());
}
