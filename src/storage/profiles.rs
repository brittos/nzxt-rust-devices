//! Profile storage and persistence.
//!
//! Handles saving and loading profiles to/from disk.
//! Cross-platform: uses appropriate config directories for each OS.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

use crate::error::{KrakenError, Result};

// =============================================================================
// Config Path
// =============================================================================

const APP_NAME: &str = "nzxt-rust";
const CONFIG_FILE: &str = "config.json";

/// Get the configuration directory path.
/// - Linux: ~/.config/nzxt-rust/
/// - Windows: %APPDATA%\nzxt-rust\
pub fn get_config_dir() -> Result<PathBuf> {
    dirs::config_dir()
        .map(|p| p.join(APP_NAME))
        .ok_or_else(|| KrakenError::InvalidProfile("Could not find config directory".into()))
}

/// Get the full path to the config file.
pub fn get_config_path() -> Result<PathBuf> {
    Ok(get_config_dir()?.join(CONFIG_FILE))
}

// =============================================================================
// Storage Structures
// =============================================================================

/// Startup configuration for the `start` command
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StartupConfig {
    /// Display mode: "radial", "image", "gif"
    #[serde(default = "default_display_mode")]
    pub display_mode: String,

    /// Path to image (used when display_mode = "image")
    #[serde(default)]
    pub image_path: Option<String>,

    /// Path to GIF (used when display_mode = "gif")
    #[serde(default)]
    pub gif_path: Option<String>,

    /// Cooling profile: "silent", "performance", "fixed"
    #[serde(default = "default_cooling_profile")]
    pub cooling_profile: String,

    /// Temperature source: "liquid" or "cpu"
    #[serde(default = "default_temp_source")]
    pub temperature_source: String,

    /// Update interval in seconds
    #[serde(default = "default_interval")]
    pub interval: u64,

    /// LCD brightness (0-100)
    #[serde(default = "default_brightness")]
    pub brightness: u8,

    /// LCD orientation (0, 90, 180, 270)
    #[serde(default)]
    pub orientation: u16,
}

fn default_display_mode() -> String {
    "radial".to_string()
}

fn default_cooling_profile() -> String {
    "silent".to_string()
}

fn default_interval() -> u64 {
    2
}

fn default_brightness() -> u8 {
    100
}

impl Default for StartupConfig {
    fn default() -> Self {
        Self {
            display_mode: default_display_mode(),
            image_path: None,
            gif_path: None,
            cooling_profile: default_cooling_profile(),
            temperature_source: default_temp_source(),
            interval: default_interval(),
            brightness: default_brightness(),
            orientation: 0,
        }
    }
}

/// Main configuration file structure
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AppConfig {
    /// Startup configuration for the `start` command
    #[serde(default)]
    pub startup: StartupConfig,
    /// Cooling profiles by name
    pub profiles: HashMap<String, StoredCoolingProfile>,
    /// LCD profiles by name
    pub lcd: HashMap<String, StoredLcdProfile>,
    /// Currently active profile name
    pub active_profile: Option<String>,
}

/// Stored cooling profile
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoredCoolingProfile {
    pub pump: Option<StoredChannel>,
    pub fan: Option<StoredChannel>,
}

/// Stored channel configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoredChannel {
    pub mode: String,
    pub fixed: Option<u8>,
    pub curve: Vec<(u8, u8)>,
    /// Temperature source: "Liquid" (default) or "CPU"
    #[serde(default = "default_temp_source")]
    pub temperature_source: String,
}

fn default_temp_source() -> String {
    "Liquid".to_string()
}

