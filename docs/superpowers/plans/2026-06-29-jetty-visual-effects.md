# JeTTY Visual Effects Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add parametric, polished visual effects to JeTTY — an animated CRT post-process (curvature, shadow-mask, scanlines, bloom, chromatic aberration, vignette, flicker, jitter), a keypress caret effect (CPU flash+pulse + optional GPU glow/ripple), a scrollable Settings "Effects" tab, and a fix for the missing macOS app icon.

**Architecture:** A new single-pass `Crt` post-effect (modeled on `phosphor.rs`) samples a persistent offscreen texture and composites to the surface, doing its own rounded-corner SDF alpha. All effect params live in one `EffectsConfig` struct, persisted to `config.toml`, mirrored on `App`, and fed as GPU uniforms each frame. Every effect defaults OFF (except the cheap caret flash+pulse) so the 0-CPU idle invariant is preserved; animated CRT sub-features are independent toggles that opt into continuous redraw.

**Tech Stack:** Rust, wgpu 29, WGSL (embedded `const &str`), glyphon (text), serde + toml (config), winit 0.30, custom hand-built GPU settings panel.

## Global Constraints

- GPU API: **wgpu 29 + WGSL** only; shaders embedded as `const &str`, compiled at runtime. No WGSL `vec3` in uniforms (host/GPU layout parity — pad to `vec4`/scalars; see `phosphor.rs:18`).
- Surface format **sRGB `Rgba8UnormSrgb`**; all blend math in linear space.
- Alpha mode **PostMultiplied on macOS/Metal, PreMultiplied on Vulkan** — route through `gpu.premultiply_clear` (`app.rs:3946`); any new compositing pass must preserve the transparent rounded window corners (replicate the `cov` SDF gate at `phosphor.rs:74`).
- **0-CPU idle is sacred:** idle (no animation active) must schedule zero redraws. All effects default OFF except `caret_flash_enabled=true`. Animated CRT toggles (roll/flicker/jitter) and active caret bursts are the only things allowed to drive continuous/bursty redraws, and only while enabled.
- Config defaults must keep **out-of-box look & behavior unchanged**; old `config.toml` (no `[effects]` table) loads with all defaults via `#[serde(default)]`.
- Follow existing panel/input/config patterns exactly (opacity slider, focus-autohide toggle pill, summon `‹/›` cycler, `#[serde(default = "...")]`).
- Settings panel fixed size `PANEL_W=380`, `PANEL_H=560` (`panel.rs:146-147`); the Effects tab **scrolls** within that height (no window growth).
- Branch: `feat/visual-effects`. Commit after every task.

---

## File Structure

**New files**
- `crates/jetty-render/src/crt.rs` — `Crt` post-effect: struct, embedded WGSL, `new`, `apply`.
- `crates/jetty-render/src/caret_fx.rs` — `CaretFx` optional caret glow/ripple additive pass.

**Modified files**
- `crates/jetty-app/src/config.rs` — `EffectsConfig` struct + `Config.effects` + defaults + clamps.
- `crates/jetty-app/src/app.rs` — runtime `fx` mirror, `crt`/`caret_fx` fields, `crt_clock`, `caret_anim`, `effects_scroll`; offscreen routing; CRT/caret dispatch; redraw guard; keypress trigger; `persist`; `handle_settings_action`; wheel→scroll; `min(3)→min(4)`.
- `crates/jetty-render/src/panel.rs` — 5th tab, Effects layout, RGB triples, scroll content-height + scissor.
- `crates/jetty-app/src/input.rs` — new `MouseAction`s + scroll-aware Effects hit-tests.
- `crates/jetty-render/src/text.rs` — caret flash+pulse cursor modulation.
- `crates/jetty-render/src/lib.rs` — export `Crt`, `CaretFx`.
- `scripts/make-macos-app.sh` — hardening + version injection.
- `README.md` — macOS bundle/run instructions.

**Execution ordering:** Tasks 1–12 are largely **serial** because they share `app.rs`/`panel.rs`/`input.rs` — run fresh subagent per task with review between, NOT parallel worktrees. **Task 13 (macOS icon)** is fully independent and may run at any time / in parallel.

---

## Task 1: `EffectsConfig` data model + defaults + clamps

**Files:**
- Modify: `crates/jetty-app/src/config.rs`
- Test: inline `#[cfg(test)]` in `config.rs`

**Interfaces:**
- Produces: `pub struct EffectsConfig { crt_enabled: bool, crt_curvature: f32, crt_scanline: f32, crt_mask: f32, crt_bloom: f32, crt_chromatic: f32, crt_vignette: f32, crt_scanline_tint: [f32;3], crt_animate_roll: bool, crt_flicker: bool, crt_jitter: bool, caret_flash_enabled: bool, caret_glow_enabled: bool, caret_flash_ms: f32, caret_flash_color: [f32;3] }`, `Config.effects: EffectsConfig`, `EffectsConfig::clamped(self) -> EffectsConfig`.

- [ ] **Step 1: Write the failing test**

