use crate::error::{Error, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct State {
    pub last_update: u64,
    pub region_hash: String,
}

pub fn path(dir: &Path) -> PathBuf {
    dir.join("state.toml")
}

pub fn load(dir: &Path) -> Result<Option<State>> {
    let path = path(dir);
    if !path.exists() {
        return Ok(None);
    }
    let text = fs::read_to_string(&path)
        .map_err(|err| Error::io("read", path.display().to_string(), err))?;
    Ok(Some(toml::from_str(&text).map_err(|err| {
        Error::toml_decode(path.display().to_string(), err)
    })?))
}

pub fn save(dir: &Path, state: &State) -> Result<()> {
    fs::create_dir_all(dir).map_err(|err| Error::io("create", dir.display().to_string(), err))?;
    let path = path(dir);
    let text = toml::to_string(state)
        .map_err(|err| Error::toml_encode(path.display().to_string(), err))?;
    fs::write(&path, text).map_err(|err| Error::io("write", path.display().to_string(), err))
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