/// Stored LCD profile
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoredLcdProfile {
    pub brightness: f32,
    pub rotation: u16,
    pub display_mode: Option<String>,
    /// Custom configuration for the radial gauge visual
    #[serde(default)]
    pub radial_gauge: Option<StoredRadialGaugeConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoredRadialGaugeConfig {
    pub outer_radius: Option<f32>,
    pub inner_radius: Option<f32>,
    pub start_angle_deg: Option<f32>,
    pub end_angle_deg: Option<f32>,
    pub gradient: Vec<StoredGradientStop>,
    pub background_color: Option<String>,
}

impl Default for StoredRadialGaugeConfig {
    fn default() -> Self {
        Self {
            outer_radius: Some(152.5),
            inner_radius: Some(130.0),
            // Bottom-right start
            start_angle_deg: Some(-136.0),
            // Bottom-left end
            end_angle_deg: Some(137.1),
            gradient: vec![
                StoredGradientStop {
                    color: "FF0000".to_string(), // Red
                    alpha: 255,
                    position: 0.0,
                },
                StoredGradientStop {
                    color: "FF3C00".to_string(), // Red-Orange
                    alpha: 255,
                    position: 0.5,
                },
                StoredGradientStop {
                    color: "FF5000".to_string(), // Orange
                    alpha: 100,
                    position: 1.0,
                },
            ],
            background_color: Some("000000".to_string()),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoredGradientStop {
    /// Hex color code (e.g., "FF0000")
    pub color: String,
    /// Alpha channel (0-255)
    pub alpha: u8,
    /// Position in gradient (0.0 - 1.0)
    pub position: f32,
}

// =============================================================================
// Storage Functions
// =============================================================================

/// Load configuration from disk.
pub fn load_config() -> Result<AppConfig> {
    let path = get_config_path()?;

    if !path.exists() {
        return Ok(AppConfig::default());
    }

    let content = std::fs::read_to_string(&path)
        .map_err(|e| KrakenError::InvalidProfile(format!("Failed to read config: {}", e)))?;

    serde_json::from_str(&content)
        .map_err(|e| KrakenError::InvalidProfile(format!("Failed to parse config: {}", e)))
}

/// Save configuration to disk.
pub fn save_config(config: &AppConfig) -> Result<()> {
    let dir = get_config_dir()?;
    let path = dir.join(CONFIG_FILE);

    // Create directory if needed
    std::fs::create_dir_all(&dir)
        .map_err(|e| KrakenError::InvalidProfile(format!("Failed to create config dir: {}", e)))?;

    let content = serde_json::to_string_pretty(config)
        .map_err(|e| KrakenError::InvalidProfile(format!("Failed to serialize config: {}", e)))?;

    std::fs::write(&path, content)
        .map_err(|e| KrakenError::InvalidProfile(format!("Failed to write config: {}", e)))?;

    Ok(())
}

/// Ensure that the configuration file exists.
/// If it doesn't exist, create it with default values (including Radial Gauge defaults).
pub fn ensure_config_exists() -> Result<()> {
    let path = get_config_path()?;
    if path.exists() {
        return Ok(());
    }

    println!("Config file not found. Creating default at {:?}", path);
    // Create an empty config but populate it if desired?
    // The requirement says "se nao tiver o arquivo, gerar com essas configuracoes".
    // Does this mean generating a default PROFILE with these settings?
    // Or just an empty config?
    // Usually config.json has "profiles": {}.
    // If the user wants to use these settings, they likely need a profile defined.
    // I'll create a "Default_Gauge" profile in the LCD section.

    let mut config = AppConfig::default();

    // Create a default LCD profile with the gauge settings
    let default_lcd = StoredLcdProfile {
        brightness: 1.0,
        rotation: 0,
        display_mode: Some("Radial".to_string()),
        radial_gauge: Some(StoredRadialGaugeConfig::default()),
    };

    config.lcd.insert("default_gauge".to_string(), default_lcd);
    config.active_profile = Some("default_gauge".to_string());

    save_config(&config)?;
    Ok(())
}

// Removed import_profiles function.

/// Get an LCD profile by name.
pub fn get_lcd_profile(name: &str) -> Result<StoredLcdProfile> {
    let config = load_config()?;
    config
        .lcd
        .get(&name.to_lowercase())
        .cloned()
        .ok_or_else(|| KrakenError::InvalidProfile(format!("LCD profile '{}' not found", name)))
}
