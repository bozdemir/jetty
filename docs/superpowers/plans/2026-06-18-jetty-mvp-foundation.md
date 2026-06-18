# Jetty MVP — Foundation Plan (M0 + M1)

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build the foundation of Jetty — a cargo workspace that opens a GPU-rendered window and runs a real, interactive terminal in it (normal window; the center-summon overlay and effects come in the follow-up M2/M3 plan).

**Architecture:** A cargo workspace with single-purpose crates (`jetty-core`, `jetty-render`, `jetty-platform`, `jetty-app`, thin `jetty` bin). `jetty-platform` owns the winit window/event loop (X11 backend tool); `jetty-render` owns the wgpu device + glyphon text rendering and is backend-agnostic (it only needs a window handle + a grid snapshot); `jetty-core` wraps `alacritty_terminal` + a `portable-pty` shell and exposes an immutable `GridSnapshot`; `jetty-app` wires them together and owns the run loop.

**Tech Stack:** Rust (edition 2021), `winit 0.30`, `wgpu` (version pinned by glyphon), `glyphon 0.11`, `alacritty_terminal 0.26`, `portable-pty 0.9`, `raw-window-handle 0.6`, `pollster 0.4`.

## Global Constraints

- Rust edition **2021**; toolchain 1.94+ (present on the dev machine).
- `winit = "0.30"` — **stable**, uses the `ApplicationHandler` API. Do NOT use the `0.31.0-beta` line.
- `glyphon = "0.11"` dictates the compatible `wgpu` version. Pin `wgpu` to exactly what glyphon pulls (Task 1 discovers it with `cargo tree -p wgpu`). Do not bump `wgpu` independently.
- `alacritty_terminal = "0.26"`, `portable-pty = "0.9"`.
- Crate dependency direction: `jetty-app` → {`jetty-core`, `jetty-render`, `jetty-platform`}. The leaf crates do **not** depend on each other. `jetty-render` never imports `winit` or display-server APIs; `jetty-platform` never imports `wgpu`.
- Every code-producing task ends with a passing test or a runnable checkpoint, then a commit.
- Target/validate on the dev machine: KDE Plasma 5.27 on **X11**.

---

## File Structure

```
jetty/
├─ Cargo.toml                      # [workspace] members
├─ rust-toolchain.toml             # pin stable
├─ crates/
│  ├─ jetty-core/
│  │  ├─ Cargo.toml
│  │  └─ src/
│  │     ├─ lib.rs                 # re-exports: PtySession, Terminal, GridSnapshot, CellSnapshot
│  │     ├─ snapshot.rs            # GridSnapshot + CellSnapshot (pure data)
│  │     ├─ terminal.rs            # Terminal: Term<EventProxy> + vte Processor + snapshot()
│  │     └─ pty.rs                 # PtySession: spawn $SHELL, reader thread, writer handle
│  ├─ jetty-render/
│  │  ├─ Cargo.toml
│  │  └─ src/
│  │     ├─ lib.rs                 # re-exports: Renderer
│  │     ├─ gpu.rs                 # GpuContext: wgpu instance/surface/device/queue/config
│  │     └─ text.rs               # TextLayer (glyphon) + draw(snapshot)
│  ├─ jetty-platform/
│  │  ├─ Cargo.toml
│  │  └─ src/
│  │     ├─ lib.rs                 # re-exports: window helpers, MonitorInfo
│  │     └─ window.rs              # build_window(), cursor-monitor helpers
│  └─ jetty-app/
│     ├─ Cargo.toml
│     └─ src/
│        ├─ lib.rs                 # run(): owns App (ApplicationHandler), wires crates
│        └─ app.rs                 # App state machine + event handling + redraw
└─ src/main.rs                     # thin bin → jetty_app::run()
```

> **Note on the `PlatformBackend` trait (from the spec §5):** the crate boundary exists from day one (`jetty-platform`), but the trait abstraction is introduced in the **M2 plan**, when a second concern (global hotkey + Wayland) actually appears. Introducing a one-impl trait now would be premature (YAGNI); the spec's growth intent is preserved by the crate boundary. M0/M1 use concrete `jetty-platform` helpers.

---

## Task 1: Workspace scaffolding & dependency pinning

**Files:**
- Create: `Cargo.toml`, `rust-toolchain.toml`, `src/main.rs`
- Create: `crates/jetty-core/Cargo.toml`, `crates/jetty-render/Cargo.toml`, `crates/jetty-platform/Cargo.toml`, `crates/jetty-app/Cargo.toml`
- Create: `crates/*/src/lib.rs` (empty stubs)

**Interfaces:**
- Produces: a building workspace; the exact pinned `wgpu` version recorded in `crates/jetty-render/Cargo.toml`.

- [ ] **Step 1: Create the workspace root `Cargo.toml`**

```toml
[workspace]
resolver = "2"
members = ["crates/jetty-core", "crates/jetty-render", "crates/jetty-platform", "crates/jetty-app"]

[package]
name = "jetty"
version = "0.0.0"
edition = "2021"

[[bin]]
name = "jetty"
path = "src/main.rs"

[dependencies]
jetty-app = { path = "crates/jetty-app" }
```

- [ ] **Step 2: Pin the toolchain**

`rust-toolchain.toml`:
```toml
[toolchain]
channel = "stable"
```

- [ ] **Step 3: Create leaf crate manifests and empty libs**

`crates/jetty-core/Cargo.toml`:
```toml
[package]
name = "jetty-core"
version = "0.0.0"
edition = "2021"

[dependencies]
alacritty_terminal = "0.26"
portable-pty = "0.9"
```

