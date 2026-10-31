use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct State {
    pub last_update: u64,
    pub hosts_path: String,
    pub region_hash: String,
    #[serde(default)]
    pub counts: BTreeMap<String, usize>,
}

pub fn path(dir: &Path) -> PathBuf {
    dir.join("state.toml")
}

pub fn load(dir: &Path) -> Result<Option<State>, Box<dyn std::error::Error>> {
    let path = path(dir);
    if !path.exists() {
        return Ok(None);
    }
    let text = fs::read_to_string(&path)?;
    Ok(Some(toml::from_str(&text)?))
}

pub fn save(dir: &Path, state: &State) -> Result<(), Box<dyn std::error::Error>> {
    fs::create_dir_all(dir)?;
    fs::write(path(dir), toml::to_string(state)?)?;
    Ok(())
}

/// FNV-1a 64 of the region bytes, for detecting edits made outside the tool.
pub fn hash_region(region: &str) -> String {
    let mut hash = 0xcbf2_9ce4_8422_2325u64;
    for byte in region.as_bytes() {
        hash ^= *byte as u64;
        hash = hash.wrapping_mul(0x100_0000_01b3);
    }
    format!("{hash:016x}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hash_is_stable_and_distinguishes_content() {
        assert_eq!(
            hash_region("# BEGIN blk\n# END blk\n"),
            hash_region("# BEGIN blk\n# END blk\n")
        );
        assert_ne!(hash_region("a"), hash_region("b"));
        assert_eq!(hash_region("").len(), 16);
    }
}
