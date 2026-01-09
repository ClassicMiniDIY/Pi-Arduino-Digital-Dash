# Building the Arduino Digital Dash (Rust Version)

Complete build and installation guide for the Rust-based Arduino Digital Dash firmware and Pi receiver.

## Table of Contents

1. [Prerequisites](#prerequisites)
2. [Toolchain Installation](#toolchain-installation)
3. [Building Arduino Firmware](#building-arduino-firmware)
4. [Flashing to Arduino](#flashing-to-arduino)
5. [Building Pi Receiver](#building-pi-receiver)
6. [Verification](#verification)
7. [Troubleshooting](#troubleshooting)

## Prerequisites

### Hardware Requirements

- **Arduino Mega 2560** (8KB RAM, 256KB flash)
- **Raspberry Pi 5** (or any Linux/macOS system for testing)
- **USB cable** (A-to-B for Arduino)
- **ProSport sensors** (thermistors, boost sensor)
- **Pressure senders** (oil, fuel)

### Software Requirements

- **Rust** (nightly for AVR, stable for Pi)
- **AVR toolchain** (avr-gcc, avr-libc, avrdude)
- **Git** (for cloning repository)

## Toolchain Installation

### Automated Installation (Recommended)

Run the automated installation script:

```bash
cd rust
./scripts/install-toolchain.sh
```

This script will install:
- Rust nightly toolchain
- AVR target support
- avr-gcc and avr-libc
- avrdude (for flashing)
- ravedude (automated flashing tool)

### Manual Installation

#### 1. Install Rust

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source $HOME/.cargo/env
```

#### 2. Install Rust Nightly (Required for AVR)

```bash
rustup install nightly
rustup component add rust-src --toolchain nightly
rustup default nightly
```

#### 3. Install AVR Toolchain

**macOS (Homebrew):**
```bash
brew tap osx-cross/avr
brew install avr-gcc avrdude
```

**Linux (Debian/Ubuntu):**
```bash
sudo apt-get update
sudo apt-get install gcc-avr avr-libc avrdude
```

**Linux (Arch):**
```bash
sudo pacman -S avr-gcc avr-libc avrdude
```

#### 4. Install ravedude (Optional but Recommended)

```bash
cargo install ravedude
```

## Building Arduino Firmware

### 1. Navigate to Arduino Firmware Crate

```bash
cd rust/crates/arduino-firmware
```

### 2. Build for AVR Target

```bash
cargo build --release
```

This will:
- Compile all sensor libraries (`protocol`, `sensors`)
- Build firmware for `avr-atmega2560` target
- Optimize for size (`opt-level = "z"`)
- Output ELF file to `../../target/avr-atmega2560/release/arduino-firmware.elf`

**Expected output:**
```
   Compiling protocol v0.1.0
   Compiling sensors v0.1.0
   Compiling arduino-firmware v0.1.0
    Finished release [optimized] target(s) in 45.2s
```

### 3. Check Binary Size

```bash
avr-size ../../target/avr-atmega2560/release/arduino-firmware.elf
```

**Example output:**
```
   text    data     bss     dec     hex filename
  28432     124    1856   30412    76cc arduino-firmware.elf
```

**Memory Budget:**
- **Flash (text + data)**: ~28KB of 256KB (11% used) ✓
- **RAM (data + bss)**: ~2KB of 8KB (25% used) ✓

## Flashing to Arduino

### Method 1: Automated (ravedude)

**Easiest method** - ravedude handles everything automatically:

```bash
cargo run --release
```

This will:
1. Build the firmware
2. Convert ELF to HEX
3. Detect Arduino on `/dev/ttyACM*` or `/dev/ttyUSB*`
4. Flash using avrdude
5. Open serial monitor

**Expected output:**
```
Compiling arduino-firmware v0.1.0
    Finished release [optimized] target(s)
   Searching for connected Arduino...
   Found Arduino Mega 2560 on /dev/ttyACM0
   Flashing...
avrdude: AVR device initialized and ready to accept instructions
avrdude: Device signature = 0x1e9801 (probably m2560)
avrdude: erasing chip
avrdude: writing flash (28556 bytes):
avrdude: 28556 bytes of flash written
avrdude: verifying flash memory against firmware.hex:
avrdude: 28556 bytes of flash verified

avrdude done.  Thank you.

✓ Flashing complete!
```

### Method 2: Manual (avrdude)

If ravedude doesn't work or you need more control:

#### 1. Convert ELF to HEX

```bash
avr-objcopy -O ihex \
  ../../target/avr-atmega2560/release/arduino-firmware.elf \
  firmware.hex
```

#### 2. Identify Serial Port

**macOS:**
```bash
ls /dev/tty.usbmodem*
# Example: /dev/tty.usbmodem14201
```

**Linux:**
```bash
ls /dev/ttyACM*
# Example: /dev/ttyACM0
```

#### 3. Flash with avrdude

```bash
avrdude -v \
  -p atmega2560 \
  -c wiring \
  -P /dev/ttyACM0 \
  -b 115200 \
  -D \
  -U flash:w:firmware.hex:i
```

**Flags explained:**
- `-p atmega2560`: Target MCU
- `-c wiring`: Programmer type (Arduino bootloader)
- `-P /dev/ttyACM0`: Serial port
- `-b 115200`: Baud rate
- `-D`: Don't erase entire chip (faster)
- `-U flash:w:firmware.hex:i`: Write HEX to flash

## Building Pi Receiver

The Pi receiver is a standard Rust application (no cross-compilation needed).

### 1. Navigate to Pi Receiver Crate

```bash
cd rust/crates/pi-receiver
```

### 2. Build

```bash
cargo build --release
```

### 3. Install System-Wide (Optional)

```bash
cargo install --path .
```

This installs `pi-receiver` to `~/.cargo/bin/`, which should be in your PATH.

### Cross-Compilation for ARM64 (Pi 5)

If building on a different architecture:

```bash
rustup target add aarch64-unknown-linux-gnu
cargo build --release --target aarch64-unknown-linux-gnu
```

## Verification

### 1. Query Arduino Signature

```bash
cd rust/crates/pi-receiver
cargo run -- --port /dev/ttyACM0 query
```

**Expected output:**
```
Querying Arduino...

Signature: speeduino-travis
Version:   Travis Digital Dash v1.0
```

### 2. Read Raw OCH Block

```bash
cargo run -- --port /dev/ttyACM0 raw-och
```

**Expected output:**
```
Reading raw OCH block...

87-byte OCH block (hex):
0000: 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00
0010: 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00
...

Parsed values:
  RPM:              0
  Speed (kph):      0
  Coolant (°C):     -40
  Oil Temp (°C):    -40
  Oil Pressure:     0 psi
  ...
```

### 3. Monitor Real-Time Data

```bash
cargo run -- --port /dev/ttyACM0 monitor
```

**Expected output:**
```
Monitoring sensor data (Ctrl+C to exit)...

  RPM  Speed Coolant   Oil T   Oil P   Boost   Odometer
----------------------------------------------------------------------
    0      0    -40°C    -40°C      0psi      0kPa     2200.0mi
  850      0     92°C     88°C     45psi    -15kPa     2200.0mi
 3200     45    105°C     98°C     60psi     12kPa     2200.1mi
```

### 4. Test with TunerStudio (Optional)

1. Open TunerStudio
2. Create new project with `travis.cea-TS-DASH-config-Final.ini`
3. Select serial port (e.g., `/dev/ttyACM0` or `COM3`)
4. Connect at 115200 baud
5. Dashboard should display live sensor data

## Troubleshooting

### Build Errors

**Error: `avr-gcc` not found**

```
Solution: Install AVR toolchain (see "Manual Installation" above)
```

**Error: `error: no default toolchain configured`**

```bash
rustup default nightly
```

**Error: `can't find crate for core`**

```bash
rustup component add rust-src --toolchain nightly
```

### Flashing Errors

**Error: `avrdude: stk500v2_ReceiveMessage(): timeout`**

```
Solutions:
1. Try a different baud rate: -b 57600
2. Press reset button on Arduino right before flashing
3. Check USB cable (some are charge-only)
4. Try different USB port
```

**Error: `Permission denied` on `/dev/ttyACM0`**

```bash
# Linux: Add user to dialout group
sudo usermod -a -G dialout $USER
# Log out and back in

# macOS: Check cable and try different port
```

### Runtime Errors

**Pi receiver shows: `Failed to open serial port`**

```
Solutions:
1. Check port name: ls /dev/tty* or ls /dev/ttyACM*
2. Ensure Arduino is powered and connected
3. Check permissions (Linux: dialout group)
4. Try specifying port explicitly: --port /dev/ttyACM0
```

**Arduino not responding to commands**

```
Solutions:
1. Verify baud rate matches (115200)
2. Try resetting Arduino (press reset button)
3. Flash firmware again
4. Check serial monitor for garbage output (baud mismatch)
```

**VSS shows speed when vehicle is stationary**

```
This is expected behavior for new firmware. The VSS processor requires:
- Engine RPM > 1800 (suppresses idle-correlated noise)
- 4 consecutive good windows of ≥6 pulses each
- Speed ≥ 5 kph

If you still see false speeds, check VSS wiring for noise.
```

## Development Workflow

### Run Tests

```bash
cd rust
cargo test --workspace
```

### Check Code

```bash
cargo clippy --workspace
cargo fmt --check
```

### Build Documentation

```bash
cargo doc --open
```

### Clean Build Artifacts

```bash
cargo clean
```

## Memory Profiling

### Check Stack Usage

```bash
avr-nm -S --size-sort \
  target/avr-atmega2560/release/arduino-firmware.elf \
  | grep -i stack
```

### Check Global Variables

```bash
avr-nm -S --size-sort \
  target/avr-atmega2560/release/arduino-firmware.elf \
  | head -20
```

## Next Steps

- See [HARDWARE.md](HARDWARE.md) for pin mappings and sensor wiring
- See [PROTOCOL.md](PROTOCOL.md) for TunerStudio protocol details
- See [../README.md](../README.md) for project overview