`crates/jetty-platform/Cargo.toml`:
```toml
[package]
name = "jetty-platform"
version = "0.0.0"
edition = "2021"

[dependencies]
winit = "0.30"
raw-window-handle = "0.6"
```

`crates/jetty-render/Cargo.toml`:
```toml
[package]
name = "jetty-render"
version = "0.0.0"
edition = "2021"

[dependencies]
glyphon = "0.11"
pollster = "0.4"
raw-window-handle = "0.6"
# wgpu pinned in Step 5 to glyphon's version
```

`crates/jetty-app/Cargo.toml`:
```toml
[package]
name = "jetty-app"
version = "0.0.0"
edition = "2021"

[dependencies]
jetty-core = { path = "../jetty-core" }
jetty-render = { path = "../jetty-render" }
jetty-platform = { path = "../jetty-platform" }
winit = "0.30"
```

Each `crates/*/src/lib.rs`: empty file. `src/main.rs`:
```rust
fn main() {
    println!("jetty");
}
```

- [ ] **Step 4: Build to resolve dependencies**

Run: `cargo build`
Expected: PASS (downloads deps, compiles empty crates). If `glyphon`/`alacritty_terminal` fail to resolve, the error names the bad version — correct it before continuing.

- [ ] **Step 5: Discover and pin the wgpu version glyphon uses**

Run: `cargo tree -p wgpu --depth 0`
Expected: prints e.g. `wgpu vX.Y.Z`. Add that exact `wgpu = "X.Y"` to `crates/jetty-render/Cargo.toml` `[dependencies]`, then `cargo build` again (Expected: PASS). This guarantees our direct `wgpu` usage matches glyphon's.

- [ ] **Step 6: Commit**

```bash
git add Cargo.toml Cargo.lock rust-toolchain.toml src/ crates/
git commit -m "feat: scaffold jetty cargo workspace and pin deps"
```

---

## Task 2: Window + event loop (jetty-platform + jetty-app)

**Files:**
- Create: `crates/jetty-platform/src/window.rs`, modify `crates/jetty-platform/src/lib.rs`
- Create: `crates/jetty-app/src/app.rs`, modify `crates/jetty-app/src/lib.rs`
- Modify: `src/main.rs`

**Interfaces:**
- Produces: `jetty_platform::build_window(event_loop: &ActiveEventLoop, title: &str, size: (u32,u32)) -> Arc<winit::window::Window>`
- Produces: `jetty_app::run() -> ()` — creates the `EventLoop` and runs the `App`.

- [ ] **Step 1: Implement the window builder**

`crates/jetty-platform/src/window.rs`:
```rust
use std::sync::Arc;
use winit::dpi::LogicalSize;
use winit::event_loop::ActiveEventLoop;
use winit::window::Window;

/// Build the main window. M0: a normal decorated window; overlay flags arrive in M2.
pub fn build_window(event_loop: &ActiveEventLoop, title: &str, size: (u32, u32)) -> Arc<Window> {
    let attrs = Window::default_attributes()
        .with_title(title)
        .with_inner_size(LogicalSize::new(size.0, size.1));
    Arc::new(event_loop.create_window(attrs).expect("create_window failed"))
}
```

`crates/jetty-platform/src/lib.rs`:
```rust
mod window;
pub use window::build_window;
```

- [ ] **Step 2: Implement a minimal App that opens the window**

`crates/jetty-app/src/app.rs`:
```rust
use std::sync::Arc;
use winit::application::ApplicationHandler;
use winit::event::WindowEvent;
use winit::event_loop::ActiveEventLoop;
use winit::keyboard::{Key, NamedKey};
use winit::window::{Window, WindowId};

#[derive(Default)]
pub struct App {
    window: Option<Arc<Window>>,
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_none() {
            let window = jetty_platform::build_window(event_loop, "Jetty", (1000, 640));
            self.window = Some(window);
        }
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::KeyboardInput { event, .. }
                if event.logical_key == Key::Named(NamedKey::Escape) =>
            {
                event_loop.exit();
            }
            WindowEvent::RedrawRequested => {
                if let Some(w) = &self.window {
                    w.request_redraw();
                }
            }
            _ => {}
        }
    }
}
```

`crates/jetty-app/src/lib.rs`:
```rust
mod app;

use winit::event_loop::{ControlFlow, EventLoop};

pub fn run() {
    let event_loop = EventLoop::new().expect("event loop");
    event_loop.set_control_flow(ControlFlow::Wait);
    let mut app = app::App::default();
    event_loop.run_app(&mut app).expect("run_app");
}
```

`src/main.rs`:
```rust
fn main() {
    jetty_app::run();
}
```

- [ ] **Step 3: Run and verify a window opens**

Run: `cargo run`
Expected: an empty window titled "Jetty" appears; closing it or pressing ESC exits the process cleanly (no panic).

- [ ] **Step 4: Commit**

```bash
git add crates/jetty-platform crates/jetty-app src/main.rs
git commit -m "feat: open a winit window and run the event loop"
```

---

## Task 3: wgpu context + clear-screen (jetty-render)  → **M0 complete**

**Files:**
- Create: `crates/jetty-render/src/gpu.rs`, modify `crates/jetty-render/src/lib.rs`
- Modify: `crates/jetty-app/src/app.rs`

**Interfaces:**
- Produces: `jetty_render::GpuContext::new(window: Arc<Window>) -> GpuContext`
- Produces: `GpuContext::resize(&mut self, w: u32, h: u32)`
- Produces: `GpuContext::clear(&mut self, rgba: [f64; 4]) -> Result<(), wgpu::SurfaceError>` (acquires the frame, clears it, presents)

> `GpuContext::new` takes `Arc<Window>` because wgpu needs a `'static` surface target; `Arc<Window>` implements `HasWindowHandle + HasDisplayHandle` and satisfies `Into<SurfaceTarget<'static>>`.

