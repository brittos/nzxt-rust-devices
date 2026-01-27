//! NZXT Kraken Z63 Control CLI
//!
//! Command-line interface for monitoring and controlling NZXT Kraken Z-series coolers.

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use nzxt_rust_devices::device::KrakenZ63;

use nzxt_rust_devices::storage;
use nzxt_rust_devices::utils::parsing::{parse_channel, parse_speed_profile};
use nzxt_rust_devices::utils::sensors::SystemSensors;

// =============================================================================
// CLI Arguments
// =============================================================================

/// NZXT Kraken Z63 Control Tool
#[derive(Parser, Debug)]
#[command(name = "nzxt-kraken-cli")]
#[command(author, version, about, long_about = None)]
struct Args {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand, Debug)]
enum Command {
    /// Show current device status
    Status,

    /// Continuously monitor device status
    Monitor {
        /// Update interval in seconds
        #[arg(short, long, default_value = "1")]
        interval: u64,
    },

    /// Set fixed pump speed
    SetPump {
        /// Duty cycle percentage (20-100)
        #[arg(value_parser = clap::value_parser!(u8).range(20..=100))]
        duty: u8,
    },

    /// Set fixed fan speed
    SetFan {
        /// Duty cycle percentage (0-100)
        #[arg(value_parser = clap::value_parser!(u8).range(0..=100))]
        duty: u8,
    },

    /// Set LCD brightness
    SetBrightness {
        /// Brightness percentage (0-100)
        #[arg(value_parser = clap::value_parser!(u8).range(0..=100))]
        brightness: u8,
    },

    /// Set LCD visual mode
    SetLcdMode {
        /// Mode ID (1=CPU, 2=GPU, 3=Liquid, 4=Infographic/Dual)
        mode: u8,

        /// Bucket Index or Sensor/Layout selection (default 0)
        #[arg(default_value = "0")]
        index: u8,
    },

    /// Set LCD orientation
    SetOrientation {
        /// Orientation ID (0=0°, 1=90°, 2=180°, 3=270°)
        orientation: u8,
    },

    /// Delete all LCD memory buckets (Reset visual memory)
    DeleteBuckets,

    /// List LCD memory buckets status
    ListBuckets,

    /// Upload an image to the LCD
    UploadImage {
        /// Path to the image file (jpg, png, gif)
        path: PathBuf,
    },

    /// Apply a speed profile
    Profile {
        /// Profile name: silent, performance, or fixed:XX
        name: String,

        /// Channel to apply profile: fan or pump
        #[arg(short, long, default_value = "fan")]
        channel: String,
    },

    /// Apply an LCD visual profile
    LcdProfile {
        /// Profile name: off, night, day, max
        name: String,
    },

    /// List connected Kraken devices
    List,

    /// Show device firmware version
    Info,

    /// Debug: show raw HID bytes to find correct offsets
    Debug {
        /// Number of reads to perform
        #[arg(short, long, default_value = "5")]
        count: u32,
    },

    /// Debug: Dump raw LCD information (0x30 0x01)
    DebugLcd,

    /// Discovery: Sweep through Mode 4 indices to find presets
    DiscoverPresets {
        /// Mode ID (default 4)
        #[arg(default_value = "4")]
        mode: u8,
        /// Maximum index to scan (default 20)
        #[arg(short, long, default_value = "20")]
        max: u8,
    },

    /// Check if USB bulk interface is available (for image uploads)
    CheckBulk,

    /// Generate and upload a stats image to the LCD
    LcdStats,

    /// Continuously update LCD with live stats
    LcdMonitor {
        /// Update interval in seconds (default 5)
        #[arg(short, long, default_value = "5")]
        interval: u64,
    },

    /// Diagnostic: List all available system sensors
    Sensors,

    /// Run cooling daemon with temperature-based fan/pump control
    CoolingDaemon {
        /// Profile name: silent, performance, fixed (default: silent)
        #[arg(short, long, default_value = "silent")]
        profile: String,

        /// Temperature source: liquid or cpu (default: liquid)
        #[arg(short, long, default_value = "liquid")]
        source: String,

        /// Update interval in seconds (default: 2)
        #[arg(short, long, default_value = "2")]
        interval: u64,
    },

    /// Start unified LCD monitor + Cooling daemon
    Start {
        /// Cooling profile name: silent, performance, fixed (default: silent)
        #[arg(short, long, default_value = "silent")]
        profile: String,

        /// Temperature source: liquid or cpu (default: liquid)
        #[arg(short, long, default_value = "liquid")]
        source: String,

        /// Update interval in seconds (default: 2)
        #[arg(short = 'n', long, default_value = "2")]
        interval: u64,
    },
}

// =============================================================================
// Main
// =============================================================================

fn main() -> Result<()> {
    let args = Args::parse();

    match args.command {
        Command::Status => cmd_status(),
        Command::Monitor { interval } => cmd_monitor(interval),
        Command::SetPump { duty } => cmd_set_pump(duty),
        Command::SetFan { duty } => cmd_set_fan(duty),
        Command::SetBrightness { brightness } => cmd_set_brightness(brightness),
        Command::SetLcdMode { mode, index } => cmd_set_lcd_mode(mode, index),
        Command::SetOrientation { orientation } => cmd_set_orientation(orientation),
        Command::DeleteBuckets => cmd_delete_buckets(),
        Command::ListBuckets => cmd_list_buckets(),
        Command::UploadImage { path } => cmd_upload_image(&path),
        Command::LcdProfile { name } => cmd_lcd_profile(&name),
        Command::Profile { name, channel } => cmd_profile(&name, &channel),
        Command::List => cmd_list(),
        Command::Info => cmd_info(),
        Command::Debug { count } => cmd_debug(count),
        Command::DebugLcd => cmd_debug_lcd(),
        Command::DiscoverPresets { mode, max } => cmd_discover_presets(mode, max),
        Command::CheckBulk => cmd_check_bulk(),
        Command::LcdStats => cmd_lcd_stats(),
        Command::LcdMonitor { interval } => cmd_lcd_monitor(interval),
        Command::Sensors => cmd_sensors(),
        Command::CoolingDaemon {
            profile,
            source,
            interval,
        } => cmd_cooling_daemon(&profile, &source, interval),
        Command::Start {
            profile,
            source,
            interval,
        } => cmd_start(&profile, &source, interval),
    }
}

