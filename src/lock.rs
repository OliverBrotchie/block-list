use std::fs::{self, File, OpenOptions};
use std::os::unix::io::AsRawFd;
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};

use crate::error::{Error, Result};

static HELD: AtomicBool = AtomicBool::new(false);

/// Serializes writers across processes. Nested acquisition in one process is a no-op.
pub struct Lock {
    file: Option<File>,
}

impl Lock {
    pub fn acquire(dir: &Path) -> Result<Lock> {
        if HELD.swap(true, Ordering::SeqCst) {
            return Ok(Lock { file: None });
        }
        let result = open(dir).and_then(|file| {
            let rc = unsafe { libc::flock(file.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB) };
            if rc == 0 {
                Ok(file)
            } else {
                Err(Error::Busy)
            }
        });
        match result {
            Ok(file) => Ok(Lock { file: Some(file) }),
            Err(error) => {
                HELD.store(false, Ordering::SeqCst);
                Err(error)
            }
        }
    }
}

fn open(dir: &Path) -> Result<File> {
    fs::create_dir_all(dir).map_err(|err| Error::io("create", dir.display().to_string(), err))?;
    let path = dir.join("lock");
    OpenOptions::new()
        .create(true)
        .truncate(false)
        .read(true)
        .write(true)
        .open(&path)
        .map_err(|err| Error::io("open", path.display().to_string(), err))
}

impl Drop for Lock {
    fn drop(&mut self) {
        if self.file.is_some() {
            HELD.store(false, Ordering::SeqCst);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn nested_acquisition_is_reentrant() {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir = std::env::temp_dir().join(format!("blk-lock-{unique}"));

        let first = Lock::acquire(&dir).unwrap();
        let second = Lock::acquire(&dir).unwrap();
        drop(second);
        let third = Lock::acquire(&dir).unwrap();
        drop(third);
        drop(first);

        let after = Lock::acquire(&dir).unwrap();
        drop(after);
        fs::remove_dir_all(dir).ok();
    }
}
