//! Boost Pressure Sensor Processor
//!
//! Handles ProSport boost sensors with voltage-to-PSI lookup and EMA filtering.

use crate::filtering::EmaFilter;

/// Boost processor for ProSport boost sensors
///
/// Converts voltage readings to boost pressure using:
/// - 5-point voltage-to-PSI lookup table
/// - Linear interpolation between points
/// - KOEO (Key On Engine Off) zero-trim calibration
/// - EMA filtering for smooth output
/// - PSI to kPa conversion
pub struct BoostProcessor {
    // EMA filter for smoothing
    filter: EmaFilter,

    // Lookup tables
    voltage_points: &'static [f32],
    psi_points: &'static [f32],

    // Configuration
    zero_trim_psi: f32,
    psi_to_kpa: f32,
}

impl BoostProcessor {
    /// PSI to kPa conversion factor
    const PSI_TO_KPA: f32 = 6.89476;

    /// Default EMA alpha for boost (fast response)
    const DEFAULT_ALPHA: f32 = 0.25;

    /// Create a new boost processor
    ///
    /// # Arguments
    /// * `voltage_points` - Voltage lookup table (ascending order)
    /// * `psi_points` - PSI lookup table (corresponding to voltages)
    /// * `zero_trim_psi` - KOEO zero-offset calibration (typically +8.0 PSI)
    ///
    /// # ProSport Standard Curve (5 points)
    /// ```ignore
    /// const BOOST_V: [f32; 5] = [0.02, 1.00, 2.00, 3.00, 4.00];
    /// const BOOST_PSI: [f32; 5] = [-14.5, 0.0, 14.5, 29.0, 43.51];
    /// let boost = BoostProcessor::new(&BOOST_V, &BOOST_PSI, 8.0);
    /// ```
    pub const fn new(
        voltage_points: &'static [f32],
        psi_points: &'static [f32],
        zero_trim_psi: f32,
    ) -> Self {
        Self {
            filter: EmaFilter::new(Self::DEFAULT_ALPHA),
            voltage_points,
            psi_points,
            zero_trim_psi,
            psi_to_kpa: Self::PSI_TO_KPA,
        }
    }

    /// Read boost pressure from voltage
    ///
    /// # Arguments
    /// * `voltage` - Sensor voltage (0.0-5.0V typically)
    ///
    /// # Returns
    /// Boost pressure in kPa (gauge pressure, not absolute)
    pub fn read_boost_kpa(&mut self, voltage: f32) -> f32 {
        // Convert voltage to PSI using lookup table
        let mut psi = self.voltage_to_psi(voltage);

        // Apply KOEO zero trim
        psi += self.zero_trim_psi;

        // Apply EMA filtering
        psi = self.filter.update(psi);

        // Convert to kPa
        let mut boost_kpa = psi * self.psi_to_kpa;

        // Physical clamps
        if boost_kpa < -101.325 {
            boost_kpa = -101.325; // Full vacuum
        }
        if boost_kpa > 300.0 {
            boost_kpa = 300.0; // Reasonable max boost
        }

        boost_kpa
    }

    /// Convert voltage to PSI using linear interpolation
    fn voltage_to_psi(&self, voltage: f32) -> f32 {
        let n = self.voltage_points.len();

        // Below minimum voltage
        if voltage <= self.voltage_points[0] {
            return self.psi_points[0];
        }

        // Above maximum voltage
        if voltage >= self.voltage_points[n - 1] {
            return self.psi_points[n - 1];
        }

        // Find interpolation range
        for i in 0..n - 1 {
            let v_lo = self.voltage_points[i];
            let v_hi = self.voltage_points[i + 1];

            if voltage >= v_lo && voltage <= v_hi {
                let psi_lo = self.psi_points[i];
                let psi_hi = self.psi_points[i + 1];

                // Linear interpolation
                let t = (voltage - v_lo) / (v_hi - v_lo);
                return psi_lo + t * (psi_hi - psi_lo);
            }
        }

        // Fallback (should never reach here)
        self.psi_points[n - 1]
    }

