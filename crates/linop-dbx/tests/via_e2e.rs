//! via 隧道 e2e:postgres 容器 + SSH 跳板(testsupport)→ 隧道端点注入 → 查询。

use linop_core::{Connection as SshConnection, Auth, DbProfile, DbRegistry, Registry};
use linop_dbx::Dbx;
use linop_testsupport::TestSshServer;

/// RAII 的 Postgres 测试容器。
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
        eprintln!("docker 不可用,跳过 via e2e");
        return None;
    }
    let name = format!("linop-via-pg-{}", std::process::id());
    let _ = std::process::Command::new("docker")
        .args(["rm", "-f", &name])
        .output();
    let out = std::process::Command::new("docker")
        .args([
            "run", "-d", "--name", &name,
            "-e", "POSTGRES_PASSWORD=linop",
            "-p", "127.0.0.1::5432",
            "postgres:18",
        ])
        .output()
        .expect("docker run");
    assert!(out.status.success(), "容器启动失败:{}", String::from_utf8_lossy(&out.stderr));
    let guard = PgContainer { name: name.clone() };

    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(20);
    loop {
        let ready = std::process::Command::new("docker")
            .args(["exec", &name, "pg_isready", "-U", "postgres"])
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false);
        if ready { break; }
        assert!(std::time::Instant::now() < deadline, "postgres 20s 未就绪");
        std::thread::sleep(std::time::Duration::from_millis(300));
    }

    let out = std::process::Command::new("docker")
        .args(["port", &name, "5432/tcp"])
        .output()
        .expect("docker port");
    let text = String::from_utf8_lossy(&out.stdout).into_owned();
    let port: u16 = text.lines().find_map(|l| l.rsplit(':').next()?.parse().ok())?;
    Some((port, guard))
}

fn psql(container: &PgContainer, sql: &str) {
    let out = std::process::Command::new("docker")
        .args(["exec", container.name.as_str(), "psql", "-U", "postgres", "-v", "ON_ERROR_STOP=1", "-c", sql])
        .output()
        .expect("psql");
    assert!(out.status.success(), "psql 失败({sql}):{}", String::from_utf8_lossy(&out.stderr));
}

#[tokio::test]
async fn postgres_via_ssh_tunnel() {
    let Some((pg_port, container)) = start_postgres() else { return };
    psql(&container, "CREATE TABLE tunnel_test (id serial PRIMARY KEY, val text); INSERT INTO tunnel_test (val) VALUES ('via-ok');");

    // SSH 跳板(testsupport echo 服务器支持 direct-tcpip 桥接)。
    let jump = TestSshServer::start_echo().await.unwrap();

    // connections.toml:跳板定义。
    let mut jump_conn = SshConnection::new(
        "jump",
        jump.addr.ip().to_string(),
        "u",
        Auth::password("p"),
    );
    jump_conn.port = jump.addr.port();
    let conns: Registry = vec![jump_conn].into_iter().collect();

    // databases.toml:postgres 连接带 via = "jump"。
    let mut profile = DbProfile::postgres(
        "pg-via",
        "127.0.0.1",
        pg_port,
        "postgres",
        Some("linop".into()),
        None,
    );
    profile.via = Some("jump".into());

    let dir = tempfile::tempdir().unwrap();
    let store = dir.path().join("dbx.db");
    let dbx = Dbx::open(&store, vec![profile], &conns).await.unwrap();

    // 隧道已建立(记录在 tunnels)。
    let tunnels = dbx.tunnel_names().await;
    assert_eq!(tunnels, vec!["pg-via".to_string()]);

    // 经隧道查询。
    let result = dbx.query("pg-via", "SELECT val FROM tunnel_test").await.unwrap();
    assert_eq!(result.rows.len(), 1);
    assert_eq!(result.rows[0][0], serde_json::json!("via-ok"));
}
