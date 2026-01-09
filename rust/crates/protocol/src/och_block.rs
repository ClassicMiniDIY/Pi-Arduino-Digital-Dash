//! OCH (Output Channel) Block
//!
//! 87-byte data structure that matches the TunerStudio INI file specification.
//! All multi-byte values use little-endian encoding.

#![allow(dead_code)]

/// 87-byte output block matching TunerStudio INI specification
#[repr(C)]
pub struct OchBlock {
    data: [u8; Self::SIZE],
}

impl OchBlock {
    pub const SIZE: usize = 87;

    /// Create a new OCH block, zero-initialized
    pub const fn new() -> Self {
        Self { data: [0u8; Self::SIZE] }
    }

    /// Get the block as a byte slice for transmission
    pub fn as_bytes(&self) -> &[u8] {
        &self.data
    }

    /// Clear all bytes to zero
    pub fn clear(&mut self) {
        self.data.fill(0);
    }

    // ==================================================================
    // TYPED SETTERS (ensure correct byte layout and endianness)
    // ==================================================================

    /// Set RPM value (U16 at offset 0-1, little-endian)
    /// Range: 0-30000 RPM
    pub fn set_rpm(&mut self, rpm: u16) {
        self.write_u16_le(0, rpm);
    }

    /// Set oil temperature (U8 at offset 3, encoded as °C + 40)
    /// Input: raw value = temperature_celsius + 40
    /// Range: -40°C to +215°C (raw 0-255)
    pub fn set_oil_temp_c40(&mut self, temp: u8) {
        self.data[3] = temp;
    }

    /// Set MAP/boost pressure (S16 at offset 4-5, little-endian)
    /// Units: kPa absolute (for boost: gauge kPa + 100)
    /// Range: -100 to +400 kPa
    pub fn set_map_kpa(&mut self, kpa: i16) {
        self.write_i16_le(4, kpa);
    }

    /// Set coolant temperature (U8 at offset 7, encoded as °C + 40)
    /// Input: raw value = temperature_celsius + 40
    /// Range: -40°C to +215°C (raw 0-255)
    pub fn set_coolant_temp_c40(&mut self, temp: u8) {
        self.data[7] = temp;
    }

    /// Set oil pressure (U8 at offset 10)
    /// Units: PSI
    /// Range: 0-145 PSI
    pub fn set_oil_pressure_psi(&mut self, psi: u8) {
        self.data[10] = psi;
    }

    /// Set fuel pressure (U8 at offset 11)
    /// Units: PSI
    /// Range: 0-145 PSI
    pub fn set_fuel_pressure_psi(&mut self, psi: u8) {
        self.data[11] = psi;
    }

    /// Set fuel level (U8 at offset 15)
    /// Units: Percentage
    /// Range: 0-100%
    pub fn set_fuel_level_pct(&mut self, pct: u8) {
        self.data[15] = pct;
    }

    /// Set vehicle speed (U16 at offset 19-20, little-endian)
    /// Units: km/h
    /// Range: 0-300+ kph
    pub fn set_speed_kph(&mut self, kph: u16) {
        self.write_u16_le(19, kph);
    }

    /// Set left turn indicator (U8 at offset 40)
    /// 0 = OFF, 1 = ON
    pub fn set_turn_left(&mut self, on: bool) {
        self.data[40] = if on { 1 } else { 0 };
    }

    /// Set right turn indicator (U8 at offset 41)
    /// 0 = OFF, 1 = ON
    pub fn set_turn_right(&mut self, on: bool) {
        self.data[41] = if on { 1 } else { 0 };
    }

    /// Set check engine light (U8 at offset 42)
    /// 0 = OFF, 1 = ON
    pub fn set_cel(&mut self, on: bool) {
        self.data[42] = if on { 1 } else { 0 };
    }

    /// Set high beam indicator (U8 at offset 43)
    /// 0 = OFF, 1 = ON
    pub fn set_high_beam(&mut self, on: bool) {
        self.data[43] = if on { 1 } else { 0 };
    }