    /// Reset filter state
    pub fn reset(&mut self) {
        self.filter.reset();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ProSport standard curve
    const BOOST_V: [f32; 5] = [0.02, 1.00, 2.00, 3.00, 4.00];
    const BOOST_PSI: [f32; 5] = [-14.5, 0.0, 14.5, 29.0, 43.51];

    #[test]
    fn test_boost_exact_points() {
        // Test each lookup table point (no zero trim)
        for i in 0..5 {
            // Create fresh processor for each test (EMA starts fresh)
            let mut boost = BoostProcessor::new(&BOOST_V, &BOOST_PSI, 0.0);
            let kpa = boost.read_boost_kpa(BOOST_V[i]);
            let expected_kpa = BOOST_PSI[i] * 6.89476;
            assert!((kpa - expected_kpa).abs() < 1.0, "Point {}: got {}, expected {}", i, kpa, expected_kpa);
        }
    }

    #[test]
    fn test_boost_interpolation() {
        let mut boost = BoostProcessor::new(&BOOST_V, &BOOST_PSI, 0.0);

        // Test midpoint between 1.0V (0 PSI) and 2.0V (14.5 PSI)
        // 1.5V should give ~7.25 PSI = ~50 kPa
        let kpa = boost.read_boost_kpa(1.5);
        let expected_kpa = 7.25 * 6.89476; // ~50 kPa
        assert!((kpa - expected_kpa).abs() < 5.0);
    }

    #[test]
    fn test_boost_zero_trim() {
        let mut boost = BoostProcessor::new(&BOOST_V, &BOOST_PSI, 8.0);

        // At 1.0V (0 PSI) with +8 PSI trim
        // Should give 8 PSI = ~55 kPa
        let kpa = boost.read_boost_kpa(1.0);
        let expected_kpa = 8.0 * 6.89476; // ~55 kPa
        assert!((kpa - expected_kpa).abs() < 2.0);
    }

    #[test]
    fn test_boost_boundary_conditions() {
        // Test below minimum (0.02V) - should return -14.5 PSI
        let mut boost1 = BoostProcessor::new(&BOOST_V, &BOOST_PSI, 0.0);
        let kpa1 = boost1.read_boost_kpa(0.0);
        let expected_kpa1 = -14.5 * 6.89476; // ~-100 kPa
        assert!((kpa1 - expected_kpa1).abs() < 2.0, "Got {}, expected {}", kpa1, expected_kpa1);

        // Test above maximum (4.0V) - should return 43.51 PSI
        let mut boost2 = BoostProcessor::new(&BOOST_V, &BOOST_PSI, 0.0);
        let kpa2 = boost2.read_boost_kpa(5.0);
        let expected_kpa2 = 43.51 * 6.89476; // ~300 kPa
        assert!((kpa2 - expected_kpa2).abs() < 2.0, "Got {}, expected {}", kpa2, expected_kpa2);
    }

    #[test]
    fn test_boost_vacuum_clamp() {
        let mut boost = BoostProcessor::new(&BOOST_V, &BOOST_PSI, 0.0);

        // Very low voltage (full vacuum)
        let kpa = boost.read_boost_kpa(0.0);

        // Should clamp at -101.325 kPa (full vacuum)
        assert!(kpa >= -101.325);
    }

    #[test]
    fn test_boost_positive_clamp() {
        let mut boost = BoostProcessor::new(&BOOST_V, &BOOST_PSI, 100.0);

        // High voltage with large zero trim (extreme boost)
        let kpa = boost.read_boost_kpa(4.0);

        // Should clamp at 300 kPa
        assert!(kpa <= 300.0);
    }

    #[test]
    fn test_boost_ema_filtering() {
        let mut boost = BoostProcessor::new(&BOOST_V, &BOOST_PSI, 0.0);

        // First reading establishes filter
        let kpa1 = boost.read_boost_kpa(2.0); // 14.5 PSI = ~100 kPa
        assert!((kpa1 - 100.0).abs() < 5.0);

        // Second reading with same voltage should be similar (EMA converges)
        let kpa2 = boost.read_boost_kpa(2.0);
        assert!((kpa2 - 100.0).abs() < 5.0);

        // Sudden change should be filtered (EMA smooths it)
        let kpa3 = boost.read_boost_kpa(3.0); // 29.0 PSI = ~200 kPa
        // Should be between previous and new value due to EMA
        assert!(kpa3 > kpa2);
        assert!(kpa3 < 200.0); // Not yet fully at 200 kPa
    }

    #[test]
    fn test_boost_atmospheric() {
        let mut boost = BoostProcessor::new(&BOOST_V, &BOOST_PSI, 8.0);

        // At atmospheric (1.0V = 0 PSI) with typical +8 PSI trim
        // Result should be near 55 kPa (8 PSI)
        let kpa = boost.read_boost_kpa(1.0);
        assert!(kpa > 50.0 && kpa < 60.0);
    }
}
