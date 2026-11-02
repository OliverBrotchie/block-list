use std::collections::HashSet;
use std::fs;
use std::io::{self, Write};
use std::net::IpAddr;
use std::path::Path;

pub const BEGIN: &str = "# BEGIN blk";
pub const END: &str = "# END blk";
pub const LIST_PREFIX: &str = "# list:";

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Entry {
    pub ip: IpAddr,
    pub host: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Section {
    pub name: String,
    pub entries: Vec<Entry>,
}

#[derive(Debug)]
pub struct Parsed {
    /// Byte offset of the `# BEGIN blk` line.
    begin: usize,
    /// Byte offset just past the `# END blk` line.
    end: usize,
    pub sections: Vec<Section>,
}

impl Parsed {
    /// The managed region, markers included.
    pub fn region<'a>(&self, contents: &'a str) -> &'a str {
        &contents[self.begin..self.end]
    }

    /// The file with the managed region removed.
    pub fn outside(&self, contents: &str) -> String {
        format!("{}{}", &contents[..self.begin], &contents[self.end..])
    }
}

#[derive(Debug, Clone)]
pub struct RegionError(pub String);

impl std::fmt::Display for RegionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for RegionError {}

/// Locate and read the block region. Absence is `Ok(None)`; ambiguity is an error.
pub fn parse(contents: &str) -> Result<Option<Parsed>, RegionError> {
    let mut begin = None;
    let mut end = None;
    let mut offset = 0;
    for line in contents.split_inclusive('\n') {
        let start = offset;
        offset += line.len();
        let line = line.trim_end();
        if line == BEGIN {
            if begin.replace(start).is_some() {
                return Err(RegionError(format!("{BEGIN} appears more than once")));
            }
        } else if line == END && end.replace(offset).is_some() {
            return Err(RegionError(format!("{END} appears more than once")));
        }
    }

    let (begin, end) = match (begin, end) {
        (None, None) => return Ok(None),
        (None, Some(_)) => return Err(RegionError(format!("{END} without a matching {BEGIN}"))),
        (Some(_), None) => return Err(RegionError(format!("{BEGIN} without a matching {END}"))),
        (Some(begin), Some(end)) => (begin, end),
    };
    if begin >= end {
        return Err(RegionError(format!("{END} appears before {BEGIN}")));
    }

    let mut sections: Vec<Section> = Vec::new();
    for line in contents[begin..end].lines() {
        let line = line.trim_end();
        if line.is_empty() || line == BEGIN || line == END {
            continue;
        }
        if let Some(name) = line.strip_prefix(LIST_PREFIX) {
            let name = name.trim();
            if name.is_empty() {
                return Err(RegionError(format!("{LIST_PREFIX} without a source name")));
            }
            sections.push(Section {
                name: name.to_string(),
                entries: Vec::new(),
            });
            continue;
        }
        let Some(section) = sections.last_mut() else {
            return Err(RegionError(format!(
                "line inside the region has no {LIST_PREFIX} header: {line}"
            )));
        };
        match entries_of(line) {
            Some(entries) => section.entries.extend(entries),
            None => {
                return Err(RegionError(format!(
                    "invalid hosts entry inside the region: {line}"
                )))
            }
        }
    }

    Ok(Some(Parsed {
        begin,
        end,
        sections,
    }))
}

/// Render a fresh region. One `# list:` section per source, in the given order.
pub fn render(sections: &[Section]) -> String {
    let mut out = String::new();
    out.push_str(BEGIN);
    out.push('\n');
    for section in sections {
        out.push_str(LIST_PREFIX);
        out.push(' ');
        out.push_str(&section.name);
        out.push('\n');
        for entry in &section.entries {
            out.push_str(&entry.ip.to_string());
            out.push(' ');
            out.push_str(&entry.host);
            out.push('\n');
        }
    }
    out.push_str(END);
    out.push('\n');
    out
}

/// Put `region` where the old one was, or append it after the existing content.
pub fn replace(contents: &str, parsed: Option<&Parsed>, region: &str) -> String {
    match parsed {
        Some(parsed) => format!(
            "{}{}{}",
            &contents[..parsed.begin],
            region,
            &contents[parsed.end..]
        ),
        None => {
            let base = contents.trim_end_matches('\n');
            if base.is_empty() {
                region.to_string()
            } else {
                format!("{base}\n\n{region}")
            }
        }
    }
}

/// Read a list file into entries, deduped in first-seen order.
pub fn entries_from_hosts(text: &str) -> (Vec<Entry>, Vec<String>) {
    let mut entries = Vec::new();
    let mut warnings = Vec::new();
    let mut seen = HashSet::new();
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') || line.starts_with('!') {
            continue;
        }
        match entries_of(line) {
            Some(parsed) => {
                for entry in parsed {
                    if seen.insert((entry.ip, entry.host.clone())) {
                        entries.push(entry);
                    }
                }
            }
            None => warnings.push(format!("skipped a non-hosts line: {line}")),
        }
    }
    (entries, warnings)
}

pub(crate) fn entries_of(line: &str) -> Option<Vec<Entry>> {
    let line = line.split('#').next().unwrap_or_default().trim();
    let mut parts = line.split_whitespace();
    let ip: IpAddr = parts.next()?.parse().ok()?;
    let mut entries = Vec::new();
    for host in parts {
        if !valid_host(host) {
            return None;
        }
        entries.push(Entry {
            ip,
            host: host.to_string(),
        });
    }
    if entries.is_empty() {
        None
    } else {
        Some(entries)
    }
}

