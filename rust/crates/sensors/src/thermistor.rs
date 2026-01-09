//! ProSport Thermistor Processing
//!
//! Implements log-linear interpolation for ProSport 2-wire thermistor sensors.

use crate::clamp_u8;
use libm::logf;

/// ProSport thermistor sensor processor
///
/// Converts ADC readings to temperature using a voltage divider circuit
/// and logarithmic interpolation through a resistance-temperature lookup table.
pub struct ProSportThermistor {
    pullup_ohms: f32,
    temp_points: &'static [f32],  // Temperature in Fahrenheit
    ohm_points: &'static [f32],   // Resistance in ohms
}

impl ProSportThermistor {
    /// Create a new ProSport thermistor processor
    ///
    /// # Arguments
    /// * `pullup_ohms` - Pull-up resistor value (typically 1000Ω)
    /// * `temp_points` - Temperature lookup table (°F)
    /// * `ohm_points` - Resistance lookup table (Ω)
    ///
    /// # ProSport Standard Curve (7 points)
    /// ```ignore
    /// const PS_TEMP_F: [f32; 7] = [104.0, 140.0, 176.0, 212.0, 248.0, 284.0, 302.0];
    /// const PS_OHMS: [f32; 7] = [5830.0, 3020.0, 1670.0, 975.0, 599.0, 386.0, 316.0];
    /// let thermistor = ProSportThermistor::new(1000.0, &PS_TEMP_F, &PS_OHMS);
    /// ```
    pub const fn new(
        pullup_ohms: f32,
        temp_points: &'static [f32],
        ohm_points: &'static [f32],
    ) -> Self {
        Self {
            pullup_ohms,
            temp_points,
            ohm_points,
        }
    }

    /// Convert ADC reading to resistance (voltage divider calculation)
    ///
    /// Circuit: VREF (5V) → Pull-up Resistor → Thermistor → GND
    ///                                    ↑
    ///                                   ADC
    ///
    /// Formula: R_thermistor = R_pullup × (ADC / (ADC_MAX - ADC))
    pub fn adc_to_ohms(&self, adc_raw: f32, adc_max: f32) -> f32 {
        self.pullup_ohms * (adc_raw / (adc_max - adc_raw))
    }

    /// Convert resistance to temperature using log-linear interpolation
    ///
    /// Uses logarithmic interpolation between lookup table points for
    /// better accuracy with exponential thermistor curves.
    ///
    /// Returns temperature in Fahrenheit.
    pub fn ohms_to_temp_f(&self, resistance: f32) -> f32 {
        let n = self.ohm_points.len();

        // Boundary checks (resistance decreases as temperature increases)
        if resistance >= self.ohm_points[0] {
            return self.temp_points[0];  // Below min temp
        }
        if resistance <= self.ohm_points[n - 1] {
            return self.temp_points[n - 1];  // Above max temp
        }

        // Find interpolation range
        for i in 0..n - 1 {
            let r_hi = self.ohm_points[i];
            let r_lo = self.ohm_points[i + 1];

            if resistance <= r_hi && resistance >= r_lo {
                let t_hi = self.temp_points[i];
                let t_lo = self.temp_points[i + 1];

                // Logarithmic interpolation (matches C++ exactly)
                let x = logf(resistance);
                let xh = logf(r_hi);
                let xl = logf(r_lo);

                let frac = (x - xh) / (xl - xh);
                return t_hi + frac * (t_lo - t_hi);
            }
        }

        // Fallback (should never reach here)
        self.temp_points[n - 1]
    }