Add to `config.rs` test module:
```rust
#[test]
fn effects_defaults_are_off_except_caret_flash() {
    let e = EffectsConfig::default();
    assert!(!e.crt_enabled);
    assert!(!e.crt_animate_roll && !e.crt_flicker && !e.crt_jitter);
    assert!(e.caret_flash_enabled);      // the one ON-by-default effect
    assert!(!e.caret_glow_enabled);
    assert_eq!(e.crt_scanline_tint, [1.0, 1.0, 1.0]);
}

#[test]
fn old_config_without_effects_table_loads_with_defaults() {
    // a config TOML predating the effects feature
    let toml = r#"theme = "default"
opacity = 1.0
font_size = 14.0
font_family = "monospace"
corner_radius = 8.0
"#;
    let cfg: Config = toml::from_str(toml).expect("must load");
    assert_eq!(cfg.effects, EffectsConfig::default());
}

#[test]
fn effects_clamp_out_of_range() {
    let e = EffectsConfig { crt_curvature: 9.0, crt_bloom: -1.0, caret_flash_ms: 5000.0, ..Default::default() }.clamped();
    assert!(e.crt_curvature <= 1.0 && e.crt_bloom >= 0.0);
    assert!(e.caret_flash_ms <= 400.0);
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p jetty-app effects_ -- --nocapture`
Expected: FAIL — `EffectsConfig` not found.

- [ ] **Step 3: Write minimal implementation**

Add to `config.rs` (after the `Config` struct, mirroring its `#[serde(default = "...")]` style):
```rust
/// All visual-effect parameters. Every field is `#[serde(default)]` so adding
/// the `[effects]` table is backward compatible: an old config without it (or
/// missing any field) loads with the defaults below. All effects default OFF
/// except `caret_flash_enabled`, so the out-of-box look/idle profile is unchanged.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EffectsConfig {
    #[serde(default = "ef_false")] pub crt_enabled: bool,
    #[serde(default = "ef_curvature")] pub crt_curvature: f32,
    #[serde(default = "ef_scanline")] pub crt_scanline: f32,
    #[serde(default = "ef_mask")] pub crt_mask: f32,
    #[serde(default = "ef_bloom")] pub crt_bloom: f32,
    #[serde(default = "ef_chromatic")] pub crt_chromatic: f32,
    #[serde(default = "ef_vignette")] pub crt_vignette: f32,
    #[serde(default = "ef_white")] pub crt_scanline_tint: [f32; 3],
    #[serde(default = "ef_false")] pub crt_animate_roll: bool,
    #[serde(default = "ef_false")] pub crt_flicker: bool,
    #[serde(default = "ef_false")] pub crt_jitter: bool,
    #[serde(default = "ef_true")] pub caret_flash_enabled: bool,
    #[serde(default = "ef_false")] pub caret_glow_enabled: bool,
    #[serde(default = "ef_flash_ms")] pub caret_flash_ms: f32,
    #[serde(default = "ef_white")] pub caret_flash_color: [f32; 3],
}

fn ef_false() -> bool { false }
fn ef_true() -> bool { true }
fn ef_curvature() -> f32 { 0.30 }
fn ef_scanline() -> f32 { 0.50 }
fn ef_mask() -> f32 { 0.30 }
fn ef_bloom() -> f32 { 0.40 }
fn ef_chromatic() -> f32 { 0.20 }
fn ef_vignette() -> f32 { 0.40 }
fn ef_flash_ms() -> f32 { 130.0 }
fn ef_white() -> [f32; 3] { [1.0, 1.0, 1.0] }

impl Default for EffectsConfig {
    fn default() -> Self {
        EffectsConfig {
            crt_enabled: ef_false(), crt_curvature: ef_curvature(), crt_scanline: ef_scanline(),
            crt_mask: ef_mask(), crt_bloom: ef_bloom(), crt_chromatic: ef_chromatic(),
            crt_vignette: ef_vignette(), crt_scanline_tint: ef_white(),
            crt_animate_roll: ef_false(), crt_flicker: ef_false(), crt_jitter: ef_false(),
            caret_flash_enabled: ef_true(), caret_glow_enabled: ef_false(),
            caret_flash_ms: ef_flash_ms(), caret_flash_color: ef_white(),
        }
    }
}

impl EffectsConfig {
    /// Clamp every numeric field into its valid range. Called on load.
    pub fn clamped(mut self) -> Self {
        let c01 = |v: f32| v.clamp(0.0, 1.0);
        self.crt_curvature = c01(self.crt_curvature);
        self.crt_scanline = c01(self.crt_scanline);
        self.crt_mask = c01(self.crt_mask);
        self.crt_bloom = c01(self.crt_bloom);
        self.crt_chromatic = c01(self.crt_chromatic);
        self.crt_vignette = c01(self.crt_vignette);
        for ch in &mut self.crt_scanline_tint { *ch = c01(*ch); }
        for ch in &mut self.caret_flash_color { *ch = c01(*ch); }
        self.caret_flash_ms = self.caret_flash_ms.clamp(60.0, 400.0);
        self
    }
}
```
Then add to the `Config` struct (after `show_perf_hud`):
```rust
    /// Visual effects (CRT, scanlines, caret). See `EffectsConfig`. Backward
    /// compatible: old configs without `[effects]` load with all defaults.
    #[serde(default)]
    pub effects: EffectsConfig,
