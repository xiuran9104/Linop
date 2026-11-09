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
    // 连接必须存活以维持选择器所有权;有意泄漏到进程结束。
    std::mem::forget(conn);
    let _ = LEASED.set(());
    tracing::info!("已通告伪合成器(_NET_WM_CM_S{screen}):gpui 将使用客户端装饰(CSD 单窗)");
}
