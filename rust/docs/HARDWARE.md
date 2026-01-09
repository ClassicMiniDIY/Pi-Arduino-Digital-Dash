# Hardware Configuration

Pin mappings, sensor specifications, and wiring diagrams for the Arduino Mega 2560 Digital Dash.

## Table of Contents

1. [Pin Assignments](#pin-assignments)
2. [Sensor Specifications](#sensor-specifications)
3. [Wiring Diagrams](#wiring-diagrams)
4. [Sensor Calibration](#sensor-calibration)
5. [Troubleshooting](#troubleshooting)

## Pin Assignments

### Analog Inputs (ADC)

| Pin | Function          | Sensor Type             | Range      | Notes                    |
|-----|-------------------|-------------------------|------------|--------------------------|
| A0  | Oil Pressure      | 0-5V Linear Sender      | 0-100 PSI  | Direct voltage read      |
| A1  | Fuel Pressure     | 0-5V Linear Sender      | 0-100 PSI  | Direct voltage read      |
| A2  | Coolant Temp      | ProSport Thermistor     | 104-302°F  | 1kΩ pullup, log-linear   |
| A3  | Oil Temp          | ProSport Thermistor     | 104-302°F  | 1kΩ pullup, log-linear   |
| A4  | Boost Pressure    | ProSport Boost Sensor   | -14.5 to +43.5 PSI | 5-point lookup |
| A5  | Reserved (AFR)    | Future expansion        | -          | Not currently used       |

### Digital Inputs (Interrupt-Driven)

| Pin | Function     | Signal Type      | Trigger     | Notes                          |
|-----|--------------|------------------|-------------|--------------------------------|
| D2  | RPM (Tach)   | Falling edge     | INT0        | 2 pulses/rev (Subaru)          |
| D3  | VSS (Speed)  | Falling edge     | INT1        | 8000 pulses/km                 |

### Digital Inputs (Indicators, Active Low with Pullups)

| Pin | Function       | Active State | Notes                       |
|-----|----------------|--------------|-----------------------------|
| D22 | Turn Left      | LOW          | Pull LOW when active        |
| D23 | Turn Right     | LOW          | Pull LOW when active        |
| D24 | Check Engine   | LOW          | CEL from ECU                |
| D25 | High Beam      | LOW          | High beam indicator         |
| D26 | Neutral        | LOW          | Transmission neutral switch |

### Communication

| Pin     | Function      | Protocol | Baud Rate | Notes                    |
|---------|---------------|----------|-----------|--------------------------|
| TX0/RX0 | Serial to Pi  | UART     | 115200    | TunerStudio protocol     |
| USB     | Programming   | USB      | -         | Also for serial monitor  |

## Sensor Specifications

### ProSport Thermistor (CLT & Oil Temp)

**Type**: 2-wire negative temperature coefficient (NTC) thermistor

**Wiring**:
```
Arduino 5V ──────┬─────────────── Arduino Analog Pin (A2 or A3)
                 │
              [1kΩ]  Pull-up resistor
                 │
                 └─────────────── ProSport Sensor Terminal 1

ProSport Sensor Terminal 2 ───── Arduino GND
```

**Resistance-Temperature Curve** (7 points):

| Temperature (°F) | Resistance (Ω) |
|------------------|----------------|
| 104              | 5830           |
| 140              | 3020           |
| 176              | 1670           |
| 212 (100°C)      | 975            |
| 248              | 599            |
| 284              | 386            |
| 302              | 316            |

**Conversion Formula**:
1. Voltage divider: `R_thermistor = R_pullup × (ADC / (1023 - ADC))`
2. Log-linear interpolation through lookup table
3. Apply 100°F floor, 302°F ceiling
4. Convert to Celsius: `C = (F - 32) × 5/9`
5. Encode as U8: `encoded = (C + 40)`

### ProSport Boost Sensor

**Type**: Analog voltage output (0-5V)

**Wiring**:
```
ProSport Sensor:
  Red   ──── Arduino 5V
  Black ──── Arduino GND
  White ──── Arduino A4
```

**Voltage-to-PSI Curve** (5 points):

| Voltage (V) | Pressure (PSI) |
|-------------|----------------|
| 0.02        | -14.5          |
| 1.00        | 0.0            |
| 2.00        | +14.5          |
| 3.00        | +29.0          |
| 4.00        | +43.51         |

**Conversion Formula**:
1. Linear interpolation through lookup table
2. Apply KOEO zero-trim: `PSI += 8.0` (calibration offset)
3. Apply EMA filtering (alpha = 0.25)
4. Convert to kPa: `kPa = PSI × 6.89476`
5. Clamp to range: `-101.325 to +300.0 kPa`

### Pressure Senders (Oil & Fuel)

**Type**: Linear 0-5V analog senders

**Wiring**:
```
Sender:
  Signal ──── Arduino A0 (oil) or A1 (fuel)
  Power  ──── 12V vehicle power (or Arduino 5V if compatible)
  Ground ──── Chassis ground
```

**Conversion Formula**:
```
PSI = (ADC / 1023.0) × 100.0
```

**EMA Filtering**: Alpha = 0.12 (fast response for pressure changes)

### RPM (Tachometer) Signal

**Type**: Falling-edge pulse train from ECU or ignition coil

**Specifications**:
- Pulses per revolution: **2** (Subaru boxer engines)
- Signal voltage: 5V or 12V (voltage divider if >5V)
- Minimum period: 3ms (noise rejection)
- Maximum RPM: 9000

**Wiring**:
```
Tach Signal ──┬─── Arduino D2 (INT0)
              │
           [10kΩ]  Optional pull-down for noise
              │
             GND
```

**Conversion Formula**:
```
RPM = 60,000,000 µs/min / (period_µs × pulses_per_rev)
```

**Filtering**:
- Spike rejection: Discard changes > 2000 RPM
- Asymmetric filtering: Instant acceleration, damped deceleration (22,000 RPM/sec)
- Timeout: 400ms without pulse → RPM = 0

### VSS (Vehicle Speed Sensor)

**Type**: Hall effect or magnetic pulse sensor

**Specifications**:
- Pulses per kilometer: **8000** (typical for most vehicles)
- Signal voltage: 5V or 12V (voltage divider if >5V)
- Debounce time: 800µs (noise rejection)

**Wiring**:
```
VSS Signal ──┬─── Arduino D3 (INT1)
             │
          [10kΩ]  Optional pull-down for noise
             │
            GND
```

**State Machine**:
- **Windowed pulse counting**: 250ms windows
- **Motion lock-in**: Requires 4 consecutive good windows
  - Good window: ≥6 pulses, ≥5 kph, RPM > 1800
  - Continuous pulses (no gaps > 300ms)
- **Idle suppression**: Block VSS when RPM < 1800 (prevents false motion)
- **Glitch protection**: Gradual decay (×0.75 per window) instead of instant zero
- **EMA filtering**: Adaptive alpha (0.08 below 10 kph, 0.25 above)

**Conversion Formula**:
```
pulses_per_sec = pulses / 0.25  (250ms window)
kph = (pulses_per_sec × 3600) / 8000
```

## Wiring Diagrams

### Complete System Overview

```
┌─────────────────────────────────────────────────────────────┐
│                    Arduino Mega 2560                        │
│                                                             │
│  A0 ◄─── Oil Pressure Sender (0-5V linear)                 │
│  A1 ◄─── Fuel Pressure Sender (0-5V linear)                │
│  A2 ◄─── Coolant Temp (ProSport thermistor + 1kΩ pullup)   │
│  A3 ◄─── Oil Temp (ProSport thermistor + 1kΩ pullup)       │
│  A4 ◄─── Boost Sensor (ProSport 0-5V)                      │
│  A5 ◄─── Reserved (AFR)                                    │
│                                                             │
│  D2 ◄─── RPM Signal (tach, falling edge, INT0)             │
│  D3 ◄─── VSS Signal (speed, falling edge, INT1)            │
│                                                             │
│  D22 ◄─── Turn Left (active low, pullup)                   │
│  D23 ◄─── Turn Right (active low, pullup)                  │
│  D24 ◄─── CEL (active low, pullup)                         │
│  D25 ◄─── High Beam (active low, pullup)                   │
│  D26 ◄─── Neutral (active low, pullup)                     │
│                                                             │
│  USB ──── Raspberry Pi 5 (serial @ 115200 baud)            │
│                                                             │
│  VIN ◄─── 12V vehicle power (regulated to 5V on-board)     │
│  GND ──── Chassis ground                                   │
└─────────────────────────────────────────────────────────────┘
```

### Thermistor Circuit Detail

```
                     Arduino Mega 2560
                     ┌───────────┐
                     │           │
5V ─────────┬────────┤ 5V        │
            │        │           │
         [1kΩ]       │           │
            │        │           │
            ├────────┤ A2 (CLT)  │
            │        │           │
      ProSport       │           │
      Thermistor     │           │
            │        │           │
GND ────────┴────────┤ GND       │
                     │           │
                     └───────────┘
```

**Voltage Divider Formula**:
```
V_adc = 5V × (R_thermistor / (R_pullup + R_thermistor))
```

At 212°F (100°C, typical operating temp):
```
R_thermistor ≈ 975Ω
V_adc = 5V × (975 / (1000 + 975)) = 2.47V
ADC = 2.47 / 5.0 × 1023 ≈ 505 counts
```

## Sensor Calibration

### KOEO (Key On Engine Off) Calibration

**Boost Sensor Zero-Trim**:

The boost sensor reads atmospheric pressure when the engine is off. To calibrate:

1. Turn key to ON (engine OFF)
2. Read boost sensor voltage (should be ~1.0V at sea level)
3. Expected reading: 0 PSI (atmospheric)
4. If reading is off, adjust `zero_trim_psi` in code

**Current setting**: `+8.0 PSI` (typical ProSport offset)

**Location in code**: `crates/arduino-firmware/src/main.rs:30`
```rust
let mut boost_processor = BoostProcessor::new(&BOOST_V, &BOOST_PSI, 8.0);
                                                                      ^^^
```

### Odometer Initialization

**Initial mileage** is set in EEPROM module.

**Location in code**: `crates/arduino-firmware/src/eeprom.rs:13`
```rust
const INITIAL_MILES: f32 = 2200.0;
```

**To change**:
1. Edit `INITIAL_MILES` value
2. Erase EEPROM by uploading sketch with different magic value
3. Re-flash firmware with new initial mileage

### VSS Calibration (Pulses per Kilometer)

**Default**: 8000 pulses/km (common for most vehicles)

**To calibrate**:
1. Drive a known distance (e.g., between mile markers)
2. Count VSS pulses (or let firmware estimate)
3. Calculate: `pulses_per_km = total_pulses / distance_km`
4. Update firmware constant

**Location in code**: `crates/arduino-firmware/src/main.rs:36`
```rust
let mut vss_processor = VssProcessor::new(8000.0);
                                          ^^^^^^
```

## Troubleshooting

### Temperature Readings

**Problem**: Temperature shows -40°C or 215°C constantly

**Causes**:
- **-40°C (encoded as 0)**: Thermistor disconnected or open circuit
- **215°C (encoded as 255)**: Thermistor shorted to ground

**Solutions**:
1. Check thermistor connections
2. Verify 1kΩ pullup resistor is installed
3. Test resistance with multimeter (should be ~1kΩ at room temp)

**Problem**: Temperature reads ~100°F floor even when hot

**Cause**: Floor limit is being applied (sensor reading below minimum)

**Solutions**:
1. Check thermistor wiring polarity
2. Verify correct pullup resistor value (1kΩ, not 10kΩ)
3. Check for cold solder joints

### RPM Signal

**Problem**: RPM shows 0 or fluctuates wildly

**Causes**:
- Missing pull-down resistor on signal line
- Noise from ignition system
- Incorrect pulses_per_rev setting
- Bad ground connection

**Solutions**:
1. Add 10kΩ pull-down resistor from D2 to GND
2. Use shielded cable for tach signal
3. Verify pulses_per_rev (2 for Subaru, 1 for most others)
4. Check ground connections

### VSS Signal

**Problem**: Speed shows non-zero at idle

**Expected**: This is NORMAL for initial startup. The VSS hysteresis requires:
- Engine RPM > 1800
- 4 consecutive good windows of signal
- Speed ≥ 5 kph

**If still seeing false speeds**:
1. Check RPM reading (must be > 1800 to unlock motion)
2. Verify VSS wiring is away from ignition/spark wires
3. Add 10kΩ pull-down resistor from D3 to GND
4. Check for transmission-related electrical noise

**Problem**: Speed reads too high or too low

**Cause**: Incorrect `pulses_per_km` calibration

**Solution**: Follow VSS calibration procedure above

### Boost Pressure

**Problem**: Boost shows ~55 kPa at idle (should be ~-15 kPa vacuum)

**Cause**: Incorrect KOEO zero-trim

**Solution**: Adjust `zero_trim_psi` parameter:
```rust
// Increase if reading too high (add more negative offset)
let mut boost_processor = BoostProcessor::new(&BOOST_V, &BOOST_PSI, 10.0);

// Decrease if reading too low
let mut boost_processor = BoostProcessor::new(&BOOST_V, &BOOST_PSI, 6.0);
```

## Safety Notes

⚠️ **Important Safety Warnings**:

1. **Power Supply**: Use voltage regulation if vehicle power exceeds 16V
2. **Ground Loops**: Connect all sensors to single ground point
3. **ESD Protection**: Use anti-static precautions when handling Arduino
4. **Heat**: Keep Arduino away from exhaust manifold (max temp: 85°C)
5. **Vibration**: Use vibration-damping mounts for Arduino enclosure
6. **Water**: Seal enclosure against moisture and road spray

## Next Steps

- See [BUILDING.md](BUILDING.md) for firmware build and flash instructions
- See [PROTOCOL.md](PROTOCOL.md) for TunerStudio protocol details
- See [../README.md](../README.md) for project overview
