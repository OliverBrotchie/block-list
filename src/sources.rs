use std::collections::HashSet;
use std::fs;
use std::net::{IpAddr, Ipv4Addr};
use std::path::{Path, PathBuf};
use std::time::Duration;

use crate::catalog::{self, Format};
use crate::region::{self, Entry};

#[derive(Debug)]
pub struct Loaded {
    pub name: String,
    pub entries: Vec<Entry>,
    /// Only ABP sources carry conversion counts.
    pub counts: Option<Counts>,
    pub warnings: Vec<String>,
    /// Whether any part of the source came from the network in this run.
    /// Read only by the conditional-GET test; the CLI no longer branches on it.
    #[allow(dead_code)]
    pub fetched: bool,
}

#[derive(Debug, Default, Clone, Copy)]
pub struct Counts {
    pub kept: usize,
    pub exceptions: usize,
    pub cosmetic: usize,
    pub other: usize,
}

pub fn load_all(
    dir: &Path,
    sources: &[String],
    offline: bool,
    write_cache: bool,
) -> Result<Vec<Loaded>, String> {
    let mut used = HashSet::new();
    let mut loaded = Vec::new();
    for source in sources {
        loaded.push(load(dir, source, offline, write_cache, &mut used)?);
    }
    Ok(loaded)
}

fn load(
    dir: &Path,
    source: &str,
    offline: bool,
    write_cache: bool,
    used: &mut HashSet<String>,
) -> Result<Loaded, String> {
    if let Some(entry) = catalog::get(source) {
        return load_catalog(dir, source, entry, offline, write_cache);
    }

    let name = unique_name(sanitize(source), source, used);
    let (text, fetched) = if is_url(source) {
        let path = cache_root(dir).join(&name);
        fetch_or_cache(source, &path, offline, write_cache)?
    } else {
        let text =
            fs::read_to_string(source).map_err(|err| format!("cannot read {source}: {err}"))?;
        (text, false)
    };
    let (entries, warnings) = region::entries_from_hosts(&text);
    if entries.is_empty() {
        return Err(format!("{source} contains no usable entries"));
    }
    Ok(Loaded {
        name,
        entries,
        counts: None,
        warnings,
        fetched,
    })
}

fn load_catalog(
    dir: &Path,
    id: &str,
    entry: &catalog::Source,
    offline: bool,
    write_cache: bool,
) -> Result<Loaded, String> {
    let root = cache_path(dir, id);
    let mut entries: Vec<Entry> = Vec::new();
    let mut seen = HashSet::new();
    let mut counts = Counts::default();
    let mut abp = false;
    let mut fetched = false;
    let mut warnings = Vec::new();

    for (index, url) in entry.urls.iter().enumerate() {
        let path = if entry.urls.len() == 1 {
            root.clone()
        } else {
            root.join(format!("{index:02}-{}", sanitize(url)))
        };
        let (text, downloaded) = fetch_or_cache(url, &path, offline, write_cache)?;
        fetched |= downloaded;

        match entry.format {
            Format::Hosts => {
                let (parsed, _) = region::entries_from_hosts(&text);
                if parsed.is_empty() {
                    if entry.urls.len() > 1 {
                        warnings.push(format!("{url} contains no usable entries"));
                        continue;
                    }
                    return Err(format!("{url} contains no usable entries"));
                }
                merge(&mut entries, &mut seen, parsed);
            }
            _ => {
                let parsed = catalog::parse(&text, entry.format);
                if parsed.domains.is_empty() {
                    if entry.urls.len() > 1 {
                        warnings.push(format!("{url} yielded no domains"));
                        continue;
                    }
                    return Err(format!("{url} yielded no domains"));
                }
                if entry.format == Format::Abp {
                    abp = true;
                    counts.kept += parsed.kept;
                    counts.exceptions += parsed.exceptions;
                    counts.cosmetic += parsed.cosmetic;
                    counts.other += parsed.other;
                }
                for host in parsed.domains {
                    let entry = Entry {
                        ip: IpAddr::V4(Ipv4Addr::UNSPECIFIED),
                        host,
                    };
                    if seen.insert((entry.ip, entry.host.clone())) {
                        entries.push(entry);
                    }
                }
            }
        }
    }

    Ok(Loaded {
        name: id.to_string(),
        entries,
        counts: if abp { Some(counts) } else { None },
        warnings,
        fetched,
    })
}

fn merge(entries: &mut Vec<Entry>, seen: &mut HashSet<(IpAddr, String)>, parsed: Vec<Entry>) {
    for entry in parsed {
        if seen.insert((entry.ip, entry.host.clone())) {
            entries.push(entry);
        }
    }
}

