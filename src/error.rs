use std::error::Error as StdError;
use std::fmt;
use std::io;
use std::path::PathBuf;

/// Every fallible path in `blk` returns this.
pub type Result<T, E = Error> = std::result::Result<T, E>;

/// Why a command stopped. Each failure mode is its own variant so callers can
/// match on it and every message lives in one place.
#[derive(Debug)]
pub enum Error {
    /// `update`, `apply`, or a source change ran with an empty source list.
    NoSourcesConfigured,
    /// `rm` was given a source that is not in the config.
    SourceNotConfigured { source: String },
    /// A source id is not in the catalog and is not a URL or an existing file.
    UnknownSource { source: String, known: Vec<String> },
    /// A list name was not found.
    ListNotFound { name: String },
    /// A custom list already uses this name.
    ListExists { name: String },
    /// A preset name cannot be reused, edited, or deleted.
    ListReserved { name: String },
    /// A list needs at least one source.
    ListEmpty,
    /// A list needs a name.
    EmptyListName,
    /// Another blk process holds the state lock.
    Busy,
    /// The interactive menu needs root to change the hosts file.
    MenuNeedsRoot,
    /// The config changed while sources were fetched.
    ConfigChanged,
    /// A pause started while sources were fetched.
    PauseChanged,
    /// A pause is already running.
    AlreadyPaused(crate::pause::Pause),
    /// `pause` found no block region in the hosts file.
    NoBlockRegion { hosts: PathBuf },
    /// The saved region for a pause is gone.
    MissingSnapshot { hosts: PathBuf },
    /// Writing a system file would need root.
    RootRequired { hosts: PathBuf },
    /// The atomic write was denied by the filesystem.
    PermissionDenied { path: PathBuf },
    /// `doctor` finished with at least one problem.
    DoctorProblems { count: usize },
    /// `$EDITOR` exited non-zero.
    EditorFailed,
    /// `--offline` needs a cached copy that does not exist.
    NoCachedCopy { url: String },
    /// The server answered 304 but the cache body is gone.
    EmptyCacheResponse { url: String },
    /// A source parsed to zero usable entries.
    EmptySource { source: String },
    /// A source parsed but produced no hostnames.
    NoDomains { url: String },
    /// A file operation failed. `target` is a path or URL.
    Io {
        action: &'static str,
        target: String,
        source: io::Error,
    },
    /// A TOML file could not be decoded or encoded.
    Toml {
        action: &'static str,
        target: String,
        source: Box<TomlError>,
    },
    /// A network request failed.
    Fetch {
        url: String,
        source: Box<ureq::Error>,
    },
    /// The hosts region is malformed.
    Region(crate::region::RegionError),
    /// A terminal prompt failed or was cancelled.
    Dialog(Box<dialoguer::Error>),
}

impl Error {
    pub fn io(action: &'static str, target: impl Into<String>, source: io::Error) -> Self {
        Error::Io {
            action,
            target: target.into(),
            source,
        }
    }

    pub fn toml_decode(target: impl Into<String>, source: toml::de::Error) -> Self {
        Error::Toml {
            action: "read",
            target: target.into(),
            source: Box::new(TomlError::Decode(source)),
        }
    }

    pub fn toml_encode(target: impl Into<String>, source: toml::ser::Error) -> Self {
        Error::Toml {
            action: "write",
            target: target.into(),
            source: Box::new(TomlError::Encode(source)),
        }
    }

    pub fn fetch(url: impl Into<String>, source: ureq::Error) -> Self {
        Error::Fetch {
            url: url.into(),
            source: Box::new(source),
        }
    }

