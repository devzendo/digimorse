use log::debug;
use std::path::{PathBuf, Path};
use std::fs;

#[cfg(target_os = "linux")]
const CONFIG_SUB_PATH: &str = ".digimorse";
#[cfg(target_os = "macos")]
const CONFIG_SUB_PATH: &str = "Library/ApplicationData/digimorse";
#[cfg(windows)]
const CONFIG_SUB_PATH: &str = "AppData\\Roaming\\digimorse";


pub fn configuration_directory(home_dir: Option<PathBuf>) -> Result<Box<Path>, String> {
    debug!("Home dir is {:?}", home_dir);
    let common_msg: &str = "cannot load/store configuration information.";
    match home_dir {
        None => {
            Err(format!("No home directory found, {}", common_msg))
        }
        Some(home) => {
            if !home.exists() {
                return Err(format!("Home directory {:?} does not exist, {}", home, common_msg))
            }
            let mut config_path = PathBuf::new();
            config_path.push(home);
            config_path.push(CONFIG_SUB_PATH);
            debug!("Config dir is {:?}", config_path);

            if !config_path.exists() {
                debug!("Creating config dir {:?}", config_path);
                match fs::create_dir_all(config_path.as_path()) {
                    Ok(_) => {
                        return Ok(config_path.into_boxed_path());
                    },
                    Err(err) => return Err(format!("Configuration directory '{:?}' could not be created: {:?}", config_path, err))
                }
            } else {
                if !config_path.is_dir() {
                    return Err(format!("Configuration directory '{:?}' is not a directory!", config_path))
                }
                return Ok(config_path.into_boxed_path())
            }
        }
    }
}

#[cfg(test)]
#[path = "./config_dir_spec.rs"]
mod config_dir_spec;