fn fetch_or_cache(
    url: &str,
    path: &Path,
    offline: bool,
    write_cache: bool,
) -> Result<(String, bool), String> {
    if offline {
        let body = fs::read_to_string(path)
            .map_err(|_| format!("no cached copy of {url}; run an online update first"))?;
        return Ok((body, false));
    }

    let etag_path = sidecar(path, "etag");
    let mut request = ureq::get(url).timeout(Duration::from_secs(60));
    if let Ok(etag) = fs::read_to_string(&etag_path) {
        request = request.set("If-None-Match", etag.trim());
    }

    match request.call() {
        Ok(response) if response.status() == 304 => {
            let body = fs::read_to_string(path)
                .map_err(|_| format!("{url} returned 304 but nothing is cached"))?;
            Ok((body, false))
        }
        Ok(response) => {
            let etag = response.header("etag").map(str::to_string);
            let body = response
                .into_string()
                .map_err(|error| format!("cannot read {url}: {error}"))?;
            if write_cache {
                if let Some(parent) = path.parent() {
                    fs::create_dir_all(parent)
                        .map_err(|error| format!("cannot create {}: {error}", parent.display()))?;
                }
                fs::write(path, &body).map_err(|error| format!("cannot cache {url}: {error}"))?;
                let _ = fs::write(sidecar(path, "src"), url);
                match etag {
                    Some(etag) => {
                        let _ = fs::write(&etag_path, etag);
                    }
                    None => {
                        let _ = fs::remove_file(&etag_path);
                    }
                }
            }
            Ok((body, true))
        }
        Err(ureq::Error::Status(304, _)) => {
            let body = fs::read_to_string(path)
                .map_err(|_| format!("{url} returned 304 but nothing is cached"))?;
            Ok((body, false))
        }
        Err(error) => Err(format!("cannot fetch {url}: {error}")),
    }
}

fn sidecar(path: &Path, suffix: &str) -> PathBuf {
    let mut name = path.as_os_str().to_os_string();
    name.push(format!(".{suffix}"));
    PathBuf::from(name)
}

fn cache_root(dir: &Path) -> PathBuf {
    dir.join("cache")
}

/// Where a source's cache entry lives. Catalog ids replace `/` with `-`.
pub fn cache_path(dir: &Path, source: &str) -> PathBuf {
    let slug = if catalog::get(source).is_some() {
        source.replace('/', "-")
    } else {
        sanitize(source)
    };
    cache_root(dir).join(slug)
}

fn is_url(source: &str) -> bool {
    source.starts_with("http://") || source.starts_with("https://")
}

/// A section name and cache name from a URL or path: the last segment, reduced to safe characters.
fn sanitize(source: &str) -> String {
    let base = source
        .trim_end_matches('/')
        .rsplit('/')
        .next()
        .unwrap_or(source);
    let base = base.split(['?', '#']).next().unwrap_or(base);
    let cleaned: String = base
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '.' || c == '-' || c == '_' {
                c
            } else {
                '-'
            }
        })
        .collect();
    let trimmed = cleaned.trim_matches('-');
    if trimmed.is_empty() {
        "source".to_string()
    } else {
        trimmed.to_string()
    }
}

fn unique_name(base: String, source: &str, used: &mut HashSet<String>) -> String {
    if used.insert(base.clone()) {
        return base;
    }
    let name = format!("{base}-{:08x}", fnv1a(source.as_bytes()) as u32);
    used.insert(name.clone());
    name
}

fn fnv1a(bytes: &[u8]) -> u64 {
    let mut hash = 0xcbf2_9ce4_8422_2325u64;
    for byte in bytes {
        hash ^= *byte as u64;
        hash = hash.wrapping_mul(0x100_0000_01b3);
    }
    hash
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp_dir(tag: &str) -> PathBuf {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir = std::env::temp_dir().join(format!("blk-src-{tag}-{unique}"));
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn sanitize_keeps_the_last_segment() {
        assert_eq!(
            sanitize("https://example.test/lists/ads.hosts?x=1"),
            "ads.hosts"
        );
        assert_eq!(sanitize("/tmp/my list.hosts"), "my-list.hosts");
        assert_eq!(sanitize("https://example.test/"), "example.test");
    }

    #[test]
    fn colliding_names_get_a_suffix() {
        let mut used = HashSet::new();
        let first = unique_name(sanitize("/a/list.hosts"), "/a/list.hosts", &mut used);
        let second = unique_name(sanitize("/b/list.hosts"), "/b/list.hosts", &mut used);
        assert_eq!(first, "list.hosts");
        assert!(second.starts_with("list.hosts-"));
        assert_ne!(first, second);
    }

    #[test]
    fn offline_reads_the_cache() {
        let dir = temp_dir("offline");
        fs::create_dir_all(dir.join("cache")).unwrap();
        fs::write(dir.join("cache/x.hosts"), "0.0.0.0 cached.test\n").unwrap();

        let loaded = load_all(
            &dir,
            &["https://example.test/x.hosts".to_string()],
            true,
            false,
        )
        .unwrap();
        assert_eq!(loaded[0].name, "x.hosts");
        assert_eq!(loaded[0].entries.len(), 1);
        assert!(!loaded[0].fetched);
        fs::remove_dir_all(dir).ok();
    }

    #[test]
    fn offline_without_a_cache_errors() {
        let dir = temp_dir("offline-missing");
        let error = load_all(
            &dir,
            &["https://example.test/missing.hosts".to_string()],
            true,
            false,
        )
        .unwrap_err();
        assert!(error.contains("no cached copy"), "{error}");
        fs::remove_dir_all(dir).ok();
    }

    #[test]
    #[ignore = "downloads from raw.githubusercontent.com"]
    fn conditional_get_reuses_the_cache() {
        let dir = temp_dir("online");
        let source = vec!["stevenblack/hosts".to_string()];
        let first = load_all(&dir, &source, false, true).unwrap();
        assert!(first[0].fetched);
        let cache = dir.join("cache").join("stevenblack-hosts");
        assert!(cache.exists());
        assert!(dir.join("cache").join("stevenblack-hosts.src").exists());

        let second = load_all(&dir, &source, false, true).unwrap();
        assert!(!second[0].fetched, "expected a 304");
        assert_eq!(first[0].entries.len(), second[0].entries.len());
        fs::remove_dir_all(dir).ok();
    }
}
