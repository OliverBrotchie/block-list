use std::collections::HashSet;

use crate::error::{Error, Result};
use crate::region;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Format {
    Hosts,
    Domains,
    Wildcard,
    Abp,
}

impl Format {
    pub fn name(self) -> &'static str {
        match self {
            Format::Hosts => "hosts",
            Format::Domains => "domains",
            Format::Wildcard => "wildcard",
            Format::Abp => "abp",
        }
    }
}

#[derive(Debug)]
pub struct Source {
    pub id: &'static str,
    pub format: Format,
    pub licence: &'static str,
    pub urls: &'static [&'static str],
    /// Domains shipped in the binary. These sources never fetch or cache.
    pub inline: &'static [&'static str],
}

impl Source {
    pub fn is_inline(&self) -> bool {
        !self.inline.is_empty()
    }
}

/// Paid streaming services. YouTube comes from its own source.
const STREAMING: &[&str] = &[
    "netflix.com",
    "nflxvideo.net",
    "nflximg.net",
    "nflxext.com",
    "nflxso.net",
    "primevideo.com",
    "amazonvideo.com",
    "aiv-cdn.net",
    "aiv-delivery.net",
    "disneyplus.com",
    "disney-plus.net",
    "bamgrid.com",
    "dssott.com",
    "hulu.com",
    "huluim.com",
    "max.com",
    "hbomax.com",
    "hbomaxcdn.com",
    "tv.apple.com",
    "play.itunes.apple.com",
    "twitch.tv",
    "ttvnw.net",
    "jtvnw.net",
    "crunchyroll.com",
    "vrv.co",
    "peacocktv.com",
    "paramountplus.com",
    "pluto.tv",
];

/// Social networks, with no WhatsApp domains.
const SOCIAL: &[&str] = &[
    "facebook.com",
    "fb.com",
    "fbcdn.net",
    "fbsbx.com",
    "messenger.com",
    "instagram.com",
    "cdninstagram.com",
    "tiktok.com",
    "tiktokcdn.com",
    "tiktokv.com",
    "byteoversea.com",
    "twitter.com",
    "x.com",
    "t.co",
    "twimg.com",
    "snapchat.com",
    "snap-cdn.net",
    "sc-cdn.net",
    "reddit.com",
    "redd.it",
    "redditstatic.com",
    "redditmedia.com",
    "linkedin.com",
    "licdn.com",
    "pinterest.com",
    "pinimg.com",
    "tumblr.com",
    "threads.net",
    "bsky.app",
];

const BRAVE_DEFAULT: &[&str] = &[
    "https://raw.githubusercontent.com/uBlockOrigin/uAssets/master/filters/filters.txt",
    "https://raw.githubusercontent.com/uBlockOrigin/uAssets/master/filters/filters-2020.txt",
    "https://raw.githubusercontent.com/uBlockOrigin/uAssets/master/filters/filters-2021.txt",
    "https://raw.githubusercontent.com/uBlockOrigin/uAssets/master/filters/filters-2022.txt",
    "https://raw.githubusercontent.com/uBlockOrigin/uAssets/master/filters/filters-2023.txt",
    "https://raw.githubusercontent.com/uBlockOrigin/uAssets/master/filters/filters-2024.txt",
    "https://raw.githubusercontent.com/uBlockOrigin/uAssets/master/filters/filters-2025.txt",
    "https://raw.githubusercontent.com/uBlockOrigin/uAssets/master/filters/filters-2026.txt",
    "https://raw.githubusercontent.com/uBlockOrigin/uAssets/master/filters/filters-general.txt",
    "https://raw.githubusercontent.com/uBlockOrigin/uAssets/master/filters/badware.txt",
    "https://raw.githubusercontent.com/uBlockOrigin/uAssets/master/filters/resource-abuse.txt",
    "https://raw.githubusercontent.com/uBlockOrigin/uAssets/master/filters/unbreak.txt",
    "https://raw.githubusercontent.com/uBlockOrigin/uAssets/master/filters/quick-fixes.txt",
    "https://raw.githubusercontent.com/uBlockOrigin/uAssets/master/filters/ubo-link-shorteners.txt",
    "https://easylist.to/easylist/easylist.txt",
    "https://malware-filter.gitlab.io/malware-filter/urlhaus-filter-agh-online.txt",
    "https://raw.githubusercontent.com/brave/adblock-lists/master/brave-unbreak.txt",
    "https://raw.githubusercontent.com/brave/adblock-lists/master/brave-lists/brave-specific.txt",
    "https://raw.githubusercontent.com/brave/adblock-lists/master/brave-lists/brave-social.txt",
    "https://raw.githubusercontent.com/brave/adblock-lists/master/brave-lists/brave-unbreak.txt",
    "https://raw.githubusercontent.com/brave/adblock-lists/master/brave-lists/brave-android-specific.txt",
    "https://raw.githubusercontent.com/brave/adblock-lists/master/brave-lists/brave-sugarcoat.txt",
];