```
And add `effects: EffectsConfig::default(),` to `Config`'s own `Default` impl. In `Config::load()`, apply the clamp: after parsing, set `cfg.effects = cfg.effects.clamped();`.

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test -p jetty-app effects_ -- --nocapture`
Expected: PASS (3 tests).

- [ ] **Step 5: Commit**
```bash
git add crates/jetty-app/src/config.rs
git commit -m "feat(effects): EffectsConfig model with serde defaults and clamping"
```

---

## Task 2: App runtime mirror + persist round-trip

**Files:**
- Modify: `crates/jetty-app/src/app.rs` (App struct fields ~`app.rs:231-375`; constructor load ~`app.rs:687`; `persist()` `app.rs:784-810`)
- Test: inline test asserting persist maps every field.

**Interfaces:**
- Consumes: `EffectsConfig` (Task 1).
- Produces: `App.fx: EffectsConfig` runtime mirror; `persist()` writes `self.fx` into `Config.effects`.

- [ ] **Step 1: Write the failing test**

Add an `app.rs` unit test (or a small `cfg(test)` helper) verifying mapping symmetry. If `App` is hard to construct in a unit test, instead add this to `config.rs`:
```rust
#[test]
fn effects_config_roundtrips_through_toml() {
    let mut e = EffectsConfig::default();
    e.crt_enabled = true; e.crt_curvature = 0.42; e.crt_flicker = true;
    e.caret_flash_color = [0.1, 0.2, 0.3];
    let mut cfg = Config::default();
    cfg.effects = e.clone();
    let s = toml::to_string(&cfg).unwrap();
    let back: Config = toml::from_str(&s).unwrap();
    assert_eq!(back.effects, e);
}
```

- [ ] **Step 2: Run test to verify it fails (or passes structurally)**

Run: `cargo test -p jetty-app effects_config_roundtrips -- --nocapture`
Expected: FAIL until `Config.effects` exists end-to-end (it does after Task 1 — this guards the persist wiring; keep it).

- [ ] **Step 3: Implement the runtime mirror**

- In the `App` struct (next to `settings_tab` and the effect structs near `app.rs:231-375`) add: `fx: jetty_app::config::EffectsConfig` (use the crate-local path actually in use). 
- In the constructor where config is loaded (~`app.rs:687`, next to `settings_tab`): `let fx = cfg.effects.clone();` and store it.
- In `persist()` (`app.rs:784-810`), add to the `Config { … }` literal: `effects: self.fx.clone(),`.

- [ ] **Step 4: Run tests + build**

Run: `cargo build -p jetty-app && cargo test -p jetty-app effects_ -- --nocapture`
Expected: build OK, tests PASS.

- [ ] **Step 5: Commit**
```bash
git add crates/jetty-app/src/app.rs
git commit -m "feat(effects): App runtime mirror of EffectsConfig + persist wiring"
```

---

## Task 3: 5th Settings tab plumbing ("Effects")

**Files:**
- Modify: `crates/jetty-render/src/panel.rs` (`TAB_NAMES:110`, `active_tab.min(3):211`, doc/`_=>` arms `:161,207,307`, `tab_w:340`, `tab_rects:342-343`)
- Modify: `crates/jetty-app/src/app.rs` (`self.settings_tab = i.min(3) → .min(4)` at `:2027`)

**Interfaces:**
- Produces: a reachable 5th tab index `4` named `"Effects"` with an empty content arm (widgets land in Task 4).

- [ ] **Step 1: Make the change**

In `panel.rs`:
- `pub const TAB_NAMES: [&str; 4] = [...]` → `[&str; 5]` adding `"Effects"` as the last element (`:110`).
- `let tab_w = (PANEL_W - 32.0) / 4.0;` → `/ 5.0;` (`:340`).
- `let mut tab_rects: [Rect; 4]` → `[Rect; 5]` and its init loop bound 4→5 (`:342-343`).
- `let active_tab = active_tab.min(3);` → `.min(4);` (`:211`).
- Every other `3`/`0..=3` tab cap: the `_ =>` fallthrough arm (`:307`) and doc-comment caps (`:161,207`) — extend to include index 4.
- Add a placeholder `4 => { /* Effects: filled in Task 4 */ }` arm to the band-top `match active_tab` (`:287-311`) so index 4 is handled.

In `app.rs`:
- `self.settings_tab = i.min(3);` → `.min(4);` (`:2027`).

- [ ] **Step 2: Build + manual smoke**

Run: `cargo build -p jetty-app -p jetty-render`
Expected: compiles. Launch (`cargo run -p jetty-app`), open Settings, confirm a 5th "Effects" tab appears, is clickable, shows an empty body, and the other 4 tabs still render/behave correctly with the narrower tab width.

- [ ] **Step 3: Commit**
```bash
git add crates/jetty-render/src/panel.rs crates/jetty-app/src/app.rs
git commit -m "feat(effects): add 5th Settings tab scaffold (Effects)"
```

---

