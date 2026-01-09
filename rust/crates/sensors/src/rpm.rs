//! RPM (Engine Speed) Processor
//!
//! Period-based RPM calculation with spike rejection and asymmetric filtering.

/// RPM processor for tachometer signal
///
/// Calculates engine RPM from pulse period with:
/// - Noise rejection for invalid pulses
/// - Spike rejection for >2000 RPM changes
/// - Asymmetric filtering (instant accel, damped decel)
/// - Timeout detection for engine stop
pub struct RpmProcessor {
    // Filtered RPM value
    rpm_filtered: f32,

    // Last update timestamp for deceleration damping
    last_update_us: u32,

    // Configuration
    pulses_per_rev: u32,
    max_rpm: f32,
    decel_rpm_per_sec: f32,
}

impl RpmProcessor {
    /// Minimum pulse period (microseconds) for noise rejection
    const MIN_PULSE_US: u32 = 3000; // ~375 RPM floor

    /// Maximum pulse period (microseconds) - upper bound sanity
    const MAX_PULSE_US: u32 = 80_000; // ~375 RPM

    /// Spike rejection threshold (RPM)
    const SPIKE_THRESHOLD: f32 = 2000.0;

    /// Create a new RPM processor
    ///
    /// # Arguments
    /// * `pulses_per_rev` - Number of tach pulses per engine revolution
    ///   (typically 2 for Subaru, 1 for many others)
    /// * `max_rpm` - Maximum RPM clamp (typically 9000)
    /// * `decel_rpm_per_sec` - Deceleration damping rate (typically 22000)
    ///
    /// # Example
    /// ```ignore
    /// let mut rpm = RpmProcessor::new(2, 9000.0, 22000.0);
    /// ```
    pub const fn new(pulses_per_rev: u32, max_rpm: f32, decel_rpm_per_sec: f32) -> Self {
        Self {
            rpm_filtered: 0.0,
            last_update_us: 0,
            pulses_per_rev,
            max_rpm,
            decel_rpm_per_sec,
        }
    }

    /// Update RPM calculation
    ///
    /// # Arguments
    /// * `period_us` - Time between pulses in microseconds (from ISR)
    /// * `now_us` - Current timestamp in microseconds
    /// * `timeout_ms` - Timeout for declaring engine stopped (typically 400ms)
    /// * `last_pulse_ms` - Timestamp of last pulse (milliseconds)
    ///
    /// # Returns
    /// Current RPM value
    pub fn update(
        &mut self,
        period_us: u32,
        now_us: u32,
        timeout_ms: u32,
        last_pulse_ms: u32,
    ) -> f32 {
        let now_ms = now_us / 1000;

        // Check for engine stop timeout
        if now_ms.wrapping_sub(last_pulse_ms) > timeout_ms {
            self.rpm_filtered = 0.0;
            self.last_update_us = 0;
            return 0.0;
        }

        // Check for valid period
        if period_us == 0 || period_us < Self::MIN_PULSE_US || period_us > Self::MAX_PULSE_US {
            return self.rpm_filtered;
        }

        // Calculate raw RPM from period
        // RPM = (60,000,000 µs/min) / (period_us × pulses_per_rev)
        let mut rpm_raw = 60_000_000.0 / (period_us as f32 * self.pulses_per_rev as f32);

        // Hard clamp to valid range
        if rpm_raw < 0.0 {
            rpm_raw = 0.0;
        }
        if rpm_raw > self.max_rpm {
            rpm_raw = self.max_rpm;
        }

        // Spike rejection (if we have a previous value)
        if self.rpm_filtered > 0.0 {
            let delta = (rpm_raw - self.rpm_filtered).abs();
            if delta > Self::SPIKE_THRESHOLD {
                // Reject spike, return filtered value
                return self.rpm_filtered;
            }
        }

        // Compute real time delta for deceleration damping
        let dt_sec = if self.last_update_us == 0 {
            0.01 // First update, assume 10ms
        } else {
            let dt_us = now_us.wrapping_sub(self.last_update_us);
            (dt_us as f32) * 1e-6
        };
        self.last_update_us = now_us;

        // Clamp dt to reasonable range
        let dt_sec = dt_sec.max(0.0005).min(0.02); // 0.5ms to 20ms

        // Asymmetric filtering
        if rpm_raw >= self.rpm_filtered {
            // Acceleration: immediate response
            self.rpm_filtered = rpm_raw;
        } else {
            // Deceleration: apply damping
            let max_drop = self.decel_rpm_per_sec * dt_sec;
            let delta = rpm_raw - self.rpm_filtered;

            if delta < -max_drop {
                self.rpm_filtered -= max_drop;
            } else {
                self.rpm_filtered = rpm_raw;
            }
        }

        self.rpm_filtered
    }