// =============================================================================
// Command Implementations
// =============================================================================

fn cmd_set_brightness(brightness: u8) -> Result<()> {
    let kraken = KrakenZ63::open().context("Failed to open Kraken Z63")?;
    // kraken.initialize().context("Failed to initialize device")?; // Often not needed for brightness only

    kraken
        .set_brightness(brightness)
        .context("Failed to set brightness")?;
    println!("✅ LCD brightness set to {}%", brightness);
    Ok(())
}

fn cmd_set_orientation(orientation: u8) -> Result<()> {
    let kraken = KrakenZ63::open().context("Failed to open Kraken Z63")?;

    let angle = match orientation {
        0 => "0°",
        1 => "90°",
        2 => "180°",
        3 => "270°",
        _ => "unknown",
    };

    println!(
        "🔄 Setting LCD orientation to {} (ID: {})...",
        angle, orientation
    );
    kraken
        .set_orientation(orientation)
        .context("Failed to set orientation")?;
    println!("✅ LCD orientation set successfully.");
    Ok(())
}

fn cmd_set_lcd_mode(mode: u8, index: u8) -> Result<()> {
    let mut kraken = KrakenZ63::open().context("Failed to open Kraken Z63")?;
    kraken.initialize().context("Failed to initialize device")?;

    kraken
        .set_visual_mode(mode, index)
        .context("Failed to set visual mode")?;
    println!("✅ LCD visual mode set to {} (idx {})", mode, index);
    Ok(())
}

fn cmd_delete_buckets() -> Result<()> {
    let mut kraken = KrakenZ63::open().context("Failed to open Kraken Z63")?;
    kraken.initialize().context("Failed to initialize device")?;

    println!("🗑️  Deleting all memory buckets (0-15)...");
    kraken
        .delete_all_buckets()
        .context("Failed to delete buckets")?;
    println!("✅ All buckets deleted.");
    Ok(())
}

fn cmd_debug_lcd() -> Result<()> {
    let mut kraken = KrakenZ63::open().context("Failed to open Kraken Z63")?;
    kraken.initialize().context("Failed to initialize device")?;

    println!("🔍 Requesting raw LCD Info (0x30 0x01)...");
    let (brightness, orientation, raw) = kraken.get_lcd_info_raw()?;

    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("💡 Brightness:  {}%", brightness);
    println!("🔄 Orientation: {} ({}°)", orientation, orientation * 90);
    println!("📦 Raw Bytes:   {:02X?}", raw);
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");

    Ok(())
}

fn cmd_discover_presets(mode: u8, max_index: u8) -> Result<()> {
    let mut kraken = KrakenZ63::open().context("Failed to open Kraken Z63")?;
    kraken.initialize().context("Failed to initialize device")?;

    println!("🧪 Starting preset discovery for Mode {}...", mode);
    println!("(Please observe the LCD after each step)\n");

    for i in 0..=max_index {
        print!("Testing Mode {} Index {}... ", mode, i);
        std::io::Write::flush(&mut std::io::stdout())?;

        match kraken.set_visual_mode(mode, i) {
            Ok(_) => println!("✅ Sent"),
            Err(e) => println!("❌ Error: {}", e),
        }

        std::thread::sleep(Duration::from_millis(1500));
    }

    println!("\n✅ Discovery complete.");
    Ok(())
}

fn cmd_upload_image(path: &PathBuf) -> Result<()> {
    use nzxt_rust_devices::device::bucket_manager::BucketManager;
    use nzxt_rust_devices::device::bulk;
    use nzxt_rust_devices::utils::image_processing;

    let mut kraken = KrakenZ63::open().context("Failed to open Kraken Z63")?;
    kraken.initialize().context("Failed to initialize device")?;

    // Get current LCD orientation to apply rotation
    let (_, orientation) = kraken.get_lcd_info().context("Failed to get LCD info")?;
    println!(
        "🔄 LCD Orientation: {} ({}°)",
        orientation,
        orientation as u16 * 90
    );

    // Initialize BucketManager to manage memory correctly
    let mut bucket_manager =
        BucketManager::from_device(&kraken).context("Failed to initialize BucketManager")?;

    println!("🖼️  Processing image: {:?}", path);

    // Acquire a bucket from the manager (handles deletion/FIFO if needed)
    let bucket_idx = bucket_manager.acquire(&kraken);
    println!("🪣 Selected bucket: {}", bucket_idx);

    // Check extension for GIF
    let is_gif = path
        .extension()
        .map(|e| e.to_string_lossy().to_lowercase() == "gif")
        .unwrap_or(false);

    if is_gif {
        println!("🎞️  GIF detected! Processing frames...");
        // process_gif now accepts orientation parameter
        let (frames_data, num_frames) = image_processing::process_gif(path, orientation)
            .map_err(|e| anyhow::anyhow!("Failed to process GIF: {}", e))?;

        println!(
            "📤 Starting GIF upload ({} frames, {} bytes)...",
            num_frames,
            frames_data.len()
        );
        // Use upload_image_bulk with asset_type=0x01 (GIF)
        kraken
            .upload_image_bulk(bucket_idx, &frames_data, 0x01)
            .context("Failed to upload GIF")?;
    } else {
        // Load and prepare image using bulk module with current orientation
        let image_data = bulk::load_image(path, orientation)
            .map_err(|e| anyhow::anyhow!("Failed to process image: {}", e))?;

        println!("📤 Uploading ({} bytes) via USB bulk...", image_data.len());

        // Use bucket 0 - asset_type=0x02 (Static)
        kraken
            .upload_image_bulk(bucket_idx, &image_data, 0x02)
            .context("Failed to upload image")?;
    }

    println!("✅ Image/GIF uploaded and displayed!");

    Ok(())
}

