//! SFTP 端到端测试:linop-sftp 客户端 ↔ testsupport 文件系统服务端。

use linop_core::Auth;
use linop_sftp::Sftp;
use linop_ssh::{KnownHostsPolicy, Session};
use linop_testsupport::TestSshServer;

async fn sftp_on(root: &std::path::Path) -> anyhow::Result<Sftp> {
    let server = TestSshServer::start_with_sftp(root).await?;
    let mut conn =
        linop_core::Connection::new("t", server.addr.ip().to_string(), "u", Auth::password("p"));
    conn.port = server.addr.port();
    let ssh = Session::connect_profile(&conn, KnownHostsPolicy::AcceptAll).await?;
    Ok(Sftp::open(&ssh).await?)
}

#[tokio::test]
async fn browse_upload_download() -> anyhow::Result<()> {
    let root = tempfile::tempdir()?;
    let mut sftp = sftp_on(root.path()).await?;

    // 上传两个文件 + 一个目录。
    let local_src = tempfile::tempdir()?;
    std::fs::write(local_src.path().join("a.txt"), b"alpha")?;
    std::fs::write(local_src.path().join("b.txt"), b"beta-beta")?;
    sftp.upload(local_src.path().join("a.txt").as_path(), "a.txt")
        .await?;
    sftp.upload(local_src.path().join("b.txt").as_path(), "b.txt")
        .await?;
    sftp.mkdir("docs").await?;
    sftp.upload(local_src.path().join("a.txt").as_path(), "docs/a.txt")
        .await?;

    // 浏览:目录在前,按名排序。
    let entries = sftp.list_dir(".").await?;
    let names: Vec<(String, bool)> = entries.iter().map(|e| (e.name.clone(), e.is_dir)).collect();
    assert_eq!(
        names,
        vec![
            ("docs".to_string(), true),
            ("a.txt".to_string(), false),
            ("b.txt".to_string(), false)
        ]
    );
    assert_eq!(entries[1].size, 5);
    assert_eq!(entries[2].size_h, "9 B");

    // 进入/返回。服务端以临时目录为根,cwd 是虚拟路径。
    let docs = sftp.enter("docs").await?;
    assert_eq!(docs.len(), 1);
    assert_eq!(sftp.cwd(), "/docs");
    sftp.go_parent().await?;
    assert_eq!(sftp.cwd(), "/");

    // 下载并比对内容。
    let local_dst = tempfile::tempdir()?;
    sftp.download("b.txt", &local_dst.path().join("b.txt"))
        .await?;
    assert_eq!(std::fs::read(local_dst.path().join("b.txt"))?, b"beta-beta");
    // 递归下载目录。
    sftp.download("docs", &local_dst.path().join("docs"))
        .await?;
    assert_eq!(
        std::fs::read(local_dst.path().join("docs/a.txt"))?,
        b"alpha"
    );

    // 文本编辑原语。
    assert_eq!(sftp.read_text("a.txt").await?, "alpha");
    sftp.write_text("a.txt", "alpha2").await?;
    assert_eq!(sftp.read_text("a.txt").await?, "alpha2");

    // rename / remove。
    sftp.rename("a.txt", "a2.txt").await?;
    sftp.remove_file("a2.txt").await?;
    sftp.remove_file("docs/a.txt").await?;
    sftp.remove_dir("docs").await?;
    assert!(sftp.list_dir(".").await?.iter().all(|e| e.name == "b.txt"));

    sftp.close().await?;
    Ok(())
}
