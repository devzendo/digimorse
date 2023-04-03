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
    audio_devices: AudioDevices,
    #[serde(default = "default_transceiver")]
    transceiver: Transceiver
}

#[derive(Serialize, Deserialize, Debug)]
struct Keyer {
    keyer_type: KeyerType,
    port: String,
    wpm: usize,
    sidetone_frequency: u16,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Transceiver {
    #[serde(default = "default_transmit_offset_frequency")]
    transmit_offset_frequency: u16,
    #[serde(default = "default_transmit_amplitude")]
    transmit_amplitude: f32,
}

fn default_transceiver() -> Transceiver {
    DEFAULT_CONFIG.transceiver
}

fn default_transmit_offset_frequency() -> u16 {
    DEFAULT_CONFIG.transceiver.transmit_offset_frequency
}

fn default_transmit_amplitude() -> f32 {
    DEFAULT_CONFIG.transceiver.transmit_amplitude
}

#[derive(Serialize, Deserialize, Debug)]
struct AudioDevices {
    audio_out_device: String,
    rig_out_device: String,
    rig_in_device: String,
}

const DEFAULT_CONFIG: Config = Config {
    keyer: Keyer {
        keyer_type: KeyerType::Null,
        port: String::new(),
        wpm: 20,
        sidetone_frequency: 600,
    },
    audio_devices: AudioDevices {
        audio_out_device: String::new(),
        rig_out_device: String::new(),
        rig_in_device: String::new(),
    },
    transceiver: Transceiver {
        transmit_offset_frequency: 1500,
        transmit_amplitude: 0.5
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
            Ok(ConfigurationStore {
                config_file_path: config_file_path.clone().into_boxed_path(),
                config: DEFAULT_CONFIG,
            })
        } else {
            let config = read_configuration(&config_file_path)?;
            Ok(ConfigurationStore {
                config_file_path: config_file_path.clone().into_boxed_path(),
                config,
            })
        }
    }

    pub fn get_config_file_path(&self) -> &Path {
        self.config_file_path.deref()
    }

    fn save(&self)-> Result<(), String> {
        save_configuration(&self.config_file_path.to_path_buf(), &self.config)
    }


    pub fn set_keyer_type(&mut self, new_keyer_type: KeyerType) -> Result<(), String> {
        self.config.keyer.keyer_type = new_keyer_type;
        self.save()
    }

    pub fn get_keyer_type(&self) -> KeyerType {
        self.config.keyer.keyer_type
    }

    pub fn set_port(&mut self, new_port: String) -> Result<(), String> {
        self.config.keyer.port = new_port;
        self.save()
    }

    pub fn get_port(&self) -> String {
        self.config.keyer.port.to_owned()
    }

    pub fn set_wpm(&mut self, new_wpm: usize) -> Result<(), String> {
        self.config.keyer.wpm = new_wpm;
        self.save()
    }

    pub fn get_wpm(&self) -> usize {
        self.config.keyer.wpm
    }

    pub fn set_sidetone_frequency(&mut self, new_freq: u16) -> Result<(), String> {
        self.config.keyer.sidetone_frequency = new_freq;
        self.save()
    }

    pub fn get_sidetone_frequency(&self) -> u16 {
        self.config.keyer.sidetone_frequency
    }

    pub fn set_audio_out_device(&mut self, new_device: String) -> Result<(), String> {
        self.config.audio_devices.audio_out_device = new_device;
        self.save()
    }

    pub fn get_audio_out_device(&self) -> String {
        self.config.audio_devices.audio_out_device.to_owned()
    }

    pub fn set_rig_out_device(&mut self, new_device: String) -> Result<(), String> {
        self.config.audio_devices.rig_out_device = new_device;
        self.save()
    }

    pub fn get_rig_out_device(&self) -> String {
        self.config.audio_devices.rig_out_device.to_owned()
    }

    pub fn set_rig_in_device(&mut self, new_device: String) -> Result<(), String> {
        self.config.audio_devices.rig_in_device = new_device;
        self.save()
    }

    pub fn get_rig_in_device(&self) -> String {
        self.config.audio_devices.rig_in_device.to_owned()
    }

    pub fn set_transmit_offset_frequency(&mut self, new_freq: u16) -> Result<(), String> {
        self.config.transceiver.transmit_offset_frequency = new_freq;
        self.save()
    }

    pub fn get_transmit_offset_frequency(&self) -> u16 {
        self.config.transceiver.transmit_offset_frequency
    }

    pub fn set_transmit_amplitude(&mut self, new_amplitude: f32) -> Result<(), String> {
        if new_amplitude < 0.0 || new_amplitude > 1.0 {
            panic!("Cannot store transmitter amplitude outside range [0.0, 1.0]");
        }
        self.config.transceiver.transmit_amplitude = new_amplitude;
        self.save()
    }

    pub fn get_transmit_amplitude(&self) -> f32 {
        self.config.transceiver.transmit_amplitude
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