    /// Full pipeline: ADC → Celsius (encoded as U8 = °C + 40)
    ///
    /// This matches the C++ implementation exactly:
    /// 1. Convert ADC to resistance
    /// 2. Interpolate resistance to °F
    /// 3. Apply ProSport floor (100°F minimum)
    /// 4. Apply ceiling (302°F maximum)
    /// 5. Convert to Celsius
    /// 6. Encode as U8 (°C + 40)
    ///
    /// # Arguments
    /// * `adc_raw` - Raw ADC reading (0-1023)
    ///
    /// # Returns
    /// Temperature encoded as U8 = (temperature_celsius + 40)
    /// Range: 0-255 representing -40°C to +215°C
    pub fn read_temp_c40(&self, adc_raw: f32) -> u8 {
        const ADC_MAX: f32 = 1023.0;
        const TEMP_F_FLOOR: f32 = 100.0;   // ProSport-style floor
        const TEMP_F_CEIL: f32 = 302.0;

        // Convert ADC to resistance
        let r = self.adc_to_ohms(adc_raw, ADC_MAX);

        // Interpolate to temperature (°F)
        let mut temp_f = self.ohms_to_temp_f(r);

        // Apply floor and ceiling
        if temp_f < TEMP_F_FLOOR {
            temp_f = TEMP_F_FLOOR;
        }
        if temp_f > TEMP_F_CEIL {
            temp_f = TEMP_F_CEIL;
        }

        // Convert to Celsius
        let temp_c = (temp_f - 32.0) * (5.0 / 9.0);

        // Encode as U8 (°C + 40)
        clamp_u8((temp_c + 40.0) as i32)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ProSport standard curve
    const PS_TEMP_F: [f32; 7] = [104.0, 140.0, 176.0, 212.0, 248.0, 284.0, 302.0];
    const PS_OHMS: [f32; 7] = [5830.0, 3020.0, 1670.0, 975.0, 599.0, 386.0, 316.0];

    #[test]
    fn test_exact_lookup_points() {
        let therm = ProSportThermistor::new(1000.0, &PS_TEMP_F, &PS_OHMS);

        // Test each lookup table point
        for i in 0..7 {
            let temp = therm.ohms_to_temp_f(PS_OHMS[i]);
            assert!((temp - PS_TEMP_F[i]).abs() < 0.5);
        }
    }

    #[test]
    fn test_interpolation() {
        let therm = ProSportThermistor::new(1000.0, &PS_TEMP_F, &PS_OHMS);

        // Test midpoint between 212°F (975Ω) and 248°F (599Ω)
        // Expected ~230°F at ~750Ω
        let mid_ohms = 750.0;
        let temp = therm.ohms_to_temp_f(mid_ohms);

        assert!(temp > 212.0 && temp < 248.0);
        assert!((temp - 230.0).abs() < 5.0);  // Within 5°F tolerance
    }

    #[test]
    fn test_boundary_conditions() {
        let therm = ProSportThermistor::new(1000.0, &PS_TEMP_F, &PS_OHMS);

        // Below minimum (high resistance)
        assert_eq!(therm.ohms_to_temp_f(10000.0), 104.0);

        // Above maximum (low resistance)
        assert_eq!(therm.ohms_to_temp_f(100.0), 302.0);
    }

    #[test]
    fn test_adc_to_ohms() {
        let therm = ProSportThermistor::new(1000.0, &PS_TEMP_F, &PS_OHMS);

        // At mid-scale ADC (512 counts), R = 1000Ω
        let r = therm.adc_to_ohms(512.0, 1023.0);
        assert!((r - 1000.0).abs() < 10.0);

        // At 3/4 scale (768 counts), R ≈ 3000Ω
        let r = therm.adc_to_ohms(768.0, 1023.0);
        assert!((r - 3000.0).abs() < 50.0);
    }

    #[test]
    fn test_temp_c40_encoding() {
        let therm = ProSportThermistor::new(1000.0, &PS_TEMP_F, &PS_OHMS);

        // Test at 212°F = 100°C
        // 975Ω → ADC ≈ 518 counts
        let adc = 518.0;
        let encoded = therm.read_temp_c40(adc);

        // 100°C + 40 = 140
        assert!((encoded as i32 - 140).abs() < 5);
    }

    #[test]
    fn test_floor_application() {
        let therm = ProSportThermistor::new(1000.0, &PS_TEMP_F, &PS_OHMS);

        // Very high resistance (low temp) should floor at 100°F = 37.8°C
        let encoded = therm.read_temp_c40(1000.0);  // High ADC = high R = low temp

        // 37.8°C + 40 = 77.8 ≈ 78
        assert!((encoded as i32 - 78).abs() < 3);
    }
}
