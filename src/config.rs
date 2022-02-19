use ffmpeg::util::log::level::Level as FfLevel;
use ffmpeg_next as ffmpeg;
use log::{Level, LevelFilter};
use once_cell::sync::Lazy;
use serde::Deserialize;
use std::fs::File;
use std::io::Read;
use std::sync::Arc;

#[derive(Deserialize, Clone, Debug)]
#[serde(rename_all = "lowercase")]
pub enum FileSourceType {
    Local,
    S3,
}

#[derive(Deserialize, Clone, Debug)]
#[serde(rename_all = "lowercase")]
pub enum LogLevel {
    Error,
    Warn,
    Info,
    Debug,
    Trace,
}

impl LogLevel {
    pub fn level(&self) -> Level {
        match *self {
            LogLevel::Error => Level::Error,
            LogLevel::Warn => Level::Warn,
            LogLevel::Info => Level::Info,
            LogLevel::Debug => Level::Debug,
            LogLevel::Trace => Level::Trace,
        }
    }

    pub fn ffmpeg(&self) -> FfLevel {
        match *self {
            LogLevel::Error => FfLevel::Error,
            LogLevel::Warn => FfLevel::Error,
            LogLevel::Info => FfLevel::Info,
            LogLevel::Debug => FfLevel::Debug,
            LogLevel::Trace => FfLevel::Trace,
        }
    }

    pub fn level_filter(&self) -> LevelFilter {
        match *self {
            LogLevel::Error => LevelFilter::Error,
            LogLevel::Warn => LevelFilter::Warn,
            LogLevel::Info => LevelFilter::Info,
            LogLevel::Debug => LevelFilter::Debug,
            LogLevel::Trace => LevelFilter::Trace,
        }
    }
}

#[derive(Deserialize, Clone, Debug)]
#[serde(rename_all = "lowercase")]
pub enum VideoFileType {
    Matroska,
    Mp4,
    WebM,
}

impl VideoFileType {
    pub fn extension(&self) -> &str {
        match *self {
            VideoFileType::Matroska => &"mkv",
            VideoFileType::Mp4 => &"mp4",
            VideoFileType::WebM => &"webm",
        }
    }
}

#[derive(Deserialize, Clone, Debug)]
pub struct Config {
    pub cameras: Vec<CameraConfig>,
    pub cloud: CloudConfig,
    pub motion: MotionConfig,
    pub display: DisplayConfig,
    pub storage: StorageConfig,
    pub log_level: LogLevel,
    pub ffmpeg_level: LogLevel,
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
    pub draw_contours: Option<bool>,
    pub draw_rectangles: Option<bool>,
}

#[derive(Deserialize, Clone, Debug)]
pub struct DisplayConfig {
    pub enabled: Option<bool>,
}

#[derive(Deserialize, Clone, Debug)]
pub struct StorageConfig {
    pub storage_type: FileSourceType,
    pub path: String,
    pub video_file_type: VideoFileType,
}

static GLOBAL_DATA: Lazy<Arc<Config>> = Lazy::new(|| {
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
