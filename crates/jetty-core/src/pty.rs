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
        std::thread::spawn(move || {
            let mut buf = [0u8; 8192];
            loop {
                match reader.read(&mut buf) {
                    Ok(0) | Err(_) => {
                        // EOF or error — notify the app so it can react (e.g.
                        // detect child exit) without waiting for the next tick.
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
            _child: child,
        })
    }

    pub fn output(&self) -> &Receiver<Vec<u8>> {
        &self.rx
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
