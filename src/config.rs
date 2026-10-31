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

pub fn load(dir: &Path) -> Result<Config, Box<dyn std::error::Error>> {
    let path = path(dir);
    if !path.exists() {
        return Ok(Config::default());
    }
    let text = fs::read_to_string(&path)?;
    Ok(toml::from_str(&text)?)
}

pub fn save(dir: &Path, config: &Config) -> Result<(), Box<dyn std::error::Error>> {
    fs::create_dir_all(dir)?;
    fs::write(path(dir), toml::to_string(config)?)?;
    Ok(())
}