## Task 4: Effects tab widget layout (no scroll yet)

**Files:**
- Modify: `crates/jetty-render/src/panel.rs` (`build_panel` signature `:167`, returned `PanelView`/`PanelGeom`, `active_tab==4` arm)

**Interfaces:**
- Consumes: `App.fx` (passed in as individual params or a borrowed `&EffectsConfig`).
- Produces: widget `Rect`s on `PanelGeom` for: `crt_enable_toggle`, `crt_curvature_track/handle`, `crt_scanline_*`, `crt_mask_*`, `crt_bloom_*`, `crt_chromatic_*`, `crt_vignette_*`, `crt_tint_r/g/b_*`, `crt_roll/flicker/jitter_toggle`, `caret_flash_toggle`, `caret_glow_toggle`, `caret_dur_*`, `caret_color_r/g/b_*`, plus a content-height field `effects_content_h: f32`.

- [ ] **Step 1: Extend `build_panel` signature**

Add `effects: &EffectsConfig` (or explicit params) to `build_panel(...)` (`:167`) and thread `App.fx` through the call site in `app.rs` (where the panel is built for the settings window).

- [ ] **Step 2: Lay out the Effects widgets**

In the `active_tab == 4` arm, place bands top→bottom from `content_top` with a 44px band pitch, two group headers ("CRT", "Caret"):
- Reuse the **opacity slider** geometry (track `:365`, handle `:368`, fill `:369`, 348px track) for: curvature, scanline, mask, bloom, chromatic, vignette, caret duration.
- Reuse the **focus-autohide toggle pill** for: crt_enable, roll, flicker, jitter, caret_flash, caret_glow. Lay roll/flicker/jitter as three pills on one band row; same for grouping where noted in the spec mockup (§6.2).
- **RGB triple:** three narrowed (~108px) slider tracks side-by-side on one band for `crt_scanline_tint` and `caret_flash_color`, each driving R/G/B in [0,1].
- Map each `fx` value to its handle/fill position; map each toggle's on/off to its pill state.
- Compute total `effects_content_h` (last band bottom − content_top) and store it on `PanelGeom`/`PanelView`.

- [ ] **Step 3: Build + manual smoke**

Run: `cargo run -p jetty-app`
Expected: Effects tab shows all widgets with positions reflecting current `fx` values. (Interaction comes in Task 5; widgets may overflow past 560 for now — fixed in Task 6.)

- [ ] **Step 4: Commit**
```bash
git add crates/jetty-render/src/panel.rs crates/jetty-app/src/app.rs
git commit -m "feat(effects): render Effects-tab widgets bound to EffectsConfig"
```

---

## Task 5: Effects tab input wiring (drag/toggle → fx → persist)

**Files:**
- Modify: `crates/jetty-app/src/input.rs` (`MouseAction` enum `:234-305`, `decide_mouse_press` `:325`)
- Modify: `crates/jetty-app/src/app.rs` (`handle_settings_action` `:1905`, slider-drag pattern `:1922`)

**Interfaces:**
- Consumes: `PanelGeom` rects (Task 4).
- Produces: `MouseAction` variants `ToggleCrt`, `StartCrtCurvatureDrag`, `StartScanlineDrag`, `StartMaskDrag`, `StartBloomDrag`, `StartChromaticDrag`, `StartVignetteDrag`, `StartTint{R,G,B}Drag`, `ToggleCrtRoll`, `ToggleCrtFlicker`, `ToggleCrtJitter`, `ToggleCaretFlash`, `ToggleCaretGlow`, `StartCaretDurDrag`, `StartCaretColor{R,G,B}Drag`. Each updates `self.fx`, calls `self.persist()`, requests redraw on both windows.

- [ ] **Step 1: Add the `MouseAction` variants** beside existing ones (`:234-305`).

- [ ] **Step 2: Hit-test them** in `decide_mouse_press` (`:325`), AFTER the tab strip test (`:335`) so tab switching is unaffected, in priority order (toggles before/independent of sliders; sliders use the same track-drag pattern as opacity).

- [ ] **Step 3: Handle them** in `handle_settings_action` (`:1905`), mirroring the opacity slider-drag handler (`:1922`): for sliders, map mouse-x along the track to `[0,1]` (or `[60,400]` for duration) and write `self.fx.<field>`; for toggles, flip the bool; then `self.persist()` and `request_redraw()` on both windows.

- [ ] **Step 4: Manual verification (record evidence)**

Run: `cargo run -p jetty-app`
Expected: dragging each slider moves its handle and changes `~/.config/jetty/config.toml` `[effects]` live; toggles flip; values survive restart. Confirm via `grep -A20 '\[effects\]' ~/.config/jetty/config.toml`.

- [ ] **Step 5: Commit**
```bash
git add crates/jetty-app/src/input.rs crates/jetty-app/src/app.rs
git commit -m "feat(effects): wire Effects-tab widget input to fx + persist"
```

---

## Task 6: Vertical scroll for the Effects tab

