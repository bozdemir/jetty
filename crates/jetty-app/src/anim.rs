//! Lightweight, frame-driven animation primitives.
//!
//! These are intentionally tiny: an [`Animation`] is just a start instant plus a
//! duration, and `progress()` reports a clamped 0..1 fraction of elapsed time.
//! Easing functions shape that linear fraction into the curves the UI wants.
//!
//! The whole system is **idle-free**: nothing here spawns threads, holds timers,
//! or runs on its own. The event loop drives it — while an animation is active it
//! calls `window.request_redraw()` again each frame; once every animation reports
//! `done()`, the loop stops requesting redraws and the app returns to 0 CPU. So
//! the cost of "having an animation system" when nothing is animating is zero.

use std::time::{Duration, Instant};

/// A time-based animation: a start instant and a duration. `progress()` returns
/// the clamped fraction of the duration that has elapsed (0.0 at `start`, 1.0 at
/// `start + dur` and after). Apply an easing function to the result to shape it.
#[derive(Debug, Clone, Copy)]
pub struct Animation {
    pub start: Instant,
    pub dur: Duration,
}

impl Animation {
    /// Start an animation lasting `dur`, beginning now.
    pub fn new(dur: Duration) -> Self {
        Animation { start: Instant::now(), dur }
    }

    /// Start an animation lasting `ms` milliseconds, beginning now.
    pub fn ms(ms: u64) -> Self {
        Animation::new(Duration::from_millis(ms))
    }

    /// Linear progress in `0.0..=1.0`, clamped. Returns 1.0 once the duration has
    /// elapsed (or immediately for a zero-length animation).
    pub fn progress(&self) -> f32 {
        let dur = self.dur.as_secs_f32();
        if dur <= 0.0 {
            return 1.0;
        }
        (self.start.elapsed().as_secs_f32() / dur).clamp(0.0, 1.0)
    }

    /// Whether the animation has finished (its progress has reached 1.0).
    pub fn done(&self) -> bool {
        self.start.elapsed() >= self.dur
    }
}

/// Cubic ease-out: fast start, gentle settle. `t` is clamped to `0..=1`.
/// `1 - (1 - t)^3`.
pub fn ease_out_cubic(t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    let inv = 1.0 - t;
    1.0 - inv * inv * inv
}

/// "Back" ease-out with a small, tasteful overshoot then settle — the spring-like
/// curve used for the center-summon. `t` is clamped to `0..=1`.
///
/// Uses the standard back-easing constants but with a gentle overshoot
/// (`OVERSHOOT = 1.2`, vs the textbook 1.70158) so the bounce reads as "premium"
/// rather than "bouncy/gimmicky".
pub fn ease_out_back(t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    const OVERSHOOT: f32 = 1.2;
    const C1: f32 = OVERSHOOT;
    const C3: f32 = C1 + 1.0;
    let inv = t - 1.0;
    1.0 + C3 * inv * inv * inv + C1 * inv * inv
}

/// Frame-rate-independent exponential smoothing toward `target`.
///
/// Eases `current` toward `target` by a fraction derived from the elapsed time
/// `dt` and a time constant `tau` (the e-folding time, in seconds — smaller =
/// snappier). Critically damped in feel: it approaches the target asymptotically
/// and never overshoots, so a gliding cursor never looks springy/distracting.
///
/// `alpha = 1 - exp(-dt / tau)`; `current + (target - current) * alpha`.
pub fn exp_approach(current: f32, target: f32, dt: f32, tau: f32) -> f32 {
    if tau <= 0.0 || dt <= 0.0 {
        return target;
    }
    let alpha = 1.0 - (-dt / tau).exp();
    current + (target - current) * alpha
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn progress_is_clamped_and_monotonic() {
        let a = Animation { start: Instant::now(), dur: Duration::from_millis(100) };
        let p = a.progress();
        assert!((0.0..=1.0).contains(&p));
        // A zero-length animation is instantly complete.
        let z = Animation { start: Instant::now(), dur: Duration::from_millis(0) };
        assert_eq!(z.progress(), 1.0);
        assert!(z.done());
    }

    #[test]
    fn ease_out_cubic_endpoints() {
        assert!((ease_out_cubic(0.0) - 0.0).abs() < 1e-6);
        assert!((ease_out_cubic(1.0) - 1.0).abs() < 1e-6);
        // Out-of-range clamps.
        assert_eq!(ease_out_cubic(-1.0), 0.0);
        assert_eq!(ease_out_cubic(2.0), 1.0);
        // Eases out: past the halfway point by t=0.5.
        assert!(ease_out_cubic(0.5) > 0.5);
    }

    #[test]
    fn ease_out_back_endpoints_and_overshoot() {
        assert!((ease_out_back(0.0) - 0.0).abs() < 1e-6);
        assert!((ease_out_back(1.0) - 1.0).abs() < 1e-6);
        // It overshoots 1.0 somewhere in the middle-late range, then settles.
        let peak = (1..100)
            .map(|i| ease_out_back(i as f32 / 100.0))
            .fold(0.0_f32, f32::max);
        assert!(peak > 1.0, "expected a gentle overshoot, peak={peak}");
        // ...but only gently (not a cartoon bounce).
        assert!(peak < 1.1, "overshoot should be subtle, peak={peak}");
    }

    #[test]
    fn exp_approach_moves_toward_target_without_overshoot() {
        let mut x = 0.0;
        let target = 10.0;
        for _ in 0..1000 {
            x = exp_approach(x, target, 0.016, 0.09);
            assert!(x <= target + 1e-3, "must never overshoot, x={x}");
        }
        assert!((x - target).abs() < 0.01, "should converge, x={x}");
        // Degenerate inputs snap to target.
        assert_eq!(exp_approach(0.0, 5.0, 0.0, 0.09), 5.0);
        assert_eq!(exp_approach(0.0, 5.0, 0.016, 0.0), 5.0);
    }
}
