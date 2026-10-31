use crate::error::{Error, Result};
use crate::hosts::{self, Hosts};
use crate::region::{self, Section};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

/// One pause. Written to `pause.toml`; the region it replaced sits beside it in `pause.region`.
#[derive(Debug, Serialize, Deserialize)]
pub struct Pause {
    pub paused: Vec<String>,
    pub hosts_path: PathBuf,
    pub started: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub until: Option<u64>,
}

impl Pause {
    pub fn is_expired(&self, now: u64) -> bool {
        self.until.is_some_and(|until| now >= until)
    }

    /// `until 14:30 (12m left)` or `indefinitely`.
    pub fn describe(&self) -> String {
        match self.until {
            Some(until) => format!(
                "until {} ({} left)",
                crate::timefmt::local_hhmm(until),
                crate::timefmt::human_duration(until.saturating_sub(now()))
            ),
            None => "indefinitely".to_string(),
        }
    }

    /// Plan a pause over the sections of `parsed`. No targets means every section.
    pub fn plan(
        hosts_path: &Path,
        parsed: &region::Parsed,
        targets: &[String],
        until: Option<u64>,
    ) -> Result<Pause> {
        let known: Vec<String> = parsed
            .sections
            .iter()
            .map(|section| section.name.clone())
            .collect();
        let paused = if targets.is_empty() {
            known.clone()
        } else {
            targets.to_vec()
        };
        for target in &paused {
            if !known.contains(target) {
                return Err(Error::UnknownSource {
                    source: target.clone(),
                    known,
                });
            }
        }
        Ok(Pause {
            paused,
            hosts_path: hosts_path.to_path_buf(),
            started: now(),
            until,
        })
    }
}

/// Save the record and snapshot, then empty the paused sections.
pub fn begin(state: &Path, hosts: &Hosts, record: &Pause) -> Result<()> {
    let parsed = hosts.parsed()?.ok_or_else(|| Error::NoBlockRegion {
        hosts: record.hosts_path.clone(),
    })?;
    let snapshot = hosts.region()?.unwrap_or_default();
    let kept: Vec<Section> = parsed
        .sections
        .iter()
        .filter(|section| !record.paused.contains(&section.name))
        .cloned()
        .collect();
    hosts::require_root(&record.hosts_path)?;
    save(state, record, snapshot)?;
    if let Err(err) = hosts.commit(state, &region::render(&kept)) {
        clear(state);
        return Err(err);
    }
    Ok(())
}

/// Restore the exact region saved at pause time.
pub fn resume(state: &Path, record: &Pause) -> Result<()> {
    let Some(snapshot) = load_snapshot(state) else {
        return Err(Error::MissingSnapshot {
            hosts: record.hosts_path.clone(),
        });
    };
    hosts::Hosts::read(&record.hosts_path)?.commit(state, &snapshot)?;
    clear(state);
    Ok(())
}

/// What a command found when it looked for an expired pause.
pub enum Expiry {
    None,
    Resumed,
    NeedsRoot,
}

/// Resume an expired pause. Without root, report it instead.
pub fn expire(state: &Path) -> Result<Expiry> {
    let Some(record) = load(state)? else {
        return Ok(Expiry::None);
    };
    if !record.is_expired(now()) {
        return Ok(Expiry::None);
    }
    if !hosts::can_write() {
        return Ok(Expiry::NeedsRoot);
    }
    resume(state, &record)?;
    Ok(Expiry::Resumed)
}

pub fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|elapsed| elapsed.as_secs())
        .unwrap_or(0)
}

/// Seconds for durations such as `30`, `30m`, `2h`, or `1h30m`.
/// A bare number means minutes.
pub fn parse_duration(text: &str) -> Result<u64, String> {
    let text = text.trim();
    if !text.is_empty() && text.chars().all(|c| c.is_ascii_digit()) {
        let minutes: u64 = text
            .parse()
            .map_err(|_| format!("invalid duration: {text}"))?;
        if minutes == 0 {
            return Err(format!("invalid duration: {text}"));
        }
        return minutes
            .checked_mul(60)
            .ok_or_else(|| format!("duration is too large: {text}"));
    }

    let mut total = 0u64;
    let mut number = String::new();
    let mut saw_unit = false;
    for c in text.chars() {
        if c.is_ascii_digit() {
            number.push(c);
            continue;
        }
        let value: u64 = number
            .parse()
            .map_err(|_| format!("invalid duration: {text}"))?;
        number.clear();
        let seconds = match c {
            's' => 1,
            'm' => 60,
            'h' => 3_600,
            'd' => 86_400,
            _ => return Err(format!("invalid duration unit '{c}' in {text}")),
        };
        total = value
            .checked_mul(seconds)
            .and_then(|part| total.checked_add(part))
            .ok_or_else(|| format!("duration is too large: {text}"))?;
        saw_unit = true;
    }
    if !number.is_empty() {
        return Err(format!("duration needs a unit: {text}"));
    }
    if !saw_unit || total == 0 {
        return Err(format!("invalid duration: {text}"));
    }
    Ok(total)
}

