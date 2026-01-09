//! Interrupt Handlers for RPM and VSS
//!
//! RPM: INT0 (D2) - Falling edge tachometer signal
//! VSS: INT1 (D3) - Falling edge vehicle speed sensor

use core::sync::atomic::{AtomicU32, Ordering};

/// RPM pulse period in microseconds (shared with main loop)
static TACH_PERIOD_US: AtomicU32 = AtomicU32::new(0);
static TACH_LAST_PULSE_US: AtomicU32 = AtomicU32::new(0);

/// VSS pulse timestamp (shared with main loop)
static VSS_LAST_PULSE_US: AtomicU32 = AtomicU32::new(0);

/// Minimum pulse period for noise rejection (3ms)
const MIN_PULSE_PERIOD_US: u32 = 3000;

/// Get current RPM pulse period
pub fn get_tach_period() -> u32 {
    TACH_PERIOD_US.load(Ordering::Relaxed)
}

/// Get timestamp of last tachometer pulse (milliseconds)
pub fn get_tach_last_pulse_ms() -> u32 {
    TACH_LAST_PULSE_US.load(Ordering::Relaxed) / 1000
}

/// Get timestamp of last VSS pulse (microseconds)
pub fn get_vss_last_pulse() -> u32 {
    VSS_LAST_PULSE_US.load(Ordering::Relaxed)
}

/// RPM interrupt handler (INT0)
///
/// Measures time between falling edges to calculate period.
/// Rejects pulses too close together (< 3ms) as noise.
#[avr_device::interrupt(atmega2560)]
fn INT0() {
    // Get current timestamp (microseconds since boot)
    let now_us = arduino_hal::clock::millis() as u32 * 1000;

    let last_us = TACH_LAST_PULSE_US.load(Ordering::Relaxed);

    if last_us > 0 {
        let period = now_us.wrapping_sub(last_us);

        // Noise rejection - only accept periods >= 3ms
        if period >= MIN_PULSE_PERIOD_US {
            TACH_PERIOD_US.store(period, Ordering::Relaxed);
        }
    }

    TACH_LAST_PULSE_US.store(now_us, Ordering::Relaxed);
}

/// VSS interrupt handler (INT1)
///
/// Simply records timestamp - main loop handles windowed counting.
#[avr_device::interrupt(atmega2560)]
fn INT1() {
    let now_us = arduino_hal::clock::millis() as u32 * 1000;
    VSS_LAST_PULSE_US.store(now_us, Ordering::Relaxed);
}