    /// Get current filtered RPM without updating
    pub fn current_rpm(&self) -> f32 {
        self.rpm_filtered
    }

    /// Reset RPM processor
    pub fn reset(&mut self) {
        self.rpm_filtered = 0.0;
        self.last_update_us = 0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rpm_calculation() {
        let mut rpm = RpmProcessor::new(2, 9000.0, 22000.0);

        // 10ms period, 2 pulses/rev
        // RPM = 60,000,000 / (10,000 × 2) = 3000 RPM
        let result = rpm.update(10_000, 10_000, 400, 10);

        assert!((result - 3000.0).abs() < 1.0);
    }

    #[test]
    fn test_rpm_noise_rejection() {
        let mut rpm = RpmProcessor::new(2, 9000.0, 22000.0);

        // Too short period (< 3ms) should be rejected
        let result = rpm.update(2000, 10_000, 400, 10);
        assert_eq!(result, 0.0);

        // Too long period (> 80ms) should be rejected
        let result = rpm.update(100_000, 10_000, 400, 10);
        assert_eq!(result, 0.0);
    }

    #[test]
    fn test_rpm_spike_rejection() {
        let mut rpm = RpmProcessor::new(2, 9000.0, 22000.0);

        // Establish baseline at 3000 RPM
        rpm.update(10_000, 10_000, 400, 10);
        assert!((rpm.current_rpm() - 3000.0).abs() < 1.0);

        // Try to spike to 6000 RPM (>2000 RPM change)
        // Period would be 5ms for 6000 RPM
        let result = rpm.update(5_000, 20_000, 400, 20);

        // Should reject spike and keep previous value
        assert!((result - 3000.0).abs() < 1.0);
    }

    #[test]
    fn test_rpm_instant_acceleration() {
        let mut rpm = RpmProcessor::new(2, 9000.0, 22000.0);

        // Start at 2000 RPM
        rpm.update(15_000, 10_000, 400, 10); // 15ms period
        let rpm_before = rpm.current_rpm();

        // Accelerate to 2500 RPM (within spike threshold)
        // 2500 RPM = 12ms period
        let result = rpm.update(12_000, 20_000, 400, 20);

        // Should respond instantly to acceleration
        assert!(result > rpm_before);
        assert!((result - 2500.0).abs() < 50.0);
    }

    #[test]
    fn test_rpm_damped_deceleration() {
        let mut rpm = RpmProcessor::new(2, 9000.0, 22000.0);

        // Start at 5000 RPM
        rpm.update(6_000, 10_000, 400, 10);
        let rpm_before = rpm.current_rpm();

        // Decelerate to 3000 RPM (2000 RPM drop, within threshold)
        // 3000 RPM = 10ms period
        // With dt = 10ms and decel rate = 22000 RPM/sec
        // Max drop = 22000 * 0.01 = 220 RPM
        let result = rpm.update(10_000, 20_000, 400, 20);

        // Should only drop by ~220 RPM, not the full 2000
        assert!(result < rpm_before);
        assert!(result > rpm_before - 250.0);
        assert!(result > 3000.0); // Not yet at target
    }

    #[test]
    fn test_rpm_timeout() {
        let mut rpm = RpmProcessor::new(2, 9000.0, 22000.0);

        // Establish RPM at 3000
        rpm.update(10_000, 10_000, 400, 10);
        assert!(rpm.current_rpm() > 0.0);

        // Timeout: last pulse was 500ms ago
        let result = rpm.update(10_000, 600_000, 400, 10);

        assert_eq!(result, 0.0);
        assert_eq!(rpm.current_rpm(), 0.0);
    }

    #[test]
    fn test_rpm_max_clamp() {
        let mut rpm = RpmProcessor::new(2, 9000.0, 22000.0);

        // Period that would give 10000 RPM (exceeds max of 9000)
        // 10000 RPM with 2 pulses/rev = 3000µs period
        // This is above MIN_PULSE_US (3000) so won't be rejected
        let result = rpm.update(3_000, 10_000, 400, 10);

        // Should clamp to max_rpm
        assert!((result - 9000.0).abs() < 100.0, "Got {} RPM, expected ~9000", result);
    }

    #[test]
    fn test_rpm_zero_period() {
        let mut rpm = RpmProcessor::new(2, 9000.0, 22000.0);

        // Zero period should be rejected
        let result = rpm.update(0, 10_000, 400, 10);
        assert_eq!(result, 0.0);
    }
}
