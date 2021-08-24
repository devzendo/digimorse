use log::{debug, warn};
use std::path::{Path, PathBuf};

use crate::libs::keyer_io::keyer_io::KeyerType;

use serde_derive::Deserialize;
use serde_derive::Serialize;
use std::ops::Deref;
use std::fs;

#[derive(Serialize, Deserialize, Debug)]
struct Config {
    keyer: Keyer,
}

#[derive(Serialize, Deserialize, Debug)]
struct Keyer {
    keyer_type: KeyerType,
    port: String,
    wpm: usize,
}

const DEFAULT_CONFIG: Config = Config {
    keyer: Keyer {
        keyer_type: KeyerType::Null,
        port: String::new(),
        wpm: 20,
    }
};

const CONFIG_FILE_NAME: &str = "digimorse.toml";

pub struct ConfigurationStore {
    config_file_path: Box<Path>,
    config: Config,
}

impl ConfigurationStore {
    // Precondition: the config_dir_path will have been created (by config_dir).
    pub fn new(config_dir_path: Box<Path>) -> Result<ConfigurationStore, String> {
        let mut config_file_path = PathBuf::new();
        config_file_path.push(config_dir_path);
        config_file_path.push(CONFIG_FILE_NAME);
        debug!("Config file is {:?}", config_file_path);
        if !config_file_path.exists() {
            debug!("Creating config file {:?}", config_file_path);
            save_configuration(&config_file_path, &DEFAULT_CONFIG)?;
            return Ok(ConfigurationStore {
                config_file_path: config_file_path.clone().into_boxed_path(),
                config: DEFAULT_CONFIG,
            });
        } else {
            let config = read_configuration(&config_file_path)?;
            return Ok(ConfigurationStore {
                config_file_path: config_file_path.clone().into_boxed_path(),
                config: config,
            });
        }
    }

    pub fn get_config_file_path(&self) -> &Path {
        self.config_file_path.deref()
    }
}


fn save_configuration(config_file_path: &PathBuf, config: &Config) -> Result<(), String> {
    match toml::to_string(config) {
        Ok(toml) => {
            match config_file_path.to_str() {
                None => {
                    warn!("Could not convert config file path {:?} into a String", config_file_path);
                    Err("Could not obtain the config file path".to_owned())
                }
                Some(path) => {
                    match fs::write(path, toml) {
                        Ok(_ok) => {
                            debug!("Written configuration");
                            Ok(())
                        }
                        Err(err) => {
                            warn!("Could not write configuration file: {}", err);
                            Err(err.to_string())
                        }
                    }
                }
            }
        }
        Err(err) => {
            warn!("Could not serialise configuration to TOML: {}", err);
            Err(err.to_string())
        }
    }
}

fn read_configuration(config_file_path: &PathBuf) -> Result<Config, String> {
    let file_contents = std::fs::read_to_string(config_file_path);
    match file_contents {
        Ok(toml) => {
            let x: Result<Config, toml::de::Error> = toml::from_str(&*toml);
            match x {
                Ok(config) => {
                    Ok(config)
                }
                Err(err) => {
                    Err(format!("Could not parse config file {:?}: {}", config_file_path, err))
                }
            }
        }
        Err(e) => { Err(format!("Could not read config file {:?}: {}", config_file_path, e))}
    }
}

#[cfg(test)]
#[path = "./config_file_spec.rs"]
mod config_file_spec;
