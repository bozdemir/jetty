// Test PTY echoing. Run with SHELL=/bin/cat if the default shell does not echo:
//   SHELL=/bin/cat cargo test -p jetty-core --test pty
use jetty_core::PtySession;
use std::time::{Duration, Instant};

#[test]
fn pty_echoes_written_bytes() {
    let pty = PtySession::spawn(80, 24, || {}).expect("spawn");
    {
        let mut w = pty.writer();
        // cooked PTY echoes typed input back; send a line.
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
