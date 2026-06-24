use portable_pty::{native_pty_system, CommandBuilder, PtySize};
use std::io::{Read, Write};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{channel, Receiver};
use std::sync::{Arc, Mutex};

pub struct PtySession {
    master: Arc<Mutex<Box<dyn portable_pty::MasterPty + Send>>>,
    rx: Receiver<Vec<u8>>,
    exited: Arc<AtomicBool>,
    _child: Box<dyn portable_pty::Child + Send + Sync>,
}

impl PtySession {
    /// Spawn a PTY running the user's shell.
    ///
    /// `on_data` is called from the reader thread every time a chunk of bytes
    /// arrives from the PTY (and once more on EOF/error). Use it to wake the
    /// application's event loop immediately so query replies (DSR/DA/etc.) are
    /// sent back to the shell within ~1ms instead of waiting for a polling tick.
    pub fn spawn(cols: u16, rows: u16, on_data: impl Fn() + Send + 'static) -> std::io::Result<PtySession> {
        let pty_system = native_pty_system();
        let pair = pty_system
            .openpty(PtySize { rows, cols, pixel_width: 0, pixel_height: 0 })
            .map_err(|e| std::io::Error::other(e.to_string()))?;

        let shell = std::env::var("SHELL").unwrap_or_else(|_| "/bin/bash".to_string());
        let mut cmd = CommandBuilder::new(shell);
        // Advertise a capable terminal so shells (and prompts like p10k) run
        // their capability probes and emit truecolor; without TERM set, those
        // capability checks fail and the prompt renders the red "x".
        cmd.env("TERM", "xterm-256color");
        cmd.env("COLORTERM", "truecolor");
        // Disable macOS's shell-session save/restore (/etc/zshrc writes
        // ~/.zsh_sessions/<id>.session and sources it on the next launch). A
        // window-close can interrupt the save, leaving a malformed file that the
        // next shell tries to run — e.g. `command not found: Saving`. JeTTY is a
        // quick-summon terminal; session restore isn't wanted. Harmless/ignored
        // on Linux, so set it unconditionally (no platform-specific code).
        cmd.env("SHELL_SESSIONS_DISABLE", "1");
        let child = pair
            .slave
            .spawn_command(cmd)
            .map_err(|e| std::io::Error::other(e.to_string()))?;
        drop(pair.slave);

        let mut reader = pair
            .master
            .try_clone_reader()
            .map_err(|e| std::io::Error::other(e.to_string()))?;
        let (tx, rx) = channel::<Vec<u8>>();
        let exited = Arc::new(AtomicBool::new(false));
        let exited_reader = Arc::clone(&exited);
        std::thread::spawn(move || {
            let mut buf = [0u8; 8192];
            loop {
                match reader.read(&mut buf) {
                    Ok(0) | Err(_) => {
                        // EOF or error means the shell closed the PTY (the user
                        // pressed Ctrl+D or ran `exit`). Flag it BEFORE waking the
                        // app so its post-drain child-exit check sees it and closes
                        // the window instead of hanging on a dead shell.
                        exited_reader.store(true, Ordering::SeqCst);
                        on_data();
                        break;
                    }
                    Ok(n) => {
                        if tx.send(buf[..n].to_vec()).is_err() {
                            break;
                        }
                        // Wake the app IMMEDIATELY so drain_pty runs and any
                        // query replies (\\e[6n CPR etc.) are written back to
                        // the shell within ~1ms, well inside p10k's timeout.
                        on_data();
                    }
                }
            }
        });

        Ok(PtySession {
            master: Arc::new(Mutex::new(pair.master)),
            rx,
            exited,
            _child: child,
        })
    }

    pub fn output(&self) -> &Receiver<Vec<u8>> {
        &self.rx
    }

    /// Whether the shell child has exited — the reader thread saw EOF/error on
    /// the PTY master (Ctrl+D / `exit`). The app polls this after draining the
    /// output to close the window instead of freezing on a dead shell.
    pub fn child_exited(&self) -> bool {
        self.exited.load(Ordering::SeqCst)
    }

    /// Returns a writer for the PTY master (send keystrokes to the shell).
    ///
    /// # Panics
    /// `take_writer()` is one-shot: this may only be called ONCE per `PtySession`.
    /// A second call panics.
    pub fn writer(&self) -> Box<dyn Write + Send> {
        self.master.lock().unwrap().take_writer().expect("writer() can only be called once per PtySession")
    }

    pub fn resize(&self, cols: u16, rows: u16) {
        let _ = self.master.lock().unwrap().resize(PtySize {
            rows,
            cols,
            pixel_width: 0,
            pixel_height: 0,
        });
    }
}
