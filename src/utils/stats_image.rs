//! Stats image generator for LCD display.
//!
//! Generates 320x320 RGBA images with temperature and RPM data.

use image::{Rgba, RgbaImage};
use imageproc::drawing::draw_text_mut;
use rusttype::{Font, Scale};
use std::path::Path;

use super::radial_gauge::{
    RadialGaugeConfig, draw_dynamic_gauge, interpolate_color, temp_to_position,
};

/// LCD dimensions
pub const LCD_SIZE: u32 = 320;

/// Colors for the stats display
pub mod colors {
    use image::Rgba;

    pub const BACKGROUND: Rgba<u8> = Rgba([0, 0, 0, 255]); // Pure black
    pub const TEXT_PRIMARY: Rgba<u8> = Rgba([255, 255, 255, 255]); // White
    pub const TEXT_SECONDARY: Rgba<u8> = Rgba([255, 255, 255, 255]); // White
    pub const TEMP_COLD: Rgba<u8> = Rgba([255, 255, 255, 255]); // White
    pub const TEMP_WARM: Rgba<u8> = Rgba([255, 255, 255, 255]); // White
    pub const TEMP_HOT: Rgba<u8> = Rgba([255, 255, 255, 255]); // White
}

/// Get temperature color based on value
fn temp_color(temp: f32) -> Rgba<u8> {
    if temp < 35.0 {
        colors::TEMP_COLD
    } else if temp < 45.0 {
        colors::TEMP_WARM
    } else {
        colors::TEMP_HOT
    }
}

/// Try to load a font from common system paths
fn load_font() -> Option<Font<'static>> {
    let font_paths = [
        "C:\\Windows\\Fonts\\arialbd.ttf",  // Arial Bold
        "C:\\Windows\\Fonts\\segoeuib.ttf", // Segoe UI Bold
        "C:\\Windows\\Fonts\\arial.ttf",
        "C:\\Windows\\Fonts\\segoeui.ttf",
        "C:\\Windows\\Fonts\\consola.ttf",
        "/usr/share/fonts/truetype/dejavu/DejaVuSans-Bold.ttf",
        "/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf",
    ];

    for path in font_paths {
        if Path::new(path).exists()
            && let Ok(data) = std::fs::read(path)
            && let Some(font) = Font::try_from_vec(data)
        {
            return Some(font);
        }
    }
    None
}

/// Generate a stats image with temperature and RPM data.
pub fn generate_stats_image(
    liquid_temp: f32,
    pump_rpm: u16,
    fan_rpm: u16,
    pump_duty: u8,
    fan_duty: u8,
) -> Option<RgbaImage> {
    let font = load_font()?;
    let mut img = RgbaImage::from_pixel(LCD_SIZE, LCD_SIZE, colors::BACKGROUND);

    // Title
    let title_scale = Scale::uniform(28.0);
    draw_text_mut(
        &mut img,
        colors::TEXT_SECONDARY,
        20,
        20,
        title_scale,
        &font,
        "KRAKEN Z63",
    );

    // Temperature (large)
    let temp_scale = Scale::uniform(72.0);
    let temp_text = format!("{:.1}°", liquid_temp);
    let temp_color = temp_color(liquid_temp);
    draw_text_mut(&mut img, temp_color, 50, 80, temp_scale, &font, &temp_text);

    // Label
    let label_scale = Scale::uniform(24.0);
    draw_text_mut(
        &mut img,
        colors::TEXT_SECONDARY,
        50,
        160,
        label_scale,
        &font,
        "LIQUID TEMP",
    );

    // Pump info
    let info_scale = Scale::uniform(28.0);
    let pump_text = format!("PUMP: {} RPM ({}%)", pump_rpm, pump_duty);
    draw_text_mut(
        &mut img,
        colors::TEXT_PRIMARY,
        20,
        210,
        info_scale,
        &font,
        &pump_text,
    );

    // Fan info
    let fan_text = format!("FAN:  {} RPM ({}%)", fan_rpm, fan_duty);
    draw_text_mut(
        &mut img,
        colors::TEXT_PRIMARY,
        20,
        255,
        info_scale,
        &font,
        &fan_text,
    );

    Some(img)
}

/// Generate a simple temperature-only display (minimal style)
pub fn generate_temp_only_image(liquid_temp: f32) -> Option<RgbaImage> {
    let font = load_font()?;
    let mut img = RgbaImage::from_pixel(LCD_SIZE, LCD_SIZE, colors::BACKGROUND);

    // Temperature (very large, centered)
    let temp_scale = Scale::uniform(96.0);
    let temp_text = format!("{:.1}°", liquid_temp);
    let temp_color = temp_color(liquid_temp);

    draw_text_mut(&mut img, temp_color, 60, 100, temp_scale, &font, &temp_text);

    // Label below
    let label_scale = Scale::uniform(28.0);
    draw_text_mut(
        &mut img,
        colors::TEXT_SECONDARY,
        110,
        220,
        label_scale,
        &font,
        "Liquid",
    );

    Some(img)
}