fn cmd_lcd_profile(name: &str) -> Result<()> {
    let profile = match name.to_lowercase().as_str() {
        "off" => nzxt_rust_devices::config::LcdProfile::OFF,
        "night" => nzxt_rust_devices::config::LcdProfile::NIGHT,
        "day" => nzxt_rust_devices::config::LcdProfile::DAY,
        "max" => nzxt_rust_devices::config::LcdProfile::MAX,
        _ => {
            return Err(nzxt_rust_devices::KrakenError::InvalidInput(format!(
                "Unknown LCD profile: {}",
                name
            ))
            .into());
        }
    };

    let kraken = KrakenZ63::open().context("Failed to open Kraken Z63")?;
    // Initialization might not be needed for just setting mode/brightness if already running?
    // But good practice.
    // kraken.initialize().context("Failed to initialize device")?;

    println!("🎨 Applying LCD profile: {}", name);
    println!("   Brightness: {}%", profile.brightness);
    println!("   Mode: {} (Bucket {})", profile.mode, profile.bucket);

    kraken.set_brightness(profile.brightness)?;
    kraken.set_visual_mode(profile.mode, profile.bucket)?;

    println!("✅ Profile applied successfully!");
    Ok(())
}

fn cmd_status() -> Result<()> {
    let mut kraken = KrakenZ63::open().context("Failed to open Kraken Z63")?;
    kraken.initialize().context("Failed to initialize device")?;
    let status = kraken.get_status().context("Failed to read status")?;
    print!("{}", status);
    Ok(())
}

fn cmd_list_buckets() -> Result<()> {
    let kraken = KrakenZ63::open().context("Failed to open Kraken Z63")?;

    println!("📦 LCD Memory Buckets Status:");
    println!("{}", "─".repeat(50));

    let buckets = kraken
        .query_all_buckets()
        .context("Failed to query buckets")?;

    let mut total_used: u32 = 0;
    let mut occupied_count = 0;

    for (idx, exists, start_page, size_pages) in &buckets {
        let size_kb = *size_pages as u32; // 1KB per page
        if *exists {
            println!(
                "  Bucket {:2}: ✅ Ocupado | Offset: {:5} KB | Size: {:4} KB",
                idx, start_page, size_kb
            );
            total_used += size_kb;
            occupied_count += 1;
        } else {
            println!("  Bucket {:2}: ❌ Livre", idx);
        }
    }

    println!("{}", "─".repeat(50));
    println!(
        "  📊 Resumo: {}/16 buckets ocupados | {:5} KB / 24320 KB usados",
        occupied_count, total_used
    );

    Ok(())
}

fn cmd_monitor(interval_secs: u64) -> Result<()> {
    use sysinfo::System;

    let mut kraken = KrakenZ63::open().context("Failed to open Kraken Z63")?;
    kraken.initialize().context("Failed to initialize device")?;

    let mut sys = System::new_all();
    let mut sensors = SystemSensors::new();

    // Setup Ctrl+C handler
    let running = Arc::new(AtomicBool::new(true));
    let r = running.clone();

    ctrlc::set_handler(move || {
        r.store(false, Ordering::SeqCst);
    })
    .context("Failed to set Ctrl+C handler")?;

    println!("🌡️  Monitoring Kraken Z63 (Ctrl+C to stop)...");
    println!(
        "📡 Syncing CPU/GPU telemetry to LCD every {}s\n",
        interval_secs
    );

    while running.load(Ordering::SeqCst) {
        // Refresh system data
        sys.refresh_all();
        sensors.refresh();

        let cpu_count = sys.cpus().len();
        let sensor_count = sensors.count();

        // Use SystemSensors for temperature detection
        let cpu_temp = sensors.find_cpu_temp().unwrap_or(0.0) as u8;
        let gpu_temp = sensors.find_gpu_temp().unwrap_or(0.0) as u8;

        // Send telemetry (Z3 Mode 1/3)
        let _ = kraken.set_host_info(cpu_temp, gpu_temp);

        match kraken.get_status() {
            Ok(status) => {
                // Clear screen and move cursor to top
                print!("\x1B[2J\x1B[1;1H");
                println!(
                    "📡 Telemetry Synced: CPU: {}°C | GPU: {}°C",
                    cpu_temp, gpu_temp
                );

                // Debug: If 0, list first 5 sensors to find labels
                if cpu_temp == 0 || gpu_temp == 0 {
                    println!(
                        "🔍 DEBUG: {} CPUs detected, {} sensors detected.",
                        cpu_count, sensor_count
                    );
                    if sensor_count > 0 {
                        let all_sensors = sensors.list_all();
                        let first_few: Vec<_> = all_sensors
                            .iter()
                            .take(5)
                            .map(|s| format!("{}: {:.1}°C", s.label, s.temperature))
                            .collect();
                        println!(
                            "🔍 DEBUG Sensors ({} found, first 5): {:?}",
                            sensor_count, first_few
                        );

                        // Special tip for Windows
                        if sensor_count == 1 && first_few[0].contains("Computer") {
                            println!(
                                "💡 TIP: Only generic motherboard sensor found. For per-core CPU/GPU metrics, ensure you run as ADMIN."
                            );
                        }
                    } else {
                        println!("🔍 DEBUG: No sensors detected! (Check Permissions)");
                    }
                }

                println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
                print!("{}", status);
            }
            Err(e) => {
                eprintln!("⚠️  Read error: {}", e);
            }
        }

        std::thread::sleep(Duration::from_secs(interval_secs));
    }

    println!("\n👋 Monitoring stopped.");
    Ok(())
}

