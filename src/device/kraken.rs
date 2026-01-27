//! NZXT Kraken Z63 device implementation.
//!
//! High-level interface for communicating with Kraken Z53/Z63/Z73 coolers.

use hidapi::{HidApi, HidDevice};

use crate::error::{KrakenError, Result};
use crate::protocol::{
    CMD_INIT_COMPLETE, CMD_INIT_INTERVAL, Channel, DeviceStatus, FirmwareVersion,
    HID_REPORT_LENGTH, KRAKEN_Z3_PID, NZXT_VID, RESP_BUCKET_SETUP, RESP_FIRMWARE, RESP_LED_INFO,
    RESP_SPEED_ACK, RESP_STATUS, RESP_STATUS_ALT, RESP_SUB_OK, build_fixed_speed_cmd,
    build_speed_profile_cmd, interpolate_profile,
};

// =============================================================================
// Constants
// =============================================================================

/// Default HID read timeout in milliseconds.
const READ_TIMEOUT_MS: i32 = 2000;

// =============================================================================
// KrakenZ63
// =============================================================================

/// NZXT Kraken Z63 device handle.
///
/// Provides methods for reading status, controlling fan/pump speeds,
/// and initializing the device.
///
/// # Example
///
/// ```no_run
/// use nzxt_rust_devices::device::KrakenZ63;
///
/// let mut kraken = KrakenZ63::open()?;
/// let fw = kraken.initialize()?;
/// println!("Firmware: {}", fw);
///
/// let status = kraken.get_status()?;
/// println!("{}", status);
///
/// kraken.set_pump_speed(80)?;
/// kraken.set_fan_speed(50)?;
/// # Ok::<(), nzxt_rust_devices::error::KrakenError>(())
/// ```
pub struct KrakenZ63 {
    device: HidDevice,
    firmware: Option<FirmwareVersion>,
}

impl KrakenZ63 {
    /// Open the first available Kraken Z63 device.
    ///
    /// # Errors
    /// Returns `DeviceNotFound` if no Kraken Z63 is connected.
    pub fn open() -> Result<Self> {
        let api = HidApi::new().map_err(KrakenError::HidError)?;

        for info in api.device_list() {
            if info.vendor_id() == NZXT_VID && info.product_id() == KRAKEN_Z3_PID {
                let device = info.open_device(&api).map_err(KrakenError::HidError)?;
                return Ok(Self {
                    device,
                    firmware: None,
                });
            }
        }

        Err(KrakenError::DeviceNotFound)
    }

    /// Open a Kraken Z63 by path.
    ///
    /// Useful when multiple devices are connected.
    pub fn open_path(path: &std::ffi::CStr) -> Result<Self> {
        let api = HidApi::new().map_err(KrakenError::HidError)?;
        let device = api.open_path(path).map_err(KrakenError::HidError)?;

        Ok(Self {
            device,
            firmware: None,
        })
    }

    /// List all connected Kraken Z63 devices.
    ///
    /// Returns a vector of (path, serial_number) tuples.
    pub fn list_devices() -> Result<Vec<(String, Option<String>)>> {
        let api = HidApi::new().map_err(KrakenError::HidError)?;

        let devices: Vec<_> = api
            .device_list()
            .filter(|info| info.vendor_id() == NZXT_VID && info.product_id() == KRAKEN_Z3_PID)
            .map(|info| {
                (
                    info.path().to_string_lossy().into_owned(),
                    info.serial_number().map(String::from),
                )
            })
            .collect();

        Ok(devices)
    }

