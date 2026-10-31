use crate::error::{Error, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct Config {
    #[serde(default)]
    pub sources: Vec<String>,
}

pub fn path(dir: &Path) -> PathBuf {
    dir.join("config.toml")
}

pub fn load(dir: &Path) -> Result<Config> {
    let path = path(dir);
    if !path.exists() {
        return Ok(Config::default());
    }
    let text = fs::read_to_string(&path)
        .map_err(|err| Error::io("read", path.display().to_string(), err))?;
    toml::from_str(&text).map_err(|err| Error::toml_decode(path.display().to_string(), err))
}

pub fn save(dir: &Path, config: &Config) -> Result<()> {
    fs::create_dir_all(dir).map_err(|err| Error::io("create", dir.display().to_string(), err))?;
    let path = path(dir);
    let text = toml::to_string(config)
        .map_err(|err| Error::toml_encode(path.display().to_string(), err))?;
    fs::write(&path, text).map_err(|err| Error::io("write", path.display().to_string(), err))
}
