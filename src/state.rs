use crate::error::{Error, Result};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct State {
    pub last_update: u64,
    #[serde(default)]
    pub hosts_path: PathBuf,
    pub region_hash: String,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub counts: BTreeMap<String, usize>,
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
    let _lock = crate::lock::Lock::acquire(dir)?;
    fs::create_dir_all(dir).map_err(|err| Error::io("create", dir.display().to_string(), err))?;
    let path = path(dir);
    let text = toml::to_string(state)
        .map_err(|err| Error::toml_encode(path.display().to_string(), err))?;
    crate::region::write_atomic(&path, &text)
        .map_err(|err| Error::io("write", path.display().to_string(), err))
}

/// FNV-1a 64, shared by region drift detection and cache names.
pub(crate) fn fnv1a(bytes: &[u8]) -> u64 {
    let mut hash = 0xcbf2_9ce4_8422_2325u64;
    for byte in bytes {
        hash ^= *byte as u64;
        hash = hash.wrapping_mul(0x100_0000_01b3);
    }
    hash
}

/// FNV-1a 64 of the region bytes, for detecting edits made outside the tool.
pub fn hash_region(region: &str) -> String {
    format!("{:016x}", fnv1a(region.as_bytes()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_state_file_without_the_new_fields_loads() {
        let state: State = toml::from_str("last_update = 1\nregion_hash = \"abc\"\n").unwrap();
        assert_eq!(state.last_update, 1);
        assert!(state.hosts_path.as_os_str().is_empty());
        assert!(state.counts.is_empty());
    }

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