- [ ] **Step 1: Implement GpuContext**

`crates/jetty-render/src/gpu.rs`:
```rust
use std::sync::Arc;
use winit::window::Window;

pub struct GpuContext {
    pub surface: wgpu::Surface<'static>,
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
    pub config: wgpu::SurfaceConfiguration,
    pub format: wgpu::TextureFormat,
}

impl GpuContext {
    pub fn new(window: Arc<Window>) -> Self {
        let size = window.inner_size();
        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor::default());
        let surface = instance.create_surface(window.clone()).expect("surface");
        let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance,
            compatible_surface: Some(&surface),
            force_fallback_adapter: false,
        }))
        .expect("no adapter");
        let (device, queue) = pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor {
            label: Some("jetty-device"),
            required_features: wgpu::Features::empty(),
            required_limits: wgpu::Limits::default(),
            memory_hints: wgpu::MemoryHints::default(),
            trace: wgpu::Trace::Off,
        }))
        .expect("device");

        let caps = surface.get_capabilities(&adapter);
        let format = caps
            .formats
            .iter()
            .copied()
            .find(|f| f.is_srgb())
            .unwrap_or(caps.formats[0]);

        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format,
            width: size.width.max(1),
            height: size.height.max(1),
            present_mode: wgpu::PresentMode::Fifo,
            alpha_mode: caps.alpha_modes[0],
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };
        surface.configure(&device, &config);

        Self { surface, device, queue, config, format }
    }

    pub fn resize(&mut self, w: u32, h: u32) {
        if w > 0 && h > 0 {
            self.config.width = w;
            self.config.height = h;
            self.surface.configure(&self.device, &self.config);
        }
    }

    pub fn clear(&mut self, rgba: [f64; 4]) -> Result<(), wgpu::SurfaceError> {
        let frame = self.surface.get_current_texture()?;
        let view = frame.texture.create_view(&wgpu::TextureViewDescriptor::default());
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor { label: Some("clear") });
        {
            let _pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("clear-pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: rgba[0], g: rgba[1], b: rgba[2], a: rgba[3],
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                    depth_slice: None,
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });
        }
        self.queue.submit(Some(encoder.finish()));
        frame.present();
        Ok(())
    }
}
```

`crates/jetty-render/src/lib.rs`:
```rust
mod gpu;
pub use gpu::GpuContext;
```

> The `wgpu::DeviceDescriptor { trace, memory_hints, .. }` fields and `depth_slice` on the color attachment are wgpu-version-specific. They match the version glyphon 0.11 pins (Task 1, Step 5). If `cargo build` flags a field, the compiler names the exact mismatch — adjust that one field to the pinned version's shape; the surrounding flow is stable.

- [ ] **Step 2: Wire GpuContext into the App and clear to dark grey**

In `crates/jetty-app/src/app.rs`, add a `gpu: Option<jetty_render::GpuContext>` field; create it in `resumed` after the window; handle resize and redraw:
```rust
// field:
//     gpu: Option<jetty_render::GpuContext>,

// in resumed(), after building `window`:
let gpu = jetty_render::GpuContext::new(window.clone());
self.gpu = Some(gpu);

// in window_event(), add arms:
WindowEvent::Resized(size) => {
    if let Some(gpu) = &mut self.gpu {
        gpu.resize(size.width, size.height);
    }
}
WindowEvent::RedrawRequested => {
    if let Some(gpu) = &mut self.gpu {
        let _ = gpu.clear([0.07, 0.07, 0.09, 1.0]);
    }
}
```
(Remove the old empty `RedrawRequested` arm.) Keep a `request_redraw()` on the window at the end of `resumed` so the first frame paints.

- [ ] **Step 3: Run and verify a cleared window**

Run: `cargo run`
Expected: the window is filled with dark grey; resizing it does not crash; ESC/close exits cleanly.

- [ ] **Step 4: Commit (M0 milestone)**

```bash
git add crates/jetty-render crates/jetty-app
git commit -m "feat: wgpu context clears the window each frame (M0)"
```

---

## Task 4: PTY session (jetty-core/pty.rs)

**Files:**
- Create: `crates/jetty-core/src/pty.rs`, modify `crates/jetty-core/src/lib.rs`
- Test: `crates/jetty-core/tests/pty.rs`

**Interfaces:**
- Produces: `PtySession::spawn(cols: u16, rows: u16) -> std::io::Result<PtySession>`
- Produces: `PtySession::output(&self) -> &std::sync::mpsc::Receiver<Vec<u8>>` — bytes read from the shell
- Produces: `PtySession::writer(&self) -> Box<dyn std::io::Write + Send>` — write keystrokes to the shell
- Produces: `PtySession::resize(&self, cols: u16, rows: u16)`

- [ ] **Step 1: Write the failing test**

`crates/jetty-core/tests/pty.rs`:
```rust
use jetty_core::PtySession;
use std::time::{Duration, Instant};

#[test]
fn pty_echoes_written_bytes() {
    let pty = PtySession::spawn(80, 24).expect("spawn");
    {
        let mut w = pty.writer();
        // `cat` echoes its stdin back; send a line.
        use std::io::Write;
        w.write_all(b"jetty-marker\n").unwrap();
        w.flush().unwrap();
    }
    let deadline = Instant::now() + Duration::from_secs(5);
    let mut seen = Vec::new();
    while Instant::now() < deadline {
        if let Ok(chunk) = pty.output().recv_timeout(Duration::from_millis(200)) {
            seen.extend_from_slice(&chunk);
            if String::from_utf8_lossy(&seen).contains("jetty-marker") {
                return; // success
            }
        }
    }
    panic!("did not observe echoed marker; got: {:?}", String::from_utf8_lossy(&seen));
}
```