    /// Initialize the device.
    ///
    /// Must be called after opening the device and before any control operations.
    /// This sets up the status update interval and retrieves firmware info.
    ///
    /// # Returns
    /// The firmware version of the device.
    pub fn initialize(&mut self) -> Result<FirmwareVersion> {
        // Clear any enqueued reports (like liquidctl does)
        let mut buf = [0u8; HID_REPORT_LENGTH];
        loop {
            let res = self.device.read_timeout(&mut buf, 1);
            if res.is_err() || res.unwrap() == 0 {
                break;
            }
        }

        // Request static infos (like liquidctl does)
        use crate::protocol::{CMD_FIRMWARE_INFO, CMD_LED_INFO};
        self.write(&CMD_FIRMWARE_INFO)?;

        // Read firmware version response
        let mut fw = FirmwareVersion {
            major: 0,
            minor: 0,
            patch: 0,
        };
        let mut buf = [0u8; HID_REPORT_LENGTH];

        // Try reading for up to 200ms (10 * 20ms)
        for _ in 0..10 {
            if let Ok(n) = self.device.read_timeout(&mut buf, 20)
                && n > 0
                && buf[0] == RESP_FIRMWARE[0]
                && buf[1] == RESP_FIRMWARE[1]
            {
                fw.major = buf[17];
                fw.minor = buf[18];
                fw.patch = buf[19];
                break;
            }
        }

        std::thread::sleep(std::time::Duration::from_millis(50));
        self.write(&CMD_LED_INFO)?;
        std::thread::sleep(std::time::Duration::from_millis(50));

        // Initialize device with update interval (500ms)
        self.write(&CMD_INIT_INTERVAL)?;
        std::thread::sleep(std::time::Duration::from_millis(100));

        // Complete initialization
        self.write(&CMD_INIT_COMPLETE)?;
        std::thread::sleep(std::time::Duration::from_millis(100));

        // Firmware version is now populated

        Ok(fw)
    }

    /// Get the current device status.
    ///
    /// Reads temperature, pump RPM, and pump duty from the device.
    /// Filters for status messages (header 0x75 0x01) and retries if needed.
    pub fn get_status(&self) -> Result<DeviceStatus> {
        // Clear enqueued reports
        let mut buf = [0u8; HID_REPORT_LENGTH];
        loop {
            let res = self.device.read_timeout(&mut buf, 1);
            if res.is_err() || res.unwrap() == 0 {
                break;
            }
        }

        // **CRITICAL:** Request status from device (discovered from zkraken-lib)
        use crate::protocol::CMD_REQUEST_STATUS;
        self.write(&CMD_REQUEST_STATUS)?;
        std::thread::sleep(std::time::Duration::from_millis(50));

        // Read messages until we find a status message
        // Skip info responses (0x11 firmware, 0x21 LED, 0x33 other)
        for _ in 0..10 {
            let read = self
                .device
                .read_timeout(&mut buf, READ_TIMEOUT_MS)
                .map_err(KrakenError::HidError)?;

            if read == 0 {
                continue;
            }

            // Skip info/response messages
            if buf[0] == RESP_FIRMWARE[0] || buf[0] == RESP_LED_INFO || buf[0] == RESP_BUCKET_SETUP
            {
                continue; // Skip and read next message
            }

            // Accept status messages: RESP_STATUS (preferred) or RESP_STATUS_ALT/RESP_SPEED_ACK (fallback)
            if (buf[0] == RESP_STATUS[0]
                || buf[0] == RESP_STATUS_ALT
                || buf[0] == RESP_SPEED_ACK[0])
                && buf[1] == RESP_SUB_OK
            {
                return DeviceStatus::parse(&buf);
            }
        }

        Err(KrakenError::Timeout)
    }

    /// Set the LCD brightness.
    ///
    /// # Arguments
    /// * `brightness` - Brightness level (0-100)
    pub fn set_brightness(&self, brightness: u8) -> Result<()> {
        let (_, orientation) = self.get_lcd_info()?;
        self.set_lcd_config(brightness, orientation)
    }

    /// Set the LCD orientation.
    ///
    /// # Arguments
    /// * `orientation` - Orientation (0=0°, 1=90°, 2=180°, 3=270°)
    pub fn set_orientation(&self, orientation: u8) -> Result<()> {
        let (brightness, _) = self.get_lcd_info()?;
        self.set_lcd_config(brightness, orientation)
    }

    /// Set LCD configuration (brightness and orientation).
    pub fn set_lcd_config(&self, brightness: u8, orientation: u8) -> Result<()> {
        if brightness > 100 {
            return Err(KrakenError::InvalidInput(
                "Brightness must be between 0 and 100".into(),
            ));
        }
        if orientation > 3 {
            return Err(KrakenError::InvalidInput(
                "Orientation must be between 0 and 3 (0=0, 1=90, 2=180, 3=270)".into(),
            ));
        }

        let mut buf = [0u8; HID_REPORT_LENGTH];
        buf[0..3].copy_from_slice(&crate::protocol::CMD_SET_LCD_CONFIG_HEADER);
        buf[3] = brightness;
        buf[4] = 0x00;
        buf[5] = 0x00;
        buf[6] = 0x01; // liquidctl: [0x30, 0x02, 0x01, brightness, 0x0, 0x0, 0x1, orientation]
        buf[7] = orientation;

        self.write(&buf)
    }