    pub fn unknown_source(source: impl Into<String>, known: Vec<String>) -> Self {
        Error::UnknownSource {
            source: source.into(),
            known,
        }
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::NoSourcesConfigured => {
                f.write_str("no sources configured; run `blk sources` or `blk add`")
            }
            Error::SourceNotConfigured { source } => {
                write!(f, "{source} is not configured")
            }
            Error::UnknownSource { source, known } => {
                write!(
                    f,
                    "unknown source {source}; known sources: {}",
                    known.join(", ")
                )
            }
            Error::ListNotFound { name } => write!(f, "no list named {name}"),
            Error::ListExists { name } => write!(f, "a list named {name} already exists"),
            Error::ListReserved { name } => {
                write!(f, "{name} is a preset; preset names are reserved")
            }
            Error::ListEmpty => f.write_str("a list needs at least one source"),
            Error::EmptyListName => f.write_str("a list needs a name"),
            Error::Busy => f.write_str("another blk process is running; try again"),
            Error::MenuNeedsRoot => f.write_str("the menu needs root; run `sudo blk`"),
            Error::ConfigChanged => {
                f.write_str("the config changed while sources were fetched; run the command again")
            }
            Error::PauseChanged => {
                f.write_str("a pause started while sources were fetched; run the command again")
            }
            Error::AlreadyPaused(record) => match record.until {
                Some(until) => write!(
                    f,
                    "already paused until {} ({} left); run blk resume first",
                    crate::timefmt::local_hhmm(until),
                    crate::timefmt::human_duration(until.saturating_sub(crate::pause::now()))
                ),
                None => f.write_str("already paused indefinitely; run blk resume first"),
            },
            Error::NoBlockRegion { hosts } => {
                write!(f, "{} has no block region to pause", hosts.display())
            }
            Error::MissingSnapshot { hosts } => write!(
                f,
                "the saved region for {} is missing; run `blk update` to rebuild it",
                hosts.display()
            ),
            Error::RootRequired { hosts } => {
                write!(
                    f,
                    "writing {} requires root; run with sudo",
                    hosts.display()
                )
            }
            Error::PermissionDenied { path } => {
                write!(
                    f,
                    "permission denied writing {}; run with sudo",
                    path.display()
                )
            }
            Error::DoctorProblems { count } => write!(
                f,
                "doctor found {count} problem{}",
                if *count == 1 { "" } else { "s" }
            ),
            Error::EditorFailed => f.write_str("editor exited with an error"),
            Error::NoCachedCopy { url } => {
                write!(f, "no cached copy of {url}; run an online update first")
            }
            Error::EmptyCacheResponse { url } => {
                write!(f, "{url} returned 304 but nothing is cached")
            }
            Error::EmptySource { source } => {
                write!(f, "{source} contains no usable entries")
            }
            Error::NoDomains { url } => write!(f, "{url} yielded no domains"),
            Error::Io {
                action,
                target,
                source,
            } => write!(f, "cannot {action} {target}: {source}"),
            Error::Toml {
                action,
                target,
                source,
            } => write!(f, "cannot {action} {target}: {source}"),
            Error::Fetch { url, source } => {
                // ureq repeats the URL at the start of its own message.
                let text = source.to_string();
                let prefix = format!("{url}: ");
                let detail = text.strip_prefix(&prefix).unwrap_or(&text);
                write!(f, "cannot fetch {url}: {detail}")
            }
            Error::Region(error) => error.fmt(f),
            Error::Dialog(error) => write!(f, "prompt failed: {error}"),
        }
    }
}

impl StdError for Error {
    fn source(&self) -> Option<&(dyn StdError + 'static)> {
        match self {
            Error::Io { source, .. } => Some(source),
            Error::Toml { source, .. } => Some(source.as_ref()),
            Error::Fetch { source, .. } => Some(source.as_ref()),
            Error::Region(error) => Some(error),
            Error::Dialog(error) => Some(error.as_ref()),
            _ => None,
        }
    }
}

/// TOML has separate decode and encode errors, so both are wrapped here.
#[derive(Debug)]
pub enum TomlError {
    Decode(toml::de::Error),
    Encode(toml::ser::Error),
}

impl fmt::Display for TomlError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TomlError::Decode(error) => error.fmt(f),
            TomlError::Encode(error) => error.fmt(f),
        }
    }
}