pub fn record_path(dir: &Path) -> PathBuf {
    dir.join("pause.toml")
}

pub fn snapshot_path(dir: &Path) -> PathBuf {
    dir.join("pause.region")
}

pub fn load(dir: &Path) -> Result<Option<Pause>> {
    let path = record_path(dir);
    if !path.exists() {
        return Ok(None);
    }
    let text = fs::read_to_string(&path)
        .map_err(|err| Error::io("read", path.display().to_string(), err))?;
    Ok(Some(toml::from_str(&text).map_err(|err| {
        Error::toml_decode(path.display().to_string(), err)
    })?))
}

pub fn load_snapshot(dir: &Path) -> Option<String> {
    fs::read_to_string(snapshot_path(dir)).ok()
}

pub fn save(dir: &Path, pause: &Pause, region: &str) -> Result<()> {
    fs::create_dir_all(dir).map_err(|err| Error::io("create", dir.display().to_string(), err))?;
    let path = record_path(dir);
    let text = toml::to_string(pause)
        .map_err(|err| Error::toml_encode(path.display().to_string(), err))?;
    fs::write(&path, text).map_err(|err| Error::io("write", path.display().to_string(), err))?;
    let snapshot = snapshot_path(dir);
    fs::write(&snapshot, region)
        .map_err(|err| Error::io("write", snapshot.display().to_string(), err))?;
    Ok(())
}

pub fn clear(dir: &Path) {
    let _ = fs::remove_file(record_path(dir));
    let _ = fs::remove_file(snapshot_path(dir));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plan_selects_all_or_named_sections() {
        let parsed = region::parse(&region::render(&[
            region::Section {
                name: "a".into(),
                entries: Vec::new(),
            },
            region::Section {
                name: "b".into(),
                entries: Vec::new(),
            },
        ]))
        .unwrap()
        .unwrap();
        let host = Path::new("/etc/hosts");

        let all = Pause::plan(host, &parsed, &[], Some(60)).unwrap();
        assert_eq!(all.paused, vec!["a", "b"]);
        assert!(all.until.is_some());

        let one = Pause::plan(host, &parsed, &["b".to_string()], None).unwrap();
        assert_eq!(one.paused, vec!["b"]);
        assert_eq!(one.until, None);

        let missing = Pause::plan(host, &parsed, &["nope".to_string()], None);
        assert!(matches!(missing, Err(Error::UnknownSource { .. })));
    }

    #[test]
    fn parses_durations() {
        assert_eq!(parse_duration("30m").unwrap(), 1_800);
        assert_eq!(parse_duration("2h").unwrap(), 7_200);
        assert_eq!(parse_duration("1h30m").unwrap(), 5_400);
        assert_eq!(parse_duration("45s").unwrap(), 45);
        assert_eq!(parse_duration("30").unwrap(), 1_800);
        assert_eq!(parse_duration(" 45 ").unwrap(), 2_700);
        assert!(parse_duration("0").is_err());
        assert!(parse_duration("30x").is_err());
        assert!(parse_duration("").is_err());
        assert!(parse_duration("18446744073709551615m").is_err());
    }

    #[test]
    fn expiry_uses_the_deadline() {
        let pause = Pause {
            paused: vec!["a".into()],
            hosts_path: PathBuf::from("/etc/hosts"),
            started: 100,
            until: Some(200),
        };
        assert!(!pause.is_expired(199));
        assert!(pause.is_expired(200));
        assert!(pause.is_expired(201));
    }

    #[test]
    fn indefinite_pause_never_expires() {
        let pause = Pause {
            paused: vec!["a".into()],
            hosts_path: PathBuf::from("/etc/hosts"),
            started: 100,
            until: None,
        };
        assert!(!pause.is_expired(u64::MAX));
    }
}
