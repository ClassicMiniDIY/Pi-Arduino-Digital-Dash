#![no_std]

//! Sensor Processing Library
//!
//! Platform-independent sensor reading, filtering, and processing algorithms.
//! Compatible with both AVR (Arduino) and standard Rust environments.

pub mod filtering;
pub mod thermistor;
pub mod vss;
pub mod rpm;
pub mod boost;

// Re-export commonly used types
pub use filtering::{EmaFilter, median_trim_adc};
pub use thermistor::ProSportThermistor;
pub use vss::VssProcessor;
pub use rpm::RpmProcessor;
pub use boost::BoostProcessor;

/// Hardware abstraction trait for ADC reading
pub trait AdcReader {
    /// Read raw ADC value (0-1023 for 10-bit ADC)
    fn read_raw(&mut self, pin: u8) -> u16;

    /// Read voltage (0.0-5.0V typically)
    fn read_voltage(&mut self, pin: u8) -> f32 {
        const ADC_MAX: f32 = 1023.0;
        const VREF: f32 = 5.0;
        (self.read_raw(pin) as f32 * VREF) / ADC_MAX
    }
}

/// Hardware abstraction for digital input
pub trait DigitalInput {
    /// Read digital pin state (true = HIGH, false = LOW)
    fn read(&self) -> bool;
}

/// Helper function to clamp values to u8 range
#[inline]
pub fn clamp_u8(v: i32) -> u8 {
    if v < 0 {
        0
    } else if v > 255 {
        255
    } else {
        v as u8
    }
}
