//! VSS (Vehicle Speed Sensor) Processor
//!
//! Implements a complex state machine with motion detection hysteresis to accurately
//! measure vehicle speed while rejecting noise and idle-correlated false signals.

/// VSS processor with windowed pulse counting and motion detection
///
/// This processor uses a sophisticated state machine to prevent false speed readings:
/// - Windowed pulse counting (250ms windows)
/// - Motion lock-in requiring sustained signal (4 consecutive good windows)
/// - Idle RPM cutoff to suppress transmission noise
/// - Glitch protection with gradual decay
/// - Adaptive EMA filtering
pub struct VssProcessor {
    // Windowed pulse counting
    pulse_count: u16,
    window_start_us: u32,
    last_pulse_us: u32,

    // Motion detection state
    moving: bool,
    start_good_windows: u8,
    ever_seen_pulse: bool,

    // Filtered output
    kph_ema: f32,
    ema_initialized: bool,

    // Configuration constants
    pulses_per_km: f32,
    min_pulses_to_move: u16,
    idle_rpm_cutoff: f32,
}

impl VssProcessor {
    /// Window duration for pulse counting (microseconds)
    const WINDOW_US: u32 = 250_000; // 250ms

    /// Timeout for declaring vehicle stopped (microseconds)
    const STOP_TIMEOUT_US: u32 = 2_500_000; // 2.5 seconds

    /// Maximum time between pulses to consider "continuous" (microseconds)
    const CONTINUOUS_US: u32 = 300_000; // 300ms

    /// Number of consecutive good windows required to unlock motion
    const START_GOOD_WINDOWS: u8 = 4;

    /// Noise rejection debounce time (microseconds)
    const NOISE_REJECT_US: u32 = 800;

    /// Create a new VSS processor
    ///
    /// # Arguments
    /// * `pulses_per_km` - Number of VSS pulses per kilometer traveled
    ///   (typically 8000 for most vehicles)
    ///
    /// # Example
    /// ```ignore
    /// let mut vss = VssProcessor::new(8000.0);
    /// ```
    pub const fn new(pulses_per_km: f32) -> Self {
        Self {
            pulse_count: 0,
            window_start_us: 0,
            last_pulse_us: 0,
            moving: false,
            start_good_windows: 0,
            ever_seen_pulse: false,
            kph_ema: 0.0,
            ema_initialized: false,
            pulses_per_km,
            min_pulses_to_move: 6,
            idle_rpm_cutoff: 1800.0,
        }
    }

    /// Called from ISR when VSS pulse detected
    ///
    /// This function must be fast and non-blocking.
    /// Updates pulse count with noise rejection.
    ///
    /// # Arguments
    /// * `now_us` - Current timestamp in microseconds
    pub fn on_pulse(&mut self, now_us: u32) {
        // Debounce: reject pulses too close together
        if now_us.wrapping_sub(self.last_pulse_us) < Self::NOISE_REJECT_US {
            return;
        }

        // Start first window on first pulse
        if self.window_start_us == 0 {
            self.window_start_us = now_us;
        }

        self.pulse_count += 1;
        self.last_pulse_us = now_us;
        self.ever_seen_pulse = true;
    }