fn cmd_sensors() -> Result<()> {
    use sysinfo::System;

    println!("🔍 Scanning for system sensors...");
    let sensors = SystemSensors::new();
    let count = sensors.count();

    if count == 0 {
        println!("❌ No sensors detected. (Are you running as Admin?)");
        // Try refreshing system-wide just in case
        let mut sys = System::new_all();
        sys.refresh_all();
        println!("   System detected {} CPUs.", sys.cpus().len());
        return Ok(());
    }

    println!("✅ Found {} sensors:\n", count);
    println!("{:<40} | {:<10} | {:<10}", "Label", "Temp", "Critical");
    println!("{}", "─".repeat(66));

    // Get the CPU sensor that would be selected
    let cpu_sensor = sensors.find_cpu_sensor();
    let all_sensors = sensors.list_all();

    for sensor in &all_sensors {
        let critical = sensor
            .critical
            .map(|c| format!("{:.1}°C", c))
            .unwrap_or_else(|| "-".to_string());

        // Check if this is the sensor that would be selected
        let is_selected = cpu_sensor
            .as_ref()
            .map(|s| s.label == sensor.label)
            .unwrap_or(false);

        let prefix = if is_selected { "👉" } else { "  " };

        println!(
            "{} {:<40} | {:.1}°C    | {}",
            prefix, sensor.label, sensor.temperature, critical
        );
    }

    println!("{}", "─".repeat(66));
    if cpu_sensor.is_none() {
        println!("⚠️  Warning: Current logic would NOT select any of these sensors for CPU Temp.");
    } else {
        println!("👉 = Sensor currently selected by the app");
    }

    Ok(())
}

fn cmd_set_pump(duty: u8) -> Result<()> {
    // Persist to defaults
    println!(
        "Updating 'Fixed' profile speed to {}% in defaults.json...",
        duty
    );
    if let Err(e) = nzxt_rust_devices::storage::update_fixed("pump", duty) {
        eprintln!("Warning: Failed to update defaults: {}", e);
    }

    let mut kraken = KrakenZ63::open().context("Failed to open Kraken Z63")?;
    kraken.initialize().context("Failed to initialize device")?;

    kraken
        .set_pump_speed(duty)
        .context("Failed to set pump speed")?;

    println!("✅ Pump speed set to {}%", duty);
    Ok(())
}

fn cmd_set_fan(duty: u8) -> Result<()> {
    // Persist to defaults
    println!(
        "Updating 'Fixed' profile speed to {}% in defaults.json...",
        duty
    );
    if let Err(e) = nzxt_rust_devices::storage::update_fixed("fan", duty) {
        eprintln!("Warning: Failed to update defaults: {}", e);
    }

    let mut kraken = KrakenZ63::open().context("Failed to open Kraken Z63")?;
    kraken.initialize().context("Failed to initialize device")?;

    kraken
        .set_fan_speed(duty)
        .context("Failed to set fan speed")?;

    println!("✅ Fan speed set to {}%", duty);
    Ok(())
}

fn cmd_profile(name: &str, channel_str: &str) -> Result<()> {
    let profile = parse_speed_profile(name)?;
    let channel = parse_channel(channel_str)?;

    let mut kraken = KrakenZ63::open().context("Failed to open Kraken Z63")?;
    kraken.initialize().context("Failed to initialize device")?;

    let curve = profile.to_duty_curve().context("Failed to build curve")?;

    // Convert curve to profile points for set_speed_profile
    let points: Vec<(u8, u8)> = curve
        .iter()
        .enumerate()
        .map(|(i, &duty)| (20 + i as u8, duty))
        .collect();

    kraken
        .set_speed_profile(channel, &points)
        .context("Failed to apply profile")?;

    println!("✅ Applied {} profile to {}", profile.name(), channel);
    Ok(())
}

fn cmd_list() -> Result<()> {
    let devices = KrakenZ63::list_devices().context("Failed to enumerate devices")?;

    if devices.is_empty() {
        println!("❌ No Kraken Z63 devices found.");
        return Ok(());
    }

    println!("🔍 Found {} device(s):\n", devices.len());
    for (i, (path, serial)) in devices.iter().enumerate() {
        let serial_str = serial.as_deref().unwrap_or("unknown");
        println!("  {}. Serial: {}", i + 1, serial_str);
        println!("     Path: {}", path);
    }

    Ok(())
}

fn cmd_info() -> Result<()> {
    let mut kraken = KrakenZ63::open().context("Failed to open Kraken Z63")?;
    let firmware = kraken.initialize().context("Failed to initialize device")?;

    println!("╭─────────────────────────────────╮");
    println!("│      NZXT Kraken Z63 Info       │");
    println!("├─────────────────────────────────┤");
    println!("│  Firmware: {:>19}  │", firmware);
    println!("╰─────────────────────────────────╯");

    Ok(())
}

