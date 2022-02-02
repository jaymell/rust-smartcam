use serde::Deserialize;
use std::fs::File;
use std::io::Read;
use std::sync::Arc;
use toml::Value;
use log::debug;

use once_cell::sync::Lazy;


#[derive(Deserialize, Clone, Debug)]
#[serde(rename_all = "lowercase")]
pub enum FileSourceType {
    Local,
    S3
}

#[derive(Deserialize, Clone, Debug)]
pub struct Config {
    pub cameras: Vec<CameraConfig>,
    pub cloud: CloudConfig,
    pub motion: MotionConfig,
    pub display: DisplayConfig,
    pub storage: StorageConfig
}

#[derive(Deserialize, Clone, Debug)]
pub struct CameraConfig {
    pub label: String,
    pub camera_type: String,
    pub source: Option<String>,
}

#[derive(Deserialize, Clone, Debug)]
pub struct CloudConfig {
    pub enabled: Option<bool>,
    pub bucket: String,
    pub region: Option<String>,
}

#[derive(Deserialize, Clone, Debug)]
pub struct MotionConfig {
    pub min_threshold_size: i32,
}

#[derive(Deserialize, Clone, Debug)]
pub struct DisplayConfig {
    pub enabled: Option<bool>,
}

#[derive(Deserialize, Clone, Debug)]
pub struct StorageConfig {
    pub storage_type: FileSourceType,
    pub path: String
}

static GLOBAL_DATA: Lazy<Arc<Config>> = Lazy::new( || {
    let mut config_toml = String::new();

    // let path = match path {
    //     Some(path) => path,
    //     None => "settings.toml".to_string(),
    // };

    let path = "settings.toml".to_string();

    let mut file = match File::open(&path) {
        Ok(file) => file,
        Err(_) => {
            panic!("Could not find config file!");
        }
    };

    file.read_to_string(&mut config_toml)
        .unwrap_or_else(|err| panic!("Error while reading config: [{}]", err));

    let cfg = toml::from_str(&config_toml).unwrap();
    debug!("Config: {:?}", cfg);
    Arc::new(cfg)
});


pub fn load_config(path: Option<String>) -> Arc<Config> {
    Arc::clone(&GLOBAL_DATA)
}

#[cfg(test)]
mod tests {

    use super::FileSourceType;
    use std::str::FromStr;

    #[test]
    fn should_parse_lowercase_fileSourceType() {
        let variant = FileSourceType::from_str("local").unwrap();
        assert_eq!(FileSourceType::Local, variant);
    }
}