**Files:**
- Modify: `crates/jetty-app/src/app.rs` (`effects_scroll: f32` field; wheel handling in the settings-window `WindowEvent::MouseWheel` arm; pass scroll into panel build + render)
- Modify: `crates/jetty-render/src/panel.rs` (offset Effects widget Y by `-effects_scroll`; set `set_scissor_rect` to the content region for Effects quad+text draws; emit a scrollbar indicator rect when `content_h > visible_h`)
- Modify: `crates/jetty-app/src/input.rs` (Effects hit-tests add `effects_scroll` back into compared Y; reject clicks outside the content viewport)

**Interfaces:**
- Consumes: `effects_content_h` (Task 4).
- Produces: `App.effects_scroll`, clamped to `[0, max(0, content_h - visible_h)]`.

- [ ] **Step 1:** Add `effects_scroll: f32` (default 0.0) to `App`.

- [ ] **Step 2: Wheel handler.** In the settings window's `MouseWheel` handling, when `settings_tab == 4`: `self.effects_scroll = (self.effects_scroll - delta_px).clamp(0.0, (content_h - visible_h).max(0.0));` then `request_redraw()`. Derive `delta_px` from `MouseScrollDelta` (Line→×~24px, Pixel→raw).

- [ ] **Step 3: Render clipping.** Pass `effects_scroll` to `build_panel`; offset all Effects widget Y by `-effects_scroll`. In the panel render, set the render pass scissor (`set_scissor_rect`) to the content viewport `[content_top .. PANEL_H - bottom_margin]` (in physical px) before drawing Effects quads + text, so overflow is clipped; draw tab strip/chrome outside the scissor. Emit a thin indicator rect on the right edge sized `visible_h/content_h`.

- [ ] **Step 4: Scroll-aware hit-test.** In `input.rs`, for Effects-tab widgets add `effects_scroll` back to the click Y before comparing against rects, and reject clicks whose Y is outside the content viewport.

- [ ] **Step 5: Manual verification**

Run: `cargo run -p jetty-app`
Expected: Effects content scrolls with the wheel, clamps at top/bottom, the indicator tracks position, widgets below the fold are hittable after scrolling, and clicks in the clipped region do nothing.

- [ ] **Step 6: Commit**
```bash
git add crates/jetty-app/src/app.rs crates/jetty-render/src/panel.rs crates/jetty-app/src/input.rs
git commit -m "feat(effects): scrollable Effects tab (scissor clip + scroll-aware hit-test)"
```

---

## Task 7: `Crt` module scaffold (passthrough) + export + compile test

**Files:**
- Create: `crates/jetty-render/src/crt.rs`
- Modify: `crates/jetty-render/src/lib.rs` (export `Crt`, `CrtUniform`)
- Test: a `crt.rs` unit test that builds a headless device and creates the shader module (shader-compile smoke).

**Interfaces:**
- Produces: `pub struct Crt`; `pub struct CrtUniform { /* flat scalars, no vec3 */ }`; `Crt::new(device: &wgpu::Device, format: wgpu::TextureFormat) -> Crt`; `Crt::apply(&self, device, queue, encoder, dst: &wgpu::TextureView, src: &wgpu::TextureView, width: u32, height: u32, u: &CrtUniform)`.

- [ ] **Step 1: Write the failing test** (mirror any existing render-crate test harness; if none, use `pollster` to request an adapter/device):
```rust
#[test]
fn crt_shader_compiles() {
    let instance = wgpu::Instance::default();
    let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions::default())).expect("adapter");
    let (device, _q) = pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor::default(), None)).expect("device");
    let _crt = Crt::new(&device, wgpu::TextureFormat::Rgba8UnormSrgb); // must not panic / shader must compile
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p jetty-render crt_shader_compiles`
Expected: FAIL — `Crt` undefined.

- [ ] **Step 3: Implement passthrough `crt.rs`** — copy `phosphor.rs` structure verbatim, but the WGSL fragment is a plain `textureSample(src, samp, uv)` passthrough (no effects yet). `CrtUniform` is a flat `#[repr(C)] #[derive(Pod, Zeroable)]` struct of scalars/`[f32;4]` (no vec3). `apply` does a single fullscreen-triangle (`vi<3`) pass sampling `src` → `dst`, replace blend. Export from `lib.rs`.

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p jetty-render crt_shader_compiles`
Expected: PASS. (If CI has no GPU adapter, gate the test behind a feature/`#[ignore]` and run locally — note this in the commit.)

- [ ] **Step 5: Commit**
```bash
git add crates/jetty-render/src/crt.rs crates/jetty-render/src/lib.rs
git commit -m "feat(crt): Crt post-effect scaffold (passthrough) + shader-compile test"
```

---

## Task 8: Offscreen routing — CRT samples scene → surface

**Files:**
- Modify: `crates/jetty-app/src/app.rs` (offscreen alloc guard `:3820`; scene-view routing `:3921-3928`; build `crt` field near `:640`; CRT dispatch after composite, before `present()` `:4241`; skip `mask.apply` `:4155` when CRT on)

**Interfaces:**
- Consumes: `Crt::apply` (Task 7), `App.fx.crt_enabled`.
- Produces: when `crt_enabled`, the whole scene renders to `offscreen` and the CRT pass blits offscreen→surface (passthrough for now).

