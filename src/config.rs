use serde::Deserialize;
use std::fs::File;
use std::io::Read;
use toml::Value;

#[derive(Deserialize)]
pub struct Config {
    pub cameras: Vec<CameraConfig>,
    pub cloud: CloudConfig,
}

#[derive(Deserialize)]
pub struct CameraConfig {
    pub label: String,
    pub camera_type: String,
    pub source: String,
}

#[derive(Deserialize)]
pub struct CloudConfig {
    pub bucket: String,
    pub region: Option<String>,
}

pub fn load_config(path: Option<String>) -> Config {

    let mut config_toml = String::new();

    let path = match path {
        Some(path) => path,
        None => "settings.toml".to_string()
    };

    let mut file = match File::open(&path) {
        Ok(file) => file,
        Err(_) => {
            panic!("Could not find config file!");
            // return Config::new();
        }
    };

    file.read_to_string(&mut config_toml)
        .unwrap_or_else(|err| panic!("Error while reading config: [{}]", err));

    toml::from_str(&config_toml).unwrap()

    // let mut parser = Parser::new(&config_toml);
    // let t = parser.parse();

    // if t.is_none() {
    //     for err in &parser.errors {
    //         let (loline, locol) = parser.to_linecol(err.lo);
    //         let (hiline, hicol) = parser.to_linecol(err.hi);
    //         println!("{}:{}:{}-{}:{} error: {}",
    //                  path, loline, locol, hiline, hicol, err.desc);
    //     }
    //     panic!("Exiting");
    // }

    // let config = Value::Table(t.unwrap());
    // match t.decode(config) {
    //     Some(t) => t,
    //     None => panic!("Error while deserializing config")
    // }
}