pub const SOURCES: &[Source] = &[
    Source {
        id: "brave/default",
        format: Format::Abp,
        licence: "MPL-2.0 plus upstream terms",
        urls: BRAVE_DEFAULT,
        inline: &[],
    },
    Source {
        id: "stevenblack/hosts",
        format: Format::Hosts,
        licence: "MIT",
        urls: &["https://raw.githubusercontent.com/StevenBlack/hosts/master/hosts"],
        inline: &[],
    },
    Source {
        id: "stevenblack/social",
        format: Format::Hosts,
        licence: "MIT",
        urls: &["https://raw.githubusercontent.com/StevenBlack/hosts/master/alternates/social/hosts"],
        inline: &[],
    },
    Source {
        id: "stevenblack/gambling",
        format: Format::Hosts,
        licence: "MIT",
        urls: &["https://raw.githubusercontent.com/StevenBlack/hosts/master/alternates/gambling/hosts"],
        inline: &[],
    },
    Source {
        id: "stevenblack/porn",
        format: Format::Hosts,
        licence: "MIT",
        urls: &["https://raw.githubusercontent.com/StevenBlack/hosts/master/alternates/porn/hosts"],
        inline: &[],
    },
    Source {
        id: "oisd/small",
        format: Format::Wildcard,
        licence: "GPL-3.0",
        urls: &["https://small.oisd.nl/domainswild"],
        inline: &[],
    },
    Source {
        id: "oisd/big",
        format: Format::Wildcard,
        licence: "GPL-3.0",
        urls: &["https://big.oisd.nl/domainswild"],
        inline: &[],
    },
    Source {
        id: "hagezi/normal",
        format: Format::Domains,
        licence: "GPL-3.0",
        urls: &["https://raw.githubusercontent.com/hagezi/dns-blocklists/main/wildcard/multi-onlydomains.txt"],
        inline: &[],
    },
    Source {
        id: "hagezi/pro",
        format: Format::Domains,
        licence: "GPL-3.0",
        urls: &["https://raw.githubusercontent.com/hagezi/dns-blocklists/main/wildcard/pro-onlydomains.txt"],
        inline: &[],
    },
    Source {
        id: "1hosts/lite",
        format: Format::Hosts,
        licence: "MPL-2.0",
        urls: &["https://raw.githubusercontent.com/badmojr/1Hosts/master/Lite/hosts.txt"],
        inline: &[],
    },
    Source {
        id: "peterlowe/hosts",
        format: Format::Hosts,
        licence: "unstated",
        urls: &["https://pgl.yoyo.org/adservers/serverlist.php?hostformat=hosts&showintro=0&mimetype=plaintext"],
        inline: &[],
    },
    Source {
        id: "malwarefilter/urlhaus",
        format: Format::Hosts,
        licence: "CC0 and MIT",
        urls: &["https://malware-filter.gitlab.io/malware-filter/urlhaus-filter-hosts-online.txt"],
        inline: &[],
    },
    Source {
        id: "blocklistproject/youtube",
        format: Format::Hosts,
        licence: "Unlicense",
        urls: &["https://blocklistproject.github.io/Lists/youtube.txt"],
        inline: &[],
    },
    Source {
        id: "blocklistproject/piracy",
        format: Format::Hosts,
        licence: "Unlicense",
        urls: &["https://blocklistproject.github.io/Lists/piracy.txt"],
        inline: &[],
    },
    Source {
        id: "blocklistproject/torrent",
        format: Format::Hosts,
        licence: "Unlicense",
        urls: &["https://blocklistproject.github.io/Lists/torrent.txt"],
        inline: &[],
    },
    Source {
        id: "blk/streaming",
        format: Format::Domains,
        licence: "curated, MIT",
        urls: &[],
        inline: STREAMING,
    },
    Source {
        id: "blk/social",
        format: Format::Domains,
        licence: "curated, MIT",
        urls: &[],
        inline: SOCIAL,
    },
];