- [ ] **Step 1: Build the `crt` instance.** Add `crt: Option<jetty_render::Crt>` to `App`; construct it where `phosphor` is built (~`app.rs:640`) with the surface format.

- [ ] **Step 2: Persistent offscreen + routing.** Extend the alloc guard (`:3820`) and routing (`:3921-3928`): `let want_offscreen = self.fx.crt_enabled || tier_b_active;` allocate/keep `offscreen` when `want_offscreen`; set `scene_view = &offscreen.1` when `want_offscreen` else the surface view.

- [ ] **Step 3: Corner handling.** When `self.fx.crt_enabled`, **skip** `mask.apply` (`:4155`) — the CRT pass will own corners (Task 9). Keep `mask.apply` on the non-CRT path unchanged.

- [ ] **Step 4: CRT dispatch.** After the scene/summon composite into `offscreen`, if `self.fx.crt_enabled && !tier_b_active`, call `crt.apply(... dst = surface view, src = &offscreen.1 ...)` with a passthrough `CrtUniform`. (During an active Tier-B summon, bypass CRT this frame — see spec §4.4.) Then `frame.present()`.

- [ ] **Step 5: Manual verification**

Run: enable CRT via Settings (toggle), `cargo run -p jetty-app`.
Expected: with CRT on, output looks identical (passthrough) but is now routed offscreen→surface; rounded corners still transparent; with CRT off, byte-identical to today. No idle redraws in either case yet.

- [ ] **Step 6: Commit**
```bash
git add crates/jetty-app/src/app.rs
git commit -m "feat(crt): route scene to offscreen and blit via Crt when enabled"
```

---

## Task 9: CRT shader — curvature, mask, scanlines, vignette, chromatic, bloom, corner alpha

**Files:**
- Modify: `crates/jetty-render/src/crt.rs` (WGSL fragment + `CrtUniform` fields)
- Modify: `crates/jetty-app/src/app.rs` (pack `CrtUniform` from `self.fx` each frame)

**Interfaces:**
- Consumes: `self.fx` static fields (curvature/scanline/mask/bloom/chromatic/vignette/tint), surface size, corner radius.
- Produces: full static CRT look with transparent rounded corners.

- [ ] **Step 1: Extend `CrtUniform`** with `resolution: [f32;2]`, `curvature, scanline, mask, bloom, chromatic, vignette: f32`, `tint: [f32;4]` (rgb + pad), `corner_radius: f32`, `time: f32` (used in Task 10), `flags: u32` (bitfield for roll/flicker/jitter, Task 10) — all flat, no vec3.

- [ ] **Step 2: Implement the WGSL fragment** (single sample pass; first-cut, tune later):
  - barrel-warp `uv` by `curvature`; outside [0,1] → output transparent black (bezel).
  - chromatic: sample R/G/B at `uv ± chromatic * dir` (dir grows toward edges).
  - scanline: `bright = 1 - scanline * (0.5 + 0.5*sin(uv.y * resolution.y * PI))`, tinted by `tint`.
  - mask: per-output-x RGB aperture pattern scaled by `mask`.
  - bloom: 9–13 weighted neighbor taps of the warped sample, thresholded, `+= bloom * glow`.
  - vignette: `* mix(1.0, edge_falloff, vignette)`.
  - corner alpha: rounded-rect SDF on **un-warped** output coords using `corner_radius` → multiply output alpha (replicate `phosphor.rs:74` `cov`).
- [ ] **Step 3: Pack the uniform** from `self.fx` in `app.rs` each frame and pass to `crt.apply`.

- [ ] **Step 4: Manual verification (screenshots)**

Run: enable CRT + each slider, `cargo run -p jetty-app`.
Expected: curvature bows the image; scanlines/mask/vignette/chromatic/bloom each respond to their slider; tint RGB tints scanlines; rounded corners stay transparent (no re-opaquing). Capture a before/after screenshot.

- [ ] **Step 5: Commit**
```bash
git add crates/jetty-render/src/crt.rs crates/jetty-app/src/app.rs
git commit -m "feat(crt): full static CRT shader (curvature/mask/scanlines/bloom/CA/vignette + corner alpha)"
```

---

## Task 10: Animated CRT — time uniform + roll/flicker/jitter + redraw guard

**Files:**
- Modify: `crates/jetty-app/src/app.rs` (`crt_clock: std::time::Instant`; feed `time`; set `flags` from roll/flicker/jitter; extend self-redraw guard `:4236`)
- Modify: `crates/jetty-render/src/crt.rs` (use `time` + `flags` for rolling scanline phase, global flicker, sub-pixel jitter)

**Interfaces:**
- Consumes: `self.fx.crt_animate_roll/crt_flicker/crt_jitter`.
- Produces: continuous redraw **only** while `crt_enabled && (roll||flicker||jitter)`; otherwise idle stays 0-CPU.

- [ ] **Step 1:** Add `crt_clock: std::time::Instant` to `App` (init at construction). Each frame compute `time = self.crt_clock.elapsed().as_secs_f32()` and set it on the uniform; set `flags` bits from the three toggles.

