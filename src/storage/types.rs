use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CoolingController {
    pub active_profile_id: Option<String>,
    pub profiles: Vec<CoolingProfile>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CoolingProfile {
    pub id: String,
    pub origin_id: Option<String>,
    pub name: Option<String>,
    pub channel_settings: Vec<ChannelSetting>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChannelSetting {
    pub channel_name: String,
    pub mode: Option<CoolingMode>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CoolingMode {
    pub mode_type: Option<String>,
    pub fixed_percentage: Option<u8>,
    pub custom_thresholds: Option<Vec<Threshold>>,
    pub temperature_option: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Threshold {
    pub temperature: u8,
    pub fan_percentage: u8,
}
