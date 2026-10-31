use crate::config::{Config, List};
use crate::error::{Error, Result};

pub struct Preset {
    pub name: &'static str,
    pub sources: &'static [&'static str],
}

pub const PRESETS: &[Preset] = &[
    Preset {
        name: "Ads and trackers",
        sources: &["stevenblack/hosts", "1hosts/lite"],
    },
    Preset {
        name: "Social media",
        sources: &["blk/social"],
    },
    Preset {
        name: "Media streaming",
        sources: &["blk/streaming", "blocklistproject/youtube"],
    },
    Preset {
        name: "Anime and piracy",
        sources: &["blocklistproject/piracy", "blocklistproject/torrent"],
    },
    Preset {
        name: "Gambling",
        sources: &["stevenblack/gambling"],
    },
    Preset {
        name: "Adult content",
        sources: &["stevenblack/porn"],
    },
    Preset {
        name: "Malware and phishing",
        sources: &["malwarefilter/urlhaus", "hagezi/pro"],
    },
];

#[derive(Debug)]
pub struct View {
    pub name: String,
    pub sources: Vec<String>,
    pub enabled: bool,
    pub custom: bool,
}

pub fn views(config: &Config) -> Vec<View> {
    let mut views: Vec<View> = PRESETS
        .iter()
        .map(|preset| {
            let sources: Vec<String> = preset
                .sources
                .iter()
                .map(|source| source.to_string())
                .collect();
            View {
                name: preset.name.to_string(),
                enabled: enabled(config, &sources),
                sources,
                custom: false,
            }
        })
        .collect();
    for list in &config.lists {
        let view = View {
            name: list.name.clone(),
            sources: list.sources.clone(),
            enabled: enabled(config, &list.sources),
            custom: true,
        };
        match views.iter_mut().find(|existing| existing.name == list.name) {
            Some(existing) => *existing = view,
            None => views.push(view),
        }
    }
    views
}

fn enabled(config: &Config, sources: &[String]) -> bool {
    !sources.is_empty() && sources.iter().all(|source| config.sources.contains(source))
}

fn preset(name: &str) -> Option<&'static Preset> {
    PRESETS.iter().find(|preset| preset.name == name)
}

fn find(config: &Config, name: &str) -> Option<List> {
    config
        .lists
        .iter()
        .find(|list| list.name == name)
        .cloned()
        .or_else(|| {
            preset(name).map(|preset| List {
                name: preset.name.to_string(),
                sources: preset
                    .sources
                    .iter()
                    .map(|source| source.to_string())
                    .collect(),
            })
        })
}

fn unknown(name: &str) -> Error {
    Error::ListNotFound {
        name: name.to_string(),
    }
}

// Used by the interactive list manager added in #16.
#[allow(dead_code)]
fn dedupe(sources: Vec<String>) -> Vec<String> {
    let mut out = Vec::new();
    for source in sources {
        if !out.contains(&source) {
            out.push(source);
        }
    }
    out
}

/// The state after a toggle, and which enabled lists held shared sources back.
pub struct Toggle {
    pub enabled: bool,
    pub kept_by: Vec<String>,
}

/// Flip a list. Turning one off keeps members another enabled list still needs.
pub fn toggle(config: &mut Config, name: &str) -> Result<Toggle> {
    let list = find(config, name).ok_or_else(|| unknown(name))?;
    if !enabled(config, &list.sources) {
        for source in list.sources {
            if !config.sources.contains(&source) {
                config.sources.push(source);
            }
        }
        return Ok(Toggle {
            enabled: true,
            kept_by: Vec::new(),
        });
    }

    let holders: Vec<View> = views(config)
        .into_iter()
        .filter(|view| view.name != name && view.enabled)
        .filter(|view| {
            view.sources
                .iter()
                .any(|source| list.sources.contains(source))
        })
        .collect();
    let needed: Vec<String> = holders
        .iter()
        .flat_map(|view| view.sources.iter().cloned())
        .collect();
    config
        .sources
        .retain(|source| !list.sources.contains(source) || needed.contains(source));
    Ok(Toggle {
        enabled: enabled(config, &list.sources),
        kept_by: holders.into_iter().map(|view| view.name).collect(),
    })
}

/// Replace a custom list's members, syncing sources when it is on.
#[allow(dead_code)] // used by the interactive manager in #16
pub fn edit(config: &mut Config, name: &str, sources: Vec<String>) -> Result<()> {
    let name = name.trim();
    if name.is_empty() {
        return Err(Error::EmptyListName);
    }
    if preset(name).is_some() {
        return Err(Error::ListReserved {
            name: name.to_string(),
        });
    }
    let Some(old) = config.lists.iter().find(|list| list.name == name).cloned() else {
        return Err(unknown(name));
    };
    let sources = dedupe(sources);
    if sources.is_empty() {
        return Err(Error::ListEmpty);
    }

    if enabled(config, &old.sources) {
        let needed: Vec<String> = views(config)
            .into_iter()
            .filter(|view| view.name != name && view.enabled)
            .flat_map(|view| view.sources)
            .collect();
        config.sources.retain(|source| {
            !old.sources.contains(source) || sources.contains(source) || needed.contains(source)
        });
        for source in &sources {
            if !config.sources.contains(source) {
                config.sources.push(source.clone());
            }
        }
    }

    if let Some(list) = config.lists.iter_mut().find(|list| list.name == name) {
        list.sources = sources;
    }
    Ok(())
}

