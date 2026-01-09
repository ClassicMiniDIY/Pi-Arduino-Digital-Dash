# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

This is a DIY digital dashboard system for automotive applications, designed for a 02-07 Subaru WRX/STI but adaptable to any vehicle. The system uses:
- **Raspberry Pi 5** running DietPi OS with TunerStudio Dash
- **Arduino Mega 2560** for sensor data acquisition and serial communication
- **MCP2515 CAN module** (optional) for direct CANBUS integration with standalone ECUs

The Arduino acts as a bridge between physical sensors (or CANBUS) and the TunerStudio Dash software running on the Pi, emulating the Speeduino protocol.

## Architecture

### Three-Tier Communication Model

1. **Sensor/CANBUS Layer** → Arduino reads analog sensors OR receives CANBUS frames
2. **Serial Protocol Layer** → Arduino communicates with Pi via USB serial at 115200 baud using TunerStudio protocol
3. **Display Layer** → Raspberry Pi runs TunerStudio Dash which queries Arduino and renders the dashboard

### TunerStudio Protocol Implementation

Both Arduino sketches emulate Speeduino's serial protocol:
- **Signature**: `speeduino-travis` (used for device identification)
- **Query Commands**:
  - `Q` → Returns signature string (32 bytes)
  - `S` → Returns version string (32 bytes)
  - `r` → Returns OCH (Output Channel) block (87 bytes)
- **OCH Block Structure** (87 bytes): Maps sensor data to specific byte offsets matching the INI file configuration

The `travis.cea-TS-DASH-config-Final.ini` file defines the complete protocol specification that TunerStudio uses to decode the Arduino's data packets.

### Key OCH Block Mappings

| Data Field | Type | Offset | Units/Format |
|------------|------|--------|--------------|
| RPM | U16 | 0-1 | raw RPM |
| Oil Temp | U08 | 3 | °C + 40 |
| MAP/Boost | U16 | 4-5 | kPa (absolute or gauge) |
| Coolant Temp | U08 | 7 | °C + 40 |
| Oil Pressure | U08 | 10 | PSI |
| Fuel Pressure | U08 | 11 | PSI |
| Fuel Level | U08 | 15 | % (0-100) |
| Speed | U16 | 19-20 | kph |
| Odometer | float | 60-63 | miles (little-endian) |
| Indicators | U08 | 40-44 | binary flags |

## File Structure

### Arduino Sketches (.ino)

#### travis.cea_Digital_Dash.ino
Main production sketch for analog sensor reading. Implements:
- **ADC filtering**: Multi-sample with outlier trimming and EMA smoothing
- **ProSport sensor support**: Non-linear 2-wire thermistor and 0.5-4.5V pressure senders
- **Boost sensor**: Custom voltage-to-PSI lookup table with zero-trim calibration
- **Interrupt-driven RPM/VSS**: Hardware interrupts on pins 2 (RPM) and 3 (VSS)
- **Advanced VSS filtering**: Multi-stage validation to eliminate idle-correlated noise and startup glitches
- **EEPROM odometer**: Persistent storage with magic value validation and wear-leveling

**Pin Assignments**:
- A0: Oil Pressure
- A1: Fuel Pressure
- A2: Coolant Temp
- A3: Oil Temp
- A4: Boost
- A5: Fuel Level
- D2: RPM (interrupt)
- D3: VSS (interrupt)
- D22-26: Turn signals, CEL, high beam, handbrake (INPUT_PULLUP)

**Critical Tuning Constants** (lines 17-106):
- `ODOMETER_INITIAL_MILES`: Set only on first boot
- `EMA_ALPHA_FAST/SLOW`: Filter response rates
- `PRESSURE_V_MIN/MAX`, `PRESSURE_PSI_MAX`: Linear pressure sensor calibration
- `BOOST_V[]`, `BOOST_PSI[]`: ProSport boost lookup table
- `BOOST_ZERO_TRIM_PSI`: KOEO (Key On Engine Off) zero-offset correction
- `PS_TEMP_F[]`, `PS_OHMS[]`: ProSport thermistor curve
- `VSS_PULSES_PER_KM`: Vehicle-specific speed calibration
- `PULSES_PER_REV`: Tach signal pulses per revolution

#### Haltech_To_Arduino_DRAFT.ino
Work-in-progress sketch for CANBUS integration with Haltech ECUs via MCP2515 module. Key differences:
- Uses `mcp_can.h` library (must be installed: Sketch → Include Library → Manage Libraries → search "MCP_CAN_lib")
- Receives sensor data from CANBUS frames instead of reading analog pins
- **CANBUS Configuration** (lines 38-58): Frame IDs and scaling factors must match your specific Haltech stream configuration
- Same TunerStudio serial protocol output as analog version

**SPI Pin Connections** (Arduino Mega to MCP2515):
- CS: D53
- MISO: D50
- MOSI: D51
- SCK: D52
- INT: D2

See `CANBUS_Communication_Setup.md` for detailed MCP2515 wiring and configuration.

### Configuration Files