fn cmd_debug(count: u32) -> Result<()> {
    use hidapi::HidApi;
    use nzxt_rust_devices::protocol::commands::{
        CMD_INIT_COMPLETE, CMD_INIT_INTERVAL, HID_REPORT_LENGTH, KRAKEN_Z3_PID, NZXT_VID,
        RESP_STATUS,
    };

    println!("🔍 Debug Mode - Reading raw HID bytes...\n");

    let api = HidApi::new().context("Failed to init HID")?;
    let device = api
        .open(NZXT_VID, KRAKEN_Z3_PID)
        .context("Failed to open Kraken Z63")?;

    // Initialize device first (like liquidctl does)
    println!("📡 Initializing device...");
    let mut buf64 = [0u8; HID_REPORT_LENGTH];
    buf64[..CMD_INIT_INTERVAL.len()].copy_from_slice(&CMD_INIT_INTERVAL);
    device.write(&buf64).context("Failed to write init1")?;
    buf64 = [0u8; HID_REPORT_LENGTH];
    buf64[..CMD_INIT_COMPLETE.len()].copy_from_slice(&CMD_INIT_COMPLETE);
    device.write(&buf64).context("Failed to write init2")?;

    println!("✅ Device initialized. Reading status messages...\n");

    // Read multiple times to find status messages
    let mut status_count = 0u32;
    let max_reads = count * 10; // Try more reads to find status messages

    for i in 0..max_reads {
        if status_count >= count {
            break;
        }

        let mut buf = [0u8; 64];
        let read = device
            .read_timeout(&mut buf, 1000)
            .context("Failed to read")?;

        if read == 0 {
            continue;
        }

        // Print all messages with their headers
        println!(
            "━━━ Read #{} ({} bytes) - Header: [{:#04x}, {:#04x}] ━━━",
            i + 1,
            read,
            buf[0],
            buf[1]
        );

        // Filter for status message (RESP_STATUS = [0x75, 0x01])
        let is_status = buf[0] == RESP_STATUS[0] && buf[1] == RESP_STATUS[1];

        if is_status {
            status_count += 1;
            println!("✅ STATUS MESSAGE FOUND!");
        }

        // Print all bytes in groups of 8
        for (row, chunk) in buf.chunks(8).enumerate() {
            let offset = row * 8;
            print!("[{:02}-{:02}] ", offset, offset + 7);
            for b in chunk {
                print!("{:3} ", b);
            }
            // Also print hex
            print!(" | ");
            for b in chunk {
                print!("{:02x} ", b);
            }
            println!();
        }

        // Try to interpret common offsets for status-like messages
        if buf[0] == RESP_STATUS[0] || read > 20 {
            println!("\n📊 Potential Status Interpretations:");

            // Different offset attempts
            for start in 1..20 {
                let temp_int = buf[start];
                let temp_dec = buf[start + 1];
                let temp = temp_int as f32 + (temp_dec as f32 / 10.0);

                // Only show if temp looks reasonable (25-50°C)
                if temp > 25.0 && temp < 50.0 {
                    println!(
                        "  [{:02}-{:02}] Temp: {:.1}°C (int={}, dec={})",
                        start,
                        start + 1,
                        temp,
                        temp_int,
                        temp_dec
                    );
                }
            }

            // Look for RPM values (typically 700-3000)
            for start in (1..30).step_by(2) {
                let rpm_le = (buf[start + 1] as u16) << 8 | (buf[start] as u16);
                let rpm_be = (buf[start] as u16) << 8 | (buf[start + 1] as u16);

                if rpm_le > 500 && rpm_le < 4000 {
                    println!("  [{:02}-{:02}] RPM (LE): {}", start, start + 1, rpm_le);
                }
                if rpm_be > 500 && rpm_be < 4000 && rpm_be != rpm_le {
                    println!("  [{:02}-{:02}] RPM (BE): {}", start, start + 1, rpm_be);
                }
            }
        }

        // Look for value around 36 (expected temp)
        println!("\n🎯 Bytes with value 34-38:");
        for (idx, &byte) in buf.iter().enumerate().take(32) {
            if (34..=38).contains(&byte) {
                println!("  [{}] = {} (could be temp)", idx, byte);
            }
        }

        println!();
        std::thread::sleep(std::time::Duration::from_millis(100));
    }

    Ok(())
}

fn cmd_check_bulk() -> Result<()> {
    use nzxt_rust_devices::device::bulk::BulkDevice;

    println!("🔍 Checking USB bulk interface availability...");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");

    match BulkDevice::open() {
        Ok(_device) => {
            println!("✅ Bulk interface is AVAILABLE!");
            println!("   You can upload images to the LCD.");
            println!();
            println!("   Next step: cargo run -- upload-image <path>");
        }
        Err(e) => {
            println!("❌ Bulk interface is NOT available.");
            println!("   Error: {}", e);
            println!();
            println!("   This is expected on Windows without WinUSB driver.");
            println!();
            println!("   To enable image uploads on Windows:");
            println!("   1. Download Zadig: https://zadig.akeo.ie/");
            println!("   2. Options → List All Devices");
            println!("   3. Select 'NZXT Kraken Z63 (Interface 1)'");
            println!("   4. Install WinUSB driver");
            println!();
            println!("   On Linux, it should work without additional setup.");
        }
    }

    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    Ok(())
}

fn cmd_lcd_stats() -> Result<()> {
    use nzxt_rust_devices::device::bucket_manager::BucketManager;
    use nzxt_rust_devices::utils::stats_image;

    let mut kraken = KrakenZ63::open().context("Failed to open Kraken Z63")?;
    kraken.initialize().context("Failed to initialize device")?;

    // Get LCD orientation to apply rotation
    let (_, orientation) = kraken.get_lcd_info().context("Failed to get LCD info")?;
    println!(
        "🔄 LCD Orientation: {} ({}°)",
        orientation,
        orientation as u16 * 90
    );

    // Initialize BucketManager
    let mut bucket_manager =
        BucketManager::from_device(&kraken).context("Failed to initialize BucketManager")?;

    // Ensure config exists
    nzxt_rust_devices::storage::ensure_config_exists()?;

    // Load config to check for custom gauge settings
    let app_config = nzxt_rust_devices::storage::load_config().unwrap_or_default();

    // Try to find a radial gauge config.
    // Logic: Check active profile, check 'default_gauge', or fallback.
    let gauge_config = app_config
        .active_profile
        .as_ref()
        .and_then(|name| app_config.lcd.get(name))
        .or_else(|| app_config.lcd.get("default_gauge"))
        .and_then(|p| p.radial_gauge.as_ref())
        .map(|stored| {
            nzxt_rust_devices::utils::radial_gauge::RadialGaugeConfig::from_stored(stored)
        });

    println!("📊 Generating radial stats image (NZXT CAM style)...");

    // Get current status
    let status = kraken.get_status().context("Failed to get device status")?;

    // Generate radial gauge stats image (new visual style)
    let img = stats_image::generate_radial_stats_image(
        status.liquid_temp_c,
        "LIQUID",
        status.pump_rpm,
        gauge_config.as_ref(),
    )
    .ok_or_else(|| anyhow::anyhow!("Failed to generate image. Font not found."))?;

    // Save to temp file and reload through bulk::load_image for consistent processing
    let temp_path = std::env::temp_dir().join("kraken_stats.png");
    img.save(&temp_path).context("Failed to save temp image")?;

    // Load through bulk module with current orientation
    let image_data = nzxt_rust_devices::device::bulk::load_image(&temp_path, orientation)
        .map_err(|e| anyhow::anyhow!("Failed to process image: {}", e))?;

    // Acquire proper bucket
    let bucket_idx = bucket_manager.acquire(&kraken);
    println!("📤 Uploading to bucket {}...", bucket_idx);

    kraken
        .upload_image_bulk(bucket_idx, &image_data, 0x02)
        .context("Failed to upload image")?;

    println!("✅ LCD updated with radial gauge!");
    println!("   Liquid: {:.1}°C", status.liquid_temp_c);
    println!("   Pump: {} RPM ({}%)", status.pump_rpm, status.pump_duty);
    println!("   Fan: {} RPM ({}%)", status.fan_rpm, status.fan_duty);

    Ok(())
}