impl StdError for TomlError {
    fn source(&self) -> Option<&(dyn StdError + 'static)> {
        match self {
            TomlError::Decode(error) => Some(error),
            TomlError::Encode(error) => Some(error),
        }
    }
}

impl From<crate::region::RegionError> for Error {
    fn from(error: crate::region::RegionError) -> Self {
        Error::Region(error)
    }
}

impl From<dialoguer::Error> for Error {
    fn from(error: dialoguer::Error) -> Self {
        Error::Dialog(Box::new(error))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn messages_name_the_case_and_its_data() {
        assert_eq!(
            Error::NoSourcesConfigured.to_string(),
            "no sources configured; run `blk sources` or `blk add`"
        );
        assert_eq!(
            Error::SourceNotConfigured {
                source: "oisd/big".into()
            }
            .to_string(),
            "oisd/big is not configured"
        );
        assert_eq!(
            Error::unknown_source("nope", vec!["brave/default".into()]).to_string(),
            "unknown source nope; known sources: brave/default"
        );
        assert_eq!(
            Error::NoDomains {
                url: "https://example.test/x".into()
            }
            .to_string(),
            "https://example.test/x yielded no domains"
        );
    }

    #[test]
    fn io_errors_name_the_action_and_target() {
        let error = Error::io(
            "read",
            "/tmp/hosts",
            io::Error::new(io::ErrorKind::NotFound, "gone"),
        );
        assert_eq!(error.to_string(), "cannot read /tmp/hosts: gone");
        assert!(error.source().is_some());
    }

    #[test]
    fn region_errors_convert_and_keep_their_message() {
        let error: Error = crate::region::RegionError("bad marker".into()).into();
        assert_eq!(error.to_string(), "bad marker");
        assert!(error.source().is_some());
    }

    #[test]
    fn already_paused_renders_the_record() {
        let record = crate::pause::Pause {
            paused: vec!["oisd/big".into()],
            hosts_path: PathBuf::from("/etc/hosts"),
            started: 0,
            until: None,
        };
        assert_eq!(
            Error::AlreadyPaused(record).to_string(),
            "already paused indefinitely; run blk resume first"
        );
    }

    #[test]
    fn a_timed_pause_names_the_deadline_once() {
        let now = crate::pause::now();
        let record = crate::pause::Pause {
            paused: vec!["oisd/big".into()],
            hosts_path: PathBuf::from("/etc/hosts"),
            started: now,
            until: Some(now + 60),
        };
        let text = Error::AlreadyPaused(record).to_string();
        assert!(text.starts_with("already paused until "), "{text}");
        assert!(text.contains("left"), "{text}");
        assert!(!text.contains("(("), "{text}");
    }

    #[test]
    fn list_errors_name_the_name() {
        assert_eq!(
            Error::ListNotFound {
                name: "Focus".into()
            }
            .to_string(),
            "no list named Focus"
        );
        assert_eq!(
            Error::ListExists {
                name: "Focus".into()
            }
            .to_string(),
            "a list named Focus already exists"
        );
        assert_eq!(
            Error::ListReserved {
                name: "Social media".into()
            }
            .to_string(),
            "Social media is a preset; preset names are reserved"
        );
        assert_eq!(
            Error::ListEmpty.to_string(),
            "a list needs at least one source"
        );
        assert_eq!(Error::EmptyListName.to_string(), "a list needs a name");
        assert_eq!(
            Error::Busy.to_string(),
            "another blk process is running; try again"
        );
    }

    #[test]
    fn doctor_problems_agree_in_number() {
        assert_eq!(
            Error::DoctorProblems { count: 1 }.to_string(),
            "doctor found 1 problem"
        );
        assert_eq!(
            Error::DoctorProblems { count: 2 }.to_string(),
            "doctor found 2 problems"
        );
    }

    #[test]
    fn data_only_errors_have_no_source() {
        assert!(Error::EditorFailed.source().is_none());
        assert!(Error::DoctorProblems { count: 2 }.source().is_none());
    }
}
