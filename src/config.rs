use serde::Deserialize;
use std::fs::File;
use std::io::Read;
use std::sync::Arc;
use toml::Value;

#[derive(Deserialize, Clone)]
pub struct Config {
    pub cameras: Vec<CameraConfig>,
    pub cloud: CloudConfig,
    pub motion: MotionConfig,
    pub display: DisplayConfig,
}

#[derive(Deserialize, Clone)]
pub struct CameraConfig {
    pub label: String,
    pub camera_type: String,
    pub source: Option<String>,
}

#[derive(Deserialize, Clone)]
pub struct CloudConfig {
    pub enabled: Option<bool>,
    pub bucket: String,
    pub region: Option<String>,
}

#[derive(Deserialize, Clone)]
pub struct MotionConfig {
    pub min_threshold_size: i32,
}

#[derive(Deserialize, Clone)]
pub struct DisplayConfig {
    pub enabled: Option<bool>,
}

pub fn load_config(path: Option<String>) -> Arc<Config> {
    let mut config_toml = String::new();

    let path = match path {
        Some(path) => path,
        None => "settings.toml".to_string(),
    };

    let mut file = match File::open(&path) {
        Ok(file) => file,
        Err(_) => {
            panic!("Could not find config file!");
        }
    };

    file.read_to_string(&mut config_toml)
        .unwrap_or_else(|err| panic!("Error while reading config: [{}]", err));

    Arc::new(toml::from_str(&config_toml).unwrap())
}
