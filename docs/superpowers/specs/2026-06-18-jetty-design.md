# Jetty — Design Spec (MVP: Core Summon)

**Date:** 2026-06-18
**Status:** Approved for spec review
**Author:** brainstormed with the user

---

## Özet (TR)

Jetty, Rust ile yazılmış, GPU hızlandırmalı, **ekran ortasında beliren** (Yakuake gibi yukarıdan açılan değil) hızlı bir terminal. Bir global kısayola basınca, Plasma-benzeri bir efektle (fade + scale-up + opsiyonel blur) ortada belirir; tekrar basınca / ESC ile gizlenir, süreç arka planda yaşar.

**Taşınabilirlik birinci sınıf hedef:** tek binary, **tüm Linux distroları**, **X11 + Wayland**, **masaüstü ortamından bağımsız** (KDE / GNOME / sway / Hyprland …). Bunu sağlamak için DE/display-server'a bağımlı her şey tek bir `PlatformBackend` soyutlamasının arkasındadır; efektleri KDE/KWin'e bırakmak yerine **kendimiz GPU'da çizeriz** (her yerde aynı görünür).

**Minimal başlar, büyür:** MVP = "çekirdek summon" (tek terminal, ortada, gerçek PTY, efektli beliriş). Mimari ilk günden sekme / sürükle-bırak / çoklu-monitör / Wayland backend'i temiz alacak şekilde kurgulanır (cargo workspace, ayrık crate sınırları).

---

## 1. Vision & Goals

Jetty is a summon-style, GPU-accelerated terminal that appears **centered** on the active monitor (not a top-edge dropdown like Yakuake), animated in with desktop-effect-like motion, dismissed back to the background, and re-summoned instantly via a global hotkey.

**Primary goals**
- **Blazing fast:** GPU rendering, low input latency, instant summon/dismiss.
- **Portable:** one binary across all Linux distros, X11 **and** Wayland, desktop-environment independent.
- **Effect-rich:** Plasma-like appear/disappear animation, owned by us (not delegated to any compositor) so it looks identical everywhere.
- **Built to grow:** clean module boundaries so tabs, drag-and-drop tab tear-off, and full multi-monitor support land without rework.

**Non-goals (for now)**
- Windows/macOS support (Linux first; the platform abstraction does not preclude it later).
- Being a full terminal-multiplexer replacement (tmux/SSH integrations are future, optional).

---

## 2. Requirements

### Functional (MVP)
- F1. A global hotkey toggles the terminal's visibility from anywhere.
- F2. On summon, the window appears **centered on the monitor under the cursor**.
- F3. Appearance is animated: fade-in + scale-from-95% + optional backdrop blur, ~120–150 ms ease-out; dismissal reverses it.
- F4. A real PTY runs the user's `$SHELL`; keystrokes go to the PTY; output renders correctly (colors, cursor, common VT sequences).
- F5. ESC or the hotkey hides the window; the shell/process keeps running in the background and is restored on next summon.
- F6. The window is borderless, always-on-top, and skip-taskbar.
- F7. Minimal TOML config: hotkey, font family/size, window size, opacity, effect on/off.

### Functional (later phases — see §14)
- Tabs (open/close/switch). Drag-and-drop tab reordering and tear-off to a new window. Full multi-monitor selection/movement. Wayland backend with layer-shell overlay. Theming, ligatures, scrollback UI.

### Non-functional
- N1. **Performance:** steady-state idle near-zero CPU; render only on damage/animation; vsync-paced; sub-frame input-to-PTY latency.
- N2. **Portability:** X11 and Wayland; DE-independent; no hard dependency on KDE/KWin or GNOME APIs.
- N3. **Robustness:** if the privileged/overlay path is unavailable on a given compositor, degrade gracefully (documented best-effort), never crash.
- N4. **Maintainability:** small, single-purpose crates with well-defined interfaces; pure logic unit-tested.

### Constraints / environment
- Target dev machine: KDE Plasma 5.27 on **X11**, dual 1920×1200 monitors, Intel + NVIDIA hybrid GPU. MVP is validated here first.
- Rust toolchain present (1.94+).