#### travis.cea-TS-DASH-config-Final.ini
TunerStudio configuration file (~326KB). Defines:
- Communication protocol commands and timing
- OCH block byte layout with scaling/offset formulas
- Gauge definitions, warning thresholds, and display units
- Dashboard panel configurations

This file must be loaded in TunerStudio Dash on the Raspberry Pi to properly decode the Arduino's data stream.

### Documentation

#### README.md
- Complete hardware BOM with Amazon links
- DietPi image setup instructions (pre-configured with TS Dash autostart)
- Arduino IDE upload procedure
- 3D print files link for Subaru cluster housing

#### CANBUS_Communication_Setup.md
- MCP2515 library installation steps
- Pinout tables for MCP2515 ↔ Arduino Mega and MCP2515 ↔ Haltech CAN
- Notes on 5V logic levels and SPI communication

## Development Workflow

### Testing Arduino Code

1. **Upload sketch**: Use Arduino IDE on a separate machine (not the Pi)
   - Select board: "Arduino Mega or Mega 2560"
   - Select processor: "ATmega2560"
   - Select port: Usually `/dev/cu.usbmodem*` (Mac) or `COM*` (Windows)
   - Click upload (→) button

2. **Serial monitoring**: Use Arduino IDE Serial Monitor at 115200 baud
   - Send `Q` → Should return `speeduino-travis` (padded to 32 bytes)
   - Send `S` → Should return version string
   - Send `r` → Returns 87-byte binary data (will appear as garbage in monitor)

3. **Bench testing sensors**:
   - Power Arduino via USB (5V from computer) or use 12V→5V buck converter
   - Simulate sensors with potentiometers or known voltage sources
   - Verify ADC readings and scaling in Serial Monitor (add debug prints, then remove before production)

### Modifying Sensor Calibration

When changing sensor types or calibration:

1. **Update constants** in the USER CONFIG section (lines 14-106 in main sketch)
2. **Test on bench** with known inputs (e.g., 2.5V should read 72.5 PSI with default pressure scaling)
3. **Verify OCH mapping**: Ensure correct byte offset and scaling in `updateOch()` function
4. **Match INI file**: If changing units or scaling, update the corresponding OutputChannels entry in the INI file

### Adding New Sensors

1. **Assign pin** in PINS section (lines 130-145)
2. **Add calibration constants** in USER CONFIG
3. **Read/filter in loop**: Add to `updateOch()` or create separate update function
4. **Pack into OCH block**: Write to appropriate byte offset in `och[]` array
5. **Update INI file**: Add OutputChannel entry with correct offset, type, scaling

### Working with CANBUS Version

**Important**: The Haltech sketch is a draft. Before use:
1. **Sniff your CANBUS**: Use a CAN analyzer to identify actual frame IDs and byte layouts from your ECU
2. **Update frame mappings**: Modify `ID_RPM_MAP`, `ID_TEMPS`, etc. in lines 54-58
3. **Verify scaling factors**: Haltech may send temps in 0.1°C increments, pressures in kPa, etc.
4. **Test frame reception**: Add debug Serial prints in `handleCanFrame()` during development (remove for production)

## Common Issues

### VSS (Speed) Reading Zero or Erratic
- Check `VSS_PULSES_PER_KM` calibration constant (line 83)
- Verify VSS signal wire connection to D3
- Ensure ground is shared between vehicle and Arduino
- VSS noise filtering is aggressive to prevent idle-correlated spikes; may require `VSS_IDLE_RPM_CUTOFF` adjustment (line 93)

### Odometer Not Saving/Resetting
- Odometer is saved when change > 0.1 miles (line 227)
- Magic value `0xC0DE` at EEPROM address 8 indicates initialized memory
- To reset: change `ODOMETER_INITIAL_MILES` and clear EEPROM (upload blank sketch or use EEPROM.clear())

### RPM Spikes or Drops
- Adjust `RPM_MIN_PULSE_US` for noise rejection (line 76)
- Increase decel damping: reduce `DECEL_RPM_PER_SEC` (line 79)
- Check wiring for electrical noise near ignition coils

### Temperature Reading 100°F Floor
- This is intentional "ProSport-style floor" (line 58)
- Indicates sensor ADC out of range or open circuit
- Check `TEMP_ADC_SANITY_MIN/MAX` thresholds (lines 62-63)
- Verify thermistor resistance curve matches `PS_OHMS[]` table

### TunerStudio Won't Connect
- Verify Arduino signature matches INI: should be `speeduino-travis` (line 111)
- Check baud rate: must be 115200 on both sides (line 125)
- Ensure **printer cable red wire is cut** (data transfer only, no +5V backfeed from Pi)
- Test with Serial Monitor first before connecting to Pi

## Important Notes

- This system is designed for **display only**, not engine control. The Arduino has no authority over vehicle functions.
- The Speeduino protocol emulation is **read-only**; write commands ('w', 'p') are stubbed out.
- Sensor filtering uses aggressive EMA to prevent gauge flutter; adjust alpha values (lines 30-31) for faster/slower response.
- EEPROM has limited write cycles (~100k); odometer saves are throttled to 0.1-mile increments.
- The project is open-source for personal use; author requests users not resell as complete systems.