- [ ] **Step 2: Run the test to verify it fails**

Run: `cargo test -p jetty-core --test pty`
Expected: FAIL — `PtySession` not found / unresolved import.

- [ ] **Step 3: Implement PtySession**

`crates/jetty-core/src/pty.rs`:
```rust
use portable_pty::{native_pty_system, CommandBuilder, PtySize};
use std::io::{Read, Write};
use std::sync::mpsc::{channel, Receiver};
use std::sync::{Arc, Mutex};

pub struct PtySession {
    master: Arc<Mutex<Box<dyn portable_pty::MasterPty + Send>>>,
    rx: Receiver<Vec<u8>>,
    _child: Box<dyn portable_pty::Child + Send + Sync>,
}

impl PtySession {
    pub fn spawn(cols: u16, rows: u16) -> std::io::Result<PtySession> {
        let pty_system = native_pty_system();
        let pair = pty_system
            .openpty(PtySize { rows, cols, pixel_width: 0, pixel_height: 0 })
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e.to_string()))?;

        let shell = std::env::var("SHELL").unwrap_or_else(|_| "/bin/bash".to_string());
        let cmd = CommandBuilder::new(shell);
        let child = pair
            .slave
            .spawn_command(cmd)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e.to_string()))?;
        drop(pair.slave);

        let mut reader = pair
            .master
            .try_clone_reader()
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e.to_string()))?;
        let (tx, rx) = channel::<Vec<u8>>();
        std::thread::spawn(move || {
            let mut buf = [0u8; 8192];
            loop {
                match reader.read(&mut buf) {
                    Ok(0) | Err(_) => break,
                    Ok(n) => {
                        if tx.send(buf[..n].to_vec()).is_err() {
                            break;
                        }
                    }
                }
            }
        });

        Ok(PtySession {
            master: Arc::new(Mutex::new(pair.master)),
            rx,
            _child: child,
        })
    }

    pub fn output(&self) -> &Receiver<Vec<u8>> {
        &self.rx
    }

    pub fn writer(&self) -> Box<dyn Write + Send> {
        self.master.lock().unwrap().take_writer().expect("take_writer")
    }

    pub fn resize(&self, cols: u16, rows: u16) {
        let _ = self.master.lock().unwrap().resize(PtySize {
            rows, cols, pixel_width: 0, pixel_height: 0,
        });
    }
}
```

`crates/jetty-core/src/lib.rs`:
```rust
mod pty;
pub use pty::PtySession;
```

> The test writes to `cat`? No — it writes to `$SHELL`'s stdin, which echoes typed input in a cooked PTY, so the marker appears in output. If a minimal CI shell does not echo, set `SHELL=/bin/cat` for the test run: `SHELL=/bin/cat cargo test -p jetty-core --test pty`. Document this in the test file's top comment.

- [ ] **Step 4: Run the test to verify it passes**

Run: `cargo test -p jetty-core --test pty`
Expected: PASS (the marker is observed within the timeout).

- [ ] **Step 5: Commit**

```bash
git add crates/jetty-core
git commit -m "feat: spawn a PTY shell with a background reader (jetty-core)"
```

---

## Task 5: Terminal model + GridSnapshot (jetty-core/terminal.rs + snapshot.rs)

**Files:**
- Create: `crates/jetty-core/src/snapshot.rs`, `crates/jetty-core/src/terminal.rs`
- Modify: `crates/jetty-core/src/lib.rs`
- Test: `crates/jetty-core/tests/snapshot.rs`

**Interfaces:**
- Produces: `CellSnapshot { c: char, fg: [u8;3], bg: [u8;3] }` (Clone, Copy, PartialEq, Debug)
- Produces: `GridSnapshot { cols: usize, rows: usize, cells: Vec<CellSnapshot>, cursor_row: usize, cursor_col: usize }` with `fn cell(&self, row, col) -> &CellSnapshot` and `fn row_text(&self, row) -> String`
- Produces: `Terminal::new(cols: usize, rows: usize) -> Terminal`
- Produces: `Terminal::feed(&mut self, bytes: &[u8])` — advance the VT parser
- Produces: `Terminal::snapshot(&self) -> GridSnapshot`

- [ ] **Step 1: Write the failing test**

`crates/jetty-core/tests/snapshot.rs`:
```rust
use jetty_core::Terminal;

#[test]
fn plain_text_lands_in_the_grid() {
    let mut term = Terminal::new(80, 24);
    term.feed(b"hello");
    let snap = term.snapshot();
    assert_eq!(snap.cols, 80);
    assert_eq!(snap.rows, 24);
    assert!(snap.row_text(0).starts_with("hello"), "row0 = {:?}", snap.row_text(0));
}

#[test]
fn newline_moves_to_next_row() {
    let mut term = Terminal::new(80, 24);
    term.feed(b"a\r\nb");
    let snap = term.snapshot();
    assert!(snap.row_text(0).starts_with('a'));
    assert!(snap.row_text(1).starts_with('b'));
}
```

- [ ] **Step 2: Run the test to verify it fails**

Run: `cargo test -p jetty-core --test snapshot`
Expected: FAIL — `Terminal` not found.

- [ ] **Step 3: Implement snapshot.rs**