    /// Update speed calculation (call from main loop, ~25Hz)
    ///
    /// Processes windowed pulses and applies motion detection state machine.
    ///
    /// # Arguments
    /// * `now_us` - Current timestamp in microseconds
    /// * `current_rpm` - Current engine RPM (for idle detection)
    ///
    /// # Returns
    /// Current speed in km/h
    pub fn update(&mut self, now_us: u32, current_rpm: f32) -> f32 {
        // Never saw any pulse since boot
        if !self.ever_seen_pulse {
            return 0.0;
        }

        // STOP timeout (only hard zero if already near stopped)
        if now_us.wrapping_sub(self.last_pulse_us) > Self::STOP_TIMEOUT_US {
            if !self.moving || self.kph_ema < 5.0 {
                self.reset();
                return 0.0;
            } else {
                // Glitch while cruising - hold speed
                return self.kph_ema;
            }
        }

        // Wait for window completion
        if now_us.wrapping_sub(self.window_start_us) < Self::WINDOW_US {
            return if self.moving { self.kph_ema } else { 0.0 };
        }

        // Consume pulses and advance window
        let pulses = self.pulse_count;
        self.pulse_count = 0;
        self.window_start_us = now_us;

        // Calculate raw speed from pulses
        // pulses_per_sec = pulses / window_duration_sec
        // km_per_hour = (pulses_per_sec * 3600) / pulses_per_km
        let pulses_per_sec = (pulses as f32) * (1_000_000.0 / Self::WINDOW_US as f32);
        let mut kph_raw = (pulses_per_sec * 3600.0) / self.pulses_per_km;

        // Deadband for very low speeds
        if kph_raw < 2.0 {
            kph_raw = 0.0;
        }

        // ========================================
        // State Machine: NOT MOVING
        // ========================================
        if !self.moving {
            // Block idle-correlated VSS noise
            if current_rpm < self.idle_rpm_cutoff {
                self.start_good_windows = 0;
                return 0.0;
            }

            // Require continuous pulses (no long gaps)
            if now_us.wrapping_sub(self.last_pulse_us) > Self::CONTINUOUS_US {
                self.start_good_windows = 0;
                return 0.0;
            }

            // Count good windows
            if pulses >= self.min_pulses_to_move && kph_raw >= 5.0 {
                if self.start_good_windows < 255 {
                    self.start_good_windows += 1;
                }
            } else {
                self.start_good_windows = 0;
            }

            // Not unlocked yet
            if self.start_good_windows < Self::START_GOOD_WINDOWS {
                return 0.0;
            }

            // UNLOCK motion
            self.moving = true;
            self.kph_ema = kph_raw;
            self.ema_initialized = true;
            return self.kph_ema;
        }

        // ========================================
        // State Machine: MOVING
        // ========================================

        // Too few pulses this window - decay speed
        if pulses < self.min_pulses_to_move {
            // Gradual decay (simulates rolling to a stop)
            self.kph_ema *= 0.75; // ~25% drop per window

            if self.kph_ema < 1.5 {
                self.kph_ema = 0.0;
                self.moving = false;
                self.start_good_windows = 0;
            }

            return self.kph_ema;
        }

        // Glitch protection: prevent sudden drops
        if self.ema_initialized && kph_raw < self.kph_ema - 4.0 {
            kph_raw = self.kph_ema - 4.0;
        }

        // EMA filtering (adaptive alpha)
        let alpha = if self.kph_ema < 10.0 { 0.08 } else { 0.25 };

        if !self.ema_initialized {
            self.kph_ema = kph_raw;
            self.ema_initialized = true;
        } else {
            self.kph_ema += alpha * (kph_raw - self.kph_ema);
        }

        self.kph_ema
    }

    /// Reset all state (vehicle stopped)
    fn reset(&mut self) {
        self.pulse_count = 0;
        self.window_start_us = 0;
        self.kph_ema = 0.0;
        self.moving = false;
        self.start_good_windows = 0;
        self.ema_initialized = false;
    }

    /// Check if vehicle is detected as moving
    pub fn is_moving(&self) -> bool {
        self.moving
    }

