use serde::Deserialize;
use std::fs::File;
use std::io::Read;
use toml::Value;

#[derive(Deserialize)]
pub struct Config {
    pub cameras: Vec<CameraConfig>,
    pub cloud: CloudConfig,
    pub motion: MotionConfig,
}

#[derive(Deserialize)]
pub struct CameraConfig {
    pub label: String,
    pub camera_type: String,
    pub source: String,
}

#[derive(Deserialize)]
pub struct CloudConfig {
    pub enabled: Option<bool>,
    pub bucket: String,
    pub region: Option<String>,
}

#[derive(Deserialize)]
pub struct MotionConfig {
    pub min_threshold_size: i32,
}


pub fn load_config(path: Option<String>) -> Config {
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

    toml::from_str(&config_toml).unwrap()
}
