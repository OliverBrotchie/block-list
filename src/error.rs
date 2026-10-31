use std::error::Error as StdError;
use std::fmt;
use std::io;

/// Every fallible path in `blk` returns this.
pub type Result<T, E = Error> = std::result::Result<T, E>;

/// Why a command stopped.
#[derive(Debug)]
pub enum Error {
    /// A failure described for the user, with no underlying cause.
    Message(String),
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
    pub fn msg(message: impl Into<String>) -> Self {
        Error::Message(message.into())
    }

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
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Message(message) => f.write_str(message),
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
            Error::Fetch { url, source } => write!(f, "cannot fetch {url}: {source}"),
            Error::Region(error) => error.fmt(f),
            Error::Dialog(error) => write!(f, "prompt failed: {error}"),
        }
    }
}

impl StdError for Error {
    fn source(&self) -> Option<&(dyn StdError + 'static)> {
        match self {
            Error::Message(_) => None,
            Error::Io { source, .. } => Some(source),
            Error::Toml { source, .. } => Some(source.as_ref()),
            Error::Fetch { source, .. } => Some(source.as_ref()),
            Error::Region(error) => Some(error),
            Error::Dialog(error) => Some(error.as_ref()),
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
    fn message_errors_display_unchanged() {
        assert_eq!(Error::msg("boom").to_string(), "boom");
        assert!(Error::msg("boom").source().is_none());
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
}
