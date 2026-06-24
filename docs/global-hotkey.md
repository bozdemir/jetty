# Global F9 Hotkey (Yakuake-style summon)

Jetty supports a global F9 hotkey to show/hide the window from anywhere on
the desktop — no need to click the taskbar or alt-tab.

## X11

On X11, Jetty automatically registers a system-wide F9 key grab at startup
using the `global-hotkey` crate. No configuration is needed.

Press F9 once to hide the window; press again to summon it, centred on the
current monitor with keyboard focus.

## Wayland

Global key grabs are not available to regular apps on Wayland. Instead, Jetty
provides a `--toggle` IPC mechanism: bind `jetty --toggle` (or just `jetty`)
to F9 in your compositor.

When a Jetty instance is already running, a second invocation of `jetty` (or
`jetty --toggle`) connects to the running instance via a Unix socket
(`$XDG_RUNTIME_DIR/jetty.sock`), sends a toggle message, and exits — so the
running window shows or hides instantly.

If no instance is running, `jetty --toggle` starts a fresh instance (so the
very first F9 press launches Jetty; subsequent presses toggle it).

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

## Notes

- The PTY (shell) keeps running while the window is hidden — nothing is killed.
- On X11, both mechanisms are active: the built-in grab AND the IPC socket.
  Either works; the hotkey grab is faster (no process fork).
- The socket is cleaned up on normal exit; stale sockets from crashes are
  automatically removed at next startup.
