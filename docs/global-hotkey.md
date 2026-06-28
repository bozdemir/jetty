# Global F9 Hotkey (Yakuake-style summon)

Jetty supports a global F9 hotkey to show/hide the window from anywhere on
the desktop — no need to click the taskbar or alt-tab.

## X11

On X11, Jetty automatically registers a system-wide F9 key grab at startup
using the `global-hotkey` crate. No configuration is needed.

F9 is a toggle: press it to hide the window, press it again to summon it. On
summon the window is re-centred on the current monitor (or re-docked to the top
in Dropdown mode), takes keyboard focus, and replays the reveal effect.
(Jetty launches visible, so the first F9 press after startup hides it.)

## Wayland

Old-style global key grabs (`XGrabKey`) are not available to regular apps on
Wayland. Jetty handles this in two layers — a **native portal shortcut** first,
with the **IPC toggle** as a universal fallback.

### Native: the XDG GlobalShortcuts portal (preferred)

On a Wayland session Jetty registers a global shortcut through the freedesktop
`org.freedesktop.portal.GlobalShortcuts` portal — the standard, desktop-
environment-independent way for an unprivileged app to claim a system-wide key.
No `jetty` re-launch and no manual binding script is needed; the shortcut is
delivered straight to the running instance over D-Bus.

The preferred trigger is **F9**, but the **compositor owns the final binding**:
the first time Jetty registers, your desktop may show the shortcut in its
keyboard-settings UI for you to confirm or rebind (KDE: System Settings →
Shortcuts; GNOME: Settings → Keyboard). After that it just works, globally.

This path needs `xdg-desktop-portal` plus a backend that implements the
GlobalShortcuts interface — e.g. **KDE Plasma** (`xdg-desktop-portal-kde`),
**GNOME 45+** (`xdg-desktop-portal-gnome`), or **Hyprland**
(`xdg-desktop-portal-hyprland`). If the portal or a GlobalShortcuts backend is
missing, Jetty logs one line at startup and falls back to the IPC toggle below —
it never fails hard.

> Window **positioning** is a separate matter: in Dropdown mode, top-edge
> anchoring relies on the app placing its own window, which Wayland does not
> permit for ordinary `xdg-toplevel` surfaces (it needs `wlr-layer-shell`, not
> yet implemented — see the roadmap). The summon **shortcut** above works
> regardless; the dropdown **dock geometry** is still X11-only for now.

### Fallback: single-instance IPC toggle

When the portal is unavailable, bind the `jetty` command to a key in your
compositor. When a Jetty instance is already running, launching `jetty` again
connects to it over a Unix socket (`$XDG_RUNTIME_DIR/jetty.sock`, falling back to
`/tmp/jetty.sock` if `XDG_RUNTIME_DIR` is unset), sends a toggle message, and
exits immediately — so the running window shows or hides instantly.

If no instance is running, `jetty` starts a fresh instance (so the first key
press launches Jetty; subsequent presses toggle it).

Note: Jetty does not currently parse command-line flags. `jetty --toggle` and
plain `jetty` behave identically (extra arguments are ignored); use plain
`jetty` in your binding.

### KDE Plasma (Wayland)

System Settings → Shortcuts → Custom Shortcuts → New → Global Shortcut →
Command: `jetty`, Trigger: F9

### GNOME (Wayland)

Settings → Keyboard → View and Customize Shortcuts → Custom Shortcuts →
Add shortcut, Command: `jetty`, Shortcut: F9

### Sway / i3 (Wayland/X11)

```
bindsym F9 exec jetty
```

### Hyprland

```
bind = , F9, exec, jetty
```

## macOS

The global hotkey is plain **F9** (no `fn` modifier is added by Jetty;
registration is `HotKey::new(None, Code::F9)`). On a Mac keyboard where the
function-row keys default to media actions, press `fn`+`F9` so the OS delivers
F9, or enable "Use F1, F2, etc. keys as standard function keys" in
System Settings → Keyboard.

macOS requires Jetty to be granted Accessibility (and on some versions Input
Monitoring) permission before a system-wide key tap is delivered: System
Settings → Privacy & Security → Accessibility → enable Jetty. Without this the
F9 grab is silently inactive; the IPC `jetty` toggle still works as a fallback
(bind `jetty` to a shortcut via a launcher).

Known limitation: macOS global-hotkey support is best-effort (the manager is
registered off the main thread, which upstream documents as fragile on macOS).
If F9 does not toggle, bind the `jetty` command to a shortcut via a launcher,
as on Wayland.

## Notes

- The PTY (shell) keeps running while the window is hidden — nothing is killed.
- On X11, both mechanisms are active: the built-in grab AND the IPC socket.
  Either works; the hotkey grab is faster (no process fork).
- The socket is cleaned up on normal exit; stale sockets from crashes are
  automatically removed at next startup.
- The built-in global grab uses the `global-hotkey` crate, which supports a
  system-wide grab on X11, macOS, and Windows. On Wayland the crate cannot
  register a grab; instead Jetty uses the **XDG GlobalShortcuts portal** for a
  native global shortcut, with the compositor-binding + IPC toggle as the
  fallback when no portal backend is present. (Jetty targets Linux and macOS;
  Windows is untested.)
- The portal worker runs on its own thread and only wakes the event loop when the
  shortcut actually fires (it blocks on a D-Bus signal — no polling), so it does
  not affect the ~0% idle CPU.
