# TunerStudio Protocol Reference

Complete specification of the Speeduino-compatible serial protocol used by the Arduino Digital Dash.

## Table of Contents

1. [Overview](#overview)
2. [Commands](#commands)
3. [OCH Block Structure](#och-block-structure)
4. [Data Encoding](#data-encoding)
5. [Example Transactions](#example-transactions)
6. [Integration with TunerStudio](#integration-with-tunerstudio)

## Overview

### Protocol Basics

- **Baud Rate**: 115200
- **Data Bits**: 8
- **Parity**: None
- **Stop Bits**: 1
- **Flow Control**: None
- **Signature**: `speeduino-travis`
- **Protocol Type**: Speeduino-compatible

### Communication Model

```
┌──────────────┐         ┌────────────────┐         ┌─────────────┐
│  TunerStudio │  USB    │ Arduino Mega   │  USB    │ Raspberry   │
│   (Windows)  │ ◄────► │ Digital Dash   │ ◄────► │   Pi 5      │
│              │         │ (Firmware)     │         │ (Receiver)  │
└──────────────┘         └────────────────┘         └─────────────┘
```

**Supported clients**:
- TunerStudio MS (Windows/Linux/macOS)
- Custom Pi receiver CLI
- Any serial terminal (minicom, screen, PuTTY)

## Commands

All commands are single-byte ASCII characters.

### Command Summary

| Command | Byte | Description              | Response Size | Notes                  |
|---------|------|--------------------------|---------------|------------------------|
| `Q`     | 0x51 | Query Signature          | 32 bytes      | Identifies firmware    |
| `S`     | 0x53 | Query Version            | 32 bytes      | Firmware version       |
| `r`     | 0x72 | Read OCH Block           | 87 bytes      | Real-time sensor data  |
| `F`     | 0x46 | Read Flags               | 1 byte (ACK)  | Not implemented        |
| `p`     | 0x70 | Read Page                | 128 bytes     | Returns zeros          |
| `b`     | 0x62 | Burn Page                | 1 byte (ACK)  | Not implemented        |
| `d`     | 0x64 | Get CRC32                | 4 bytes       | Returns 0xFFFFFFFF     |

### Command Details

#### `Q` - Query Signature (0x51)

**Request**:
```
Hex:  51
ASCII: 'Q'
```

**Response**: 32 bytes (zero-padded)
```
speeduino-travis\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0
```

**Purpose**: Identifies firmware as Speeduino-compatible for TunerStudio auto-detection.

---

#### `S` - Query Version (0x53)

**Request**:
```
Hex:  53
ASCII: 'S'
```

**Response**: 32 bytes (zero-padded)
```
Travis Digital Dash v1.0\0\0\0\0\0\0\0\0
```

**Purpose**: Reports firmware version string.

---

#### `r` - Read OCH Block (0x72)

**Request**:
```
Hex:  72
ASCII: 'r'
```

**Response**: 87 bytes (see [OCH Block Structure](#och-block-structure))

**Purpose**: Read all real-time sensor data in a single transaction.

**Update Rate**: Limited by serial bandwidth
- At 115200 baud: ~13 OCH blocks/second (77ms per block)
- Firmware updates at ~40Hz internally, serial is the bottleneck

---

#### `F` - Read Flags (0x46)

**Request**:
```
Hex:  46
ASCII: 'F'
```

**Response**: 1 byte (acknowledgment)
```
Hex:  21
ASCII: '!'
```

**Purpose**: Read configuration flags (not implemented).

---

#### `p` - Read Page (0x70)

**Request**:
```
Hex:  70
ASCII: 'p'
```

**Response**: 128 bytes (all zeros)

**Purpose**: Read configuration page (not implemented - dash is read-only).

---

#### `b` - Burn Page (0x62)

**Request**:
```
Hex:  62
ASCII: 'b'
```

**Response**: 1 byte (acknowledgment)
```
Hex:  21
ASCII: '!'
```

**Purpose**: Write configuration to EEPROM (not implemented).

---

#### `d` - Get CRC32 (0x64)

**Request**:
```
Hex:  64
ASCII: 'd'
```

**Response**: 4 bytes (little-endian)
```
Hex:  FF FF FF FF
```

**Purpose**: Get page CRC checksum (not implemented - returns dummy value).

## OCH Block Structure

The **Output Channel (OCH) block** is an 87-byte structure containing all real-time sensor data.

### Byte Map

| Offset | Size | Type  | Name              | Range / Encoding              | Units       |
|--------|------|-------|-------------------|-------------------------------|-------------|
| 0      | 2    | u16   | RPM               | 0-9000 (little-endian)        | RPM         |
| 2      | 2    | u16   | Speed             | 0-300 (little-endian)         | km/h        |
| 4      | 1    | u8    | Oil Pressure      | 0-255                         | PSI         |
| 5      | 1    | u8    | Fuel Pressure     | 0-255                         | PSI         |
| 6      | 1    | u8    | Battery Voltage   | V × 10 (e.g., 142 = 14.2V)    | decivolts   |
| 7      | 1    | u8    | Coolant Temp      | °C + 40 (0 = -40°C)           | °C + 40     |
| 8      | 1    | u8    | Oil Temp          | °C + 40 (0 = -40°C)           | °C + 40     |
| 9      | 2    | i16   | Boost Pressure    | kPa (little-endian, signed)   | kPa (gauge) |
| 11     | 1    | u8    | Turn Left         | 0 = off, 1 = on               | boolean     |
| 12     | 1    | u8    | Turn Right        | 0 = off, 1 = on               | boolean     |
| 13     | 1    | u8    | Check Engine      | 0 = off, 1 = on               | boolean     |
| 14     | 1    | u8    | High Beam         | 0 = off, 1 = on               | boolean     |
| 15     | 1    | u8    | Neutral           | 0 = off, 1 = on               | boolean     |
| 16-59  | 44   | -     | Reserved          | Zeros (future expansion)      | -           |
| 60     | 4    | f32   | Odometer          | IEEE 754 (little-endian)      | miles       |
| 64-86  | 23   | -     | Reserved          | Zeros (padding to 87 bytes)   | -           |

### Field Details

#### RPM (bytes 0-1)

**Type**: 16-bit unsigned integer (little-endian)

**Encoding**:
```
rpm_bytes[0] = rpm & 0xFF          // Low byte
rpm_bytes[1] = (rpm >> 8) & 0xFF   // High byte
```

**Example**:
```
3000 RPM → 0xB8 0x0B → [0xB8, 0x0B] in OCH
```

**Decoding** (Python):
```python
rpm = (och[1] << 8) | och[0]
```

---

#### Speed (bytes 2-3)

**Type**: 16-bit unsigned integer (little-endian)

**Encoding**: Same as RPM

**Example**:
```
120 km/h → 0x78 0x00 → [0x78, 0x00] in OCH
```

---

#### Temperatures (bytes 7-8)

**Type**: 8-bit unsigned integer

**Encoding**: `value = temperature_celsius + 40`

**Range**: -40°C to +215°C (0 to 255)

**Example**:
```
100°C → 100 + 40 = 140 → 0x8C
 25°C →  25 + 40 =  65 → 0x41
-40°C → -40 + 40 =   0 → 0x00
```

**Decoding** (Python):
```python
temp_c = och[7] - 40  # Coolant
temp_c = och[8] - 40  # Oil
```

---

#### Boost Pressure (bytes 9-10)

**Type**: 16-bit signed integer (little-endian)

**Encoding**: Direct kPa value (gauge pressure, not absolute)

**Range**: -101.325 kPa (full vacuum) to +300 kPa (boost)

**Example**:
```
 15 kPa →  15 → 0x0F 0x00
-20 kPa → -20 → 0xEC 0xFF (two's complement)
```

**Decoding** (Python):
```python
import struct
boost_kpa = struct.unpack('<h', och[9:11])[0]  # Signed 16-bit LE
```

---

#### Boolean Indicators (bytes 11-15)

**Type**: 8-bit unsigned integer (0 or 1)

**Encoding**:
```
0 = indicator OFF
1 = indicator ON
```

**Example**:
```
Turn left ON → och[11] = 0x01
CEL active   → och[13] = 0x01
```

---

#### Odometer (bytes 60-63)

**Type**: 32-bit IEEE 754 floating-point (little-endian)

**Encoding**:
```
bytes = miles.to_le_bytes()  // IEEE 754 little-endian
```

**Example**:
```
2345.6 miles → IEEE 754 → 0xCD 0xCC 0x12 0x45
```

**Decoding** (Python):
```python
import struct
odometer_miles = struct.unpack('<f', och[60:64])[0]
```

## Data Encoding

### Little-Endian Multi-Byte Values

All multi-byte values (u16, i16, f32) use **little-endian** byte order.

**Example** (RPM = 3000):
```
Binary:     0000 1011 1011 1000 (0x0BB8)
Byte order: [0xB8, 0x0B]
             ^^^^  ^^^^
             LSB   MSB
```

### Temperature Encoding

Temperatures use **offset binary** encoding to support negative values with u8:

```
Encoded = Celsius + 40

-40°C → 0
  0°C → 40
100°C → 140
215°C → 255
```

This allows -40°C to +215°C range in a single unsigned byte.

### Boolean Encoding

Boolean values use simple 0/1 encoding:

```
0x00 = False / OFF
0x01 = True  / ON
```

### IEEE 754 Float Encoding

The odometer uses standard IEEE 754 single-precision (32-bit) float:

```
Sign (1 bit) | Exponent (8 bits) | Mantissa (23 bits)
```

Always stored in **little-endian** byte order.

## Example Transactions

### Example 1: Query Signature

**Request** (hex):
```
51
```

**Response** (hex, 32 bytes):
```
73 70 65 65 64 75 69 6E 6F 2D 74 72 61 76 69 73
00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00
```

**Response** (ASCII):
```
"speeduino-travis\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0"
```

---

### Example 2: Read OCH Block

**Request** (hex):
```
72
```

**Response** (hex, first 16 bytes shown):
```
B8 0B   78 00   3C   28   8E   8C   86   0F 00
↑↑↑↑↑   ↑↑↑↑↑   ↑↑   ↑↑   ↑↑   ↑↑   ↑↑   ↑↑↑↑↑
RPM     Speed   Oil  Fuel Batt CLT  Oil  Boost
3000    120     60   40   142  100  94   15 kPa
```

**Parsed values**:
- RPM: 3000 (0x0BB8 little-endian)
- Speed: 120 km/h (0x0078 little-endian)
- Oil Pressure: 60 PSI
- Fuel Pressure: 40 PSI
- Battery: 14.2V (142 ÷ 10)
- Coolant: 100°C (140 - 40)
- Oil Temp: 94°C (134 - 40)
- Boost: 15 kPa (0x000F little-endian)

---

### Example 3: Complete Python Decoder

```python
import struct
import serial

def read_och_block(port):
    """Read and parse OCH block from Arduino"""
    port.write(b'r')
    och = port.read(87)

    if len(och) != 87:
        raise ValueError(f"Expected 87 bytes, got {len(och)}")

    # Parse multi-byte values (little-endian)
    rpm = struct.unpack('<H', och[0:2])[0]
    speed_kph = struct.unpack('<H', och[2:4])[0]
    boost_kpa = struct.unpack('<h', och[9:11])[0]  # Signed
    odometer = struct.unpack('<f', och[60:64])[0]

    # Parse single-byte values
    oil_pressure = och[4]
    fuel_pressure = och[5]
    battery_v = och[6] / 10.0
    coolant_c = och[7] - 40
    oil_temp_c = och[8] - 40

    # Parse boolean indicators
    turn_left = bool(och[11])
    turn_right = bool(och[12])
    cel = bool(och[13])
    high_beam = bool(och[14])
    neutral = bool(och[15])

    return {
        'rpm': rpm,
        'speed_kph': speed_kph,
        'oil_pressure_psi': oil_pressure,
        'fuel_pressure_psi': fuel_pressure,
        'battery_volts': battery_v,
        'coolant_celsius': coolant_c,
        'oil_temp_celsius': oil_temp_c,
        'boost_kpa': boost_kpa,
        'odometer_miles': odometer,
        'turn_left': turn_left,
        'turn_right': turn_right,
        'check_engine': cel,
        'high_beam': high_beam,
        'neutral': neutral,
    }

# Usage
with serial.Serial('/dev/ttyACM0', 115200, timeout=1) as port:
    data = read_och_block(port)
    print(f"RPM: {data['rpm']}")
    print(f"Speed: {data['speed_kph']} km/h")
    print(f"Coolant: {data['coolant_celsius']}°C")
```

## Integration with TunerStudio

### INI File Configuration

TunerStudio uses an INI file to define the protocol mapping.

**Key sections** (from `travis.cea-TS-DASH-config-Final.ini`):

```ini
[MegaTune]
signature = "speeduino-travis"  ; Must match 'Q' command response

[Constants]
; No tunable parameters (read-only dash)

[OutputChannels]
ochGetCommand = "r"  ; Command to read OCH block
ochBlockSize = 87    ; Total size of OCH block

; Field definitions (name = type, offset)
RPM           = scalar, U16, 0,    "RPM", 1.000, 0.00
Speed         = scalar, U16, 2,    "km/h", 1.000, 0.00
OilPressure   = scalar, U08, 4,    "PSI", 1.000, 0.00
FuelPressure  = scalar, U08, 5,    "PSI", 1.000, 0.00
BatteryVolts  = scalar, U08, 6,    "V", 0.100, 0.00
CoolantTemp   = scalar, U08, 7,    "°C", 1.000, -40.00
OilTemp       = scalar, U08, 8,    "°C", 1.000, -40.00
BoostPressure = scalar, S16, 9,    "kPa", 1.000, 0.00
Odometer      = scalar, F32, 60,   "miles", 1.000, 0.00
```

### Dashboard Configuration

Create custom gauges in TunerStudio:

1. Open TunerStudio
2. Go to: **Dashboard → New Gauge**
3. Select data field (e.g., `RPM`)
4. Configure display range and units
5. Save dashboard layout

**Example gauge configurations**:

- **Tachometer**: 0-9000 RPM, red line at 7000
- **Speedometer**: 0-200 km/h
- **Coolant Temp**: 0-120°C, warning at 105°C
- **Oil Pressure**: 0-100 PSI, warning below 10 PSI
- **Boost**: -101 to +200 kPa

### Serial Connection Settings

**TunerStudio → Communications**:

1. Port: `COM3` (Windows) or `/dev/ttyACM0` (Linux)
2. Baud Rate: `115200`
3. Data Bits: `8`
4. Parity: `None`
5. Stop Bits: `1`

## Next Steps

- See [BUILDING.md](BUILDING.md) for firmware compilation
- See [HARDWARE.md](HARDWARE.md) for pin mappings and wiring
- See [../README.md](../README.md) for project overview