fn cmd_lcd_monitor(interval: u64) -> Result<()> {
    use nzxt_rust_devices::device::BucketManager;
    use nzxt_rust_devices::utils::stats_image;

    let mut kraken = KrakenZ63::open().context("Failed to open Kraken Z63")?;
    kraken.initialize().context("Failed to initialize device")?;

    // Get LCD orientation to apply rotation (read once at start)
    let (_, orientation) = kraken.get_lcd_info().context("Failed to get LCD info")?;
    println!(
        "🔄 LCD Orientation: {} ({}°)",
        orientation,
        orientation as u16 * 90
    );

    // Ensure config exists
    nzxt_rust_devices::storage::ensure_config_exists()?;

    // Load config
    let app_config = nzxt_rust_devices::storage::load_config().unwrap_or_default();
    let gauge_config = app_config
        .active_profile
        .as_ref()
        .and_then(|name| app_config.lcd.get(name))
        .or_else(|| app_config.lcd.get("default_gauge"))
        .and_then(|p| p.radial_gauge.as_ref())
        .map(|stored| {
            nzxt_rust_devices::utils::radial_gauge::RadialGaugeConfig::from_stored(stored)
        });

    // Delete all buckets at start to ensure clean state
    println!("🗑️  Clearing LCD memory...");
    kraken.delete_all_buckets().ok();
    std::thread::sleep(Duration::from_millis(100));

    // Initialize BucketManager (starts empty after delete_all_buckets)
    let mut bucket_manager = BucketManager::new();

    println!("📊 Starting LCD radial monitor (Ctrl+C to stop)...");
    println!("   Update interval: {} seconds", interval);
    println!("   Visual: Radial gauge (NZXT CAM style)");
    println!("   Strategy: FIFO (15 buckets)");
    println!();

    // Setup Ctrl+C handler
    let running = Arc::new(AtomicBool::new(true));
    let r = running.clone();
    ctrlc::set_handler(move || {
        r.store(false, Ordering::SeqCst);
    })?;

    let temp_path = std::env::temp_dir().join("kraken_stats_monitor.png");
    let mut cycle_count: u64 = 0;

    while running.load(Ordering::SeqCst) {
        cycle_count += 1;

        // Acquire next available bucket using FIFO strategy
        // Automatically frees oldest bucket if full
        let bucket_idx = bucket_manager.acquire(&kraken);

        // Get current status
        match kraken.get_status() {
            Ok(status) => {
                // Generate radial gauge image
                if let Some(img) = stats_image::generate_radial_stats_image(
                    status.liquid_temp_c,
                    "LIQUID",
                    status.pump_rpm,
                    gauge_config.as_ref(),
                ) {
                    // Save to temp file and process
                    if let Err(e) = img.save(&temp_path) {
                        eprintln!("[{}] ⚠️  Failed to save temp image: {}", cycle_count, e);
                        continue;
                    }

                    // Load with current orientation
                    match nzxt_rust_devices::device::bulk::load_image(&temp_path, orientation) {
                        Ok(image_data) => {
                            // Upload to the acquired bucket
                            if let Err(e) = kraken.upload_image_bulk(bucket_idx, &image_data, 0x02)
                            {
                                eprintln!("[{}] ⚠️  Upload failed: {}", cycle_count, e);
                                // If upload failed, maybe release the bucket or handle it?
                                // For now, we keep going.
                            } else {
                                println!(
                                    "[{}] 🌡️  {:.0}°C | Pump: {} RPM | Bucket: {}",
                                    cycle_count, status.liquid_temp_c, status.pump_rpm, bucket_idx
                                );
                            }
                        }
                        Err(e) => {
                            eprintln!("[{}] ⚠️  Image processing failed: {}", cycle_count, e);
                        }
                    }
                }
            }
            Err(e) => {
                eprintln!("[{}] ⚠️  Failed to get status: {}", cycle_count, e);
            }
        }

        std::thread::sleep(Duration::from_secs(interval));
    }

    println!("\n✅ LCD monitor stopped after {} cycles.", cycle_count);
    Ok(())
}

// =============================================================================
// Cooling Daemon
// =============================================================================

