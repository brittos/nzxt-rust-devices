# NZXT Kraken Z3 Rust Driver (nzxt-kraken)

A customized Rust driver and CLI tool for controlling NZXT Kraken Z-series liquid coolers (Z53, Z63, Z73). This tool provides precise control over pump/fan speeds, LCD visual modes, and supports uploading custom images and **animated GIFs**.

## Features

-   **Monitoring**: Real-time status (Liquid Temp, Pump Speed, Fan Speed).
-   **Control**: Set fixed speeds or custom profiles for Pump and Fan.
-   **LCD Control**:
    -   Adjust Brightness and Orientation.
    -   Set Visual Modes (Info, Liquid, etc.).
    -   **Image Upload**: Upload static images (JPG/PNG).
    -   **GIF Support**: Upload animated GIFs with automatic resizing and re-encoding.
    -   **LCD Monitor**: Real-time dashboard on the cooler's LCD (Customizable Radial Gauge).
    -   **Customization**: Configure colors and styles via `config.json`.
-   **Cross-Platform**: Designed for Linux (and compatible with Windows structure).

## Installation

### Prerequisites

-   [Rust & Cargo](https://www.rust-lang.org/tools/install) installed.

### USB Driver Setup (Windows Only)

For **Image and GIF uploading** to work on Windows, the device's bulk interface requires the `WinUSB` driver.

1.  Download [Zadig](https://zadig.akeo.ie/).
2.  Open Zadig and select `Options -> List All Devices`.
3.  Find `NZXT Kraken Z Device` (or similar).
    *   *Note: There may be multiple entries. You need the one associated with the **Bulk** interface (often interface 1 or 2).*
4.  Replace the driver with **WinUSB**.

*On Linux, standard `libusb` and udev rules are usually sufficient.*

### Build

```bash
cargo build --release
```


## Usage

Run the CLI using `cargo run -- <command>` or direct execution of the binary.

### Initialization & Status

**Check Status**:
```bash
cargo run -- status
# Output: Liquid: 32°C | Pump: 2200rpm | Fan: 800rpm
```

**Monitor**:
Continuously monitor status in the terminal.
```bash
cargo run -- monitor --interval 2
```

### Start (Unified Mode)

The **`start`** command combines LCD monitoring (Radial Gauge) and Cooling Daemon into a single unified loop. It reads default settings from `config.json` and allows CLI overrides.

```bash
# Start with defaults from config.json
cargo run -- start

# Override profile and source
cargo run -- start --profile performance --source cpu --interval 5
```

**`config.json` `startup` section:**
```json
{
  "startup": {
    "display_mode": "radial",   // "radial", "image", or "gif"
    "cooling_profile": "silent",
    "temperature_source": "liquid", // "liquid" or "cpu"
    "interval": 2,
    "brightness": 100,
    "orientation": 0
  }
}
```

### Cooling Control

**Set Fixed Speed** (Duty 0-100%):
```bash
cargo run -- set-pump 80  # Set pump to 80%
cargo run -- set-fan 50   # Set fan to 50%
```

**Cooling Daemon (Smart Control)**:
Continuously adjust pump/fan speeds based on temperature curves (Liquid or CPU).
```bash
# Run with 'silent' profile using Liquid temp (default)
cargo run -- cooling-daemon

# Run with 'performance' profile using CPU temp
cargo run -- cooling-daemon --profile performance --source cpu --interval 2
```


### LCD Control

**Brightness & Orientation**:
```bash
cargo run -- set-brightness 100  # Set brightness to 100%
cargo run -- set-orientation 1   # 0=0°, 1=90°, 2=180°, 3=270°
```

**Visual Modes**:
```bash
cargo run -- set-lcd-mode 1 0    # Mode 1 (Liquid), Index 0
cargo run -- set-lcd-mode 2 0    # Mode 2 (CPU Info), Index 0
cargo run -- set-lcd-mode 4 0    # Mode 4 (Infographic), Index 0
```

### Image & GIF Upload

Uploads are automatically resized to 320x320.

**Static Image**:
```bash
cargo run -- upload-image ./my_photo.jpg
```

**Animated GIF**:
The tool automatically detects `.gif` files, re-encodes them to the correct device format, and manages the upload.
```bash
cargo run -- upload-image ./Linus.gif
```

### LCD Monitor (Radial Gauge)

Display real-time system stats (CPU/GPU Temp) on the LCD with a custom Radial Gauge design.

This mode is **fully customizable**. The application typically creates a `config.json`. You can edit this file to change:
-   **Colors**: Gradient stops (start color, end color).
-   **Background**: The background color of the screen.
-   **Geometry**: Radius and angles of the gauge.

```bash
cargo run -- lcd-monitor --interval 2
```

### Memory Management

Manage the "buckets" used for storing images on the LCD.

**List Buckets**:
View which buckets are occupied.
```bash
cargo run -- list-buckets
```

**Delete All Buckets**:
Clear all LCD memory (useful if uploads fail or memory is full).
```bash
cargo run -- delete-buckets
```

### Profiles

**Save/Apply Speed Profiles**:
```bash
# Apply "performance" profile to fan
cargo run -- profile performance --channel fan

# Apply "silent" profile to pump
cargo run -- profile silent --channel pump
```

**LCD Profiles**:
Quickly switch between presets.
```bash
cargo run -- lcd-profile night  # Low brightness
cargo run -- lcd-profile day    # High brightness
```

### Diagnostics

**List Devices**:
```bash
cargo run -- list
```

**Device Info**:
```bash
cargo run -- info
```

**HID Debug**:
Read raw HID packets for debugging protocol issues.
```bash
cargo run -- debug --count 10
```

**LCD Debug**:
Dump raw LCD configuration bytes (brightness, orientation, etc).
```bash
cargo run -- debug-lcd
```

**List Sensors**:
View detected system sensors to verify temperature readings.
```bash
cargo run -- sensors
```


**Discover Presets**:
Sweep through visual mode indices to find hidden presets.
```bash
cargo run -- discover-presets --mode 4 --max 20
```

### Development & Examples

**Radial Gauge Preview**:
Generate test images of the radial stat gauge in the `tmp/` folder. Useful for testing UI changes without the device.
```bash
cargo run --example radial_preview
```

**One-Time Stats Upload**:
Generate and upload a single frame of the stats image (useful for testing).
```bash
cargo run -- lcd-stats
```

## Troubleshooting

-   **"Failed to open bulk device"**: Ensure you have installed the **WinUSB** driver using Zadig (Windows).
-   **"Access Denied"**: Close NZXT CAM completely (Systray -> Exit). It conflicts with this tool.
-   **Temp = 0°C**: Run the terminal as **Administrator** to access hardware sensors.


#  Disclaimer

I provide no guarantees or warranties for the code or the functionality provided. **Use at your own risk.**