    /// Get the current LCD info (brightness, orientation).
    pub fn get_lcd_info(&self) -> Result<(u8, u8)> {
        let (brightness, orientation, _) = self.get_lcd_info_raw()?;
        Ok((brightness, orientation))
    }

    /// Get the current LCD info including raw bytes.
    pub fn get_lcd_info_raw(&self) -> Result<(u8, u8, [u8; HID_REPORT_LENGTH])> {
        self.write(&crate::protocol::CMD_LCD_INFO)?;

        // Wait for response 0x31 0x01
        let mut buf = [0u8; HID_REPORT_LENGTH];
        for _ in 0..10 {
            let read = self.device.read_timeout(&mut buf, 100)?;
            if read == 0 {
                continue;
            }
            if buf[0] == 0x31 && buf[1] == 0x01 {
                let brightness = buf[0x18];
                let orientation = buf[0x1A];
                return Ok((brightness, orientation, buf));
            }
        }

        Err(KrakenError::Timeout)
    }

    /// Set the LCD visual mode.
    ///
    /// # Arguments
    /// * `mode` - Visual mode ID (e.g., 2 for Liquid Temp)
    /// * `index` - Memory bucket index or Layout/Sensor selection
    pub fn set_visual_mode(&self, mode: u8, index: u8) -> Result<()> {
        let mut cmd = [0u8; 4];
        cmd[0..2].copy_from_slice(&crate::protocol::CMD_SET_VISUAL_MODE_HEADER);
        cmd[2] = mode;
        cmd[3] = index;
        self.write(&cmd)
    }

    /// Set host telemetry info (CPU/GPU temperature).
    ///
    /// This is required for LCD modes 1 (CPU Temp) and 3 (GPU Temp).
    /// These values should be pushed periodically (e.g. every 1-2 seconds).
    ///
    /// # Arguments
    /// * `cpu_temp` - CPU temperature in Celsius
    /// * `gpu_temp` - GPU temperature in Celsius
    pub fn set_host_info(&self, cpu_temp: u8, gpu_temp: u8) -> Result<()> {
        let mut buf = [0u8; HID_REPORT_LENGTH];
        buf[0..2].copy_from_slice(&crate::protocol::CMD_SET_HOST_INFO);
        buf[2] = cpu_temp;
        buf[3] = gpu_temp;

        self.write(&buf)?;
        Ok(())
    }

    /// Delete a specific memory bucket.
    ///
    /// # Arguments
    /// * `index` - Bucket index (0-15)
    pub fn delete_bucket(&self, index: u8) -> Result<()> {
        use crate::protocol::{CMD_BUCKET_OP, OP_BUCKET_DELETE};
        let cmd = [CMD_BUCKET_OP, OP_BUCKET_DELETE, index, 0x00];
        self.write(&cmd)
    }

