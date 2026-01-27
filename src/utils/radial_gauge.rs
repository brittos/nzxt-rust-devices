//! Radial gauge rendering module for LCD display.
//!
//! Renders a radial arc gauge with gradient colors (red → orange-red → orange)
//! and a moving indicator ball based on temperature value.

use image::{Rgba, RgbaImage};
use std::f64::consts::PI;

/// Default gradient stops (similar to NZXT CAM)
pub struct GradientStop {
    pub color: Rgba<u8>,
    pub position: f32, // 0.0 to 1.0
}

/// Configuration for the radial gauge
pub struct RadialGaugeConfig {
    /// Center X coordinate
    pub center_x: i32,
    /// Center Y coordinate
    pub center_y: i32,
    /// Outer radius of the arc
    pub outer_radius: f32,
    /// Inner radius of the arc (creates a "ring" effect)
    pub inner_radius: f32,
    /// Start angle in degrees (measured from top, clockwise)
    pub start_angle_deg: f32,
    /// End angle in degrees (measured from top, clockwise)
    pub end_angle_deg: f32,
    /// Gradient color stops
    pub gradient: Vec<GradientStop>,
    /// Radius of the indicator ball
    pub indicator_radius: f32,
    /// Minimum temperature for the gauge
    pub min_temp: f32,
    /// Maximum temperature for the gauge
    pub max_temp: f32,
    /// Background color of the whole display
    pub background_color: Rgba<u8>,
}

/// Convert hex string (e.g. "FF0000" or "#FF0000") to Rgba<u8>
fn hex_to_rgba(hex: &str, alpha: u8) -> Rgba<u8> {
    let hex = hex.trim_start_matches('#');
    if hex.len() != 6 {
        return Rgba([255, 0, 0, alpha]); // Fallback red
    }

    let r = u8::from_str_radix(&hex[0..2], 16).unwrap_or(255);
    let g = u8::from_str_radix(&hex[2..4], 16).unwrap_or(0);
    let b = u8::from_str_radix(&hex[4..6], 16).unwrap_or(0);

    Rgba([r, g, b, alpha])
}

impl RadialGaugeConfig {
    pub fn from_stored(stored: &crate::storage::StoredRadialGaugeConfig) -> Self {
        // Use defaults if fields are missing (fallback to "original" defaults logic)
        // Hardcoded here as "safe fallback" only.
        let default_start = -136.0;
        let default_end = 137.1;

        let start_angle = stored.start_angle_deg.unwrap_or(default_start);
        let end_angle = stored.end_angle_deg.unwrap_or(default_end);

        let gradient: Vec<GradientStop> = if stored.gradient.is_empty() {
            vec![
                GradientStop {
                    color: Rgba([255, 0, 0, 255]),
                    position: 0.0,
                }, // Red
                GradientStop {
                    color: Rgba([255, 60, 0, 255]),
                    position: 0.5,
                }, // Red-Orange
                GradientStop {
                    color: Rgba([255, 80, 0, 100]),
                    position: 1.0,
                }, // Orange
            ]
        } else {
            stored
                .gradient
                .iter()
                .map(|s| GradientStop {
                    color: hex_to_rgba(&s.color, s.alpha),
                    position: s.position,
                })
                .collect()
        };

        Self {
            center_x: 160, // Fixed for Z3
            center_y: 160, // Fixed for Z3
            outer_radius: stored.outer_radius.unwrap_or(152.5),
            inner_radius: stored.inner_radius.unwrap_or(130.0),
            start_angle_deg: start_angle,
            end_angle_deg: end_angle,
            gradient,
            indicator_radius: 10.0, // Could be configurable too if needed
            min_temp: 0.0,
            max_temp: 100.0,
            background_color: hex_to_rgba(
                stored.background_color.as_deref().unwrap_or("000000"),
                255,
            ),
        }
    }
}

// Removing impl Default since we prefer explicit configuration from Storage,
// but keeping a minimal default for tests/fallback might be useful locally.
impl Default for RadialGaugeConfig {
    fn default() -> Self {
        Self::from_stored(&crate::storage::StoredRadialGaugeConfig::default())
    }
}

/// Convert degrees to radians
fn deg_to_rad(deg: f32) -> f32 {
    deg * (PI as f32) / 180.0
}