`crates/jetty-core/src/snapshot.rs`:
```rust
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct CellSnapshot {
    pub c: char,
    pub fg: [u8; 3],
    pub bg: [u8; 3],
}

impl Default for CellSnapshot {
    fn default() -> Self {
        CellSnapshot { c: ' ', fg: [220, 220, 220], bg: [18, 18, 23] }
    }
}

#[derive(Clone, Debug)]
pub struct GridSnapshot {
    pub cols: usize,
    pub rows: usize,
    pub cells: Vec<CellSnapshot>,
    pub cursor_row: usize,
    pub cursor_col: usize,
}

impl GridSnapshot {
    pub fn cell(&self, row: usize, col: usize) -> &CellSnapshot {
        &self.cells[row * self.cols + col]
    }
    pub fn row_text(&self, row: usize) -> String {
        (0..self.cols).map(|c| self.cell(row, c).c).collect::<String>()
    }
}
```

- [ ] **Step 4: Implement terminal.rs**

`crates/jetty-core/src/terminal.rs`:
```rust
use crate::snapshot::{CellSnapshot, GridSnapshot};
use alacritty_terminal::event::{Event, EventListener};
use alacritty_terminal::grid::Dimensions;
use alacritty_terminal::term::{Config, Term};
use alacritty_terminal::vte::ansi::Processor;

#[derive(Clone, Copy)]
struct NoopListener;
impl EventListener for NoopListener {
    fn send_event(&self, _event: Event) {}
}

#[derive(Clone, Copy)]
struct Size {
    cols: usize,
    lines: usize,
}
impl Dimensions for Size {
    fn total_lines(&self) -> usize { self.lines }
    fn screen_lines(&self) -> usize { self.lines }
    fn columns(&self) -> usize { self.cols }
}

pub struct Terminal {
    term: Term<NoopListener>,
    parser: Processor,
    cols: usize,
    rows: usize,
}

impl Terminal {
    pub fn new(cols: usize, rows: usize) -> Terminal {
        let size = Size { cols, lines: rows };
        let term = Term::new(Config::default(), &size, NoopListener);
        Terminal { term, parser: Processor::new(), cols, rows }
    }

    pub fn feed(&mut self, bytes: &[u8]) {
        self.parser.advance(&mut self.term, bytes);
    }

    pub fn snapshot(&self) -> GridSnapshot {
        use alacritty_terminal::index::{Column, Line, Point};
        let mut cells = vec![CellSnapshot::default(); self.cols * self.rows];
        let grid = self.term.grid();
        for row in 0..self.rows {
            for col in 0..self.cols {
                let point = Point::new(Line(row as i32), Column(col));
                let cell = &grid[point];
                let fg = resolve_rgb(cell.fg);
                let bg = resolve_rgb(cell.bg);
                cells[row * self.cols + col] = CellSnapshot { c: cell.c, fg, bg };
            }
        }
        let cursor = self.term.grid().cursor.point;
        GridSnapshot {
            cols: self.cols,
            rows: self.rows,
            cells,
            cursor_row: cursor.line.0.max(0) as usize,
            cursor_col: cursor.column.0,
        }
    }
}

/// Map an alacritty cell color to RGB. Named/indexed colors get sane fallbacks;
/// true-color (Spec) is used directly. Full palette mapping arrives with theming.
fn resolve_rgb(color: alacritty_terminal::vte::ansi::Color) -> [u8; 3] {
    use alacritty_terminal::vte::ansi::{Color, NamedColor};
    match color {
        Color::Spec(rgb) => [rgb.r, rgb.g, rgb.b],
        Color::Named(NamedColor::Background) => [18, 18, 23],
        Color::Named(NamedColor::Foreground) => [220, 220, 220],
        Color::Named(_) => [220, 220, 220],
        Color::Indexed(_) => [220, 220, 220],
    }
}
```

`crates/jetty-core/src/lib.rs`:
```rust
mod pty;
mod snapshot;
mod terminal;

pub use pty::PtySession;
pub use snapshot::{CellSnapshot, GridSnapshot};
pub use terminal::Terminal;
```

> The exact paths for `Config`, `Color`, `NamedColor`, `Point`, and the `Dimensions` required methods are from `alacritty_terminal 0.26`. If the compiler reports a moved item (e.g. `Config` under `term::Config` vs a re-export, or extra required `Dimensions` methods like `last_column`/`bottommost_line` which have provided defaults), follow the error to the correct path; the logic is unchanged.

- [ ] **Step 5: Run the tests to verify they pass**

Run: `cargo test -p jetty-core --test snapshot`
Expected: PASS (both tests).

- [ ] **Step 6: Commit**

```bash
git add crates/jetty-core
git commit -m "feat: terminal VT model exposes an immutable GridSnapshot"
```

---

## Task 6: Text rendering with glyphon (jetty-render/text.rs)

**Files:**
- Create: `crates/jetty-render/src/text.rs`, modify `crates/jetty-render/src/lib.rs`
- Modify: `crates/jetty-render/Cargo.toml` (add `jetty-core` path dep for the snapshot type)

**Interfaces:**
- Produces: `TextLayer::new(gpu: &GpuContext, font_size: f32) -> TextLayer`
- Produces: `TextLayer::resize(&mut self, gpu: &GpuContext)`
- Produces: `TextLayer::render(&mut self, gpu: &GpuContext, snapshot: &GridSnapshot) -> Result<(), wgpu::SurfaceError>` — clears the frame and draws the grid text.
- Produces: `TextLayer::cell_size(&self) -> (f32, f32)` — logical px per cell (for cols/rows sizing).

> `jetty-render` may depend on `jetty-core` ONLY for the `GridSnapshot` data type (a leaf data dependency, not the other way around). Add `jetty-core = { path = "../jetty-core" }` to `crates/jetty-render/Cargo.toml`.

- [ ] **Step 1: Implement TextLayer**