---

## 3. Approaches Considered

**A — `winit` (X11 backend) + `wgpu` + `alacritty_terminal`, custom renderer. ✅ CHOSEN**
Reuse the proven terminal core (`alacritty_terminal`: PTY, VT parser, grid model) as a library. Render text and effects ourselves with `wgpu`. Drive windowing/input/monitors through the platform layer (`winit` is the tool inside the X11 backend). Gives full control over the summon animation and overlay semantics — the heart of the product — while not reinventing VT handling.
*Cost:* we write the renderer ourselves.

**B — Higher-level GUI toolkit (egui / iced / Slint) embedding a terminal widget. ✗**
Tabs/drag-drop UI come easier, but control over the summon/effect animation, overlay window semantics, and Wayland layer-shell is weaker, and toolkit overhead fights the "blazing fast" goal. Embedding a real PTY-backed terminal is awkward. May borrow ideas for the tab bar later, not for the core.

**C — Fork an existing terminal app (Alacritty / wezterm). ✗**
Instant fast terminal, but bending an existing app into center-overlay + our own effects + our architecture is heavy, and we inherit its windowing decisions. Rejected as an app — but its *library*, `alacritty_terminal`, is exactly what Approach A reuses.

**Decision: A.**

---

## 4. Architecture

A cargo **workspace** with single-purpose crates. MVP does not fill every crate, but the boundaries exist from day one so growth is clean ("yapıyı öyle kurgula").

```
jetty/                      (cargo workspace)
├─ crates/
│  ├─ jetty-core/      Terminal session: wraps alacritty_terminal, owns the PTY,
│  │                   exposes the grid/cursor state and an input sink. No I/O of its own
│  │                   beyond the PTY. Pure-ish, unit-testable.
│  ├─ jetty-render/    wgpu renderer: glyph atlas (monospace), grid draw pass, and
│  │                   effect passes (fade / scale / blur). Backend-agnostic: consumes a
│  │                   raw-window-handle surface; knows nothing about X11/Wayland.
│  ├─ jetty-platform/  PlatformBackend trait + x11/ (MVP) and wayland/ (later) impls.
│  │                   Owns: overlay surface creation, global hotkey, monitor enumeration,
│  │                   window flags, event pumping. The ONLY crate that touches display-server APIs.
│  ├─ jetty-ipc/       Unix-socket control plane: `jetty toggle` talks to the running
│  │                   instance. Universal summon fallback (works on every compositor).
│  └─ jetty-app/       Orchestration: the event loop, the visibility state machine,
│                       config loading, and wiring core↔render↔platform↔ipc together.
└─ jetty (bin)         Thin main → jetty-app::run(); also dispatches `jetty toggle` to IPC.
```

**Dependency direction:** `jetty-app` depends on the other crates; `jetty-render`, `jetty-core`, `jetty-platform`, `jetty-ipc` do **not** depend on each other (they meet only in `jetty-app`). The renderer depends only on a raw-window-handle + the grid snapshot; the platform layer depends only on display-server APIs. This keeps each crate understandable and testable in isolation.

**Key architectural decision — windowing lives *inside* the platform backend, not above it.**
`wgpu` renders into whatever surface the active `PlatformBackend` hands it (via `raw-window-handle`). For the **X11** backend, `winit` is the implementation detail that creates the window, pumps input, and enumerates monitors. For the future **Wayland** backend, the implementation may bypass `winit` and use `smithay-client-toolkit` + `wlr-layer-shell` to get a true centered, always-on-top overlay surface (supported by KWin and wlroots compositors; GNOME-Mutter lacks layer-shell → best-effort xdg-toplevel). Because the renderer and app depend only on the trait + a raw handle, swapping backends never touches rendering or core logic. This is a deliberate refinement of "use winit for both": winit is *an* X11 backend tool, not a global dependency — so the dual-backend, DE-independent goal holds.

---

## 5. The `PlatformBackend` Trait (design sketch)

Everything that differs across display server / DE sits behind this. Sketch (not final API):