fn cmd_cooling_daemon(profile_name: &str, source: &str, interval: u64) -> Result<()> {
    use nzxt_rust_devices::cooling::{TempSource, interpolate_duty};
    use nzxt_rust_devices::storage;

    // Ensure defaults exist
    storage::ensure_defaults_exist().context("Failed to initialize defaults")?;

    // Load profile from defaults
    let profile = storage::get_profile(profile_name)
        .with_context(|| format!("Failed to load profile '{}'", profile_name))?;

    // Parse temperature source from CLI
    let temp_source = TempSource::from(source);

    // Initialize device
    let mut kraken = KrakenZ63::open().context("Failed to open Kraken Z63")?;
    kraken.initialize().context("Failed to initialize device")?;

    // Initialize sensors
    let mut sensors = SystemSensors::new();

    // Setup Ctrl+C handler
    let running = Arc::new(AtomicBool::new(true));
    let r = running.clone();
    ctrlc::set_handler(move || {
        r.store(false, Ordering::SeqCst);
    })
    .context("Failed to set Ctrl+C handler")?;

    println!("🌡️  Cooling Daemon Started (Ctrl+C to stop)");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("   Profile: {}", profile_name);
    println!("   Source:  {}", temp_source);
    println!("   Interval: {}s", interval);
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!();

    // Extract curves from profile
    let pump_curve: Vec<(u8, u8)> = profile
        .channel_settings
        .iter()
        .find(|c| c.channel_name.to_lowercase() == "pump")
        .and_then(|c| c.mode.as_ref())
        .and_then(|m| m.custom_thresholds.as_ref())
        .map(|t| {
            t.iter()
                .map(|th| (th.temperature, th.fan_percentage))
                .collect()
        })
        .unwrap_or_default();

    let fan_curve: Vec<(u8, u8)> = profile
        .channel_settings
        .iter()
        .find(|c| c.channel_name.to_lowercase() == "fan")
        .and_then(|c| c.mode.as_ref())
        .and_then(|m| m.custom_thresholds.as_ref())
        .map(|t| {
            t.iter()
                .map(|th| (th.temperature, th.fan_percentage))
                .collect()
        })
        .unwrap_or_default();

    println!("📊 Pump curve: {} points", pump_curve.len());
    println!("📊 Fan curve:  {} points", fan_curve.len());
    println!();

    let mut cycle_count: u64 = 0;

    while running.load(Ordering::SeqCst) {
        cycle_count += 1;

        // Get current temperatures
        let status = kraken.get_status().context("Failed to get device status")?;
        sensors.refresh();

        let liquid_temp = status.liquid_temp_c as u8;
        let cpu_temp = sensors.find_cpu_temp().unwrap_or(0.0) as u8;

        // Select temperature based on source
        let current_temp = match temp_source {
            TempSource::Liquid => liquid_temp,
            TempSource::Cpu => cpu_temp,
        };

        // Calculate and apply pump duty
        let pump_duty = if pump_curve.is_empty() {
            70 // Default if no curve
        } else {
            interpolate_duty(&pump_curve, current_temp)
        };
        kraken.set_pump_speed(pump_duty.max(20))?;

        // Calculate and apply fan duty
        let fan_duty = if fan_curve.is_empty() {
            50 // Default if no curve
        } else {
            interpolate_duty(&fan_curve, current_temp)
        };
        kraken.set_fan_speed(fan_duty)?;

        // Display status
        println!(
            "[{:4}] {} {}°C | Pump: {:3}% ({} RPM) | Fan: {:3}%",
            cycle_count,
            match temp_source {
                TempSource::Liquid => "💧",
                TempSource::Cpu => "🔥",
            },
            current_temp,
            pump_duty,
            status.pump_rpm,
            fan_duty
        );

        std::thread::sleep(Duration::from_secs(interval));
    }

    println!("\n✅ Cooling daemon stopped after {} cycles.", cycle_count);
    Ok(())
}

// =============================================================================
// Unified Start Command (LCD Monitor + Cooling Daemon)
// =============================================================================