/// Generate a radial gauge stats image (NZXT CAM style).
///
/// Features:
/// - Pure black background
/// - Gradient arc (green → yellow → red)
/// - Moving indicator ball that follows the temperature
/// - Large centered temperature display
/// - Dynamic label ("LIQUID" or "CPU")
/// - Pump RPM display
pub fn generate_radial_stats_image(
    temp: f32,
    label: &str,
    pump_rpm: u16,
    config: Option<&RadialGaugeConfig>,
) -> Option<RgbaImage> {
    let font = load_font()?;

    // Configure the radial gauge
    let default_config = RadialGaugeConfig::default();
    let config = config.unwrap_or(&default_config);

    // Use configured background color
    let mut img = RgbaImage::from_pixel(LCD_SIZE, LCD_SIZE, config.background_color);

    // Draw the dynamic gauge (Fill + Gap + Pill)
    draw_dynamic_gauge(&mut img, config, temp);

    // Get color based on temperature position in gradient (unused regarding text color now)
    // let position = temp_to_position(&config, temp);
    // let temp_display_color = interpolate_color(&config.gradient, position);

    // Temperature text (large, centered, WHITE)
    // We render the number and the degree symbol separately to handle sizing/positioning better
    let temp_val_text = format!("{:.0}", temp);
    let temp_scale = Scale::uniform(105.0); //Large number font size temperature
    let deg_scale = Scale::uniform(46.5); // Smaller degree symbol

    // Calculate approximate widths to center the group
    // This is rough estimation as we don't have exact font metrics easily accessible without a glyph pass
    let val_width = temp_val_text.len() as i32 * 30;
    let deg_width = 15;
    let total_width = val_width + deg_width;

    let start_x = (LCD_SIZE as i32 - total_width) / 2 - 20;
    let text_y = 100; // Moved up slightly

    // Draw Value
    draw_text_mut(
        &mut img,
        colors::TEXT_PRIMARY, // Using White/Primary color instead of gradient color
        start_x,
        text_y,
        temp_scale,
        &font,
        &temp_val_text,
    );

    // Draw Degree Symbol
    draw_text_mut(
        &mut img,
        colors::TEXT_PRIMARY,
        start_x + val_width + 50, //Align degree symbol to the right of the number
        text_y + 10,              // Align top (or adjust for baseline)
        deg_scale,
        &font,
        "°",
    );

    // Dynamic Label (LIQUID/CPU)
    let label_width = label.len() as i32 * 10;
    let label_x = (LCD_SIZE as i32 - label_width) / 2 - 10; // Move label left slightly
    let label_y = 210; // Move label down slightly

    draw_text_mut(
        &mut img,
        colors::TEXT_PRIMARY,
        label_x,
        label_y,
        Scale::uniform(24.0),
        &font,
        label,
    );

    // Pump RPM Label
    let rpm_text = format!("{} RPM", pump_rpm);
    // Estimate width: 8 chars * 8px approx?
    let rpm_width = rpm_text.len() as i32 * 9;
    let rpm_x = (LCD_SIZE as i32 - rpm_width) / 2 - 5; // Centered
    let rpm_y = label_y + 30; // Below LIQUID

    draw_text_mut(
        &mut img,
        colors::TEXT_PRIMARY,
        rpm_x,
        rpm_y,
        Scale::uniform(20.0),
        &font,
        &rpm_text,
    );

    Some(img)
}

/// Generate a radial gauge with full stats (temperature, pump RPM, fan RPM).
pub fn generate_radial_full_stats_image(
    liquid_temp: f32,
    pump_rpm: u16,
    fan_rpm: u16,
    pump_duty: u8,
    fan_duty: u8,
) -> Option<RgbaImage> {
    let font = load_font()?;
    let mut img = RgbaImage::from_pixel(LCD_SIZE, LCD_SIZE, colors::BACKGROUND);

    // Configure the radial gauge - slightly smaller to fit more info
    let config = RadialGaugeConfig {
        center_y: 140, // Move gauge up
        outer_radius: 120.0,
        inner_radius: 100.0,
        ..Default::default()
    };

    // Draw the dynamic gauge
    draw_dynamic_gauge(&mut img, &config, liquid_temp);

    // Get color based on temperature position in gradient
    let position = temp_to_position(&config, liquid_temp);
    let temp_display_color = interpolate_color(&config.gradient, position);

    // Temperature text (large, centered)
    let temp_scale = Scale::uniform(56.0);
    let temp_text = format!("{:.0}°", liquid_temp);

    let text_width = temp_text.len() as i32 * 22;
    let text_x = (LCD_SIZE as i32 - text_width) / 2;
    let text_y = 110;

    draw_text_mut(
        &mut img,
        temp_display_color,
        text_x,
        text_y,
        temp_scale,
        &font,
        &temp_text,
    );

    // "LIQUID" label
    let label_scale = Scale::uniform(18.0);
    draw_text_mut(
        &mut img,
        colors::TEXT_SECONDARY,
        130,
        170,
        label_scale,
        &font,
        "Liquid",
    );

    // Stats below the gauge
    let info_scale = Scale::uniform(20.0);

    // Pump info
    let pump_text = format!("PUMP {} RPM ({}%)", pump_rpm, pump_duty);
    draw_text_mut(
        &mut img,
        colors::TEXT_PRIMARY,
        40,
        265,
        info_scale,
        &font,
        &pump_text,
    );

    // Fan info
    let fan_text = format!("FAN  {} RPM ({}%)", fan_rpm, fan_duty);
    draw_text_mut(
        &mut img,
        colors::TEXT_PRIMARY,
        40,
        290,
        info_scale,
        &font,
        &fan_text,
    );

    Some(img)
}

/// Convert an RgbaImage to raw bytes for upload
pub fn image_to_bytes(img: &RgbaImage) -> Vec<u8> {
    img.as_raw().clone()
}