- [ ] **Step 2: Shader.** When the roll bit is set, add `time*roll_speed` to the scanline phase; when flicker bit set, modulate global brightness by `1 - k*fract-noise(time)`; when jitter bit set, offset sample `uv.x` by a small `time`-driven sub-pixel amount.

- [ ] **Step 3: Redraw guard.** Extend the self-redraw scheduler (`:4236`) so it keeps requesting redraws when `self.fx.crt_enabled && (self.fx.crt_animate_roll || self.fx.crt_flicker || self.fx.crt_jitter)`.

- [ ] **Step 4: 0-CPU idle regression check (critical)**

Run: `cargo run -p jetty-app`, then observe CPU at idle (e.g. `top -pid <pid>`):
- CRT off → ~0% idle.
- CRT on, all three animate toggles off → ~0% idle (static composites only into damage frames).
- Any animate toggle on → continuous redraw (nonzero CPU), and returns to ~0% when toggled off.

- [ ] **Step 5: Commit**
```bash
git add crates/jetty-app/src/app.rs crates/jetty-render/src/crt.rs
git commit -m "feat(crt): animated roll/flicker/jitter gated behind toggles (idle-safe)"
```

---

## Task 11: Caret flash + pulse (CPU path, default ON)

**Files:**
- Modify: `crates/jetty-app/src/app.rs` (`caret_anim: Option<Instant>` field `:317`-area; trigger in `KeyAction::Send` `:3661-3681`; compute `caret_t` `:3844`; pass into `text.render_to`; redraw guard `:4236`)
- Modify: `crates/jetty-render/src/text.rs` (`render_to` `:405`; cursor `TextArea` modulation `:557-573`)

**Interfaces:**
- Consumes: `self.fx.caret_flash_enabled`, `caret_flash_ms`, `caret_flash_color`.
- Produces: cursor color lerp + scale pulse over `caret_t∈[0,1]`, ease-out; bursty redraw that returns to idle.

- [ ] **Step 1: Trigger.** Add `caret_anim: Option<Instant>` to `App`. In `KeyAction::Send(bytes)` (`:3661-3681`), after the PTY write: if `self.fx.caret_flash_enabled && is_printable_keystroke(&bytes)` then `self.caret_anim = Some(Instant::now()); request_redraw();`. Add a small helper `is_printable_keystroke` that rejects pure control/escape sequences (e.g. bytes starting with `0x1b`, or all < 0x20 except none).

- [ ] **Step 2: Progress.** Near `:3844`: `let caret_t = self.caret_anim.map(|s| (s.elapsed().as_secs_f32() / (self.fx.caret_flash_ms/1000.0)).min(1.0)); if caret_t == Some(1.0) { self.caret_anim = None; }`. Thread `caret_t`, `caret_flash_color` into `text.render_to`.

- [ ] **Step 3: Modulate cursor.** In `text.rs` cursor block (`:557-573`), before `prepare()`: with `e = ease_out(caret_t)`, set cursor `TextArea.default_color = lerp(cursor_rgb, caret_flash_color, e)` and `scale = 1.0 + 0.15*e*(1.0-e)*4.0` (peak ~1.15 mid-burst), keeping the glyph centered on its cell.

- [ ] **Step 4: Redraw guard.** Add `self.caret_anim.is_some()` to the self-redraw scheduler (`:4236`).

- [ ] **Step 5: Manual verification**

Run: `cargo run -p jetty-app`, type characters.
Expected: cursor flashes toward `caret_flash_color` and gently pulses per keystroke; rapid typing → continuous pulse; stops (idle) shortly after typing stops; no flash on arrow keys / escape sequences. Duration slider changes the timing.

- [ ] **Step 6: Commit**
```bash
git add crates/jetty-app/src/app.rs crates/jetty-render/src/text.rs
git commit -m "feat(caret): keypress flash+pulse (CPU path, idle-safe burst)"
```

---

## Task 12: Caret glow / ripple (GPU path, optional toggle)

**Files:**
- Create: `crates/jetty-render/src/caret_fx.rs`
- Modify: `crates/jetty-render/src/lib.rs` (export `CaretFx`)
- Modify: `crates/jetty-app/src/app.rs` (build `caret_fx`; dispatch when `caret_glow_enabled` and `caret_anim.is_some()`)

**Interfaces:**
- Produces: `pub struct CaretFx`; `CaretFx::new(device, format)`; `CaretFx::apply(&self, device, queue, encoder, dst, uniform: &CaretFxUniform)` where `CaretFxUniform { resolution:[f32;2], cursor_px:[f32;2], cell:[f32;2], t:f32, intensity:f32, color:[f32;4] }`.

- [ ] **Step 1: Implement `caret_fx.rs`** (phosphor.rs pattern): additive fullscreen pass drawing, around `cursor_px`, a soft radial glow halo (falls off over ~2 cells) plus an expanding ring whose radius grows with `t`, faded by `1-t`. Color `color`, scaled by `intensity`. Additive blend so it only brightens; respect corner transparency by gating to the unwarped content region.

- [ ] **Step 2: Shader-compile test** like Task 7 (`caret_fx_shader_compiles`).