/// Interpolate between two colors based on factor (0.0 to 1.0)
fn lerp_color(c1: &Rgba<u8>, c2: &Rgba<u8>, t: f32) -> Rgba<u8> {
    let t = t.clamp(0.0, 1.0);
    Rgba([
        (c1[0] as f32 + (c2[0] as f32 - c1[0] as f32) * t) as u8,
        (c1[1] as f32 + (c2[1] as f32 - c1[1] as f32) * t) as u8,
        (c1[2] as f32 + (c2[2] as f32 - c1[2] as f32) * t) as u8,
        255,
    ])
}

/// Interpolate color from gradient based on position (0.0 to 1.0)
pub fn interpolate_color(gradient: &[GradientStop], position: f32) -> Rgba<u8> {
    let position = position.clamp(0.0, 1.0);

    // Find the two stops to interpolate between
    for i in 0..gradient.len() - 1 {
        let start = &gradient[i];
        let end = &gradient[i + 1];

        if position >= start.position && position <= end.position {
            let range = end.position - start.position;
            if range == 0.0 {
                return start.color;
            }
            let t = (position - start.position) / range;
            return lerp_color(&start.color, &end.color, t);
        }
    }

    // Return last color if position is at or beyond end
    gradient
        .last()
        .map(|s| s.color)
        .unwrap_or(Rgba([255, 255, 255, 255]))
}

/// Convert temperature to angle position on the arc
pub fn temp_to_angle(config: &RadialGaugeConfig, temp: f32) -> f32 {
    let normalized =
        ((temp - config.min_temp) / (config.max_temp - config.min_temp)).clamp(0.0, 1.0);
    config.start_angle_deg + normalized * (config.end_angle_deg - config.start_angle_deg)
}

/// Convert temperature to position (0.0 to 1.0) for gradient
pub fn temp_to_position(config: &RadialGaugeConfig, temp: f32) -> f32 {
    ((temp - config.min_temp) / (config.max_temp - config.min_temp)).clamp(0.0, 1.0)
}

/// Blend a color onto the image at the specified position with alpha blending
fn blend_pixel(img: &mut RgbaImage, x: u32, y: u32, color: Rgba<u8>) {
    if x >= img.width() || y >= img.height() {
        return;
    }

    let bg = img.get_pixel_mut(x, y);
    let alpha = color[3] as f32 / 255.0;

    // Simple alpha blending: Source OVER Destination
    // out = src * alpha + dst * (1 - alpha)

    for i in 0..3 {
        bg[i] = (color[i] as f32 * alpha + bg[i] as f32 * (1.0 - alpha)) as u8;
    }

    // Alpha accumulation (simplified)
    // usually we want to keep the highest alpha or blend them, but since we are drawing on opaque/black mostly:
    // If background is transparent, this matches standard blending.
    // If background is opaque, the alpha doesn't change much.
    // Let's enforce full opacity if we are blending onto a background that we treat as the final layer.
    // But to be correct for `put_pixel`:
    bg[3] = (color[3] as f32 + bg[3] as f32 * (1.0 - alpha)).min(255.0) as u8;
}

/// Draw a filled circle with Anti-Aliasing
fn draw_filled_circle(img: &mut RgbaImage, cx: i32, cy: i32, radius: f32, color: Rgba<u8>) {
    let r_ceil = radius.ceil() as i32 + 1;

    for dy in -r_ceil..=r_ceil {
        for dx in -r_ceil..=r_ceil {
            let dist_sq = (dx * dx + dy * dy) as f32;

            // Optimization: fully inside check
            if dist_sq < (radius - 1.0).powi(2) {
                let px = cx + dx;
                let py = cy + dy;
                if px >= 0 && py >= 0 {
                    // Since lines logic uses i32 loops but blend_pixel checks bounds, we are safe to call blend or put
                    // For inner pixels (fully opaque relative to the color passed), put is faster if alpha is 255,
                    // but to support transparent colors (like the orange track), we must blend.
                    blend_pixel(img, px as u32, py as u32, color);
                }
                continue;
            }

            let dist = dist_sq.sqrt();
            if dist < radius + 0.5 {
                // +0.5 allows for 1px soft edge straddling the boundary
                let aa_alpha = ((radius + 0.5) - dist).clamp(0.0, 1.0);

                let mut pixel_color = color;
                pixel_color[3] = (color[3] as f32 * aa_alpha) as u8;

                let px = cx + dx;
                let py = cy + dy;
                if px >= 0 && py >= 0 {
                    blend_pixel(img, px as u32, py as u32, pixel_color);
                }
            }
        }
    }
}

