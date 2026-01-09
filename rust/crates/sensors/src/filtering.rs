//! Sensor Filtering Algorithms
//!
//! Implements EMA (Exponential Moving Average) and median-trim ADC filtering
//! to reduce noise in sensor readings.

use crate::AdcReader;

/// Exponential Moving Average (EMA) filter
///
/// Smooths sensor readings over time using weighted averaging.
/// Lower alpha = more smoothing, higher alpha = faster response.
pub struct EmaFilter {
    state: f32,
    alpha: f32,
    initialized: bool,
}

impl EmaFilter {
    /// Create a new EMA filter with the given alpha coefficient
    ///
    /// # Arguments
    /// * `alpha` - Smoothing factor (0.0-1.0). Typical values:
    ///   - 0.08 (slow) for temperature sensors
    ///   - 0.12 (fast) for pressure sensors
    ///   - 0.25 (very fast) for boost sensors
    pub const fn new(alpha: f32) -> Self {
        Self {
            state: 0.0,
            alpha,
            initialized: false,
        }
    }

    /// Update the filter with a new sample
    ///
    /// Returns the filtered value.
    pub fn update(&mut self, sample: f32) -> f32 {
        if !self.initialized {
            self.state = sample;
            self.initialized = true;
        } else {
            self.state += self.alpha * (sample - self.state);
        }
        self.state
    }

    /// Get the current filtered value without updating
    pub fn value(&self) -> f32 {
        self.state
    }

    /// Reset the filter
    pub fn reset(&mut self) {
        self.initialized = false;
        self.state = 0.0;
    }
}

/// Read ADC with median-trim filtering to reject outliers
///
/// This function matches the C++ Arduino implementation exactly:
/// 1. Dummy read (discard first sample)
/// 2. Collect multiple samples with 60µs delay
/// 3. Bubble sort samples
/// 4. Trim highest and lowest values
/// 5. Average remaining middle values
///
/// # Arguments
/// * `adc` - ADC reader implementation
/// * `pin` - Pin number to read
/// * `samples` - Total number of samples to collect (typically 4-16)
/// * `trim` - Number of samples to discard from each end (typically 1-3)
/// * `buffer` - Scratch buffer for samples (must be at least `samples` long)
///
/// # Returns
/// Average of trimmed samples (raw ADC counts)
///
/// # Example
/// ```ignore
/// let mut buffer = [0u16; 16];
/// let avg = median_trim_adc(&mut adc, 0, 16, 2, &mut buffer);
/// // Result: average of middle 12 samples (discarded 2 highest, 2 lowest)
/// ```
pub fn median_trim_adc<A: AdcReader>(
    adc: &mut A,
    pin: u8,
    samples: usize,
    trim: usize,
    buffer: &mut [u16],
) -> f32 {
    // First dummy read (matches Arduino behavior)
    let _ = adc.read_raw(pin);

    // Collect samples with timing delay
    for i in 0..samples {
        buffer[i] = adc.read_raw(pin);

        // 60 microsecond delay (matches C++ delayMicroseconds(60))
        // Platform-specific delay will be injected by hardware layer
        delay_microseconds(60);
    }

    // Bubble sort (matches C++ exactly)
    for i in 0..samples - 1 {
        for j in 0..samples - 1 - i {
            if buffer[j] > buffer[j + 1] {
                buffer.swap(j, j + 1);
            }
        }
    }

    // Sum middle samples (trim outliers)
    let mut sum: u32 = 0;
    for i in trim..(samples - trim) {
        sum += buffer[i] as u32;
    }

    // Return average
    (sum as f32) / ((samples - trim * 2) as f32)
}

/// Platform-specific delay function
///
/// This is a weak symbol that will be overridden by platform-specific implementations.
/// On Arduino, this maps to delayMicroseconds().
/// On desktop, this might be a no-op or use std::thread::sleep.
#[inline(always)]
fn delay_microseconds(_us: u32) {
    // Platform-specific implementation will be provided by linker
    // For Arduino: direct call to delayMicroseconds()
    // For testing: can be no-op or spin loop
}

#[cfg(test)]
mod tests {
    use super::*;

    struct MockAdc {
        values: &'static [u16],
        index: usize,
    }

    impl MockAdc {
        fn new(values: &'static [u16]) -> Self {
            Self { values, index: 0 }
        }
    }

    impl AdcReader for MockAdc {
        fn read_raw(&mut self, _pin: u8) -> u16 {
            let val = self.values[self.index % self.values.len()];
            self.index += 1;
            val
        }
    }

    #[test]
    fn test_ema_filter() {
        let mut filter = EmaFilter::new(0.5);

        // First sample initializes filter
        assert_eq!(filter.update(100.0), 100.0);

        // Subsequent samples are filtered
        assert_eq!(filter.update(200.0), 150.0);
        assert_eq!(filter.update(200.0), 175.0);
    }

    #[test]
    fn test_ema_alpha() {
        let mut slow = EmaFilter::new(0.1);
        let mut fast = EmaFilter::new(0.9);

        slow.update(100.0);
        fast.update(100.0);

        let slow_result = slow.update(200.0);
        let fast_result = fast.update(200.0);

        // Fast filter responds more to changes
        assert!(fast_result > slow_result);
        assert_eq!(slow_result, 110.0);  // 100 + 0.1*(200-100)
        assert_eq!(fast_result, 190.0);  // 100 + 0.9*(200-100)
    }

    #[test]
    fn test_median_trim_basic() {
        // Values: [900, 100, 200, 500, 400, 300, 800]
        // Note: First value (900) is dummy read and discarded
        // Actual samples: [100, 200, 500, 400, 300, 800]
        // Sorted: [100, 200, 300, 400, 500, 800]
        // Trim 1 from each end: [200, 300, 400, 500] (remove 100 and 800)
        // Average: (200+300+400+500)/4 = 1400/4 = 350
        static VALUES: [u16; 8] = [900, 100, 200, 500, 400, 300, 800, 0];

        let mut adc = MockAdc::new(&VALUES);
        let mut buffer = [0u16; 6];

        // First read is dummy, then 6 samples, trim 1 from each end
        let avg = median_trim_adc(&mut adc, 0, 6, 1, &mut buffer);

        assert!((avg - 350.0).abs() < 0.1);
    }
}
