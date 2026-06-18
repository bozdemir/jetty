use std::io::Write;
use std::sync::Arc;
use jetty_core::{PtySession, Terminal};
use jetty_render::{GpuContext, TextLayer};
use winit::application::ApplicationHandler;
use winit::event::WindowEvent;
use winit::event_loop::{ActiveEventLoop, EventLoopProxy};
use winit::event::MouseScrollDelta;
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
    /// Index into jetty_core::theme::PRESETS for the current theme.
    theme_idx: usize,
    /// Background opacity (0.0..=1.0); modifies theme bg alpha at runtime.
    opacity: f32,
    /// Track held modifier keys so Ctrl+Shift combos can be detected.
    modifiers: winit::keyboard::ModifiersState,
}

impl App {
    pub fn new(proxy: EventLoopProxy<()>) -> Self {
        // Resolve initial theme index from JETTY_THEME env var.
        let theme_name = std::env::var("JETTY_THEME").unwrap_or_default();
        let theme_idx = jetty_core::theme::PRESETS
            .iter()
            .position(|&n| n == theme_name.as_str())
            .unwrap_or(0);

        // Resolve initial opacity from JETTY_OPACITY env var.
        let opacity = std::env::var("JETTY_OPACITY")
            .ok()
            .and_then(|s| s.parse::<f32>().ok())
            .map(|v| v.clamp(0.0, 1.0))
            .unwrap_or(1.0);

        let mut app = App {
            proxy,
            window: None,
            gpu: None,
            text: None,
            terminal: Terminal::new(COLS, ROWS),
            pty: None,
            writer: None,
            theme_idx,
            opacity,
            modifiers: winit::keyboard::ModifiersState::empty(),
        };
        // Apply the initial theme+opacity so Terminal::new env defaults are
        // overridden by our managed state (avoids double-reads from env).
        app.apply_theme();
        app
    }

    /// Build the current theme from `theme_idx`, apply `opacity` to its bg
    /// alpha, and push it into the terminal.
    fn apply_theme(&mut self) {
        let mut t = jetty_core::Theme::by_name(jetty_core::theme::PRESETS[self.theme_idx]);
        t.bg[3] = (self.opacity.clamp(0.0, 1.0) * 255.0) as u8;
        self.terminal.set_theme(t);
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
        let size = window.inner_size();
        let gpu = GpuContext::new(window.clone(), size.width, size.height);
        let text = TextLayer::new(&gpu.device, &gpu.queue, gpu.format, 16.0);

        let pty = PtySession::spawn(COLS as u16, ROWS as u16).expect("pty");
        let writer = pty.writer();

        self.window = Some(window);
        self.gpu = Some(gpu);
        self.text = Some(text);
        self.pty = Some(pty);
        self.writer = Some(writer);

        spawn_waker(self.proxy.clone());
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
            WindowEvent::ModifiersChanged(m) => {
                self.modifiers = m.state();
            }
            WindowEvent::MouseWheel { delta, .. } => {
                // Positive y = wheel up = scroll into history (older output).
                let lines = match delta {
                    MouseScrollDelta::LineDelta(_, y) => (y.round() as i32) * 3,
                    MouseScrollDelta::PixelDelta(p) => {
                        // Approximate cell height; use 20.0 as a reasonable default.
                        const CELL_H: f64 = 20.0;
                        (p.y / CELL_H).round() as i32
                    }
                };
                if lines != 0 {
                    self.terminal.scroll_lines(lines);
                    if let Some(w) = &self.window {
                        w.request_redraw();
                    }
                }
            }
            WindowEvent::KeyboardInput { event, .. } if event.state.is_pressed() => {
                // --- Ctrl+Shift hotkeys (theme switch + opacity adjust) ---
                // Must be checked BEFORE PageUp/Down and before key_to_bytes so
                // the intercept key is not also forwarded to the PTY.
                if self.modifiers.control_key() && self.modifiers.shift_key() {
                    match &event.logical_key {
                        Key::Character(s) if s == "t" || s == "T" => {
                            self.theme_idx = (self.theme_idx + 1)
                                % jetty_core::theme::PRESETS.len();
                            self.apply_theme();
                            if let Some(w) = &self.window {
                                w.request_redraw();
                            }
                            return;
                        }
                        Key::Character(s) if s == "+" || s == "=" => {
                            self.opacity = (self.opacity + 0.05).min(1.0);
                            self.apply_theme();
                            if let Some(w) = &self.window {
                                w.request_redraw();
                            }
                            return;
                        }
                        Key::Character(s) if s == "-" || s == "_" => {
                            self.opacity = (self.opacity - 0.05).max(0.1);
                            self.apply_theme();
                            if let Some(w) = &self.window {
                                w.request_redraw();
                            }
                            return;
                        }
                        _ => {}
                    }
                }

                // PageUp/Down scroll the terminal without sending to the PTY.
                match &event.logical_key {
                    Key::Named(NamedKey::PageUp) => {
                        self.terminal.scroll_page(true);
                        if let Some(w) = &self.window {
                            w.request_redraw();
                        }
                        return;
                    }
                    Key::Named(NamedKey::PageDown) => {
                        self.terminal.scroll_page(false);
                        if let Some(w) = &self.window {
                            w.request_redraw();
                        }
                        return;
                    }
                    _ => {}
                }
                if let Some(bytes) = key_to_bytes(&event.logical_key) {
                    // Any real keystroke jumps back to the bottom so the user sees their input.
                    self.terminal.scroll_to_bottom();
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

fn spawn_waker(proxy: EventLoopProxy<()>) {
    std::thread::spawn(move || loop {
        std::thread::sleep(std::time::Duration::from_millis(16));
        if proxy.send_event(()).is_err() {
            break;
        }
    });
}
