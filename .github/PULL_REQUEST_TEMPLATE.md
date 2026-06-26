## What does this PR do?

<!-- One paragraph or bullet list describing the change and why. Reference any
related issue: "Closes #123" or "Part of #456". -->

## Checklist

### Build & tests
- [ ] `cargo build --release --bin jetty` passes (release binary builds cleanly)
- [ ] `cargo test` passes (all workspace tests green)

### Performance (required if you touched the hot path)
- [ ] Ran `cargo run --release -p jetty-app --bin jetty-bench` before and after
      and the numbers did NOT regress (frame render ≤ 6.9 ms, snapshot ≤ 1 ms,
      throughput ≥ 150 MB/s, idle CPU = 0%)

Bench output before: <!-- paste here or "n/a — hot path not touched" -->
Bench output after:  <!-- paste here or "n/a — hot path not touched" -->

### Visual self-test (required if you touched any UI surface)
- [ ] Ran `jetty-shot` (headless render to PNG) to verify the change looks
      correct — screenshot attached below

Screenshot(s): <!-- drag image(s) here or "n/a — no UI change" -->

### Invariants
- [ ] No DE-specific code added (no KDE/GNOME/compositor-specific libraries or
      behaviour branches — everything must work on any X11/Wayland compositor
      and macOS)
- [ ] All 5 themes verified: Catppuccin Mocha, Tokyo Night, Gruvbox Dark,
      Dracula, Onyx — new UI surfaces derive colours from `theme.bg`/`theme.fg`
      (no hardcoded colours)

### Docs
- [ ] `CHANGELOG.md` updated (add an entry under `[Unreleased]` or the next
      version section)
- [ ] Relevant docs updated if behaviour changed (`README.md`, `docs/`,
      `CONTRIBUTING.md`, in-app help overlay in `crates/jetty-render/src/help.rs`)

## Notes for reviewers

<!-- Anything the reviewer should pay special attention to, tradeoffs made,
follow-up work left for later, etc. -->
