use log::debug;
use std::path::{Path, PathBuf};

use crate::libs::keyer_io::keyer_io::KeyerType;

use serde_derive::Deserialize;
use serde_derive::Serialize;

#[derive(Serialize, Deserialize, Debug)]
struct Config {
    keyer: Keyer,
}

#[derive(Serialize, Deserialize, Debug)]
struct Keyer {
    keyerType: KeyerType,
    port: String,
    wpm: usize,
}

const CONFIG_FILE_NAME: &str = "digimorse.toml";

pub struct ConfigurationStore {
    config_file_path: Box<Path>,
    config: Config,
}

impl ConfigurationStore {
    pub fn new(config_path: Box<Path>) -> Result<ConfigurationStore, String> {
        let mut config_file_path = PathBuf::new();
        config_file_path.push(config_path);
        config_file_path.push(CONFIG_FILE_NAME);
        debug!("Config file is {:?}", config_file_path);
        if !config_file_path.exists() {
            debug!("Creating config dir {:?}", config_file_path);
            let config = Config {
                keyer: Keyer {
                    keyerType: KeyerType::Null,
                    port: "".to_string(),
                    wpm: 20
                }
            };
            save_configuration(&config_file_path, &config)?;
            return Ok(ConfigurationStore {
                config_file_path: config_file_path.clone().into_boxed_path(),
                config: config,
            });
        } else {
            let config = read_configuration(&config_file_path)?;
            return Ok(ConfigurationStore {
                config_file_path: config_file_path.clone().into_boxed_path(),
                config: config,
            });
        }
    }
}

fn save_configuration(config_file_path: &PathBuf, config: &Config) -> Result<Config, String> {
    todo!()
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
