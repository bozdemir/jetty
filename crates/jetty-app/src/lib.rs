mod anim;
mod app;
mod config;
pub mod clipboard;
pub mod input;

use app::AppEvent;
use winit::event_loop::{ControlFlow, EventLoop};

/// Unix-socket path used for single-instance IPC. Any running primary Jetty
/// instance listens here; secondary invocations (including `jetty --toggle`)
/// connect and send a toggle message, then exit immediately.
fn ipc_socket_path() -> String {
    let runtime_dir = std::env::var("XDG_RUNTIME_DIR").unwrap_or_else(|_| "/tmp".into());
    format!("{}/jetty.sock", runtime_dir)
}

/// Try to connect to a live Jetty instance and send a toggle message.
/// Returns `true` if a live instance was found and the message was sent
/// (i.e. this process should exit).
fn try_toggle_running_instance(sock_path: &str) -> bool {
    use std::io::Write;
    use std::os::unix::net::UnixStream;

    if let Ok(mut stream) = UnixStream::connect(sock_path) {
        let _ = stream.write_all(b"toggle");
        let _ = stream.flush();
        return true;
    }
    false
}

pub fn run() {
    // One-line build banner so a user can confirm which build they are running.
    // JETTY_BUILD is injected at compile time (e.g. the git short SHA via
    // `JETTY_BUILD=$(git rev-parse --short HEAD) cargo build`); falls back to
    // "dev" for plain local builds.
    eprintln!(
        "jetty {} ({})",
        env!("CARGO_PKG_VERSION"),
        option_env!("JETTY_BUILD").unwrap_or("dev")
    );

    let sock_path = ipc_socket_path();

    // Single-instance: always attempt to forward a toggle to a running instance.
    // This means both `jetty` and `jetty --toggle` behave the same way when an
    // instance is already running — toggle that instance and exit.
    // On the very first launch there is no live instance, so we fall through to
    // becoming the primary.
    if try_toggle_running_instance(&sock_path) {
        std::process::exit(0);
    }

    // No live instance found — become the primary.
    // Remove any stale socket file left over from a previous crash, then bind.
    std::fs::remove_file(&sock_path).ok();
    let listener: Option<std::os::unix::net::UnixListener> =
        match std::os::unix::net::UnixListener::bind(&sock_path) {
            Ok(l) => {
                eprintln!("jetty: IPC socket bound at {sock_path}");
                Some(l)
            }
            Err(e) => {
                eprintln!("jetty: could not bind IPC socket at {sock_path}: {e} — single-instance IPC disabled");
                None
            }
        };

    let event_loop = EventLoop::<AppEvent>::with_user_event().build().expect("event loop");
    event_loop.set_control_flow(ControlFlow::Wait);
    let proxy = event_loop.create_proxy();

    // Spawn the IPC accept thread (primary only). Any incoming connection
    // triggers a ToggleVisibility event regardless of message content.
    // This gives Wayland users a working toggle via `jetty --toggle` bound to
    // F9 in their compositor settings, sharing the same code path as the X11
    // global-hotkey grab.
    if let Some(listener) = listener {
        let proxy_ipc = proxy.clone();
        let sock_cleanup = sock_path.clone();
        std::thread::spawn(move || {
            for stream in listener.incoming() {
                if let Ok(mut s) = stream {
                    let mut buf = [0u8; 16];
                    let _ = std::io::Read::read(&mut s, &mut buf);
                    // Any incoming message means "toggle". If the proxy is gone
                    // (event loop exited), stop listening.
                    if proxy_ipc.send_event(AppEvent::ToggleVisibility).is_err() {
                        break;
                    }
                }
            }
            // Clean up the socket when the accept loop ends.
            let _ = std::fs::remove_file(&sock_cleanup);
        });
    }

    let mut app = app::App::new(proxy);
    event_loop.run_app(&mut app).expect("run_app");

    // Best-effort cleanup on normal exit. Crashes are handled by the
    // remove-stale-on-bind logic at the start of the next launch.
    let _ = std::fs::remove_file(&sock_path);
}