    /// Set handbrake/parking brake indicator (U8 at offset 44)
    /// 0 = OFF, 1 = ON
    pub fn set_handbrake(&mut self, on: bool) {
        self.data[44] = if on { 1 } else { 0 };
    }

    /// Set odometer reading (F32 at offset 60-63, little-endian IEEE 754)
    /// Units: Miles
    /// Range: 0 to ~9,999,999 miles
    pub fn set_odometer_miles(&mut self, miles: f32) {
        self.write_f32_le(60, miles);
    }

    // ==================================================================
    // INTERNAL ENCODING HELPERS
    // ==================================================================

    #[inline]
    fn write_u16_le(&mut self, offset: usize, val: u16) {
        self.data[offset] = (val & 0xFF) as u8;
        self.data[offset + 1] = (val >> 8) as u8;
    }

    #[inline]
    fn write_i16_le(&mut self, offset: usize, val: i16) {
        self.write_u16_le(offset, val as u16);
    }

    #[inline]
    fn write_f32_le(&mut self, offset: usize, val: f32) {
        let bytes = val.to_le_bytes();
        self.data[offset..offset + 4].copy_from_slice(&bytes);
    }
}

impl Default for OchBlock {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_och_block_size() {
        assert_eq!(OchBlock::SIZE, 87);
        assert_eq!(core::mem::size_of::<OchBlock>(), 87);
    }

    #[test]
    fn test_rpm_encoding() {
        let mut och = OchBlock::new();
        och.set_rpm(3000);

        let bytes = och.as_bytes();
        assert_eq!(u16::from_le_bytes([bytes[0], bytes[1]]), 3000);
    }

    #[test]
    fn test_speed_encoding() {
        let mut och = OchBlock::new();
        och.set_speed_kph(100);

        let bytes = och.as_bytes();
        assert_eq!(u16::from_le_bytes([bytes[19], bytes[20]]), 100);
    }

    #[test]
    fn test_temperature_encoding() {
        let mut och = OchBlock::new();

        // 85°C = 85 + 40 = 125
        och.set_coolant_temp_c40(125);
        assert_eq!(och.as_bytes()[7], 125);

        // -10°C = -10 + 40 = 30
        och.set_oil_temp_c40(30);
        assert_eq!(och.as_bytes()[3], 30);
    }

    #[test]
    fn test_map_boost_encoding() {
        let mut och = OchBlock::new();

        // 100 kPa = atmospheric (0 boost)
        och.set_map_kpa(100);
        let bytes = och.as_bytes();
        assert_eq!(i16::from_le_bytes([bytes[4], bytes[5]]), 100);

        // 150 kPa = 50 kPa boost
        och.set_map_kpa(150);
        let bytes = och.as_bytes();
        assert_eq!(i16::from_le_bytes([bytes[4], bytes[5]]), 150);

        // 50 kPa = -50 kPa vacuum
        och.set_map_kpa(50);
        let bytes = och.as_bytes();
        assert_eq!(i16::from_le_bytes([bytes[4], bytes[5]]), 50);
    }

    #[test]
    fn test_odometer_encoding() {
        let mut och = OchBlock::new();
        och.set_odometer_miles(2200.5);

        let bytes = och.as_bytes();
        let odo = f32::from_le_bytes([bytes[60], bytes[61], bytes[62], bytes[63]]);

        // Allow small float rounding error
        assert!((odo - 2200.5).abs() < 0.01);
    }

    #[test]
    fn test_boolean_indicators() {
        let mut och = OchBlock::new();

        och.set_turn_left(true);
        och.set_cel(false);
        och.set_high_beam(true);

        let bytes = och.as_bytes();
        assert_eq!(bytes[40], 1);  // turn left ON
        assert_eq!(bytes[42], 0);  // CEL OFF
        assert_eq!(bytes[43], 1);  // high beam ON
    }

    #[test]
    fn test_clear() {
        let mut och = OchBlock::new();

        och.set_rpm(5000);
        och.set_speed_kph(120);
        och.set_cel(true);

        och.clear();

        // All bytes should be zero
        for byte in och.as_bytes() {
            assert_eq!(*byte, 0);
        }
    }
}
