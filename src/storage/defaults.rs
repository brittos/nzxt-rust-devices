//! Management of local default profiles (defaults.json).
//!
//! Acts as the local "database" of profile definitions, matching NZXT CAM's coolingController.json format.

use super::profiles::get_config_dir;
use crate::config::{
    PROFILE_PERFORMANCE, PROFILE_PUMP_PERFORMANCE, PROFILE_PUMP_SILENT, PROFILE_SILENT,
};
use crate::error::{KrakenError, Result};
use crate::storage::types::{
    ChannelSetting, CoolingController, CoolingMode, CoolingProfile, Threshold,
};
use std::path::PathBuf;

const DEFAULTS_FILE: &str = "defaults.json";

/// Get the path to the defaults.json file.
pub fn get_defaults_path() -> Result<PathBuf> {
    Ok(get_config_dir()?.join(DEFAULTS_FILE))
}

/// Load defaults from disk.
pub fn load_defaults() -> Result<CoolingController> {
    let path = get_defaults_path()?;

    if !path.exists() {
        return Err(KrakenError::InvalidProfile(
            "Defaults file not found".into(),
        ));
    }

    let content = std::fs::read_to_string(&path)
        .map_err(|e| KrakenError::InvalidProfile(format!("Failed to read defaults: {}", e)))?;

    serde_json::from_str(&content)
        .map_err(|e| KrakenError::InvalidProfile(format!("Failed to parse defaults: {}", e)))
}

/// Save defaults to disk.
pub fn save_defaults(controller: &CoolingController) -> Result<()> {
    let path = get_defaults_path()?;
    // Ensure dir exists (should be handled by storage, but good to be safe)
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).ok();
    }

    let content = serde_json::to_string_pretty(controller)
        .map_err(|e| KrakenError::InvalidProfile(format!("Failed to serialize defaults: {}", e)))?;

    std::fs::write(&path, content)
        .map_err(|e| KrakenError::InvalidProfile(format!("Failed to write defaults: {}", e)))?;

    Ok(())
}

/// Ensure defaults.json exists, creating it with built-in defaults if missing.
pub fn ensure_defaults_exist() -> Result<()> {
    let path = get_defaults_path()?;
    if path.exists() {
        return Ok(());
    }

    // Create default structure matching NZXT CAM
    let controller = CoolingController {
        active_profile_id: Some("Silent".into()),
        profiles: vec![
            create_profile("Silent", "Silent", &PROFILE_PUMP_SILENT, &PROFILE_SILENT),
            create_profile(
                "Performance",
                "Performance",
                &PROFILE_PUMP_PERFORMANCE,
                &PROFILE_PERFORMANCE,
            ),
            create_fixed_profile(),
        ],
    };

    save_defaults(&controller)?;
    println!("Created default profiles at: {}", path.display());

    Ok(())
}

/// Get a profile by originId (e.g., "Silent", "Performance", "Fixed").
/// Case-insensitive search.
pub fn get_profile(origin_id: &str) -> Result<CoolingProfile> {
    // Ensure defaults exist before loading
    ensure_defaults_exist()?;

    let defaults = load_defaults()?;
    let search = origin_id.to_lowercase();

    defaults
        .profiles
        .into_iter()
        .find(|p| {
            p.origin_id
                .as_ref()
                .map(|id| id.to_lowercase() == search)
                .unwrap_or(false)
                || p.id.to_lowercase() == search
        })
        .ok_or_else(|| {
            KrakenError::InvalidProfile(format!("Profile '{}' not found in defaults", origin_id))
        })
}

/// Update fixed values for specific channel in "Fixed" profile.
pub fn update_fixed(channel_name: &str, duty: u8) -> Result<()> {
    ensure_defaults_exist()?;
    let mut defaults = load_defaults()?;

    let profile = defaults
        .profiles
        .iter_mut()
        .find(|p| p.origin_id.as_deref() == Some("Fixed") || p.id == "Fixed")
        .ok_or_else(|| KrakenError::InvalidProfile("Fixed profile not found in defaults".into()))?;

    let channel = profile
        .channel_settings
        .iter_mut()
        .find(|c| c.channel_name.to_lowercase() == channel_name.to_lowercase())
        .ok_or_else(|| {
            KrakenError::InvalidProfile(format!(
                "Channel '{}' not found in Fixed profile",
                channel_name
            ))
        })?;

    if let Some(mode) = &mut channel.mode {
        mode.fixed_percentage = Some(duty);
    } else {
        // Create mode if missing (unlikely for valid defaults)
        channel.mode = Some(CoolingMode {
            mode_type: Some("Fixed".into()),
            fixed_percentage: Some(duty),
            custom_thresholds: Some(vec![]),
            temperature_option: None,
        });
    }

    save_defaults(&defaults)
}

// Helper to create profiles
fn create_profile(
    id: &str,
    name: &str,
    pump_curve: &[(u8, u8)],
    fan_curve: &[(u8, u8)],
) -> CoolingProfile {
    CoolingProfile {
        id: id.into(),
        origin_id: Some(id.into()),
        name: Some(name.into()),
        channel_settings: vec![
            ChannelSetting {
                channel_name: "pump".into(),
                mode: Some(create_curve_mode(id, pump_curve)),
            },
            ChannelSetting {
                channel_name: "fan".into(),
                mode: Some(create_curve_mode(id, fan_curve)),
            },
        ],
    }
}

fn create_curve_mode(mode_type: &str, points: &[(u8, u8)]) -> CoolingMode {
    CoolingMode {
        mode_type: Some(mode_type.into()),
        fixed_percentage: None,
        custom_thresholds: Some(
            points
                .iter()
                .map(|&(t, f)| Threshold {
                    temperature: t,
                    fan_percentage: f,
                })
                .collect(),
        ),
        temperature_option: Some("Liquid".into()),
    }
}

fn create_fixed_profile() -> CoolingProfile {
    CoolingProfile {
        id: "Fixed".into(),
        origin_id: Some("Fixed".into()),
        name: Some("Fixed".into()),
        channel_settings: vec![
            ChannelSetting {
                channel_name: "pump".into(),
                mode: Some(CoolingMode {
                    mode_type: Some("Fixed".into()),
                    fixed_percentage: Some(100), // Default safe pump speed
                    custom_thresholds: Some(vec![]),
                    temperature_option: None,
                }),
            },
            ChannelSetting {
                channel_name: "fan".into(),
                mode: Some(CoolingMode {
                    mode_type: Some("Fixed".into()),
                    fixed_percentage: Some(50), // Default fan speed
                    custom_thresholds: Some(vec![]),
                    temperature_option: None,
                }),
            },
        ],
    }
}
