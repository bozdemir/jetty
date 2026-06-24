use std::sync::Arc;
use raw_window_handle::{HasWindowHandle, RawWindowHandle};
use winit::dpi::LogicalSize;
use winit::event_loop::ActiveEventLoop;
use winit::window::{Icon, Window};

/// Ask KWin (KDE) to blur whatever shows through this window's transparent
/// pixels — the frosted-glass "KDE Plasma" look. Sets the
/// `_KDE_NET_WM_BLUR_BEHIND_REGION` property to an empty region, which means
/// "blur the entire window". It only has a visible effect when the background
/// is actually translucent (opacity < 100%) and the compositor supports it
/// (KDE/KWin on X11). On Wayland / other compositors / any failure it is a
/// silent no-op, so nothing breaks.
fn enable_kde_blur(window: &Window) {
    let xid: u32 = match window.window_handle().map(|h| h.as_raw()) {
        Ok(RawWindowHandle::Xlib(h)) => h.window as u32,
        Ok(RawWindowHandle::Xcb(h)) => h.window.get(),
        _ => return, // Wayland or non-X11 — KWin Wayland blur is a separate protocol.
    };
    let Ok((conn, _screen)) = x11rb::connect(None) else { return };
    use x11rb::connection::Connection as _;
    use x11rb::protocol::xproto::{AtomEnum, ConnectionExt as _, PropMode};
    use x11rb::wrapper::ConnectionExt as _;
    let Ok(cookie) = conn.intern_atom(false, b"_KDE_NET_WM_BLUR_BEHIND_REGION") else { return };
    let Ok(reply) = cookie.reply() else { return };
    // Empty CARDINAL array => blur the whole window.
    let empty: &[u32] = &[];
    let _ = conn.change_property32(PropMode::REPLACE, xid, reply.atom, AtomEnum::CARDINAL, empty);
    let _ = conn.flush();
}

/// Decode the embedded JeTTY app icon into a winit `Icon` (shown in the
/// taskbar / Alt-Tab / when minimized). The 256px RGBA PNG is baked into the
/// binary so there is nothing to install. Returns `None` if decoding fails,
/// in which case the window simply has no custom icon.
fn app_icon() -> Option<Icon> {
    let bytes: &[u8] = include_bytes!("../../../assets/icons/jetty-256.png");
    let decoder = png::Decoder::new(std::io::Cursor::new(bytes));
    let mut reader = decoder.read_info().ok()?;
    let mut buf = vec![0u8; reader.output_buffer_size()?];
    let info = reader.next_frame(&mut buf).ok()?;
    if info.color_type != png::ColorType::Rgba {
        return None;
    }
    buf.truncate(info.buffer_size());
    Icon::from_rgba(buf, info.width, info.height).ok()
}

/// Build the main window: a borderless (client-side decorations) window with
/// our custom titlebar + the JeTTY app icon.
pub fn build_window(event_loop: &ActiveEventLoop, title: &str, size: (u32, u32)) -> Arc<Window> {
    let attrs = Window::default_attributes()
        .with_title(title)
        .with_window_icon(app_icon())
        .with_inner_size(LogicalSize::new(size.0, size.1))
        .with_resizable(true)
        .with_min_inner_size(LogicalSize::new(200.0_f64, 120.0_f64))
        // Client-side decorations: drop the OS title bar/frame and draw our own
        // custom titlebar (min/max/close + drag) in the tab strip. Transparency
        // keeps the runtime opacity working and the rounded corners.
        .with_decorations(false)
        .with_transparent(true);
    let window = Arc::new(event_loop.create_window(attrs).expect("create_window failed"));
    // Frosted-glass blur behind the translucent terminal (KDE/X11).
    enable_kde_blur(&window);
    window
}

/// Build a fixed-size, non-resizable utility window (e.g. the settings dialog).
/// A normal decorated OS window the user can move anywhere; also carries the icon.
pub fn build_fixed_window(
    event_loop: &ActiveEventLoop,
    title: &str,
    size: (u32, u32),
) -> Arc<Window> {
    let attrs = Window::default_attributes()
        .with_title(title)
        .with_window_icon(app_icon())
        .with_inner_size(LogicalSize::new(size.0, size.1))
        .with_resizable(false);
    Arc::new(event_loop.create_window(attrs).expect("create_window failed"))
}