pub fn get(id: &str) -> Option<&'static Source> {
    SOURCES.iter().find(|source| source.id == id)
}

pub fn ids() -> Vec<&'static str> {
    SOURCES.iter().map(|source| source.id).collect()
}

pub fn get_or_error(id: &str) -> Result<&'static Source> {
    get(id)
        .ok_or_else(|| Error::unknown_source(id, ids().iter().map(|id| id.to_string()).collect()))
}

#[derive(Debug, Default)]
pub struct Conversion {
    /// Domains to block, deduped in first-seen order, with exceptions already removed.
    pub domains: Vec<String>,
    /// Domains named by `@@` exception rules.
    pub removed: Vec<String>,
    pub kept: usize,
    pub exceptions: usize,
    pub cosmetic: usize,
    pub other: usize,
    pub invalid: usize,
}

pub fn parse(text: &str, format: Format) -> Conversion {
    match format {
        Format::Hosts => hosts(text),
        Format::Domains => domains(text, false),
        Format::Wildcard => domains(text, true),
        Format::Abp => abp(text),
    }
}

fn hosts(text: &str) -> Conversion {
    let mut parsed = Conversion::default();
    let mut seen = HashSet::new();
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') || line.starts_with('!') {
            continue;
        }
        match region::entries_of(line) {
            Some(entries) => {
                parsed.kept += 1;
                for entry in entries {
                    if seen.insert(entry.host.clone()) {
                        parsed.domains.push(entry.host);
                    }
                }
            }
            None => parsed.invalid += 1,
        }
    }
    parsed
}

fn domains(text: &str, wildcard: bool) -> Conversion {
    let mut parsed = Conversion::default();
    let mut seen = HashSet::new();
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') || line.starts_with('!') {
            continue;
        }
        let host = if wildcard {
            line.strip_prefix("*.").unwrap_or(line)
        } else {
            line
        };
        if region::valid_host(host) {
            parsed.kept += 1;
            if seen.insert(host.to_string()) {
                parsed.domains.push(host.to_string());
            }
        } else {
            parsed.invalid += 1;
        }
    }
    parsed
}

fn abp(text: &str) -> Conversion {
    let mut parsed = Conversion::default();
    let mut removed = HashSet::new();
    let mut seen = HashSet::new();
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('!') || line.starts_with('[') {
            continue;
        }
        if is_cosmetic(line) {
            parsed.cosmetic += 1;
            continue;
        }
        if let Some(rest) = line.strip_prefix("@@") {
            match abp_domain(rest) {
                Some(domain) => {
                    parsed.exceptions += 1;
                    if removed.insert(domain.to_string()) {
                        parsed.removed.push(domain.to_string());
                    }
                }
                None => parsed.other += 1,
            }
            continue;
        }
        match abp_domain(line) {
            Some(domain) => {
                parsed.kept += 1;
                if seen.insert(domain.to_string()) {
                    parsed.domains.push(domain.to_string());
                }
            }
            None => parsed.other += 1,
        }
    }
    parsed.domains.retain(|domain| !removed.contains(domain));
    parsed
}

fn abp_domain(rule: &str) -> Option<&str> {
    let rest = rule.strip_prefix("||")?;
    let domain = rest.strip_suffix('^')?;
    if region::valid_host(domain) {
        Some(domain)
    } else {
        None
    }
}