/// Helper to get the center point of the arc at a specific angle
fn get_arc_point(config: &RadialGaugeConfig, angle_deg: f32) -> (i32, i32) {
    let angle_rad = deg_to_rad(angle_deg - 90.0); // -90 to align with standard math (0 is right)
    let radius = (config.inner_radius + config.outer_radius) / 2.0;

    let px = config.center_x as f32 + angle_rad.cos() * radius;
    let py = config.center_y as f32 + angle_rad.sin() * radius; // y increases downwards
    (px as i32, py as i32)
}

/// Draw a segment of the arc (from start_angle to end_angle) with Anti-Aliasing
fn draw_arc_segment(
    img: &mut RgbaImage,
    config: &RadialGaugeConfig,
    segment_start_deg: f32,
    segment_end_deg: f32,
) {
    let width = img.width() as i32;
    let height = img.height() as i32;

    // Bounds for optimization
    // A full scan is acceptable for 320x320

    // AA logic: We will fade the alpha at inner_radius and outer_radius.
    // We will NOT AA the angular start/end here because Caps handle that.

    for y in 0..height {
        for x in 0..width {
            let dx = x - config.center_x;
            let dy = y - config.center_y;
            let dist_sq = (dx * dx + dy * dy) as f32;
            let dist = dist_sq.sqrt();

            // Expand bounds slightly for AA (0.5px margin)
            let inner_limit = config.inner_radius - 0.5;
            let outer_limit = config.outer_radius + 0.5;

            if dist >= inner_limit && dist <= outer_limit {
                // Calculate angle from Top
                let angle_rad = (dx as f32).atan2(-(dy as f32));
                let angle_deg = angle_rad * 180.0 / (PI as f32);

                // Segment Check
                let in_segment = if segment_start_deg <= segment_end_deg {
                    angle_deg >= segment_start_deg && angle_deg <= segment_end_deg
                } else {
                    angle_deg >= segment_start_deg || angle_deg <= segment_end_deg
                };

                if in_segment {
                    // Calculate AA Factor
                    // 1.0 if fully inside, decaying to 0.0 at edges
                    let mut alpha_factor: f32 = 1.0;

                    if dist < config.inner_radius + 0.5 {
                        alpha_factor = alpha_factor.min(dist - (config.inner_radius - 0.5));
                    }
                    if dist > config.outer_radius - 0.5 {
                        alpha_factor = alpha_factor.min((config.outer_radius + 0.5) - dist);
                    }
                    alpha_factor = alpha_factor.clamp(0.0_f32, 1.0_f32);

                    // Color calculation
                    let total_arc_range = config.end_angle_deg - config.start_angle_deg;
                    let angle_from_start = angle_deg - config.start_angle_deg;
                    let position = (angle_from_start / total_arc_range).clamp(0.0, 1.0);

                    let mut color = interpolate_color(&config.gradient, position);
                    color[3] = (color[3] as f32 * alpha_factor) as u8;

                    blend_pixel(img, x as u32, y as u32, color);
                }
            }
        }
    }
}

