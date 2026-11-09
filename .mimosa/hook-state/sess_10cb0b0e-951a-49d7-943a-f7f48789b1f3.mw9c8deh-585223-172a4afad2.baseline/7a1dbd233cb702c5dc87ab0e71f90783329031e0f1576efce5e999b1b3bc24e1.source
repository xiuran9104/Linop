//! WSLg/X11 窗口装饰选择。
//!
//! gpui X11 平台以 `_NET_WM_CM_S{screen}` 选择器的持有者判断是否存在合成器;
//! WSLg 的 Weston(XWayland)不持有该选择器 → gpui 判定"无合成器"并回退
//! **服务端装饰**,WM(Weston XWM)给每个窗口套一层装饰外壳窗(即用户看到的
//! 外层无名框窗,四周 ~38px 空白)。
//!
//! 这里在 gpui 初始化前抢先持有该选择器,使 gpui 走**客户端装饰**(CSD,
//! 单窗、无外壳);窗口拖动/缩放由 gpui 的 CSD 边缘/标题区处理,移动仍经
//! `_NET_WM_MOVERESIZE` 交给 WM 执行。`LINOP_X11_SSD=1` 可回退。

#[cfg(target_os = "linux")]
pub fn announce_fake_compositor() {
    use x11rb::connection::{Connection as _, RequestConnection as _};
    use std::sync::OnceLock;
    static LEASED: OnceLock<()> = OnceLock::new();
    if LEASED.get().is_some() {
        return;
    }
    if std::env::var("LINOP_X11_SSD").ok().as_deref() == Some("1") {
        tracing::info!("LINOP_X11_SSD=1:保留 X11 服务端窗口装饰");
        return;
    }
    let Ok((conn, screen)) = x11rb::rust_connection::RustConnection::connect(None) else {
        return;
    };
    let root = match conn.setup().roots.get(screen) {
        Some(root) => root.root,
        None => return,
    };
    let Ok(atom_reply) =
        conn.intern_atom(false, format!("_NET_WM_CM_S{screen}").as_bytes())
    else {
        return;
    };
    let atom = match atom_reply.reply() {
        Ok(reply) => reply.atom,
        Err(_) => return,
    };
    let Ok(win) = conn.generate_id() else {
        return;
    };
    use x11rb::protocol::xproto::*;
    if conn
        .create_window(
            x11rb::COPY_FROM_PARENT as u8,
            win,
            root,
            -1,
            -1,
            1,
            1,
            0,
            WindowClass::INPUT_OUTPUT,
            0,
            &CreateWindowAux::new()
                .override_redirect(1)
                .event_mask(EventMask::STRUCTURE_NOTIFY),
        )
        .is_err()
    {
        return;
    }
    if conn.set_selection_owner(win, atom, x11rb::CURRENT_TIME).is_err() {
        return;
    }
    // gpui 还要求 root 的 _NET_SUPPORTED 通告 _GTK_FRAME_EXTENTS,
    // 否则 CSD 判定不支持而回退服务端装饰(外壳窗)。WSLg Weston 不通告,
    // 这里代为写入(保留既有列表追加)。
    let Ok(net_supported) = conn
        .intern_atom(false, "_NET_SUPPORTED".as_bytes())
        .map(|r| r.reply().map(|reply| reply.atom))
        .map(|inner| inner.unwrap_or(0))
    else {
        return;
    };
    let Ok(gtk_frame_extents) = conn
        .intern_atom(false, "_GTK_FRAME_EXTENTS".as_bytes())
        .map(|r| r.reply().map(|reply| reply.atom))
        .map(|inner| inner.unwrap_or(0))
    else {
        return;
    };
    if net_supported != 0 && gtk_frame_extents != 0 {
        let mut supported: Vec<u32> = conn
            .get_property(false, root, net_supported, AtomEnum::ATOM, 0, 1024)
            .ok()
            .and_then(|cookie| cookie.reply().ok())
            .map(|reply| {
                reply
                    .value
                    .chunks_exact(4)
                    .filter_map(|c| c.try_into().ok().map(u32::from_ne_bytes))
                    .collect()
            })
            .unwrap_or_default();
        if !supported.contains(&gtk_frame_extents) {
            supported.push(gtk_frame_extents);
            let bytes: Vec<u8> = supported
                .iter()
                .flat_map(|a| a.to_ne_bytes())
                .collect();
            let _ = conn.change_property(
                PropMode::REPLACE,
                root,
                net_supported,
                AtomEnum::ATOM,
                32,
                supported.len() as u32,
                &bytes,
            );
        }
    }
    // 连接必须存活以维持选择器所有权;有意泄漏到进程结束。
    std::mem::forget(conn);
    let _ = LEASED.set(());
    tracing::info!("已通告伪合成器(_NET_WM_CM_S{screen}):gpui 将使用客户端装饰(CSD 单窗)");
}