    /// Delete all memory buckets (0-15).
    ///
    /// This is useful to clear the device memory before uploading new images
    /// or to reset the visual state.
    pub fn delete_all_buckets(&self) -> Result<()> {
        for i in 0..16 {
            self.delete_bucket(i)?;
            // Small delay to ensure device processes the deletion
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
        Ok(())
    }

    /// Query the status of a specific memory bucket.
    ///
    /// # Arguments
    /// * `index` - Bucket index (0-15)
    ///
    /// # Returns
    /// Tuple of (exists: bool, asset_type: u8, start_page: u16, size_pages: u16)
    pub fn query_bucket(&self, index: u8) -> Result<(bool, u8, u16, u16)> {
        use crate::protocol::CMD_BUCKET_QUERY;
        let cmd = [CMD_BUCKET_QUERY[0], CMD_BUCKET_QUERY[1], index];
        self.write(&cmd)?;

        // Wait for response 0x31 0x04
        let mut buf = [0u8; HID_REPORT_LENGTH];
        for _ in 0..10 {
            let read = self.device.read_timeout(&mut buf, 100)?;
            if read == 0 {
                continue;
            }
            if buf[0] == 0x31 && buf[1] == 0x04 {
                // Parse bucket info from response (offsets from liquidctl)
                // 17-18: Start Memory Address (LE)
                // 19-20: Memory Size (LE)
                let start_page = u16::from_le_bytes([buf[17], buf[18]]);
                let size_pages = u16::from_le_bytes([buf[19], buf[20]]);

                // If size > 0, the bucket exists/is used
                let exists = size_pages > 0;
                let asset_type = 0; // Not critical for us based on liquidctl usage

                return Ok((exists, asset_type, start_page, size_pages));
            }
        }

        // Bucket doesn't exist or no response
        Ok((false, 0, 0, 0))
    }

    /// Wait for a specific response header from the device.
    ///
    /// # Arguments
    /// * `expected_header` - First byte of expected response
    /// * `expected_sub` - Second byte of expected response (optional, use 0xFF to ignore)
    fn wait_for_response(
        &self,
        expected_header: u8,
        expected_sub: u8,
    ) -> Result<[u8; HID_REPORT_LENGTH]> {
        let mut buf = [0u8; HID_REPORT_LENGTH];
        for _ in 0..10 {
            let read = self.device.read_timeout(&mut buf, 200)?;
            if read == 0 {
                continue;
            }
            if buf[0] == expected_header && (expected_sub == 0xFF || buf[1] == expected_sub) {
                return Ok(buf);
            }
        }
        Err(KrakenError::Timeout)
    }

    /// Upload an asset (image or GIF) to the device using the bulk endpoint (nusb).
    ///
    /// # Arguments
    /// * `index` - Bucket index (0-15)
    /// * `data` - The asset data (RGBA pixels for static, GIF file bytes for GIF)
    /// * `asset_type` - 0x02 for Static, 0x01 for GIF
    ///
    /// Sequence:
    /// 1. Handshake:    36 03
    /// 2. Query buckets to find memory offset
    /// 3. Delete bucket: 32 02 [idx]
    /// 4. Setup bucket: 32 01 [idx] [id] [mem_lo] [mem_hi] [size_lo] [size_hi] 01
    /// 5. Start bulk:   36 01 [idx]
    /// 6. Bulk header:  12 FA 01 E8 AB CD EF 98 76 54 32 10 [type] 00 00 00 [size_le]
    /// 7. Bulk data:    [data]
    /// 8. End bulk:     36 02
    /// 9. Switch mode:  38 01 04 [idx]
    pub fn upload_image_bulk(&self, index: u8, data: &[u8], asset_type: u8) -> Result<()> {
        use super::bulk::BulkDevice;

        let bulk = BulkDevice::open()
            .map_err(|e| KrakenError::InvalidInput(format!("Failed to open bulk device: {}", e)))?;

        let bucket_index = index;
        let bucket_id = index + 1; // ID = Index + 1
        let size_bytes = data.len();
        // Calculate pages (1024 bytes). If < 1024, at least 1?
        // liquidctl uses bytes count in header, but setup command uses 1KB pages.
        // math.ceil((len(header) + len(data)) / 1024)
        // header is 20 bytes.
        let page_count = (size_bytes + 20).div_ceil(1024) as u16;

        println!("  Step 1: Handshake (36 03)...");
        self.write(&[0x36, 0x03])?;
        std::thread::sleep(std::time::Duration::from_millis(50));

        // Step 2: Query all buckets to find memory layout
        println!("  Step 2: Querying buckets...");
        let buckets = self.query_all_buckets()?;

        // Step 3: Find next unoccupied bucket or use requested index
        // The instruction implies using the provided `index` directly, so `find_or_prepare_bucket` is no longer needed here.
        // The `bucket_index` is already set to `index`.

        // Step 4: Calculate memory offset
        // let size_pages = ((image_data.len() + 1023) / 1024) as u16; // Round up to 1KB pages
        let memory_start = self.calculate_memory_offset(&buckets, bucket_index, page_count)?;

        println!("  Step 3: Delete bucket {}...", bucket_index);
        let _ = self.delete_bucket(bucket_index);
        std::thread::sleep(std::time::Duration::from_millis(20));

        // Step 4: Setup bucket
        println!(
            "  Step 4: Setup bucket {} at memory offset {}...",
            bucket_index, memory_start
        );
        // let bucket_id = bucket_index + 1;

        // [0x32, 0x1, startBucketIndex, endBucketIndex,
        //  startingMemoryAddress[0], startingMemoryAddress[1],
        //  memorySize[0], memorySize[1], 0x1]
        let mut setup_cmd = [0u8; 64];
        setup_cmd[0] = 0x32; // CMD_BUCKET_OP
        setup_cmd[1] = 0x01; // OP_BUCKET_SET
        setup_cmd[2] = bucket_index;
        setup_cmd[3] = bucket_id;
        // Memory start address (little-endian)
        setup_cmd[4] = (memory_start & 0xFF) as u8;
        setup_cmd[5] = ((memory_start >> 8) & 0xFF) as u8;
        // Size in pages (little-endian)
        setup_cmd[6] = (page_count & 0xFF) as u8;
        setup_cmd[7] = ((page_count >> 8) & 0xFF) as u8;
        // Frames count? always sends 1 for "setup_bucket",
        // regardless of whether it's a GIF or Static. The GIF file itself contains frames.
        setup_cmd[8] = 0x01;
        setup_cmd[9] = 0x00;

        self.write(&setup_cmd)?;
        // Wait for setup confirmation (0x33 0x01)
        let _ = self.wait_for_response(0x33, 0x01);
        std::thread::sleep(std::time::Duration::from_millis(20));

        // Step 5: Start bulk transfer (36 01 [index])
        println!("  Step 5: Start bulk transfer...");
        self.write(&[0x36, 0x01, bucket_index])?;
        // Wait for confirmation (0x37 0x01)
        let _ = self.wait_for_response(0x37, 0x01);

        // Step 6: Send bulk data
        println!(
            "  Step 6: Send bulk data ({} bytes, Type 0x{:02X})...",
            size_bytes, asset_type
        );
        // asset_type: 0x01 = GIF, 0x02 = Static
        bulk.upload_asset(data, asset_type)
            .map_err(|e| KrakenError::InvalidInput(format!("Bulk transfer failed: {}", e)))?;

        println!("  Step 7: End bulk transfer...");
        self.write(&[0x36, 0x02])?; // End bulk

        // Wait for confirmation (0x37 0x02)
        let _ = self.wait_for_response(0x37, 0x02);

        // Step 8: Switch to newly written bucket
        // Always Mode 4 (LCD_MODE_ONE_FRAME) for liquidctl?
        // Wait, uses Mode 2 (Liquid) sometimes?
        // But for static/gif, it uses: _switch_bucket(bucketIndex) -> defaults to mode 0x4.
        println!("  Step 8: Switch to bucket {} (Mode 4)...", bucket_index);
        self.set_visual_mode(4, bucket_index)?;

        println!("  Upload complete!");
        Ok(())
    }

    /// Query all 16 buckets and return their info.
    ///
    /// Returns a vector of tuples: (bucket_index, exists, start_page, size_pages)
    pub fn query_all_buckets(&self) -> Result<Vec<(u8, bool, u16, u16)>> {
        let mut buckets = Vec::with_capacity(16);
        for i in 0..16 {
            let (exists, _, start_page, size_pages) = self.query_bucket(i)?;
            buckets.push((i, exists, start_page, size_pages));
        }
        Ok(buckets)
    }

    /// Calculate memory offset for new bucket (following liquidctl logic).
    fn calculate_memory_offset(
        &self,
        buckets: &[(u8, bool, u16, u16)],
        target_idx: u8,
        needed_size: u16,
    ) -> Result<u16> {
        // Find target bucket's current info
        let target = buckets.iter().find(|(i, _, _, _)| *i == target_idx);

        if let Some((_, exists, current_start, current_size)) = target {
            // If bucket exists and has enough space, reuse its offset
            if *exists && *current_size >= needed_size {
                return Ok(*current_start);
            }
        }

        // Find the end of all occupied memory (EXCLUDING the target bucket)
        let max_end: u16 = buckets
            .iter()
            .filter(|(i, exists, _, _)| *exists && *i != target_idx)
            .map(|(_, _, start, size)| start + size)
            .max()
            .unwrap_or(0);

        // Find the minimum occupied start (EXCLUDING target)
        let min_start: u16 = buckets
            .iter()
            .filter(|(i, exists, _, _)| *exists && *i != target_idx)
            .map(|(_, _, start, _)| *start)
            .min()
            .unwrap_or(0xFFFF);

        // Total available memory: 24320 KB
        const LCD_TOTAL_MEMORY: u16 = 24320;

        // 1. Check if we can fit at the end of occupied memory
        if max_end + needed_size <= LCD_TOTAL_MEMORY {
            return Ok(max_end);
        }

        // 2. Check if we can fit at 0 (if valid data starts later)
        if min_start != 0xFFFF && needed_size <= min_start {
            return Ok(0);
        }

        // 3. Fallback: If we are the only one or can't fit elsewhere, try 0 and hope ignoring others is fine (liquidctl logic is more complex here)
        // If max_end == 0 (no other buckets), returns 0.
        Ok(0)
    }

    /// Set a fixed pump speed.
    ///
    /// # Arguments
    /// * `duty` - Duty cycle percentage (20-100)
    ///
    /// # Errors
    /// Returns `InvalidDuty` if duty is outside valid range.
    pub fn set_pump_speed(&self, duty: u8) -> Result<()> {
        let cmd = build_fixed_speed_cmd(Channel::Pump, duty)?;
        self.write(&cmd)
    }

    /// Set a fixed fan speed.
    ///
    /// # Arguments
    /// * `duty` - Duty cycle percentage (0-100)
    ///
    /// # Errors
    /// Returns `InvalidDuty` if duty is outside valid range.
    pub fn set_fan_speed(&self, duty: u8) -> Result<()> {
        let cmd = build_fixed_speed_cmd(Channel::Fan, duty)?;
        self.write(&cmd)
    }

    /// Set a speed profile for a channel.
    ///
    /// The profile is specified as (temperature, duty) pairs which are interpolated
    /// into a full 40-point curve (20°C to 59°C).
    ///
    /// # Arguments
    /// * `channel` - The channel to configure (Pump or Fan)
    /// * `profile` - Temperature/duty pairs, e.g., `[(20, 30), (40, 60), (55, 100)]`
    ///
    /// # Example
    /// ```no_run
    /// use nzxt_rust_devices::protocol::Channel;
    /// # use nzxt_rust_devices::device::KrakenZ63;
    /// # let kraken = KrakenZ63::open()?;
    ///
    /// // Silent profile: low speed until 45°C, then ramp up
    /// kraken.set_speed_profile(Channel::Fan, &[
    ///     (20, 25),
    ///     (45, 25),
    ///     (50, 50),
    ///     (55, 75),
    ///     (59, 100),
    /// ])?;
    /// # Ok::<(), nzxt_rust_devices::error::KrakenError>(())
    /// ```
    pub fn set_speed_profile(&self, channel: Channel, profile: &[(u8, u8)]) -> Result<()> {
        let duties = interpolate_profile(profile)?;

        // Validate all duties for this channel
        for &duty in &duties {
            channel.validate_duty(duty)?;
        }

        let cmd = build_speed_profile_cmd(channel, &duties);
        self.write(&cmd)
    }

    /// Get the firmware version.
    ///
    /// Returns `None` if `initialize()` has not been called.
    pub fn firmware_version(&self) -> Option<FirmwareVersion> {
        self.firmware
    }

    // =========================================================================
    // Private Helpers
    // =========================================================================

    fn write(&self, data: &[u8]) -> Result<()> {
        let mut buf = [0u8; HID_REPORT_LENGTH];
        let len = data.len().min(HID_REPORT_LENGTH);
        buf[..len].copy_from_slice(&data[..len]);

        self.device.write(&buf).map_err(KrakenError::HidError)?;
        Ok(())
    }
}

impl std::fmt::Debug for KrakenZ63 {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("KrakenZ63")
            .field("firmware", &self.firmware)
            .finish_non_exhaustive()
    }
}
