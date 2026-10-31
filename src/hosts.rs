use std::fs;
use std::path::{Path, PathBuf};

use crate::error::{Error, Result};
use crate::region::{self, Parsed};

/// A hosts file read once, with its managed region located.
pub struct Hosts {
    path: PathBuf,
    contents: String,
    region: std::result::Result<Option<Parsed>, region::RegionError>,
}

impl Hosts {
    /// Read and locate the region, rejecting a malformed one.
    pub fn read(path: &Path) -> Result<Self> {
        let hosts = Self::read_lenient(path)?;
        hosts.parsed()?;
        Ok(hosts)
    }

    /// Read without rejecting a malformed region. `parsed` reports the error.
    pub fn read_lenient(path: &Path) -> Result<Self> {
        let contents = fs::read_to_string(path)
            .map_err(|err| Error::io("read", path.display().to_string(), err))?;
        let region = region::parse(&contents);
        Ok(Self {
            path: path.to_path_buf(),
            contents,
            region,
        })
    }

    pub fn contents(&self) -> &str {
        &self.contents
    }

    pub fn parsed(&self) -> Result<Option<&Parsed>> {
        match &self.region {
            Ok(parsed) => Ok(parsed.as_ref()),
            Err(err) => Err(Error::Region(err.clone())),
        }
    }

    pub fn region(&self) -> Result<Option<&str>> {
        Ok(self.parsed()?.map(|parsed| parsed.region(&self.contents)))
    }

    pub fn outside(&self) -> Result<String> {
        Ok(match self.parsed()? {
            Some(parsed) => parsed.outside(&self.contents),
            None => self.contents.clone(),
        })
    }

    /// Replace the managed region, after backing up the file.
    pub fn commit(&self, state: &Path, rendered: &str) -> Result<()> {
        let next = region::replace(&self.contents, self.parsed()?, rendered);
        require_root(&self.path)?;
        backup(state, &self.path)?;
        write_atomic(&self.path, &next)
    }
}

/// Writing needs root. BLK_DIR marks a test run, where the target is a temp file.
pub fn can_write() -> bool {
    std::env::var_os("BLK_DIR").is_some() || unsafe { libc::geteuid() } == 0
}

/// Fail before touching the state directory when the write would be refused.
pub fn require_root(hosts: &Path) -> Result<()> {
    if can_write() {
        return Ok(());
    }
    Err(Error::RootRequired {
        hosts: hosts.to_path_buf(),
    })
}

/// Keep the first version of the hosts file this tool ever modifies.
fn backup(state: &Path, hosts: &Path) -> Result<()> {
    fs::create_dir_all(state)
        .map_err(|err| Error::io("create", state.display().to_string(), err))?;
    let backup = state.join("hosts.bak");
    if !backup.exists() {
        fs::copy(hosts, &backup)
            .map_err(|err| Error::io("write", backup.display().to_string(), err))?;
    }
    Ok(())
}

fn write_atomic(path: &Path, contents: &str) -> Result<()> {
    region::write_atomic(path, contents).map_err(|err| {
        if err.kind() == std::io::ErrorKind::PermissionDenied {
            Error::PermissionDenied {
                path: path.to_path_buf(),
            }
        } else {
            Error::io("write", path.display().to_string(), err)
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::region::{Entry, Section};
    use std::fs;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp_dir(tag: &str) -> PathBuf {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir = std::env::temp_dir().join(format!("blk-hosts-{tag}-{unique}"));
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn section(name: &str, host: &str) -> Section {
        Section {
            name: name.to_string(),
            entries: vec![Entry {
                ip: "0.0.0.0".parse().unwrap(),
                host: host.to_string(),
            }],
        }
    }

    static ENV: std::sync::Mutex<()> = std::sync::Mutex::new(());

    #[test]
    fn require_root_refuses_without_a_test_directory() {
        let _guard = ENV.lock().unwrap();
        // Root would not be refused, and CI may run as root.
        if unsafe { libc::geteuid() } == 0 {
            return;
        }
        let previous = std::env::var_os("BLK_DIR");
        std::env::remove_var("BLK_DIR");
        assert!(matches!(
            require_root(Path::new("/etc/hosts")),
            Err(Error::RootRequired { .. })
        ));
        if let Some(value) = previous {
            std::env::set_var("BLK_DIR", value);
        }
    }

    #[test]
    fn region_and_outside_split_at_the_markers() {
        let dir = temp_dir("slice");
        let path = dir.join("hosts");
        let rendered = region::render(&[section("a", "x.test")]);
        fs::write(&path, format!("head\n{rendered}tail\n")).unwrap();

        let hosts = Hosts::read(&path).unwrap();
        assert_eq!(hosts.region().unwrap(), Some(rendered.as_str()));
        assert_eq!(hosts.outside().unwrap(), "head\ntail\n");
        fs::remove_dir_all(dir).ok();
    }

    #[test]
    fn commit_replaces_the_region_and_keeps_the_first_backup() {
        let _guard = ENV.lock().unwrap();
        let dir = temp_dir("commit");
        let state = dir.join("state");
        let path = dir.join("hosts");
        let original = "127.0.0.1 localhost\n";
        fs::write(&path, original).unwrap();
        std::env::set_var("BLK_DIR", &state);

        let hosts = Hosts::read(&path).unwrap();
        hosts
            .commit(&state, &region::render(&[section("a", "x.test")]))
            .unwrap();
        let once = fs::read_to_string(&path).unwrap();
        assert!(once.starts_with(original));
        assert!(once.contains("0.0.0.0 x.test"));

        let hosts = Hosts::read(&path).unwrap();
        hosts
            .commit(&state, &region::render(&[section("a", "y.test")]))
            .unwrap();
        let twice = fs::read_to_string(&path).unwrap();
        assert!(!twice.contains("x.test"));
        assert!(twice.contains("0.0.0.0 y.test"));
        assert_eq!(
            fs::read_to_string(state.join("hosts.bak")).unwrap(),
            original
        );

        std::env::remove_var("BLK_DIR");
        fs::remove_dir_all(dir).ok();
    }
}