fn is_cosmetic(rule: &str) -> bool {
    ["##", "#@#", "#?#", "#$#", "#%#"]
        .iter()
        .any(|mark| rule.contains(mark))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn catalog_ids_are_unique_and_have_a_source() {
        let mut seen = HashSet::new();
        for source in SOURCES {
            assert!(seen.insert(source.id), "duplicate id {}", source.id);
            if source.is_inline() {
                assert!(source.urls.is_empty(), "{} has urls", source.id);
                for domain in source.inline {
                    assert!(region::valid_host(domain), "invalid domain {domain}");
                }
            } else {
                assert!(!source.urls.is_empty(), "{} has no urls", source.id);
            }
        }
    }

    #[test]
    fn unknown_ids_name_the_candidates() {
        let error = get_or_error("nope").unwrap_err();
        assert!(error.to_string().contains("brave/default"), "{error}");
        assert!(get("oisd/small").is_some());
    }

    #[test]
    fn hosts_format_takes_every_hostname() {
        let text = "# comment\n0.0.0.0 a.test b.test\n::1 c.test\n0.0.0.0\nbroken line\n";
        let parsed = parse(text, Format::Hosts);
        assert_eq!(parsed.domains, vec!["a.test", "b.test", "c.test"]);
        assert_eq!(parsed.kept, 2);
        assert_eq!(parsed.invalid, 2);
    }

    #[test]
    fn wildcard_domains_lose_the_star() {
        let text = "# comment\n! comment\nexample.test\n*.wild.test\nbad host\n";
        let parsed = parse(text, Format::Wildcard);
        assert_eq!(parsed.domains, vec!["example.test", "wild.test"]);
        assert_eq!(parsed.invalid, 1);
    }

    #[test]
    fn plain_domains_keep_the_line() {
        let parsed = parse("example.test\n*.not-stripped.test\n", Format::Domains);
        assert_eq!(parsed.domains, vec!["example.test"]);
        assert_eq!(parsed.invalid, 1);
    }

    #[test]
    fn abp_strict_keeps_domain_rules_and_subtracts_exceptions() {
        let text = "\
! comment
[Adblock Plus 2.0]
||ads.test^
||track.test^
@@||ads.test^
||path.test/track^
||option.test^$third-party
example.test##.ad
##.generic
/ads[0-9]+/
||disabled.test^$badfilter
";
        let parsed = parse(text, Format::Abp);
        assert_eq!(parsed.domains, vec!["track.test"]);
        assert_eq!(parsed.removed, vec!["ads.test"]);
        assert_eq!(parsed.kept, 2);
        assert_eq!(parsed.exceptions, 1);
        assert_eq!(parsed.cosmetic, 2);
        assert_eq!(parsed.other, 4);
    }

    #[test]
    #[ignore = "downloads the 22 Brave bundle URLs"]
    fn brave_default_live_counts() {
        let source = get("brave/default").unwrap();
        let mut kept = 0;
        let mut exceptions = 0;
        let mut cosmetic = 0;
        let mut other = 0;
        let mut domains = 0;
        for url in source.urls {
            let body = ureq::get(url)
                .call()
                .unwrap_or_else(|error| panic!("{}: {}", url, error))
                .into_string()
                .unwrap_or_else(|error| panic!("{}: {}", url, error));
            let parsed = parse(&body, source.format);
            println!(
                "{url}: kept={} exceptions={} cosmetic={} other={} invalid={}",
                parsed.kept, parsed.exceptions, parsed.cosmetic, parsed.other, parsed.invalid
            );
            kept += parsed.kept;
            exceptions += parsed.exceptions;
            cosmetic += parsed.cosmetic;
            other += parsed.other;
            domains += parsed.domains.len();
        }
        println!(
            "brave/default totals: kept={kept} exceptions={exceptions} cosmetic={cosmetic} other={other} domains={domains}"
        );
        assert!(kept > 20_000, "kept={kept}");
        assert!(cosmetic > 10_000, "cosmetic={cosmetic}");
    }
}