#[allow(dead_code)] // used by the interactive manager in #16
pub fn create(config: &mut Config, name: &str, sources: Vec<String>) -> Result<()> {
    let name = name.trim();
    if name.is_empty() {
        return Err(Error::EmptyListName);
    }
    if preset(name).is_some() {
        return Err(Error::ListReserved {
            name: name.to_string(),
        });
    }
    if config.lists.iter().any(|list| list.name == name) {
        return Err(Error::ListExists {
            name: name.to_string(),
        });
    }
    let sources = dedupe(sources);
    if sources.is_empty() {
        return Err(Error::ListEmpty);
    }
    config.lists.push(List {
        name: name.to_string(),
        sources,
    });
    Ok(())
}

/// Remove a custom list. Its sources stay configured.
#[allow(dead_code)] // used by the interactive manager in #16
pub fn delete(config: &mut Config, name: &str) -> Result<()> {
    if preset(name).is_some() {
        return Err(Error::ListReserved {
            name: name.to_string(),
        });
    }
    let Some(index) = config.lists.iter().position(|list| list.name == name) else {
        return Err(unknown(name));
    };
    config.lists.remove(index);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn config_with(sources: &[&str]) -> Config {
        Config {
            sources: sources.iter().map(|source| source.to_string()).collect(),
            lists: Vec::new(),
        }
    }

    fn owned(sources: &[&str]) -> Vec<String> {
        sources.iter().map(|source| source.to_string()).collect()
    }

    #[test]
    fn toggling_adds_and_removes_members() {
        let mut config = Config::default();
        assert!(toggle(&mut config, "Social media").unwrap().enabled);
        assert_eq!(config.sources, owned(&["blk/social"]));
        assert!(!toggle(&mut config, "Social media").unwrap().enabled);
        assert!(config.sources.is_empty());
    }

    #[test]
    fn turning_a_list_off_keeps_members_another_list_needs() {
        let mut config = Config::default();
        create(&mut config, "A", owned(&["oisd/small"])).unwrap();
        create(&mut config, "B", owned(&["oisd/small"])).unwrap();
        assert!(toggle(&mut config, "A").unwrap().enabled);

        let kept = toggle(&mut config, "A").unwrap();
        assert!(kept.enabled);
        assert_eq!(kept.kept_by, vec!["B"]);
        assert_eq!(config.sources, owned(&["oisd/small"]));

        delete(&mut config, "B").unwrap();
        let off = toggle(&mut config, "A").unwrap();
        assert!(!off.enabled);
        assert!(off.kept_by.is_empty());
        assert!(config.sources.is_empty());
    }

    #[test]
    fn editing_an_enabled_list_syncs_sources() {
        let mut config = config_with(&["oisd/small"]);
        create(&mut config, "Focus", owned(&["oisd/small"])).unwrap();

        edit(&mut config, "Focus", owned(&["hagezi/normal"])).unwrap();
        assert_eq!(config.sources, owned(&["hagezi/normal"]));
    }

    #[test]
    fn preset_names_are_reserved() {
        let mut config = Config::default();
        assert!(matches!(
            create(&mut config, "Social media", owned(&["blk/social"])),
            Err(Error::ListReserved { .. })
        ));
        assert!(matches!(
            edit(&mut config, "Social media", owned(&["blk/social"])),
            Err(Error::ListReserved { .. })
        ));
        assert!(matches!(
            delete(&mut config, "Social media"),
            Err(Error::ListReserved { .. })
        ));
    }

    #[test]
    fn duplicate_custom_names_are_rejected() {
        let mut config = Config::default();
        create(&mut config, "Focus", owned(&["oisd/small"])).unwrap();
        assert!(matches!(
            create(&mut config, "Focus", owned(&["oisd/big"])),
            Err(Error::ListExists { .. })
        ));
    }

    #[test]
    fn deleting_a_list_keeps_its_sources() {
        let mut config = Config::default();
        create(&mut config, "Focus", owned(&["oisd/small"])).unwrap();
        toggle(&mut config, "Focus").unwrap();

        delete(&mut config, "Focus").unwrap();
        assert!(config.lists.is_empty());
        assert_eq!(config.sources, owned(&["oisd/small"]));
    }

    #[test]
    fn names_and_members_are_required() {
        let mut config = Config::default();
        assert!(matches!(
            create(&mut config, "", owned(&["oisd/small"])),
            Err(Error::EmptyListName)
        ));
        assert!(matches!(
            create(&mut config, "Focus", Vec::new()),
            Err(Error::ListEmpty)
        ));
        assert!(matches!(
            toggle(&mut config, "nope"),
            Err(Error::ListNotFound { .. })
        ));
    }

    #[test]
    fn views_put_presets_first() {
        let mut config = config_with(&["blk/social"]);
        create(&mut config, "Focus", owned(&["oisd/small"])).unwrap();
        let views = views(&config);

        assert_eq!(views[0].name, "Ads and trackers");
        let social = views
            .iter()
            .find(|view| view.name == "Social media")
            .unwrap();
        assert!(social.enabled);
        assert!(!social.custom);
        let focus = views.iter().find(|view| view.name == "Focus").unwrap();
        assert!(!focus.enabled);
        assert!(focus.custom);
    }
}
