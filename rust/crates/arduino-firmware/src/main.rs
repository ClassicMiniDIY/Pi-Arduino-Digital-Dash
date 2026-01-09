//! Arduino Mega 2560 Digital Dash Firmware
//!
//! Main firmware integrating:
//! - TunerStudio serial protocol
//! - Sensor processing (thermistors, pressure, boost)
//! - RPM and VSS interrupt-driven measurement
//! - EEPROM odometer persistence
//! - Digital inputs (turn signals, CEL, etc.)

#![no_std]
#![no_main]
#![feature(abi_avr_interrupt)]

mod eeprom;
mod interrupts;

use panic_halt as _;

use arduino_hal::{prelude::*, Adc, Peripherals};
use protocol::{Command, OchBlock};
use sensors::*;

// ProSport thermistor curve (7 points)
const PS_TEMP_F: [f32; 7] = [104.0, 140.0, 176.0, 212.0, 248.0, 284.0, 302.0];
const PS_OHMS: [f32; 7] = [5830.0, 3020.0, 1670.0, 975.0, 599.0, 386.0, 316.0];

// ProSport boost curve (5 points)
const BOOST_V: [f32; 5] = [0.02, 1.00, 2.00, 3.00, 4.00];
const BOOST_PSI: [f32; 5] = [-14.5, 0.0, 14.5, 29.0, 43.51];

/// Main update rate (40Hz / 25ms)
const UPDATE_INTERVAL_MS: u32 = 25;

/// Sensor read interval (slower to reduce ADC noise)
const SENSOR_READ_INTERVAL_MS: u32 = 100;

/// EEPROM save interval (every 10 seconds)
const EEPROM_SAVE_INTERVAL_MS: u32 = 10000;

/// RPM timeout (400ms without pulse = engine stopped)
const RPM_TIMEOUT_MS: u32 = 400;