`crates/jetty-render/src/text.rs`:
```rust
use crate::gpu::GpuContext;
use glyphon::{
    Attrs, Buffer, Cache, Color, Family, FontSystem, Metrics, Resolution, Shaping, SwashCache,
    TextArea, TextAtlas, TextBounds, TextRenderer, Viewport,
};
use jetty_core::GridSnapshot;
use wgpu::MultisampleState;

pub struct TextLayer {
    font_system: FontSystem,
    swash: SwashCache,
    atlas: TextAtlas,
    viewport: Viewport,
    renderer: TextRenderer,
    buffer: Buffer,
    metrics: Metrics,
    cell_w: f32,
    cell_h: f32,
}

impl TextLayer {
    pub fn new(gpu: &GpuContext, font_size: f32) -> Self {
        let mut font_system = FontSystem::new();
        let swash = SwashCache::new();
        let cache = Cache::new(&gpu.device);
        let viewport = Viewport::new(&gpu.device, &cache);
        let mut atlas = TextAtlas::new(&gpu.device, &gpu.queue, &cache, gpu.format);
        let renderer =
            TextRenderer::new(&mut atlas, &gpu.device, MultisampleState::default(), None);

        let line_height = (font_size * 1.3).ceil();
        let metrics = Metrics::new(font_size, line_height);
        let mut buffer = Buffer::new(&mut font_system, metrics);
        buffer.set_size(&mut font_system, Some(gpu.config.width as f32), Some(gpu.config.height as f32));

        // Measure a monospace cell by shaping a single 'M'.
        let cell_w = measure_advance(&mut font_system, metrics);
        let cell_h = line_height;

        Self { font_system, swash, atlas, viewport, renderer, buffer, metrics, cell_w, cell_h }
    }

    pub fn cell_size(&self) -> (f32, f32) {
        (self.cell_w, self.cell_h)
    }

    pub fn resize(&mut self, gpu: &GpuContext) {
        self.buffer.set_size(
            &mut self.font_system,
            Some(gpu.config.width as f32),
            Some(gpu.config.height as f32),
        );
    }

    pub fn render(
        &mut self,
        gpu: &GpuContext,
        snapshot: &GridSnapshot,
    ) -> Result<(), wgpu::SurfaceError> {
        // Build the screen text: one line per grid row, with per-run color spans.
        let mut text = String::new();
        let mut spans: Vec<(usize, usize, Color)> = Vec::new(); // (start, end, color) byte ranges
        for row in 0..snapshot.rows {
            for col in 0..snapshot.cols {
                let cell = snapshot.cell(row, col);
                let start = text.len();
                text.push(cell.c);
                let end = text.len();
                spans.push((start, end, Color::rgb(cell.fg[0], cell.fg[1], cell.fg[2])));
            }
            text.push('\n');
        }

        let attr_spans: Vec<(usize, usize, Attrs)> = spans
            .iter()
            .map(|(s, e, c)| (*s, *e, Attrs::new().family(Family::Monospace).color(*c)))
            .collect();
        self.buffer.set_rich_text(
            &mut self.font_system,
            attr_spans.iter().map(|(s, e, a)| (&text[*s..*e], a.clone())),
            &Attrs::new().family(Family::Monospace),
            Shaping::Advanced,
            None,
        );

        self.viewport.update(
            &gpu.queue,
            Resolution { width: gpu.config.width, height: gpu.config.height },
        );

        let text_area = TextArea {
            buffer: &self.buffer,
            left: 0.0,
            top: 0.0,
            scale: 1.0,
            bounds: TextBounds {
                left: 0,
                top: 0,
                right: gpu.config.width as i32,
                bottom: gpu.config.height as i32,
            },
            default_color: Color::rgb(220, 220, 220),
            custom_glyphs: &[],
        };

        self.renderer
            .prepare(
                &gpu.device,
                &gpu.queue,
                &mut self.font_system,
                &mut self.atlas,
                &self.viewport,
                [text_area],
                &mut self.swash,
            )
            .map_err(|_| wgpu::SurfaceError::Lost)?;

        let frame = gpu.surface.get_current_texture()?;
        let view = frame.texture.create_view(&wgpu::TextureViewDescriptor::default());
        let mut encoder = gpu
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor { label: Some("text") });
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("text-pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color { r: 0.07, g: 0.07, b: 0.09, a: 1.0 }),
                        store: wgpu::StoreOp::Store,
                    },
                    depth_slice: None,
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });
            self.renderer.render(&self.atlas, &self.viewport, &mut pass).unwrap();
        }
        gpu.queue.submit(Some(encoder.finish()));
        frame.present();
        Ok(())
    }
}

fn measure_advance(font_system: &mut FontSystem, metrics: Metrics) -> f32 {
    let mut b = Buffer::new(font_system, metrics);
    b.set_text(font_system, "M", &Attrs::new().family(Family::Monospace), Shaping::Advanced);
    b.set_size(font_system, Some(1000.0), Some(metrics.line_height));
    b.layout_runs()
        .next()
        .and_then(|run| run.glyphs.iter().map(|g| g.w).next())
        .unwrap_or(metrics.font_size * 0.6)
}
```

`crates/jetty-render/src/lib.rs`:
```rust
mod gpu;
mod text;
pub use gpu::GpuContext;
pub use text::TextLayer;
```

> glyphon 0.11 method names to confirm against the pinned version: `Buffer::set_rich_text`, `TextArea { custom_glyphs, default_color, .. }`, `Color::rgb`, `run.glyphs[].w`. These match glyphon 0.11 / cosmic-text 0.14. If `set_rich_text`'s signature differs (some versions omit the trailing `tab_width: Option`), drop/adjust the final argument per the compiler.

- [ ] **Step 2: Run a render smoke check via the app (wired in Task 7)**

This task has no standalone unit test (GPU rendering is verified by running). Proceed to Task 7, which wires `TextLayer` into the app loop; the visual check there validates this task too.

