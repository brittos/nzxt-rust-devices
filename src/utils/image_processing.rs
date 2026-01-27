use anyhow::{Context, Result};
use image::AnimationDecoder;
use image::imageops::FilterType;
use std::fs::File;
use std::path::Path;

/// Proposed LCD resolution for Kraken Z3
const LCD_WIDTH: u32 = 320;
const LCD_HEIGHT: u32 = 320;

/// Process an image file for upload to the Kraken LCD.
///
/// This function:
/// 1. Opens the image.
/// 2. Resizes it to 320x320.
/// 3. Converts it to the raw byte format expected by the device.
///
/// Returns raw bytes.
pub fn process_image(path: &Path) -> Result<Vec<u8>> {
    let img = image::open(path).context("Failed to open image file")?;

    // Resize image to 320x320
    // Uses Triangle filter for decent quality/speed balance
    let resized = img.resize_exact(LCD_WIDTH, LCD_HEIGHT, FilterType::Triangle);

    // Convert to RGBA8
    let rgba = resized.to_rgba8();

    // The device expects raw pixel data.
    // Based on zkraken-lib and liquidctl:
    // It seems to expect 32-bit RGBA OR 16-bit RGB565.
    // Liquidctl mentions: "The Kraken Z3 LCD is a 320x320 pixel display... 24-bit color" but sent as 32-bit?
    // zkraken-lib sends as RGBA8 (4 bytes per pixel).

    // Total bytes = 320 * 320 * 4 = 409,600 bytes
    Ok(rgba.into_raw())
}

/// Process a GIF file for upload to the Kraken LCD.
///
/// This function:
/// 1. Decodes the GIF frames.
/// 2. Selects up to 50 frames (decimation).
/// 3. Rotates each frame based on orientation.
/// 4. Resizes each frame to 320x320.
/// 5. Re-encodes the frames into a new GIF byte vector.
///
/// # Arguments
/// * `path` - Path to the GIF file
/// * `orientation` - LCD orientation (0=0°, 1=90°, 2=180°, 3=270°)
///
/// Returns: (GIF Bytes, Frame Count) - Frame count is returned just for info/logging.
pub fn process_gif(path: &Path, orientation: u8) -> Result<(Vec<u8>, u16)> {
    let file = File::open(path).context("Failed to open GIF file")?;
    let decoder =
        image::codecs::gif::GifDecoder::new(file).context("Failed to create GIF decoder")?;
    let frames = decoder
        .into_frames()
        .collect_frames()
        .context("Failed to collect GIF frames")?;

    if frames.is_empty() {
        anyhow::bail!("GIF has no frames");
    }

    // Decimate
    let total_frames = frames.len();
    let max_frames = 50;
    let step = if total_frames > max_frames {
        (total_frames as f32 / max_frames as f32).ceil() as usize
    } else {
        1
    };

    // Prepare output buffer
    let mut output_buffer = Vec::new();
    {
        // Use a scope to drop the encoder when done -> finishes the file
        // We probably need to set Repeat::Infinite.
        let mut encoder = image::codecs::gif::GifEncoder::new_with_speed(&mut output_buffer, 10); // speed 10 = fast
        encoder.set_repeat(image::codecs::gif::Repeat::Infinite)?;

        let mut count = 0;
        for (i, frame) in frames.into_iter().enumerate() {
            if i % step == 0 && count < max_frames {
                // Get frame as DynamicImage
                let img = frame.buffer();
                let dynamic_img = image::DynamicImage::ImageRgba8(img.clone());

                // Apply rotation based on orientation (same logic as bulk::prepare_image)
                let rotated = match orientation {
                    1 => dynamic_img.rotate90(),
                    2 => dynamic_img.rotate180(),
                    3 => dynamic_img.rotate270(),
                    _ => dynamic_img,
                };

                // Resize after rotation
                let resized = rotated.resize_exact(LCD_WIDTH, LCD_HEIGHT, FilterType::Triangle);

                // Convert back to frame
                // We need to keep the delay from original frame if possible.
                // The original frame `frame` has delay().
                let new_frame = image::Frame::from_parts(resized.to_rgba8(), 0, 0, frame.delay());

                encoder.encode_frame(new_frame)?;
                count += 1;
            }
        }
    } // encoder dropped here, data flushed to output_buffer

    // Return the GIF file bytes and the effective frame count (just for log)
    // Note: Protocol actually expects num_frames=1 in assignment if it treats it as 1 asset file.
    Ok((output_buffer, 1))
}
