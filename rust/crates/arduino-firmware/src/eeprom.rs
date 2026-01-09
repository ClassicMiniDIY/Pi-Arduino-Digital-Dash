//! EEPROM Odometer Module
//!
//! Manages persistent odometer storage in EEPROM with:
//! - Magic value validation (0xC0DE)
//! - Wear leveling (save only when changed > 0.1 miles)
//! - First-boot initialization
//! - Atomic updates

use arduino_hal::Eeprom;

/// EEPROM addresses
const MAGIC_ADDR: u16 = 8;
const MILES_ADDR: u16 = 10;

/// Magic value to detect first boot
const MAGIC_VALUE: u16 = 0xC0DE;

/// Initial odometer reading (miles)
const INITIAL_MILES: f32 = 2200.0;

/// Minimum change to trigger EEPROM write (wear leveling)
const SAVE_THRESHOLD_MILES: f32 = 0.1;

/// Odometer with EEPROM persistence
pub struct Odometer {
    current_miles: f32,
    last_saved_miles: f32,
    fractional_accumulator: f32,
}

impl Odometer {
    /// Create a new odometer instance
    pub const fn new() -> Self {
        Self {
            current_miles: 0.0,
            last_saved_miles: 0.0,
            fractional_accumulator: 0.0,
        }
    }

    /// Load odometer from EEPROM
    ///
    /// Checks magic value at address 8:
    /// - If 0xC0DE: load miles from address 10
    /// - Otherwise: first boot, initialize to INITIAL_MILES
    pub fn load(&mut self, eeprom: &mut Eeprom) {
        // Read magic value (2 bytes, little-endian)
        let magic_low = eeprom.read_byte(MAGIC_ADDR);
        let magic_high = eeprom.read_byte(MAGIC_ADDR + 1);
        let magic = u16::from_le_bytes([magic_low, magic_high]);

        if magic == MAGIC_VALUE {
            // Valid EEPROM data - load odometer
            let mut bytes = [0u8; 4];
            for i in 0..4 {
                bytes[i] = eeprom.read_byte(MILES_ADDR + i as u16);
            }
            self.current_miles = f32::from_le_bytes(bytes);
            self.last_saved_miles = self.current_miles;
        } else {
            // First boot - initialize EEPROM
            self.current_miles = INITIAL_MILES;
            self.last_saved_miles = INITIAL_MILES;
            self.save(eeprom);
        }
    }

    /// Update odometer based on speed
    ///
    /// # Arguments
    /// * `kph` - Current speed in kilometers per hour
    /// * `dt_sec` - Time delta since last update (seconds)
    pub fn update(&mut self, kph: f32, dt_sec: f32) {
        // Convert kph to mph
        let mph = kph * 0.621371;

        // Distance traveled in this interval (miles)
        let delta_miles = (mph * dt_sec) / 3600.0;

        self.fractional_accumulator += delta_miles;
        self.current_miles += delta_miles;
    }

    /// Save odometer to EEPROM if changed enough
    ///
    /// Uses wear leveling - only writes if changed > 0.1 miles
    pub fn save_if_needed(&mut self, eeprom: &mut Eeprom) {
        let delta = (self.current_miles - self.last_saved_miles).abs();
        if delta >= SAVE_THRESHOLD_MILES {
            self.save(eeprom);
        }
    }

    /// Force save to EEPROM (called on shutdown or explicit save)
    fn save(&mut self, eeprom: &mut Eeprom) {
        // Write magic value
        let magic_bytes = MAGIC_VALUE.to_le_bytes();
        eeprom.write_byte(MAGIC_ADDR, magic_bytes[0]);
        eeprom.write_byte(MAGIC_ADDR + 1, magic_bytes[1]);

        // Write odometer value (IEEE 754 float, little-endian)
        let miles_bytes = self.current_miles.to_le_bytes();
        for i in 0..4 {
            eeprom.write_byte(MILES_ADDR + i as u16, miles_bytes[i]);
        }

        self.last_saved_miles = self.current_miles;
    }

    /// Get current odometer reading
    pub fn miles(&self) -> f32 {
        self.current_miles
    }

    /// Get fractional accumulator (for debugging)
    pub fn fractional(&self) -> f32 {
        self.fractional_accumulator
    }
}
