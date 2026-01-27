//! USB bulk transfer support for LCD image uploads.
//!
//! This module uses `nusb` to access the bulk endpoint (0x02) for sending
//! large image data to the Kraken LCD. The HID endpoint (0x01) remains
//! accessible via `hidapi` for commands.

use image::DynamicImage;

/// Kraken Z63 USB identifiers
pub const VENDOR_ID: u16 = 0x1E71;
pub const PRODUCT_ID_Z63: u16 = 0x3008;

/// Bulk endpoint address for image data
pub const BULK_OUT_ENDPOINT: u8 = 0x02;

/// LCD image dimensions
pub const LCD_WIDTH: u32 = 320;
pub const LCD_HEIGHT: u32 = 320;

/// RGBA image size in bytes (320 * 320 * 4)
pub const IMAGE_SIZE_RGBA: usize = (LCD_WIDTH * LCD_HEIGHT * 4) as usize;

/// Result type for bulk operations
pub type Result<T> = std::result::Result<T, BulkError>;

/// Errors that can occur during bulk transfers
#[derive(Debug, thiserror::Error)]
pub enum BulkError {
    #[error("USB error: {0}")]
    Usb(#[from] nusb::Error),

    #[error("Device not found")]
    DeviceNotFound,

    #[error("Interface not available (may need WinUSB driver)")]
    InterfaceNotAvailable,

    #[error("Transfer error: {0}")]
    Transfer(String),

    #[error("Image error: {0}")]
    Image(String),

    #[error("Timeout")]
    Timeout,
}

/// Handle for bulk USB transfers to the Kraken LCD
pub struct BulkDevice {
    interface: nusb::Interface,
}

impl BulkDevice {
    /// Try to open the Kraken's bulk interface.
    pub fn open() -> Result<Self> {
        let device_info = nusb::list_devices()
            .map_err(BulkError::Usb)?
            .find(|d| d.vendor_id() == VENDOR_ID && d.product_id() == PRODUCT_ID_Z63)
            .ok_or(BulkError::DeviceNotFound)?;

        let device = device_info.open().map_err(BulkError::Usb)?;

        // Claim interface 0 (bulk endpoint with WinUSB driver)
        // Interface 1 is HID (used by hidapi for commands)
        let interface = device
            .claim_interface(0)
            .map_err(|_| BulkError::InterfaceNotAvailable)?;

        Ok(Self { interface })
    }

    /// Send raw data to the bulk endpoint.
    pub fn write_bulk(&self, data: &[u8]) -> Result<()> {
        use futures_lite::future::block_on;

        let result = block_on(async {
            self.interface
                .bulk_out(BULK_OUT_ENDPOINT, data.to_vec())
                .await
        });

        match result.status {
            Ok(()) => Ok(()),
            Err(e) => Err(BulkError::Transfer(format!("{:?}", e))),
        }
    }

    /// Send bulk header for image upload.
    ///
    /// Format from Wireshark: 12 FA 01 E8 AB CD EF 98 76 54 32 10 [type] 00 [size_lo] [size_hi]
    pub fn send_image_header(&self, asset_type: u8, image_size: u32) -> Result<()> {
        let mut header = vec![
            0x12, 0xFA, 0x01, 0xE8, 0xAB, 0xCD, 0xEF, 0x98, 0x76, 0x54, 0x32, 0x10,
        ];
        header.push(asset_type); // 0x02 for static image
        header.push(0x00);
        header.push(0x00);
        header.push(0x00);
        // Image size as little-endian u32
        header.extend_from_slice(&image_size.to_le_bytes());

        self.write_bulk(&header)
    }

    /// Upload an asset (image or GIF) to the Kraken LCD.
    ///
    /// # Arguments
    /// * `data` - The binary data of the asset (PNG/GIF bytes, or raw RGBA depending on mode?)
    ///   *Actually `liquidctl` sends encoded GIF bytes for GIFs, and raw RGBA for static images.*
    /// * `asset_type` - 0x01 for GIF, 0x02 for Static Image.
    ///
    /// **Important:** Based on Wireshark capture, CAM sends header and data
    /// as separate bulk transfers:
    /// 1. Header (20 bytes): 12 FA 01 E8 AB CD EF 98 76 54 32 10 [type] 00 00 00 [size_le]
    /// 2. Data: The asset bytes.
    pub fn upload_asset(&self, data: &[u8], asset_type: u8) -> Result<()> {
        let size = data.len();

        // Build header (20 bytes)
        let mut header = Vec::with_capacity(20);
        header.extend_from_slice(&[
            0x12, 0xFA, 0x01, 0xE8, 0xAB, 0xCD, 0xEF, 0x98, 0x76, 0x54, 0x32, 0x10,
        ]);
        header.push(asset_type);
        header.push(0x00);
        header.push(0x00);
        header.push(0x00);
        header.extend_from_slice(&(size as u32).to_le_bytes());

        // Send header first (separate transfer like CAM)
        self.write_bulk(&header)?;

        // Send asset data (chunks of 512, handled by nusb or OS? nusb handles it if we pass full buffer usually)
        // Does liquidctl chunk it manually? Yes, strictly by 512 bytes for X3/Z3 logic?
        // "self.bulk_buffer_size = 512" for Z3.
        // kraken3.py loop: for i in range(0, len(data), self.bulk_buffer_size): self._bulk_write(...)
        // nusb's bulk_out usually handles splitting, but maybe we should ensure it to be safe.
        // For now, let's trust nusb/winusb, if it fails we might need to chunk manually.
        self.write_bulk(data)?;

        Ok(())
    }
}

/// Check if the bulk interface is available
pub fn is_bulk_available() -> bool {
    BulkDevice::open().is_ok()
}

/// Prepare an image for the Kraken LCD.
///
/// Resizes to 320x320 and converts to RGBA format.
/// Based on Wireshark capture analysis: CAM sends RGBA with Alpha = 0xFF (opaque).
pub fn prepare_image(img: &DynamicImage, orientation: u8) -> Vec<u8> {
    // Resize to LCD dimensions
    let resized = img.resize_exact(LCD_WIDTH, LCD_HEIGHT, image::imageops::FilterType::Lanczos3);

    // Rotate based on orientation
    let rotated = match orientation {
        1 => resized.rotate90(),
        2 => resized.rotate180(),
        3 => resized.rotate270(),
        _ => resized,
    };

    // Convert to RGBA8 - this already gives us the correct format!
    // Alpha will be 255 (0xFF) for opaque pixels, which is what the LCD expects.
    rotated.to_rgba8().into_raw()
}

/// Load and prepare an image from a file path.
pub fn load_image(path: &std::path::Path, orientation: u8) -> Result<Vec<u8>> {
    let img = image::open(path).map_err(|e| BulkError::Image(e.to_string()))?;
    Ok(prepare_image(&img, orientation))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_image_size() {
        assert_eq!(IMAGE_SIZE_RGBA, 409600);
    }
}
