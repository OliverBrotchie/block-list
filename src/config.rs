use crate::error::{Error, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct Config {
    #[serde(default)]
    pub sources: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub lists: Vec<List>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct List {
    pub name: String,
    #[serde(default)]
    pub sources: Vec<String>,
}

pub fn path(dir: &Path) -> PathBuf {
    dir.join("config.toml")
}

/// Keep first occurrences, preserving order.
pub fn dedupe(sources: Vec<String>) -> Vec<String> {
    let mut out = Vec::new();
    for source in sources {
        if !out.contains(&source) {
            out.push(source);
        }
    }
    out
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
    let _lock = crate::lock::Lock::acquire(dir)?;
    fs::create_dir_all(dir).map_err(|err| Error::io("create", dir.display().to_string(), err))?;
    let path = path(dir);
    let text = toml::to_string(config)
        .map_err(|err| Error::toml_encode(path.display().to_string(), err))?;
    fs::write(&path, text).map_err(|err| Error::io("write", path.display().to_string(), err))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_config_without_lists_loads() {
        let config: Config = toml::from_str("sources = [\"blk/social\"]\n").unwrap();
        assert_eq!(config.sources, vec!["blk/social"]);
        assert!(config.lists.is_empty());
    }

    #[test]
    fn lists_round_trip() {
        let config = Config {
            sources: vec!["blk/social".into()],
            lists: vec![List {
                name: "Focus".into(),
                sources: vec!["oisd/small".into()],
            }],
        };
        let text = toml::to_string(&config).unwrap();
        let back: Config = toml::from_str(&text).unwrap();
        assert_eq!(back.lists[0].name, "Focus");
        assert_eq!(back.lists[0].sources, vec!["oisd/small"]);
    }
}