pub(crate) fn valid_host(host: &str) -> bool {
    !host.is_empty()
        && !host.starts_with('.')
        && !host.ends_with('.')
        && host
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '.' || c == '-' || c == '_')
}

/// Write through a temp file in the same directory, fsync, rename, keep the mode.
/// A symlink is followed, so the target is replaced and the link survives.
pub fn write_atomic(path: &Path, contents: &str) -> io::Result<()> {
    let resolved = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
    let path = resolved.as_path();
    let dir = path.parent().unwrap_or_else(|| Path::new("."));
    let tmp = dir.join(format!(".blk.tmp.{}", std::process::id()));
    let _ = fs::remove_file(&tmp);
    let mode = fs::metadata(path).ok().map(|meta| meta.permissions());

    {
        let mut file = fs::File::create(&tmp)?;
        file.write_all(contents.as_bytes())?;
        file.sync_all()?;
    }
    if let Some(mode) = mode {
        fs::set_permissions(&tmp, mode)?;
    }
    fs::rename(&tmp, path)?;

    if let Ok(dir) = fs::File::open(dir) {
        let _ = dir.sync_all();
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(ip: &str, host: &str) -> Entry {
        Entry {
            ip: ip.parse().unwrap(),
            host: host.to_string(),
        }
    }

    #[test]
    fn no_region_is_none() {
        assert!(parse("127.0.0.1 localhost\n").unwrap().is_none());
    }

    #[test]
    fn unbalanced_markers_are_errors() {
        assert!(parse("# BEGIN blk\n0.0.0.0 a\n").is_err());
        assert!(parse("# END blk\n").is_err());
    }

    #[test]
    fn duplicate_markers_are_errors() {
        assert!(parse("# BEGIN blk\n# BEGIN blk\n# END blk\n").is_err());
        assert!(parse("# BEGIN blk\n# END blk\n# END blk\n").is_err());
    }

    #[test]
    fn reads_sections() {
        let text =
            "# BEGIN blk\n# list: a\n0.0.0.0 x.test\n# list: b\n0.0.0.0 y.test z.test\n# END blk\n";
        let parsed = parse(text).unwrap().unwrap();
        assert_eq!(parsed.sections.len(), 2);
        assert_eq!(parsed.sections[0].name, "a");
        assert_eq!(parsed.sections[1].entries.len(), 2);
    }

    #[test]
    fn entry_outside_section_is_an_error() {
        assert!(parse("# BEGIN blk\n0.0.0.0 x.test\n# END blk\n").is_err());
    }

    #[test]
    fn render_parses_back() {
        let sections = vec![
            Section {
                name: "a".to_string(),
                entries: vec![entry("0.0.0.0", "x.test")],
            },
            Section {
                name: "b".to_string(),
                entries: vec![entry("0.0.0.0", "y.test")],
            },
        ];
        let text = render(&sections);
        let parsed = parse(&text).unwrap().unwrap();
        assert_eq!(parsed.sections, sections);
    }

    #[test]
    fn replace_appends_once_and_is_idempotent() {
        let region = render(&[Section {
            name: "a".to_string(),
            entries: vec![entry("0.0.0.0", "x.test")],
        }]);
        let first = replace("127.0.0.1 localhost\n", None, &region);
        assert!(first.starts_with("127.0.0.1 localhost\n\n# BEGIN blk\n"));
        let parsed = parse(&first).unwrap();
        let second = replace(&first, parsed.as_ref(), &region);
        assert_eq!(first, second);
    }

    #[test]
    fn replace_keeps_content_after_the_region() {
        let region = render(&[Section {
            name: "a".to_string(),
            entries: vec![entry("0.0.0.0", "x.test")],
        }]);
        let text = format!("head\n{region}tail\n");
        let parsed = parse(&text).unwrap();
        let updated = replace(&text, parsed.as_ref(), &region);
        assert!(updated.ends_with("tail\n"));
        assert_eq!(updated, text);
    }

    #[test]
    #[cfg(unix)]
    fn write_atomic_follows_a_symlink() {
        use std::os::unix::fs::symlink;
        use std::time::{SystemTime, UNIX_EPOCH};

        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir = std::env::temp_dir().join(format!("blk-region-{unique}"));
        fs::create_dir_all(&dir).unwrap();
        let target = dir.join("target");
        let link = dir.join("link");
        fs::write(&target, "old").unwrap();
        symlink(&target, &link).unwrap();

        write_atomic(&link, "new").unwrap();

        assert_eq!(fs::read_to_string(&target).unwrap(), "new");
        assert!(fs::symlink_metadata(&link)
            .unwrap()
            .file_type()
            .is_symlink());
        fs::remove_dir_all(dir).ok();
    }

    #[test]
    fn entries_skip_invalid_lines_and_dedupe() {
        let text = "0.0.0.0 a.test b.test\n0.0.0.0 a.test\nnot an entry\n# comment\n0.0.0.0\n0.0.0.0 c.test # trailing note\n";
        let (entries, warnings) = entries_from_hosts(text);
        assert_eq!(entries.len(), 3);
        assert!(entries.iter().any(|entry| entry.host == "c.test"));
        assert_eq!(warnings.len(), 2);
    }
}