```rust
pub trait PlatformBackend {
    /// Create the borderless overlay surface; return a handle wgpu can render into.
    fn create_overlay(&mut self, cfg: &WindowCfg) -> Result<RawSurface>;

    /// Register the global summon hotkey; presses are delivered via pump_events.
    fn register_hotkey(&mut self, spec: HotkeySpec) -> Result<()>;

    /// Monitor geometry, scale factor, and which one holds the cursor.
    fn monitors(&self) -> Vec<MonitorInfo>;

    /// Show centered on `monitor` with the given flags (always-on-top, skip-taskbar…).
    fn show_centered(&mut self, monitor: MonitorId, flags: WindowFlags);

    /// Hide the surface; the process stays alive.
    fn hide(&mut self);

    /// Drain platform events (key/text input, resize, monitor change, hotkey) into the sink.
    fn pump_events(&mut self, sink: &mut dyn PlatformEventSink);
}
```

**Summon hotkey portability matrix**
- X11: `XGrabKey` — universal across WMs/DEs. (MVP path.)
- Wayland: XDG `org.freedesktop.portal.GlobalShortcuts` where supported (KDE, GNOME 45+, …).
- **Universal fallback (all compositors):** `jetty-ipc` exposes a unix socket; `jetty toggle` signals the running instance. The user binds a key to `jetty toggle` in their own DE settings. Works literally everywhere; documented as the guaranteed path when no global-grab/portal exists.

---

## 6. Rendering & Effects (`jetty-render`)

- **Text:** build a monospace glyph atlas (rasterize via `swash`/`cosmic-text`-style path), upload to a GPU texture, draw the grid as instanced quads sampling the atlas. Redraw only on damage or during animation (N1).
- **Effects (owned by us, compositor-independent):**
  - *Summon:* opacity 0→1 and scale 0.95→1.0 over ~120–150 ms, ease-out; optional backdrop blur (sample a captured/blurred backdrop or a translucent dim) behind the surface.
  - *Dismiss:* the reverse.
  - Driven by an animation clock in `jetty-app`; the renderer just receives a `t∈[0,1]` and effect parameters.
- **Transparency:** request an alpha-capable surface so opacity/blur read correctly over the desktop (X11: ARGB visual; Wayland: alpha is native).

---

## 7. Terminal Core (`jetty-core`)

- Wrap `alacritty_terminal`: spawn the PTY for `$SHELL`, feed bytes into its VT parser, own the `Term`/grid model, expose a read-only **grid snapshot** (cells, colors, cursor) for the renderer and an **input sink** for keystrokes.
- Background thread reads PTY output; the parser updates the grid; the app is notified of damage to schedule a redraw.
- No display-server knowledge here — it is headless and unit-testable (feed bytes, assert grid state).

---

## 8. Visibility State Machine (`jetty-app`)

```
Hidden ──summon──▶ Summoning ──anim done──▶ Visible
  ▲                                            │
  └──anim done── Dismissing ◀──ESC / hotkey────┘
```

- `Summoning` / `Dismissing` advance an animation clock each frame; the renderer is fed `t`.
- The PTY/shell lives across `Hidden` — never killed on hide; restored on next summon.
- Hotkey and `jetty toggle` (IPC) both drive the same transitions.

---

## 9. Config

Minimal TOML at `$XDG_CONFIG_HOME/jetty/jetty.toml`:

