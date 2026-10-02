//! Frame pacing: a monotonic-clock scheduler (sleep, then spin for the last stretch) and the
//! audio-clocked period adjustment. The arithmetic lives in pure functions so it can be tested.

use std::time::{Duration, Instant};

/// Scale `base` so that the audio ring drifts back toward `target` frames.
///
/// A ring above target means the emulator runs fast relative to the DAC, so the period grows;
/// below target it shrinks. The adjustment is proportional to the relative error and clamped to
/// `+-max_adjust`. Because the fill level integrates the rate error, plain proportional control is
/// stable and converges without oscillation.
pub fn paced_period(base: Duration, fill: usize, target: usize, max_adjust: f64) -> Duration {
    if target == 0 {
        return base;
    }
    let err = (fill as f64 - target as f64) / target as f64;
    let factor = 1.0 + err.clamp(-1.0, 1.0) * max_adjust;
    Duration::from_secs_f64(base.as_secs_f64() * factor)
}

/// Frames the loop is behind schedule before it stops trying to catch up and re-anchors.
pub const MAX_CATCHUP_FRAMES: u32 = 4;

/// Sleeps until `deadline`, waking `margin` early and spinning the rest. Returns the oversleep
/// (how far past the sleep's wake target the OS actually woke us), used to adapt the margin.
pub fn sleep_until(deadline: Instant, margin: Duration) -> Duration {
    let mut oversleep = Duration::ZERO;
    let now = Instant::now();
    if deadline > now + margin {
        let wake = deadline - margin;
        std::thread::sleep(wake - now);
        let after = Instant::now();
        if after > wake {
            oversleep = after - wake;
        }
    }
    while Instant::now() < deadline {
        std::hint::spin_loop();
    }
    oversleep
}

/// Runs the loop at one iteration per `period`, correcting for the OS timer's granularity.
pub struct Pacer {
    next: Instant,
    /// How early to leave `thread::sleep` before spinning. Adapted to observed oversleep.
    margin: Duration,
    /// Frames that ran late (deadline already past when `wait` was called).
    pub late_frames: u64,
}

impl Pacer {
    pub const MIN_MARGIN: Duration = Duration::from_micros(800);
    pub const MAX_MARGIN: Duration = Duration::from_millis(16);

    pub fn new() -> Self {
        Pacer { next: Instant::now(), margin: Duration::from_micros(1500), late_frames: 0 }
    }

    /// Forget the schedule (after a pause or fast-forward) so the next frame runs immediately.
    pub fn reset(&mut self) {
        self.next = Instant::now();
    }

    pub fn margin(&self) -> Duration {
        self.margin
    }

    /// Block until the next frame slot, then advance the schedule by `period`.
    pub fn wait(&mut self, period: Duration) {
        let now = Instant::now();
        if now > self.next {
            self.late_frames += 1;
            if now - self.next > period * MAX_CATCHUP_FRAMES {
                // Hopelessly behind (e.g. window drag): re-anchor instead of racing to catch up.
                self.next = now;
            }
        }
        let oversleep = sleep_until(self.next, self.margin);
        self.margin = next_margin(self.margin, oversleep);
        self.next += period;
    }
}

/// Adapt the sleep margin: jump up to cover an observed oversleep, decay slowly otherwise.
pub fn next_margin(margin: Duration, oversleep: Duration) -> Duration {
    let needed = oversleep + Duration::from_micros(200);
    let m = if needed > margin { needed } else { margin.mul_f64(0.995) };
    m.clamp(Pacer::MIN_MARGIN, Pacer::MAX_MARGIN)
}

/// Emulated frames per wall second, sampled over a window.
pub struct FpsCounter {
    frames: u32,
    window_start: Instant,
    pub fps: f64,
}

impl FpsCounter {
    pub fn new() -> Self {
        FpsCounter { frames: 0, window_start: Instant::now(), fps: 0.0 }
    }

    pub fn tick(&mut self) {
        self.frames += 1;
        let dt = self.window_start.elapsed();
        if dt >= Duration::from_secs(1) {
            self.fps = self.frames as f64 / dt.as_secs_f64();
            self.frames = 0;
            self.window_start = Instant::now();
        }
    }

    pub fn reset(&mut self) {
        self.frames = 0;
        self.window_start = Instant::now();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn paced_period_direction_and_clamp() {
        let base = Duration::from_millis(100);
        assert_eq!(paced_period(base, 500, 500, 0.02), base, "on target: unchanged");
        let slow = paced_period(base, 750, 500, 0.02);
        assert!(slow > base, "ring above target slows down");
        assert!((slow.as_secs_f64() - 0.101).abs() < 1e-9, "half over -> +1%");
        let fast = paced_period(base, 250, 500, 0.02);
        assert!((fast.as_secs_f64() - 0.099).abs() < 1e-9, "half under -> -1%");
        // Clamped at +-2% no matter how far off.
        assert!((paced_period(base, 100_000, 500, 0.02).as_secs_f64() - 0.102).abs() < 1e-9);
        assert!((paced_period(base, 0, 500, 0.02).as_secs_f64() - 0.098).abs() < 1e-9);
        assert_eq!(paced_period(base, 123, 0, 0.02), base, "no target: unchanged");
    }

    #[test]
    fn margin_adapts() {
        let m = Duration::from_millis(1);
        assert_eq!(next_margin(m, Duration::from_millis(3)), Duration::from_micros(3200));
        let decayed = next_margin(Duration::from_millis(2), Duration::ZERO);
        assert!(decayed < Duration::from_millis(2) && decayed > Duration::from_micros(1900));
        assert_eq!(next_margin(Pacer::MIN_MARGIN, Duration::ZERO), Pacer::MIN_MARGIN);
        assert_eq!(next_margin(m, Duration::from_secs(1)), Pacer::MAX_MARGIN);
    }

    #[test]
    fn pacer_holds_average_period() {
        let mut p = Pacer::new();
        let period = Duration::from_millis(5);
        let t0 = Instant::now();
        let n = 20;
        for _ in 0..n {
            p.wait(period);
        }
        let elapsed = t0.elapsed();
        // The first wait returns immediately (the schedule starts "now"), so n waits span n-1 periods.
        let expected = period * (n - 1);
        assert!(elapsed >= expected - Duration::from_micros(500), "ran early: {elapsed:?}");
        assert!(elapsed < expected + Duration::from_millis(40), "ran late: {elapsed:?}");
    }

    #[test]
    fn pacer_reanchors_when_far_behind() {
        let mut p = Pacer::new();
        let period = Duration::from_millis(2);
        p.next = Instant::now() - period * 100;
        let t0 = Instant::now();
        p.wait(period);
        assert!(t0.elapsed() < Duration::from_millis(50));
        assert_eq!(p.late_frames, 1);
        assert!(p.next > t0, "schedule re-anchored to now + period");
    }
}