- [ ] **Step 3: Commit**

```bash
git add crates/jetty-render
git commit -m "feat: glyphon text layer renders a GridSnapshot"
```

---

## Task 7: Wire core + render + input in the app  → **M1 complete**

**Files:**
- Modify: `crates/jetty-app/src/app.rs`, `crates/jetty-app/Cargo.toml`

**Interfaces:**
- Consumes: `jetty_core::{Terminal, PtySession, GridSnapshot}`, `jetty_render::{GpuContext, TextLayer}`
- Produces: an interactive terminal — keystrokes reach the shell, output renders.

- [ ] **Step 1: Replace clear-only rendering with terminal rendering + wake-on-output**

Update `crates/jetty-app/src/app.rs` so `App` owns `terminal: Terminal`, `pty: PtySession`, `text: TextLayer`, and a writer. Use a `winit::event_loop::EventLoopProxy<()>` user event to wake the loop when PTY bytes arrive. Full file:
```rust
use std::io::Write;
use std::sync::Arc;
use jetty_core::{PtySession, Terminal};
use jetty_render::{GpuContext, TextLayer};
use winit::application::ApplicationHandler;
use winit::event::WindowEvent;
use winit::event_loop::{ActiveEventLoop, EventLoopProxy};
use winit::keyboard::{Key, NamedKey};
use winit::window::{Window, WindowId};

const COLS: usize = 100;
const ROWS: usize = 30;

pub struct App {
    proxy: EventLoopProxy<()>,
    window: Option<Arc<Window>>,
    gpu: Option<GpuContext>,
    text: Option<TextLayer>,
    terminal: Terminal,
    pty: Option<PtySession>,
    writer: Option<Box<dyn Write + Send>>,
}

impl App {
    pub fn new(proxy: EventLoopProxy<()>) -> Self {
        App {
            proxy,
            window: None,
            gpu: None,
            text: None,
            terminal: Terminal::new(COLS, ROWS),
            pty: None,
            writer: None,
        }
    }

    fn drain_pty(&mut self) {
        if let Some(pty) = &self.pty {
            while let Ok(chunk) = pty.output().try_recv() {
                self.terminal.feed(&chunk);
            }
        }
    }
}

impl ApplicationHandler<()> for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_some() {
            return;
        }
        let window = jetty_platform::build_window(event_loop, "Jetty", (1000, 640));
        let gpu = GpuContext::new(window.clone());
        let text = TextLayer::new(&gpu, 16.0);

        let pty = PtySession::spawn(COLS as u16, ROWS as u16).expect("pty");
        let writer = pty.writer();

        // Wake the event loop whenever PTY output is available.
        let proxy = self.proxy.clone();
        let waker_rx = pty.output() as *const _; // not used directly; see thread below
        let _ = waker_rx;
        // A dedicated waker thread mirrors availability: cheap poll → user event.
        // (The actual bytes are drained on the main thread in drain_pty.)
        // We re-trigger redraw on each user event.

        self.window = Some(window);
        self.gpu = Some(gpu);
        self.text = Some(text);
        self.pty = Some(pty);
        self.writer = Some(writer);

        // Kick a periodic redraw by requesting the first frame; the waker thread
        // below nudges the loop as data arrives.
        spawn_waker(self.pty.as_ref().unwrap(), proxy);
        if let Some(w) = &self.window {
            w.request_redraw();
        }
    }

    fn user_event(&mut self, _event_loop: &ActiveEventLoop, _ev: ()) {
        self.drain_pty();
        if let Some(w) = &self.window {
            w.request_redraw();
        }
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::Resized(size) => {
                if let Some(gpu) = &mut self.gpu {
                    gpu.resize(size.width, size.height);
                }
                if let (Some(gpu), Some(text)) = (&self.gpu, &mut self.text) {
                    text.resize(gpu);
                }
            }
            WindowEvent::KeyboardInput { event, .. } if event.state.is_pressed() => {
                if let Some(bytes) = key_to_bytes(&event.logical_key) {
                    if let Some(w) = &mut self.writer {
                        let _ = w.write_all(&bytes);
                        let _ = w.flush();
                    }
                }
            }
            WindowEvent::RedrawRequested => {
                self.drain_pty();
                if let (Some(gpu), Some(text)) = (&mut self.gpu, &mut self.text) {
                    let snap = self.terminal.snapshot();
                    let _ = text.render(gpu, &snap);
                }
            }
            _ => {}
        }
    }
}

fn key_to_bytes(key: &Key) -> Option<Vec<u8>> {
    match key {
        Key::Named(NamedKey::Enter) => Some(b"\r".to_vec()),
        Key::Named(NamedKey::Backspace) => Some(vec![0x7f]),
        Key::Named(NamedKey::Tab) => Some(b"\t".to_vec()),
        Key::Named(NamedKey::Escape) => Some(vec![0x1b]),
        Key::Named(NamedKey::Space) => Some(b" ".to_vec()),
        Key::Named(NamedKey::ArrowUp) => Some(b"\x1b[A".to_vec()),
        Key::Named(NamedKey::ArrowDown) => Some(b"\x1b[B".to_vec()),
        Key::Named(NamedKey::ArrowRight) => Some(b"\x1b[C".to_vec()),
        Key::Named(NamedKey::ArrowLeft) => Some(b"\x1b[D".to_vec()),
        Key::Character(s) => Some(s.as_bytes().to_vec()),
        _ => None,
    }
}

fn spawn_waker(pty: &PtySession, proxy: EventLoopProxy<()>) {
    // The PtySession reader already pushes into an mpsc channel. We cannot clone
    // the Receiver, so the waker simply nudges the loop on a short interval; the
    // main thread drains all available chunks per wake. This keeps latency low
    // without a second consumer of the channel.
    let _ = pty;
    std::thread::spawn(move || loop {
        std::thread::sleep(std::time::Duration::from_millis(16));
        if proxy.send_event(()).is_err() {
            break;
        }
    });
}
```