/// Main function to draw the dynamic gauge with detached tip and background track
pub fn draw_dynamic_gauge(img: &mut RgbaImage, config: &RadialGaugeConfig, current_temp: f32) {
    // 1. Calculate angles
    let current_angle = temp_to_angle(config, current_temp);

    // Configuration for styling
    // Arc thickness is 30px. Cap radius is 15px.
    // At 125px radius, 1 degree ~ 2.18px.
    // Cap radius in degrees ~ 15 / 2.18 ~ 6.8 degrees. Let's say 7.0.
    let cap_radius_deg = 7.0;

    // Gap visually between edges
    let gap_visual_deg = 4.0;

    // To separate two caps (Radius R) by Gap G:
    // Distance between centers = R + G + R = 2R + G.
    let center_separation = (2.0 * cap_radius_deg) + gap_visual_deg;

    // We want the indicator to be a "Bolinha" (Circle/Ball).
    // So Tip Start = Tip End = Current Angle.
    // If we want a slight elongation (Pill), we add width.
    // Let's make it a slight pill to hold the "separator" look firmly.
    let tip_half_width = 0.0; // pure circle centered on value

    let tip_start_deg = current_angle - tip_half_width;
    let tip_end_deg = current_angle + tip_half_width;

    // Check bounds for tip to not fly off
    let tip_start_deg = tip_start_deg.clamp(config.start_angle_deg, config.end_angle_deg);
    let tip_end_deg = tip_end_deg.clamp(config.start_angle_deg, config.end_angle_deg);

    // Body ends before the tip
    let body_end_deg = tip_start_deg - center_separation;

    // Track starts after the tip
    let track_start_deg = tip_end_deg + center_separation;

    // Draw Main Body (Gradient)
    // From Start to Body End
    if body_end_deg > config.start_angle_deg {
        let actual_start = config.start_angle_deg;
        // Clamp body end to be safe
        let actual_end = body_end_deg.min(config.end_angle_deg);

        if actual_end > actual_start {
            draw_arc_segment(img, config, actual_start, actual_end);

            // Draw the End Caps for the main body
            draw_cap(img, config, actual_start, true); // <--- Start Cap (Left)
            draw_cap(img, config, actual_end, true); // <--- End Cap of the body (before indicator)
        }
    }

    // Draw Tip (Indicator)
    // Always draw if within global bounds roughly
    if tip_start_deg >= config.start_angle_deg && tip_end_deg <= config.end_angle_deg {
        // Draw segment (might be zero length)
        if tip_end_deg > tip_start_deg {
            draw_arc_segment(img, config, tip_start_deg, tip_end_deg);
        }
        // Draw the Indicator Caps (Ball/Pill)
        draw_cap(img, config, tip_start_deg, true);
        if tip_end_deg > tip_start_deg {
            draw_cap(img, config, tip_end_deg, true);
        }
    }

    // Draw Track (Remainder of the Gradient)
    // From Track Start to End
    if track_start_deg < config.end_angle_deg {
        let actual_track_start = track_start_deg.max(config.start_angle_deg);
        let actual_track_end = config.end_angle_deg;

        if actual_track_end > actual_track_start {
            // User requested to "continue the color", so we use the gradient for the track too
            draw_arc_segment(img, config, actual_track_start, actual_track_end);

            // Draw the Track Caps (Empty/Remaining part)
            draw_cap(img, config, actual_track_start, true); // <--- Track Start Cap (after indicator)
            draw_cap(img, config, actual_track_end, true); // <--- Gauge End Cap (Right)
        }
    }
}

/// Modified draw_cap to take color source choice
fn draw_cap(img: &mut RgbaImage, config: &RadialGaugeConfig, angle_deg: f32, use_gradient: bool) {
    let (cx, cy) = get_arc_point(config, angle_deg);
    let cap_radius = (config.outer_radius - config.inner_radius) / 2.0;

    let color = if use_gradient {
        let total_arc_range = config.end_angle_deg - config.start_angle_deg;
        let angle_from_start = angle_deg - config.start_angle_deg;
        let position = (angle_from_start / total_arc_range).clamp(0.0, 1.0);
        interpolate_color(&config.gradient, position)
    } else {
        Rgba([40, 40, 40, 255])
    };

    draw_filled_circle(img, cx, cy, cap_radius, color);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_interpolate_color() {
        let gradient = vec![
            GradientStop {
                color: Rgba([0, 255, 0, 255]),
                position: 0.0,
            },
            GradientStop {
                color: Rgba([255, 0, 0, 255]),
                position: 1.0,
            },
        ];

        let start = interpolate_color(&gradient, 0.0);
        assert_eq!(start, Rgba([0, 255, 0, 255]));

        let end = interpolate_color(&gradient, 1.0);
        assert_eq!(end, Rgba([255, 0, 0, 255]));

        let mid = interpolate_color(&gradient, 0.5);
        assert_eq!(mid[0], 127); // approximately half
        assert_eq!(mid[1], 127); // approximately half
    }

    #[test]
    fn test_temp_to_position() {
        let mut config = RadialGaugeConfig::default();
        config.min_temp = 20.0;
        config.max_temp = 60.0;

        assert_eq!(temp_to_position(&config, 20.0), 0.0);
        assert_eq!(temp_to_position(&config, 60.0), 1.0);
        assert!((temp_to_position(&config, 40.0) - 0.5).abs() < 0.01);
    }
}