- [ ] **Step 3: Dispatch.** Build `caret_fx: Option<jetty_render::CaretFx>` on `App`. In the render loop, when `self.fx.caret_glow_enabled && self.caret_anim.is_some()`, after the scene (and, when CRT on, into the offscreen before the CRT blit, or onto the surface when CRT off), call `caret_fx.apply(...)` with the cursor pixel pos (`text.rs:565-566` mapping) and `t = caret_t`.

- [ ] **Step 4: Manual verification**

Run: enable "Glow / ripple" toggle, `cargo run -p jetty-app`, type.
Expected: glow halo + expanding ring around the cursor on each keystroke; off when toggle disabled; composes correctly with CRT on and off; corners stay transparent.

- [ ] **Step 5: Commit**
```bash
git add crates/jetty-render/src/caret_fx.rs crates/jetty-render/src/lib.rs crates/jetty-app/src/app.rs
git commit -m "feat(caret): optional GPU glow/ripple pass"
```

---

## Task 13: macOS app icon — script hardening + README (INDEPENDENT)

**Files:**
- Modify: `scripts/make-macos-app.sh`
- Modify: `README.md` (macOS section ~lines 78-89)

**Interfaces:** none (build/docs only). Can run any time, in parallel with Tasks 1–12.

- [ ] **Step 1: Harden the script.** At the top after `set -e`: verify tools and inputs —
```sh
command -v sips >/dev/null 2>&1 || { echo "error: sips not found (macOS only)"; exit 1; }
command -v iconutil >/dev/null 2>&1 || { echo "error: iconutil not found (macOS only)"; exit 1; }
[ -f assets/icons/jetty-256.png ] || { echo "error: assets/icons/jetty-256.png missing"; exit 1; }
```
After `iconutil` runs, verify the `.icns` exists: `[ -f "$APP/Contents/Resources/jetty.icns" ] || { echo "error: .icns not generated"; exit 1; }`. Replace hardcoded `0.1.0` (script lines ~43-44) with `VERSION="$(grep -m1 '^version' Cargo.toml | sed -E 's/.*"(.*)".*/\1/')"` and use `$VERSION` for `CFBundleVersion`/`CFBundleShortVersionString`.

- [ ] **Step 2: Document in README.** In the macOS install section, add:
```md
### macOS (.app bundle with Dock icon)
cargo build --release
sh scripts/make-macos-app.sh      # builds JeTTY.app with the Dock/Finder icon
open JeTTY.app                     # run the bundle, NOT ./target/release/jetty
```
Note: the bare binary cannot show a Dock icon on macOS (winit limitation); the `.app` bundle is required. If the icon is cached stale, run `killall Dock` once.

- [ ] **Step 3: Verify on macOS**

Run: `cargo build --release && sh scripts/make-macos-app.sh && open JeTTY.app`
Expected: script succeeds with tool checks; `JeTTY.app/Contents/Resources/jetty.icns` exists; the Dock/Finder shows the JeTTY icon (after `killall Dock` if cached).

- [ ] **Step 4: Commit**
```bash
git add scripts/make-macos-app.sh README.md
git commit -m "fix(macos): wire up app bundle icon (harden script + document)"
```

---

## Task 14: Final verification & screenshots

**Files:** none (verification) — optionally add effect screenshots under `assets/screenshots/`.

- [ ] **Step 1: Full build + tests + lint**

Run: `cargo build --workspace && cargo test --workspace && cargo clippy --workspace -- -D warnings`
Expected: all green.

- [ ] **Step 2: 0-CPU idle regression matrix** (record results): off; CRT-static; CRT-animated; caret typing→idle — per Task 10 Step 4 / Task 11 Step 5.

- [ ] **Step 3: Perf budget.** Compare CRT-on frame cost to `docs/perf-budget.md`; record the delta in the PR description.

- [ ] **Step 4: Screenshots.** Regenerate effect screenshots via the repo's existing screenshot path (used in commit `6a50ec3`) for CRT + caret; drop into `assets/screenshots/` and reference from README if appropriate.

- [ ] **Step 5: Commit**
```bash
git add -A
git commit -m "test(effects): verification pass + effect screenshots"
```

---

## Self-Review

**Spec coverage:** §4 CRT → Tasks 7-10; §5 caret → Tasks 11-12; §6 Effects tab + scroll → Tasks 3-6; §7 config/persist → Tasks 1-2; §8 macOS icon → Task 13; §9 params → Task 1 table; §11 testing → Tasks 1,7,10,11,14. No gap.

**Placeholder scan:** code provided for deterministic tasks; shader tasks carry first-cut WGSL described concretely with tuning expected and verified by screenshot — acceptable for GPU work. No "TBD/handle edge cases" left.

**Type consistency:** `EffectsConfig` field names are identical across Tasks 1-11; `App.fx` used uniformly; `Crt::new/apply`, `CrtUniform`, `CaretFx::new/apply`, `CaretFxUniform` signatures consistent across Tasks 7-12. `MouseAction` variant names consistent Tasks 5-6.

**Idle invariant:** guarded explicitly in Tasks 10/11 with a regression test in Task 14.