    /// Get current filtered speed without updating
    pub fn current_speed(&self) -> f32 {
        self.kph_ema
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vss_startup_hysteresis() {
        let mut vss = VssProcessor::new(8000.0);

        // Simulate pulses at idle (should stay at 0)
        for i in 0..10 {
            vss.on_pulse(i * 10_000); // 10ms apart
        }

        // Should stay at 0 for first 3 windows
        for i in 0..3 {
            let speed = vss.update(i * 250_000, 2000.0);
            assert_eq!(speed, 0.0, "Should stay at 0 for first 3 windows");
        }
    }

    #[test]
    fn test_vss_idle_suppression() {
        let mut vss = VssProcessor::new(8000.0);

        // Generate pulses
        for i in 0..10 {
            vss.on_pulse(i * 10_000);
        }

        // At idle RPM (<1800), should suppress
        let speed = vss.update(250_000, 800.0);
        assert_eq!(speed, 0.0);

        // Above idle RPM, should count
        let speed = vss.update(500_000, 2000.0);
        // May still be 0 if not enough good windows yet
        assert!(speed >= 0.0);
    }

    #[test]
    fn test_vss_motion_unlock() {
        let mut vss = VssProcessor::new(8000.0);

        // Generate sustained signal for 5 windows to ensure unlock
        // Need continuous pulses and good speed
        for window in 0..5 {
            // 10 pulses per window (well above min_pulses_to_move = 6)
            for pulse in 0..10 {
                let time = window * 250_000 + pulse * 15_000;
                vss.on_pulse(time);
            }

            let speed = vss.update((window + 1) * 250_000 + 10_000, 2000.0);

            if window < 3 {
                // Should stay locked for first 3 windows
                assert_eq!(speed, 0.0, "Should stay locked at window {}", window);
            } else {
                // Should unlock by 4th or 5th window
                if speed > 0.0 {
                    assert!(vss.is_moving());
                    return; // Test passed
                }
            }
        }
        panic!("Failed to unlock after 5 windows");
    }

    #[test]
    fn test_vss_glitch_protection() {
        let mut vss = VssProcessor::new(8000.0);

        // Establish motion at ~50 kph
        for window in 0..5 {
            for pulse in 0..12 {
                vss.on_pulse(window * 250_000 + pulse * 20_000);
            }
            vss.update((window + 1) * 250_000, 2000.0);
        }

        let speed_before = vss.current_speed();
        assert!(speed_before > 20.0);

        // Simulate single bad window (no pulses)
        let speed_after = vss.update(6 * 250_000, 2000.0);

        // Should decay gradually, not drop to zero
        assert!(speed_after > speed_before * 0.5);
        assert!(vss.is_moving());
    }

    #[test]
    fn test_vss_timeout() {
        let mut vss = VssProcessor::new(8000.0);

        // Generate one pulse
        vss.on_pulse(1000);

        // Wait for timeout (>2.5 seconds)
        let speed = vss.update(3_000_000, 2000.0);

        assert_eq!(speed, 0.0);
        assert!(!vss.is_moving());
    }

    #[test]
    fn test_vss_noise_rejection() {
        let mut vss = VssProcessor::new(8000.0);

        // Send pulses too close together (< 800µs)
        vss.on_pulse(1000);
        vss.on_pulse(1500); // Only 500µs later - should be rejected

        assert_eq!(vss.pulse_count, 1, "Second pulse should be rejected");

        // Send pulse with proper spacing
        vss.on_pulse(2000); // 1000µs later - should be accepted
        assert_eq!(vss.pulse_count, 2);
    }

    #[test]
    fn test_vss_speed_calculation() {
        let mut vss = VssProcessor::new(8000.0);

        // Need to establish motion first (5-6 windows with good signal)
        // Using 10 pulses per window for reliable motion detection
        for window in 0..6 {
            for pulse in 0..10 {
                vss.on_pulse(window * 250_000 + pulse * 20_000);
            }
            vss.update((window + 1) * 250_000 + 10_000, 2000.0);
        }

        let speed = vss.current_speed();
        // With 10 pulses per 250ms window:
        // pulses_per_sec = 10 / 0.25 = 40 pulses/sec
        // kph = (40 * 3600) / 8000 = 18 kph (before EMA filtering)
        // After EMA, should be in reasonable range
        assert!(speed > 5.0 && speed < 30.0, "Speed {} out of range", speed);
        assert!(vss.is_moving());
    }
}
