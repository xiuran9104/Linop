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

/// 流式传输:多分块文件往返、进度回调、取消语义。
#[tokio::test]
async fn streaming_transfer_roundtrip_and_cancel() -> anyhow::Result<()> {
    let root = tempfile::tempdir()?;
    let sftp = sftp_on(root.path()).await?;

    // 约 10 个分块的数据,内容可校验。
    let payload: Vec<u8> = (0..300_000u32).map(|i| (i % 251) as u8).collect();
    let local_src = tempfile::tempdir()?;
    let src = local_src.path().join("big.bin");
    std::fs::write(&src, &payload)?;

    // 上传:进度应单调递增至文件总长。
    let mut seen_done: u64 = 0;
    let mut seen_total: u64 = 0;
    sftp.upload_streaming(
        &src,
        "big.bin",
        &mut |_, done, total| {
            assert!(done >= seen_done, "进度必须单调不减");
            seen_done = done;
            seen_total = total;
        },
        &linop_sftp::CancelFlag::new(),
    )
    .await?;
    assert_eq!(seen_total, payload.len() as u64);
    assert_eq!(seen_done, payload.len() as u64);

    // 下载到另一个位置并逐字节比对。
    let local_dst = tempfile::tempdir()?;
    let dst = local_dst.path().join("big.bin");
    sftp.download_streaming(
        "big.bin",
        &dst,
        &mut |_, _, _| {},
        &linop_sftp::CancelFlag::new(),
    )
    .await?;
    assert_eq!(std::fs::read(&dst)?, payload);

    // 取消:预置取消标志,下载应返回 Cancelled,不留下正式/临时文件。
    let cancel = linop_sftp::CancelFlag::new();
    cancel.cancel();
    let dst2 = local_dst.path().join("never.bin");
    let err = sftp
        .download_streaming("big.bin", &dst2, &mut |_, _, _| {}, &cancel)
        .await
        .unwrap_err();
    assert!(
        matches!(err, linop_sftp::SftpError::Cancelled),
        "应返回 Cancelled:{err}"
    );
    assert!(!dst2.exists(), "取消后不应有正式文件");
    assert!(
        !local_dst.path().join("never.bin.linop-part").exists(),
        "取消后不应残留临时文件"
    );

    // 目录递归流式:上传目录再下载回来。
    let dir_src = local_src.path().join("proj");
    std::fs::create_dir_all(&dir_src)?;
    std::fs::write(dir_src.join("a.txt"), b"aaa")?;
    std::fs::write(dir_src.join("b.txt"), b"bbb")?;
    sftp.upload_streaming(
        &dir_src,
        "proj",
        &mut |_, _, _| {},
        &linop_sftp::CancelFlag::new(),
    )
    .await?;
    let dir_dst = local_dst.path().join("proj");
    sftp.download_streaming(
        "proj",
        &dir_dst,
        &mut |_, _, _| {},
        &linop_sftp::CancelFlag::new(),
    )
    .await?;
    assert_eq!(std::fs::read(dir_dst.join("a.txt"))?, b"aaa");
    assert_eq!(std::fs::read(dir_dst.join("b.txt"))?, b"bbb");

    sftp.close().await?;
    Ok(())
}
