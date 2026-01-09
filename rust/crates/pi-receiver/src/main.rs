//! Pi Receiver CLI
//!
//! Command-line tool for communicating with the Arduino Digital Dash:
//! - Query signature and version
//! - Monitor real-time OCH data
//! - Display sensor readings in human-readable format

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use protocol::{Command, OchBlock};
use serialport::SerialPort;
use std::io::{Read, Write};
use std::time::Duration;

#[derive(Parser)]
#[command(name = "pi-receiver")]
#[command(about = "Arduino Digital Dash Receiver", long_about = None)]
struct Cli {
    /// Serial port device (e.g., /dev/ttyACM0 or /dev/ttyUSB0)
    #[arg(short, long, default_value = "/dev/ttyACM0")]
    port: String,

    /// Baud rate
    #[arg(short, long, default_value = "115200")]
    baud: u32,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Query signature and version from Arduino
    Query,

    /// Monitor real-time sensor data
    Monitor {
        /// Update interval in milliseconds
        #[arg(short, long, default_value = "250")]
        interval: u64,
    },

    /// Read raw OCH block (87 bytes hex dump)
    RawOch,
}

fn main() -> Result<()> {
    env_logger::init();

    let cli = Cli::parse();

    // Open serial port
    let mut port = serialport::new(&cli.port, cli.baud)
        .timeout(Duration::from_millis(1000))
        .open()
        .context("Failed to open serial port")?;

    log::info!("Connected to {} at {} baud", cli.port, cli.baud);

    match cli.command {
        Commands::Query => query_info(&mut port)?,
        Commands::Monitor { interval } => monitor_data(&mut port, interval)?,
        Commands::RawOch => read_raw_och(&mut port)?,
    }

    Ok(())
}

/// Query signature and version
fn query_info(port: &mut Box<dyn SerialPort>) -> Result<()> {
    println!("Querying Arduino...\n");

    // Query signature
    port.write_all(&[Command::QuerySignature.to_byte()])?;
    let mut sig_buf = [0u8; 32];
    port.read_exact(&mut sig_buf)?;

    let signature = String::from_utf8_lossy(&sig_buf)
        .trim_end_matches('\0')
        .to_string();

    println!("Signature: {}", signature);

    // Query version
    port.write_all(&[Command::QueryVersion.to_byte()])?;
    let mut ver_buf = [0u8; 32];
    port.read_exact(&mut ver_buf)?;

    let version = String::from_utf8_lossy(&ver_buf)
        .trim_end_matches('\0')
        .to_string();

    println!("Version:   {}", version);

    Ok(())
}

/// Monitor real-time data
fn monitor_data(port: &mut Box<dyn SerialPort>, interval_ms: u64) -> Result<()> {
    println!("Monitoring sensor data (Ctrl+C to exit)...\n");
    println!("{:>5} {:>6} {:>8} {:>8} {:>8} {:>8} {:>10}",
             "RPM", "Speed", "Coolant", "Oil T", "Oil P", "Boost", "Odometer");
    println!("{}", "-".repeat(70));

    loop {
        // Request OCH block
        port.write_all(&[Command::ReadOch.to_byte()])?;

        let mut och_buf = [0u8; 87];
        match port.read_exact(&mut och_buf) {
            Ok(_) => {
                let och = OchBlock::from_bytes(&och_buf);

                // Extract and display values
                let rpm = och.rpm();
                let speed_kph = och.speed_kph();
                let coolant_c = (och.coolant_temp_c40() as i16) - 40;
                let oil_temp_c = (och.oil_temp_c40() as i16) - 40;
                let oil_pressure = och.oil_pressure_psi();
                let boost_kpa = och.boost_kpa();
                let odometer = och.odometer_miles();

                println!("{:5} {:6} {:6}°C {:6}°C {:6}psi {:6}kPa {:10.1}mi",
                         rpm, speed_kph, coolant_c, oil_temp_c,
                         oil_pressure, boost_kpa, odometer);
            }
            Err(e) => {
                log::warn!("Read error: {}", e);
                continue;
            }
        }

        std::thread::sleep(Duration::from_millis(interval_ms));
    }
}

/// Read and display raw OCH block
fn read_raw_och(port: &mut Box<dyn SerialPort>) -> Result<()> {
    println!("Reading raw OCH block...\n");

    port.write_all(&[Command::ReadOch.to_byte()])?;

    let mut och_buf = [0u8; 87];
    port.read_exact(&mut och_buf)?;

    println!("87-byte OCH block (hex):");
    for (i, chunk) in och_buf.chunks(16).enumerate() {
        print!("{:04x}: ", i * 16);
        for &byte in chunk {
            print!("{:02x} ", byte);
        }
        println!();
    }

    println!("\nParsed values:");
    let och = OchBlock::from_bytes(&och_buf);
    println!("  RPM:              {}", och.rpm());
    println!("  Speed (kph):      {}", och.speed_kph());
    println!("  Coolant (°C):     {}", (och.coolant_temp_c40() as i16) - 40);
    println!("  Oil Temp (°C):    {}", (och.oil_temp_c40() as i16) - 40);
    println!("  Oil Pressure:     {} psi", och.oil_pressure_psi());
    println!("  Fuel Pressure:    {} psi", och.fuel_pressure_psi());
    println!("  Boost (kPa):      {}", och.boost_kpa());
    println!("  Odometer (miles): {:.2}", och.odometer_miles());
    println!("  Turn Left:        {}", och.turn_left());
    println!("  Turn Right:       {}", och.turn_right());
    println!("  CEL:              {}", och.cel());
    println!("  High Beam:        {}", och.high_beam());
    println!("  Neutral:          {}", och.neutral());

    Ok(())
}