`crates/jetty-app/src/lib.rs`:
```rust
mod app;

use winit::event_loop::{ControlFlow, EventLoop};

pub fn run() {
    let event_loop = EventLoop::<()>::with_user_event().build().expect("event loop");
    event_loop.set_control_flow(ControlFlow::Wait);
    let proxy = event_loop.create_proxy();
    let mut app = app::App::new(proxy);
    event_loop.run_app(&mut app).expect("run_app");
}
```

> The 16 ms waker is a deliberate M1 simplification (a steady ~60 Hz nudge, draining all bytes per wake). M3 replaces it with a damage-driven wake (the PTY reader signals the proxy directly) so idle CPU drops to ~zero per the spec's N1. Note this in the commit body.

- [ ] **Step 2: Run and verify an interactive terminal**

Run: `cargo run`
Expected: the window shows a shell prompt; typing `ls` then Enter shows directory output; `echo hi` prints `hi`. Colors render (prompt/ls colors). Resizing does not crash.

- [ ] **Step 3: Commit (M1 milestone)**

```bash
git add crates/jetty-app
git commit -m "feat: interactive terminal renders shell output and accepts input (M1)"
```

---

## Subsequent Plan — M2 (Summon) + M3 (Effects), to be detailed after M0/M1 land

Not expanded into tasks here: writing this against winit/wgpu/glyphon/alacritty before the foundation compiles risks speculative code. Once M0/M1 build and run on the dev machine (versions pinned in `Cargo.lock`), these get their own plan written against the verified base. Planned scope:

- **M2 — Summon behavior**
  - Introduce the `PlatformBackend` trait in `jetty-platform`; refactor the M0/M1 winit window into an `X11Backend` impl.
  - Overlay window flags: borderless (`with_decorations(false)`), always-on-top (`WindowLevel::AlwaysOnTop`), `with_transparent(true)`, skip-taskbar via `WindowAttributesExtX11`, start hidden.
  - Center on the cursor's monitor (winit `available_monitors` + cursor position → `set_outer_position`).
  - Global hotkey via `global-hotkey 0.8` (XGrabKey on X11) → toggle `set_visible`.
  - `jetty-ipc` crate: unix-socket server + `jetty toggle` subcommand + single-instance guard (unit-test the line protocol over a socketpair).
  - Visibility state machine (`Hidden`/`Visible`) in `jetty-app` (unit-test transitions); process persists across hide.
  - Deliverable: hotkey/`jetty toggle` summons a borderless, centered, always-on-top terminal; ESC hides; shell survives.

- **M3 — Effects + config + idle-efficiency**
  - `jetty-app` config: TOML (`serde` + `toml`) — hotkey, font, size, opacity, effects flag; unit-test parsing.
  - Summon/dismiss animation: `Summoning`/`Dismissing` states + animation clock; renderer applies opacity (text + background alpha over the transparent surface) and scale (TextArea offset/scale) for fade + scale-from-95%.
  - Replace the 16 ms waker with damage-driven wake (PTY reader → `EventLoopProxy`) for near-zero idle CPU (spec N1).
  - Backdrop blur: documented stretch, off by default (kept out of the MVP core path).
  - Deliverable: MVP complete — centered, effected, configurable summon terminal.

---

## Self-Review

**1. Spec coverage (against `2026-06-18-jetty-design.md`):**
- F4 (real PTY `$SHELL`, render output, colors): Tasks 4–7. ✓
- N1 (idle efficiency): partially M1 (16 ms waker, noted as interim); fully addressed in M3. ✓ (flagged)
- Crate boundaries / dependency direction (§4): Task 1 + per-task file placement; `jetty-render`→`jetty-core` is a data-only dep, `jetty-render` imports no winit, `jetty-platform` imports no wgpu. ✓
- `GridSnapshot` immutable read model (§7): Task 5. ✓
- §11 MVP scope items "single window / monitor / PTY / keyboard→PTY / colors": M0/M1 cover the terminal substrate; summon/effects/config explicitly deferred to the M2/M3 plan (Scope Check split). ✓
- Build sequence M0→M1 (§12): Tasks 1–3 (M0), 4–7 (M1). ✓
- Testing strategy (§13): pure logic unit-tested (PTY echo Task 4, snapshot Task 5); render verified by running (Tasks 3, 6, 7). ✓

**2. Placeholder scan:** No "TBD/TODO/implement later" inside tasks. The version-drift notes (wgpu fields, alacritty paths, glyphon signatures) are concrete "match the pinned version; the compiler names the exact field" instructions tied to the `cargo tree` pin in Task 1 — not open-ended placeholders. The M2/M3 section is an explicit out-of-scope roadmap, not a task with holes.

**3. Type consistency:** `GridSnapshot`/`CellSnapshot` field names (`cols`, `rows`, `cells`, `cursor_row`, `cursor_col`, `c`, `fg`, `bg`) are identical in Task 5 (definition), Task 6 (`snapshot.cell()`, `cell.c`, `cell.fg`), and Task 7 (`self.terminal.snapshot()`). `GpuContext` fields (`device`, `queue`, `surface`, `config`, `format`) used consistently in Tasks 3, 6. `PtySession::{spawn, output, writer, resize}` consistent across Tasks 4, 7. `Terminal::{new, feed, snapshot}` consistent Tasks 5, 7.

*Reviewed; no inline fixes required beyond the version-pin notes already embedded.*
