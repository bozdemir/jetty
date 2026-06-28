//! Native Wayland global summon shortcut via the XDG GlobalShortcuts portal
//! (`org.freedesktop.portal.GlobalShortcuts`).
//!
//! On X11 the summon hotkey is a direct key grab (the `global-hotkey` crate); on
//! Wayland a regular app cannot grab keys, so the freedesktop **portal** is the
//! native, desktop-environment-independent way to claim a global shortcut. This
//! module runs the portal session on a dedicated worker thread and forwards every
//! activation into the same [`AppEvent::ToggleVisibility`] the X11 grab and the
//! IPC toggle already use — so the toggle behaviour is identical across paths and
//! the event-loop side needs no special casing.
//!
//! Everything here is best-effort (CONTRIBUTING N3: "degrade gracefully, never
//! crash"): if there is no portal, or no GlobalShortcuts backend (an older
//! `xdg-desktop-portal`, or a compositor without one), the worker logs a single
//! line and exits, leaving the documented `jetty`-bound IPC toggle as the
//! fallback. It never panics and never touches the render path.
//!
//! Linux-only — the module is `#[cfg(target_os = "linux")]` at its declaration in
//! `lib.rs`. Off the hot path, no polling: the thread blocks on the `Activated`
//! D-Bus signal stream and only wakes the event loop on an actual activation
//! (same idle-CPU profile as the IPC accept thread and the `global-hotkey`
//! worker), so it cannot regress the 0-CPU idle.

use winit::event_loop::EventLoopProxy;

use crate::app::AppEvent;

/// Application-side id we register the shortcut under and match on activation.
/// A single shortcut is registered, so the match is mostly defensive.
const SHORTCUT_ID: &str = "toggle";

/// Spawn the portal worker thread. Returns immediately; the thread lives for the
/// program's lifetime (the activation loop only returns if the portal drops or the
/// event loop has exited). Called once per process — gated by the caller.
pub fn spawn(proxy: EventLoopProxy<AppEvent>) {
    let _ = std::thread::Builder::new()
        .name("jetty-portal-hotkey".to_string())
        .spawn(move || {
            // Drive the whole portal session + activation stream on this thread.
            // With the `async-io` feature, zbus runs its connection executor on
            // its own thread, so this `block_on` only has to advance our await
            // points — it parks (no busy-poll) between D-Bus messages.
            async_io::block_on(run(proxy));
        });
}

/// The async body: open the portal, create a session, bind an F9-preferred
/// shortcut, then forward each activation to the event loop until shutdown.
async fn run(proxy: EventLoopProxy<AppEvent>) {
    use ashpd::desktop::global_shortcuts::{GlobalShortcuts, NewShortcut};
    use futures_util::StreamExt;

    let shortcuts = match GlobalShortcuts::new().await {
        Ok(s) => s,
        Err(e) => {
            eprintln!(
                "jetty: GlobalShortcuts portal unavailable ({e}) — \
                 bind the `jetty` command to a key in your compositor instead"
            );
            return;
        }
    };

    let session = match shortcuts.create_session(Default::default()).await {
        Ok(s) => s,
        Err(e) => {
            eprintln!("jetty: GlobalShortcuts create_session failed ({e}); IPC toggle still works");
            return;
        }
    };

    // Register a single "toggle" shortcut, preferring F9 to match the X11 grab.
    // The portal/compositor owns the final binding and may surface it in the
    // desktop's shortcut settings for the user to confirm or rebind the first time.
    let shortcut = NewShortcut::new(SHORTCUT_ID, "Summon / hide JeTTY").preferred_trigger("F9");
    if let Err(e) = shortcuts
        .bind_shortcuts(&session, &[shortcut], None, Default::default())
        .await
    {
        eprintln!("jetty: GlobalShortcuts bind failed ({e}); IPC toggle still works");
        return;
    }

    let mut activated = match shortcuts.receive_activated().await {
        Ok(s) => s,
        Err(e) => {
            eprintln!("jetty: GlobalShortcuts activation stream failed ({e}); IPC toggle still works");
            return;
        }
    };

    eprintln!("jetty: GlobalShortcuts portal active (Wayland summon shortcut, preferred F9)");

    // Forward each activation to the event loop. A send error means the loop has
    // exited (app shutting down) → stop and let `session`/`shortcuts` drop. Holding
    // them for the lifetime of this loop keeps the portal session registered.
    while let Some(activation) = activated.next().await {
        if activation.shortcut_id() == SHORTCUT_ID
            && proxy.send_event(AppEvent::ToggleVisibility).is_err()
        {
            break;
        }
    }
    drop(session);
}