```toml
hotkey   = "ctrl+`"      # parsed by jetty-platform
font     = "monospace"
size     = 13
width    = 1000          # logical px (or fraction of monitor in a later phase)
height   = 640
opacity  = 0.95
effects  = true          # summon/dismiss animation on/off
```

Config parsing is pure and unit-tested (valid/invalid/missing-field cases).

---

## 10. IPC / Universal Summon (`jetty-ipc`)

- A unix domain socket under `$XDG_RUNTIME_DIR/jetty.sock`.
- `jetty toggle` (and later `show`/`hide`/`new-tab`) connects and sends a one-line command; the running instance applies it.
- Doubles as the **single-instance guard** (second launch without a subcommand focuses/toggles the existing one).
- Protocol is a tiny line-based format, unit-tested independent of any display server.

---

## 11. MVP Scope (precise boundary)

**In:** single window · single terminal · monitor under the cursor · global hotkey toggle (X11 `XGrabKey`) · summon/dismiss effect (fade + scale, optional blur) · real PTY running `$SHELL` · keyboard→PTY with color/cursor rendering via `alacritty_terminal` · ESC/hotkey hides (process persists) · borderless + always-on-top + skip-taskbar + centered · minimal TOML config · IPC `toggle` + single-instance guard.

**Out (later phases):** tabs · drag-and-drop tab reorder/tear-off · full multi-monitor selection/movement · Wayland backend implementation · portal-based hotkey · theming/ligatures/scrollback UI · non-Linux platforms.

---

## 12. Build Sequence (each milestone independently runnable)

1. **M0 — Skeleton & loop.** Cargo workspace; `jetty-platform` X11 backend opens a `winit` window; `jetty-render` clears the screen with `wgpu`; vsync-paced loop, redraw-on-damage. *Proves the render/loop/backend boundary.*
2. **M1 — Working terminal (normal window).** Wire `jetty-core` (`alacritty_terminal` + PTY); render the grid via the glyph atlas; keyboard→PTY. *A real terminal in an ordinary window.*
3. **M2 — Summon behavior.** Borderless + centered + always-on-top + skip-taskbar; global hotkey (X11) and `jetty toggle` (IPC) toggle visibility; ESC hides; process persists. *Now a summon terminal.*
4. **M3 — Effects & config.** Summon/dismiss animation (fade + scale, optional blur); TOML config. **→ MVP complete.**

Then: Phase 2 (tabs) → Phase 3 (drag-and-drop) → Phase 4 (Wayland backend) → … (see §14).

---

## 13. Testing Strategy

- **TDD for pure logic:** config parsing, IPC line protocol, the visibility state-machine transitions, and `jetty-core` grid updates (feed VT bytes, assert grid). Headless, fast.
- **Platform layer behind the trait:** mock `PlatformBackend` to test app orchestration without a display server.
- **Render & real platform:** verified by running the app (M0–M3 are each runnable checkpoints) and visual confirmation; later, golden-image/snapshot tests for the renderer if useful.

---

## 14. Growth Roadmap (beyond MVP)

- **Phase 2 — Tabs:** tab model in `jetty-core` (multiple sessions), tab bar in `jetty-render`, keybindings (new/close/next/prev). Each tab is an independent PTY session.
- **Phase 3 — Drag & drop:** reorder tabs; tear a tab off into a new window; drop to merge windows. Hit-testing + drag state in `jetty-app`/`jetty-render`.
- **Phase 4 — Wayland backend:** `wayland/` impl of `PlatformBackend` using `smithay-client-toolkit` + `wlr-layer-shell` (true centered overlay on KWin/wlroots; best-effort xdg-toplevel + portal hotkey on GNOME-Mutter). No changes to renderer/core.
- **Phase 5 — Full multi-monitor:** choose summon monitor (cursor/active/configured), move window between monitors, per-monitor DPI.
- **Later (optional):** theming, ligatures, scrollback search, splits, SSH/tmux conveniences, non-Linux backends.

---

## 15. Risks & Open Questions

- **R1 — Wayland overlay fidelity on GNOME-Mutter:** no `wlr-layer-shell`, so centered/always-on-top is best-effort there. *Mitigation:* document it; the IPC `toggle` summon path still works everywhere. (Deferred to Phase 4.)
- **R2 — Event-loop integration across backends:** winit owns its loop on X11; a smithay-based Wayland backend has a different loop. *Mitigation:* the trait exposes `pump_events`; validate the boundary during M2 so the Phase-4 Wayland backend slots in without an app rewrite.
- **R3 — Hybrid GPU (Intel + NVIDIA):** ensure `wgpu` picks a working adapter; allow adapter override via env/config if needed.
- **R4 — Glyph/atlas approach:** start monospace-only with a simple atlas; revisit for fallback fonts/ligatures in a later phase.

---

*Next step after approval: write the implementation plan (writing-plans skill), starting from M0.*