fn cmd_start(cli_profile: &str, cli_source: &str, cli_interval: u64) -> Result<()> {
    use nzxt_rust_devices::cooling::{TempSource, interpolate_duty};
    use nzxt_rust_devices::device::BucketManager;
    use nzxt_rust_devices::utils::stats_image;

    // Ensure storage exists and load configs
    storage::ensure_defaults_exist().context("Failed to initialize defaults")?;
    nzxt_rust_devices::storage::ensure_config_exists()?;

    // Load config for fallback values
    let app_config = nzxt_rust_devices::storage::load_config().unwrap_or_default();
    let startup = &app_config.startup;

    // Resolve values: CLI overrides config (check if CLI is default value)
    let profile_name = if cli_profile == "silent" {
        &startup.cooling_profile
    } else {
        cli_profile
    };
    let source = if cli_source == "liquid" {
        &startup.temperature_source
    } else {
        cli_source
    };
    let interval = if cli_interval == 2 {
        startup.interval
    } else {
        cli_interval
    };
    let brightness = startup.brightness;
    let config_orientation = startup.orientation;
    let display_mode = startup.display_mode.to_lowercase();

    // Initialize device
    let mut kraken = KrakenZ63::open().context("Failed to open Kraken Z63")?;
    kraken.initialize().context("Failed to initialize device")?;

    // Apply configured brightness
    kraken.set_brightness(brightness)?;

    // Get/apply orientation
    let (_, current_orientation) = kraken.get_lcd_info().context("Failed to get LCD info")?;
    let target_orientation = (config_orientation / 90) as u8;
    if target_orientation != current_orientation && config_orientation > 0 {
        kraken.set_orientation(target_orientation)?;
    }
    let orientation = if config_orientation > 0 {
        target_orientation
    } else {
        current_orientation
    };

    // Initialize sensors
    let mut sensors = SystemSensors::new();

    // Parse temperature source
    let temp_source = TempSource::from(source);

    // Load cooling profile
    let profile = storage::get_profile(profile_name)
        .with_context(|| format!("Failed to load profile '{}'", profile_name))?;

    // Load gauge config
    let gauge_config = app_config
        .active_profile
        .as_ref()
        .and_then(|name| app_config.lcd.get(name))
        .or_else(|| app_config.lcd.get("default_gauge"))
        .and_then(|p| p.radial_gauge.as_ref())
        .map(|stored| {
            nzxt_rust_devices::utils::radial_gauge::RadialGaugeConfig::from_stored(stored)
        });

    // Extract cooling curves from profile
    let pump_curve: Vec<(u8, u8)> = profile
        .channel_settings
        .iter()
        .find(|c| c.channel_name.to_lowercase() == "pump")
        .and_then(|c| c.mode.as_ref())
        .and_then(|m| m.custom_thresholds.as_ref())
        .map(|t| {
            t.iter()
                .map(|th| (th.temperature, th.fan_percentage))
                .collect()
        })
        .unwrap_or_default();

    let fan_curve: Vec<(u8, u8)> = profile
        .channel_settings
        .iter()
        .find(|c| c.channel_name.to_lowercase() == "fan")
        .and_then(|c| c.mode.as_ref())
        .and_then(|m| m.custom_thresholds.as_ref())
        .map(|t| {
            t.iter()
                .map(|th| (th.temperature, th.fan_percentage))
                .collect()
        })
        .unwrap_or_default();

    // Setup Ctrl+C handler
    let running = Arc::new(AtomicBool::new(true));
    let r = running.clone();
    ctrlc::set_handler(move || {
        r.store(false, Ordering::SeqCst);
    })
    .context("Failed to set Ctrl+C handler")?;

    // Print startup info
    println!("🚀 Unified Monitor Started (Ctrl+C to stop)");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("   Mode:     {}", display_mode);
    println!("   Profile:  {}", profile_name);
    println!("   Source:   {}", temp_source);
    println!("   Interval: {}s", interval);
    println!(
        "   LCD:      {}° | {}%",
        orientation as u16 * 90,
        brightness
    );
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!();

    // Handle display mode
    let is_radial_mode = match display_mode.as_str() {
        "image" => {
            if let Some(ref path) = startup.image_path {
                println!("🖼️  Uploading static image: {}", path);
                let path_buf = std::path::PathBuf::from(path);
                if let Err(e) = cmd_upload_image(&path_buf) {
                    eprintln!("⚠️  Upload failed: {}. Falling back to radial.", e);
                    true
                } else {
                    println!("✅ Image uploaded. Cooling loop active.");
                    false
                }
            } else {
                println!("⚠️  No image_path in config. Using radial mode.");
                true
            }
        }
        "gif" => {
            if let Some(ref path) = startup.gif_path {
                println!("🎞️  Uploading GIF: {}", path);
                let path_buf = std::path::PathBuf::from(path);
                if let Err(e) = cmd_upload_image(&path_buf) {
                    eprintln!("⚠️  Upload failed: {}. Falling back to radial.", e);
                    true
                } else {
                    println!("✅ GIF uploaded. Cooling loop active.");
                    false
                }
            } else {
                println!("⚠️  No gif_path in config. Using radial mode.");
                true
            }
        }
        _ => true, // radial mode
    };

    // Initialize bucket manager only for radial mode
    let mut bucket_manager = if is_radial_mode {
        println!("🗑️  Clearing LCD memory...");
        kraken.delete_all_buckets().ok();
        std::thread::sleep(Duration::from_millis(100));
        Some(BucketManager::new())
    } else {
        None
    };

    let temp_path = std::env::temp_dir().join("kraken_start_monitor.png");
    let mut cycle_count: u64 = 0;

    while running.load(Ordering::SeqCst) {
        cycle_count += 1;

        // Get status and temperatures
        let status = match kraken.get_status() {
            Ok(s) => s,
            Err(e) => {
                eprintln!("[{}] ⚠️  Failed to get status: {}", cycle_count, e);
                std::thread::sleep(Duration::from_secs(interval));
                continue;
            }
        };

        sensors.refresh();
        let liquid_temp = status.liquid_temp_c as u8;
        let cpu_temp = sensors.find_cpu_temp().unwrap_or(0.0) as u8;

        // Select temperature based on source
        let current_temp = match temp_source {
            TempSource::Liquid => liquid_temp,
            TempSource::Cpu => cpu_temp,
        };

        // === Cooling: Calculate and apply duties ===
        let pump_duty = if pump_curve.is_empty() {
            70
        } else {
            interpolate_duty(&pump_curve, current_temp)
        };
        let _ = kraken.set_pump_speed(pump_duty.max(20));

        let fan_duty = if fan_curve.is_empty() {
            50
        } else {
            interpolate_duty(&fan_curve, current_temp)
        };
        let _ = kraken.set_fan_speed(fan_duty);

        // === LCD: Generate and upload radial gauge (only in radial mode) ===
        if let Some(ref mut bm) = bucket_manager {
            let bucket_idx = bm.acquire(&kraken);

            let (display_temp, display_label) = match temp_source {
                TempSource::Liquid => (status.liquid_temp_c, "LIQUID"),
                TempSource::Cpu => (cpu_temp as f32, "CPU"),
            };

            if let Some(img) = stats_image::generate_radial_stats_image(
                display_temp,
                display_label,
                status.pump_rpm,
                gauge_config.as_ref(),
            ) && img.save(&temp_path).is_ok()
                && let Ok(image_data) =
                    nzxt_rust_devices::device::bulk::load_image(&temp_path, orientation)
            {
                let _ = kraken.upload_image_bulk(bucket_idx, &image_data, 0x02);
            }

            println!(
                "[{:4}] {} {:.0}°C | Pump: {:3}% ({} RPM) | Fan: {:3}% | LCD: bucket {}",
                cycle_count,
                match temp_source {
                    TempSource::Liquid => "💧",
                    TempSource::Cpu => "🔥",
                },
                display_temp,
                pump_duty,
                status.pump_rpm,
                fan_duty,
                bucket_idx
            );
        } else {
            // Static mode (image/gif): only cooling updates
            let display_temp = match temp_source {
                TempSource::Liquid => status.liquid_temp_c,
                TempSource::Cpu => cpu_temp as f32,
            };
            println!(
                "[{:4}] {} {:.0}°C | Pump: {:3}% ({} RPM) | Fan: {:3}%",
                cycle_count,
                match temp_source {
                    TempSource::Liquid => "💧",
                    TempSource::Cpu => "🔥",
                },
                display_temp,
                pump_duty,
                status.pump_rpm,
                fan_duty
            );
        }

        std::thread::sleep(Duration::from_secs(interval));
    }

    println!("\n✅ Unified monitor stopped after {} cycles.", cycle_count);
    Ok(())
}
