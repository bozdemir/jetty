# Changelog

All notable changes to JeTTY are documented in this file.

The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

---

## [0.1.0] — 2026-06-26

First public release of JeTTY — a blazing-fast, GPU-accelerated terminal with a
center-summon / Yakuake-style dropdown hotkey.

### Added

**Core terminal**
- True-color VT100/VT220 emulation via `alacritty_terminal`; answers host queries
  (DSR/DA), proper `TERM=xterm-256color`, 10k-line scrollback.
- PTY fork + drain loop; `Ctrl+D` closes the shell cleanly.
- Window resize with grid reflow; terminal tracks physical pixel size changes.

**GPU rendering**
- Full `wgpu`-based render pipeline (Vulkan on Linux, Metal on macOS).
- Text rendering via `glyphon` / `cosmic-text`; live font family + size at runtime.
- Sub-millisecond grid snapshot (0.047 ms / frame at full screen).
- Damage-driven redraw — idle CPU is genuinely 0% (no polling, no cursor-blink timer).
- `jetty-bench` headless benchmark for reproducible perf measurements.

**Summon hotkey & window modes**
- Global F9 hotkey via `global-hotkey` crate (X11 native grab; IPC socket fallback
  for Wayland and macOS).
- Single-instance IPC socket (`$XDG_RUNTIME_DIR/jetty.sock`, fallback `/tmp/jetty.sock`);
  subsequent `jetty` invocations toggle the running instance.
- **Center mode** — drops into the middle of the current monitor.
- **Dropdown mode** — slides down from the top edge, full screen width (Yakuake/Guake
  style), with adjustable width & height percentage.
- Five summon reveal shaders: **Phosphor Ignition** (default), **Bayer Crystallize**,
  **Liquid Drop**, **Focus Pull**, **None**.

**Tabs**
- `Ctrl+Shift+T` new tab, `Ctrl+Shift+W` close (with confirm dialog), `Ctrl+Tab` /
  `Ctrl+1–9` switch, double-click to rename.
- Per-tab PTY; closing the last tab exits the app.

**Themes (5)**
- Catppuccin Mocha (default), Tokyo Night, Gruvbox Dark, Dracula, Onyx.
- Exact community palettes; every UI surface (panel, menus, tab bar, welcome,
  confirm dialogs, help overlay) re-skins with the active theme.

**Settings dialog**
- `Ctrl+Shift+P` opens a movable settings panel; persisted to
  `~/.config/jetty/config.toml`.
- Controls: theme, opacity, corner radius, summon effect, window mode, dropdown
  size, tab-bar position, focus auto-hide, welcome splash, performance HUD, font
  family + size.

**Live performance HUD**
- Optional tab-bar overlay: `⚡ <ms> ms · <fps> fps · <cpu>% · <mb> MB/s`.
- Idle one-shot: fires once after settling, displays `⚡ idle · 0% CPU · 0 MB/s`,
  then sleeps. Never regresses idle CPU.

**Welcome overlay**
- Neofetch-style splash on first launch; dismissed on first key/click/Esc.
- Toggle with `show_welcome` in config.

**Selection & clipboard**
- Left-drag to select (auto-copies), right-click context menu (Copy / Paste /
  Select All / Clear / Close Tab), `Ctrl+Shift+C/V`, middle-click paste,
  bracketed-paste aware.

**Custom window chrome**
- Borderless client-side decorations, our own title bar, rounded corners (radius
  slider), runtime opacity, focus auto-hide.

**Packaging & distribution (Linux x86_64)**
- `cargo build --release` produces a self-contained binary.
- `install.sh` one-line installer with SHA256 checksum verification; supports
  `JETTY_PREFIX` for system-wide installs; writes absolute `Exec=` path in the
  installed `.desktop`.
- `.deb` via `cargo-deb`, AppImage via `linuxdeploy`; CI publishes all artifacts
  on `v*` tags.

**macOS (Metal)**
- Full feature parity on macOS (Metal backend); builds from source without extra
  system packages.

### Known issues

- **Resize/p10k prompt scatter** — resizing the window or changing font size can
  scatter p10k's prompt into fragments. Debounce (`RESIZE_DEBOUNCE_MS`) mitigates
  but does not fully fix this; root cause is alacritty_terminal's reflow interacting
  with complex prompt escape sequences. Investigation ongoing.
- **Wayland: no native global shortcut** — the XDG GlobalShortcuts portal is not
  yet implemented; use the compositor binding + IPC socket workaround described in
  `docs/global-hotkey.md`.
- **macOS: no prebuilt binary** — macOS users must build from source. A prebuilt
  `.app`/`.dmg` is on the roadmap.

---

[0.1.0]: https://github.com/bozdemir/JeTTY/releases/tag/v0.1.0