#[arduino_hal::entry]
fn main() -> ! {
    // Initialize hardware
    let dp = Peripherals::take().unwrap();
    let pins = arduino_hal::pins!(dp);

    // Configure serial port (115200 baud)
    let mut serial = arduino_hal::default_serial!(dp, pins, 115200);

    // Configure ADC
    let mut adc = Adc::new(dp.ADC, Default::default());

    // Configure EEPROM
    let mut eeprom = arduino_hal::Eeprom::new(dp.EEPROM);

    // Configure analog input pins
    let oil_pressure_pin = pins.a0.into_analog_input(&mut adc);
    let fuel_pressure_pin = pins.a1.into_analog_input(&mut adc);
    let coolant_temp_pin = pins.a2.into_analog_input(&mut adc);
    let oil_temp_pin = pins.a3.into_analog_input(&mut adc);
    let boost_pin = pins.a4.into_analog_input(&mut adc);
    let _afr_pin = pins.a5.into_analog_input(&mut adc); // Reserved for future

    // Configure digital input pins (active low with pullups)
    let turn_left_pin = pins.d22.into_pull_up_input();
    let turn_right_pin = pins.d23.into_pull_up_input();
    let cel_pin = pins.d24.into_pull_up_input();
    let high_beam_pin = pins.d25.into_pull_up_input();
    let neutral_pin = pins.d26.into_pull_up_input();

    // Configure interrupt pins for RPM (INT0/D2) and VSS (INT1/D3)
    let _tach_pin = pins.d2.into_pull_up_input();
    let _vss_pin = pins.d3.into_pull_up_input();

    // TODO: Configure external interrupts
    // dp.EICRA.write(|w| w.isc0().bits(0b10).isc1().bits(0b10)); // Falling edge
    // dp.EIMSK.write(|w| w.int0().set_bit().int1().set_bit()); // Enable INT0, INT1

    // Initialize sensor processors
    let thermistor = ProSportThermistor::new(1000.0, &PS_TEMP_F, &PS_OHMS);
    let mut boost_processor = BoostProcessor::new(&BOOST_V, &BOOST_PSI, 8.0);
    let mut rpm_processor = RpmProcessor::new(2, 9000.0, 22000.0);
    let mut vss_processor = VssProcessor::new(8000.0);

    // Initialize filters
    let mut ema_oil_p = EmaFilter::new(0.12);
    let mut ema_fuel_p = EmaFilter::new(0.12);
    let mut ema_clt = EmaFilter::new(0.08);
    let mut ema_oil_t = EmaFilter::new(0.08);

    // Initialize odometer
    let mut odometer = eeprom::Odometer::new();
    odometer.load(&mut eeprom);

    // Initialize timing
    let mut last_update_ms = 0u32;
    let mut last_sensor_read_ms = 0u32;
    let mut last_eeprom_save_ms = 0u32;

    // OCH block for TunerStudio
    let mut och = OchBlock::new();

    // Enable interrupts
    unsafe {
        avr_device::interrupt::enable();
    }

    // Main loop
    loop {
        let now_ms = arduino_hal::clock::millis();
        let now_us = now_ms * 1000;

        // ========================================
        // Main Update Loop (40Hz)
        // ========================================
        if now_ms.wrapping_sub(last_update_ms) >= UPDATE_INTERVAL_MS {
            let dt_sec = (now_ms.wrapping_sub(last_update_ms)) as f32 / 1000.0;
            last_update_ms = now_ms;

            // Update RPM from interrupt data
            let tach_period = interrupts::get_tach_period();
            let tach_last_ms = interrupts::get_tach_last_pulse_ms();
            let rpm = rpm_processor.update(tach_period, now_us, RPM_TIMEOUT_MS, tach_last_ms);

            // Update VSS (reads interrupt timestamps internally)
            let vss_last_us = interrupts::get_vss_last_pulse();
            if vss_last_us > 0 {
                vss_processor.on_pulse(vss_last_us);
            }
            let kph = vss_processor.update(now_us, rpm);

            // Update odometer
            odometer.update(kph, dt_sec);

            // Store in OCH block
            och.set_rpm(rpm as u16);
            och.set_speed_kph(kph as u16);
            och.set_odometer_miles(odometer.miles());
        }

        // ========================================
        // Sensor Reading (10Hz)
        // ========================================
        if now_ms.wrapping_sub(last_sensor_read_ms) >= SENSOR_READ_INTERVAL_MS {
            last_sensor_read_ms = now_ms;

            // Read pressure sensors (analog, with filtering)
            let oil_p_raw = adc.read_blocking(&oil_pressure_pin) as f32;
            let fuel_p_raw = adc.read_blocking(&fuel_pressure_pin) as f32;

            // Convert ADC to PSI (0-5V → 0-100 PSI linear)
            let oil_p_psi = (oil_p_raw / 1023.0) * 100.0;
            let fuel_p_psi = (fuel_p_raw / 1023.0) * 100.0;

            // Apply EMA filtering
            let oil_p_filtered = ema_oil_p.update(oil_p_psi);
            let fuel_p_filtered = ema_fuel_p.update(fuel_p_psi);

            och.set_oil_pressure_psi(oil_p_filtered as u8);
            och.set_fuel_pressure_psi(fuel_p_filtered as u8);

            // Read temperature sensors (ProSport thermistors)
            let clt_raw = adc.read_blocking(&coolant_temp_pin) as f32;
            let oil_t_raw = adc.read_blocking(&oil_temp_pin) as f32;

            let clt_c40 = thermistor.read_temp_c40(clt_raw);
            let oil_t_c40 = thermistor.read_temp_c40(oil_t_raw);

            // Apply EMA to temperatures (smooth out noise)
            let clt_filtered = ema_clt.update(clt_c40 as f32) as u8;
            let oil_t_filtered = ema_oil_t.update(oil_t_c40 as f32) as u8;

            och.set_coolant_temp_c40(clt_filtered);
            och.set_oil_temp_c40(oil_t_filtered);

            // Read boost sensor (voltage → kPa)
            let boost_raw = adc.read_blocking(&boost_pin) as f32;
            let boost_voltage = (boost_raw / 1023.0) * 5.0;
            let boost_kpa = boost_processor.read_boost_kpa(boost_voltage);

            och.set_boost_kpa(boost_kpa as i16);

            // Read digital inputs (active low)
            och.set_turn_left(turn_left_pin.is_low());
            och.set_turn_right(turn_right_pin.is_low());
            och.set_cel(cel_pin.is_low());
            och.set_high_beam(high_beam_pin.is_low());
            och.set_neutral(neutral_pin.is_low());
        }

        // ========================================
        // EEPROM Save (every 10 seconds)
        // ========================================
        if now_ms.wrapping_sub(last_eeprom_save_ms) >= EEPROM_SAVE_INTERVAL_MS {
            last_eeprom_save_ms = now_ms;
            odometer.save_if_needed(&mut eeprom);
        }

        // ========================================
        // Serial Command Handler
        // ========================================
        if let Ok(byte) = serial.read() {
            let cmd = Command::from_byte(byte);

            match cmd {
                Command::QuerySignature => {
                    // Send signature (32 bytes)
                    for &b in protocol::SIGNATURE {
                        let _ = serial.write(b);
                    }
                }
                Command::QueryVersion => {
                    // Send version (32 bytes)
                    for &b in protocol::VERSION {
                        let _ = serial.write(b);
                    }
                }
                Command::ReadOch => {
                    // Send OCH block (87 bytes)
                    let data = och.as_bytes();
                    for &b in data {
                        let _ = serial.write(b);
                    }
                }
                Command::ReadPage => {
                    // Not implemented - send zeros
                    for _ in 0..128 {
                        let _ = serial.write(0);
                    }
                }
                Command::GetCrc => {
                    // Not implemented - send dummy CRC
                    let _ = serial.write(0xFF);
                    let _ = serial.write(0xFF);
                    let _ = serial.write(0xFF);
                    let _ = serial.write(0xFF);
                }
                Command::BurnPage | Command::Flags => {
                    // Not implemented - acknowledge
                    let _ = serial.write(b'!');
                }
                Command::Unknown(_) => {
                    // Unknown command - ignore
                }
            }
        }
    }
}
