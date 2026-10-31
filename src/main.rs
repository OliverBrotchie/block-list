mod catalog;
mod config;
mod pause;
mod region;
mod sources;
mod state;

use std::error::Error;
use std::fs;
use std::io::IsTerminal;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use clap::{Parser, Subcommand};
use dialoguer::{Confirm, Input, MultiSelect, Select};

use region::Entry;

static INTERRUPTED: AtomicBool = AtomicBool::new(false);

#[derive(Debug, Parser)]
#[command(
    name = "blk",
    version,
    about = "Manage a block list region in a hosts file"
)]
struct Cli {
    /// Hosts file to edit.
    #[arg(long, global = true, default_value = "/etc/hosts", value_name = "PATH")]
    hosts: PathBuf,

    /// Suppress normal output.
    #[arg(long, global = true)]
    quiet: bool,

    /// Print what would change without writing.
    #[arg(long, global = true)]
    dry_run: bool,

    /// Rebuild from the cache without network access.
    #[arg(long, global = true)]
    offline: bool,

    /// Skip the confirmation prompt.
    #[arg(long, global = true)]
    yes: bool,

    #[command(subcommand)]
    command: Option<Cmd>,
}

#[derive(Debug, Subcommand)]
enum Cmd {
    /// Replace the block region with the configured sources.
    Update {
        /// Source ids, URLs, or list files. Replaces the configured list.
        #[arg(value_name = "SOURCE")]
        sources: Vec<String>,
        /// Extra URL or list file to add to the configured list.
        #[arg(long = "from", value_name = "URL_OR_PATH")]
        from: Vec<String>,
    },
    /// Show the block region and its entry counts.
    Status {
        /// Emit machine-readable JSON.
        #[arg(long)]
        json: bool,
    },
    /// Check whether a domain is blocked.
    Test { domain: String },
    /// Check the region, cache, backup, and drift.
    Doctor,
    /// Open the config file in $EDITOR.
    Config,
    /// Pause every source indefinitely.
    Off,
    /// Pause some sources, optionally for a duration such as 30m.
    Pause {
        /// Source names, then an optional duration.
        #[arg(value_name = "SOURCE_OR_DURATION")]
        args: Vec<String>,
    },
    /// Restore a paused region.
    Resume,
    /// Choose sources interactively.
    Sources,
    /// Add sources and apply them.
    Add {
        #[arg(required = true, value_name = "SOURCE")]
        sources: Vec<String>,
    },
    /// Remove sources and rebuild the region.
    Rm {
        #[arg(required = true, value_name = "SOURCE")]
        sources: Vec<String>,
    },
    /// List the built-in sources, or show one source's URLs.
    Catalog {
        #[arg(value_name = "SOURCE")]
        source: Option<String>,
    },
}

fn main() {
    let cli = Cli::parse();
    if let Err(err) = run(&cli) {
        eprintln!("error: {err}");
        std::process::exit(1);
    }
}

fn run(cli: &Cli) -> Result<(), Box<dyn Error>> {
    let state = state_dir();
    match &cli.command {
        Some(Cmd::Update { sources, from }) => update(cli, &state, sources, from),
        Some(Cmd::Status { json }) => status(cli, &state, *json),
        Some(Cmd::Test { domain }) => test_domain(cli, domain),
        Some(Cmd::Doctor) => doctor(cli, &state),
        Some(Cmd::Config) => config_command(&state),
        Some(Cmd::Off) => off(cli, &state),
        Some(Cmd::Pause { args }) => pause_command(cli, &state, args),
        Some(Cmd::Resume) => resume(&state),
        Some(Cmd::Sources) => sources_command(cli, &state),
        Some(Cmd::Add { sources }) => add_command(cli, &state, sources),
        Some(Cmd::Rm { sources }) => rm_command(cli, &state, sources),
        Some(Cmd::Catalog { source }) => catalog_command(source.as_deref()),
        None => bare(cli, &state),
    }
}

fn bare(cli: &Cli, state: &Path) -> Result<(), Box<dyn Error>> {
    if std::io::stdin().is_terminal() {
        return menu(cli, state);
    }
    status(cli, state, false)?;
    eprintln!(
        "usage: blk <status|update|sources|add|rm|pause|resume|off|test|doctor|config|catalog>"
    );
    std::process::exit(2);
}

fn menu(cli: &Cli, state: &Path) -> Result<(), Box<dyn Error>> {
    let items = [
        "Status",
        "Update",
        "Sources",
        "Pause",
        "Resume",
        "Off",
        "Test domain",
        "Doctor",
        "Edit config",
        "Quit",
    ];
    loop {
        let pick = Select::new()
            .with_prompt("blk")
            .items(&items)
            .default(0)
            .interact()?;
        match pick {
            0 => status(cli, state, false)?,
            1 => update(cli, state, &[], &[])?,
            2 => sources_command(cli, state)?,
            3 => pause_menu(cli, state)?,
            4 => resume(state)?,
            5 => off(cli, state)?,
            6 => {
                let domain: String = Input::new().with_prompt("Domain").interact_text()?;
                test_domain(cli, domain.trim())?;
            }
            7 => {
                if let Err(err) = doctor(cli, state) {
                    println!("{err}");
                }
            }
            8 => config_command(state)?,
            9 => break,
            _ => {}
        }
    }
    Ok(())
}

fn pause_menu(cli: &Cli, state: &Path) -> Result<(), Box<dyn Error>> {
    let contents = read_hosts(&cli.hosts)?;
    let Some(parsed) = region::parse(&contents)? else {
        println!("no block region");
        return Ok(());
    };
    let names: Vec<String> = parsed
        .sections
        .iter()
        .map(|section| section.name.clone())
        .collect();
    let defaults = vec![true; names.len()];
    let picked = MultiSelect::new()
        .with_prompt("Pause which sources?")
        .items(&names)
        .defaults(&defaults)
        .interact()?;
    if picked.is_empty() {
        println!("nothing selected");
        return Ok(());
    }
    let duration: String = Input::new()
        .with_prompt("Duration (blank for until Ctrl-C)")
        .allow_empty(true)
        .interact_text()?;
    let mut args: Vec<String> = picked.iter().map(|&index| names[index].clone()).collect();
    let duration = duration.trim();
    if !duration.is_empty() {
        args.push(duration.to_string());
    }
    pause_command(cli, state, &args)
}

fn update(cli: &Cli, state: &Path, args: &[String], from: &[String]) -> Result<(), Box<dyn Error>> {
    let config = config::load(state)?;
    let mut sources = if args.is_empty() {
        config.sources
    } else {
        Vec::new()
    };
    for source in args.iter().chain(from.iter()) {
        if !sources.contains(source) {
            sources.push(source.clone());
        }
    }
    let persist = !args.is_empty() || !from.is_empty();
    apply(cli, state, &sources, persist)
}

/// Fetch or read every source, then replace the region in one write.
fn apply(cli: &Cli, state: &Path, sources: &[String], persist: bool) -> Result<(), Box<dyn Error>> {
    if sources.is_empty() {
        return Err("no sources configured; run `blk sources` or `blk add`".into());
    }

    let contents = read_hosts(&cli.hosts)?;
    let parsed = region::parse(&contents)?;
    let loaded = sources::load_all(state, sources, cli.offline, !cli.dry_run)?;

    for source in &loaded {
        for warning in &source.warnings {
            eprintln!("warning: {}: {warning}", source.name);
        }
        if let Some(counts) = source.counts {
            if source.fetched && !cli.quiet {
                println!(
                    "{}: kept {}, dropped {} cosmetic, {} path or option, {} exceptions subtracted",
                    source.name, counts.kept, counts.cosmetic, counts.other, counts.exceptions
                );
            }
        }
    }

    let sections: Vec<(String, Vec<Entry>)> = loaded
        .iter()
        .map(|source| (source.name.clone(), source.entries.clone()))
        .collect();
    let total: usize = sections.iter().map(|(_, entries)| entries.len()).sum();
    let rendered = region::render(&sections);

    if cli.dry_run {
        if !cli.quiet {
            for (name, entries) in &sections {
                println!("{name}: {} entries", entries.len());
            }
            println!(
                "would write {total} entries from {} sources to {}",
                sources.len(),
                cli.hosts.display()
            );
        }
        return Ok(());
    }

    if !cli.yes && !cli.quiet && std::io::stdin().is_terminal() {
        let confirmed = Confirm::new()
            .with_prompt(format!(
                "Write {total} entries from {} sources to {}?",
                sources.len(),
                cli.hosts.display()
            ))
            .default(false)
            .interact()?;
        if !confirmed {
            return Ok(());
        }
    }

    require_root(&cli.hosts)?;
    let next = region::replace(&contents, parsed.as_ref(), &rendered);
    backup(&cli.hosts)?;
    write_hosts(&cli.hosts, &next)?;

    let counts = loaded
        .iter()
        .map(|source| (source.name.clone(), source.entries.len()))
        .collect();
    state::save(
        state,
        &state::State {
            last_update: pause::now(),
            hosts_path: cli.hosts.display().to_string(),
            region_hash: state::hash_region(&rendered),
            counts,
        },
    )?;
    if persist {
        config::save(
            state,
            &config::Config {
                sources: sources.to_vec(),
            },
        )?;
    }
    pause::clear(state);

    if !cli.quiet {
        println!(
            "Block List updated: {total} entries from {} sources",
            sources.len()
        );
    }
    Ok(())
}

fn sources_command(cli: &Cli, state: &Path) -> Result<(), Box<dyn Error>> {
    let config = config::load(state)?;
    if !std::io::stdin().is_terminal() {
        if config.sources.is_empty() {
            println!("no sources configured");
        } else {
            for source in &config.sources {
                println!("{source}");
            }
        }
        println!("add sources with `blk add <id-or-url>`; see `blk catalog`");
        return Ok(());
    }

    let ids = catalog::ids();
    let defaults: Vec<bool> = ids
        .iter()
        .map(|id| config.sources.iter().any(|source| source == id))
        .collect();
    let picked = MultiSelect::new()
        .with_prompt("Sources")
        .items(&ids)
        .defaults(&defaults)
        .interact()?;

    let mut chosen: Vec<String> = picked.iter().map(|&index| ids[index].to_string()).collect();
    for source in &config.sources {
        if catalog::get(source).is_none() && !chosen.contains(source) {
            chosen.push(source.clone());
        }
    }

    let extra: String = Input::new()
        .with_prompt("Add a URL or path (blank to skip)")
        .allow_empty(true)
        .interact_text()?;
    let extra = extra.trim();
    if !extra.is_empty() && !chosen.iter().any(|source| source == extra) {
        chosen.push(extra.to_string());
    }

    change_sources(cli, state, chosen, "sources selected")
}

fn add_command(cli: &Cli, state: &Path, sources: &[String]) -> Result<(), Box<dyn Error>> {
    for source in sources {
        validate_source(source)?;
    }
    let config = config::load(state)?;
    let mut chosen = config.sources;
    for source in sources {
        if !chosen.contains(source) {
            chosen.push(source.clone());
        }
    }
    change_sources(cli, state, chosen, "sources added")
}

fn rm_command(cli: &Cli, state: &Path, sources: &[String]) -> Result<(), Box<dyn Error>> {
    let config = config::load(state)?;
    for source in sources {
        if !config.sources.contains(source) {
            return Err(format!("{source} is not configured").into());
        }
    }
    let remaining = config
        .sources
        .into_iter()
        .filter(|source| !sources.contains(source))
        .collect();
    change_sources(cli, state, remaining, "sources removed")
}

/// Save a new source list, then apply it unless a live pause holds the region.
fn change_sources(
    cli: &Cli,
    state: &Path,
    sources: Vec<String>,
    message: &str,
) -> Result<(), Box<dyn Error>> {
    let sources = dedupe(sources);

    if cli.dry_run {
        for source in &sources {
            println!("{source}");
        }
        if !cli.quiet {
            println!("would save and apply {} sources", sources.len());
        }
        return Ok(());
    }

    if let Some(record) = pause::load(state)? {
        if !record.is_expired(pause::now()) {
            config::save(
                state,
                &config::Config {
                    sources: sources.clone(),
                },
            )?;
            if !cli.quiet {
                println!("{message}; the pause is still in effect, run `blk resume`");
            }
            return Ok(());
        }
        resume_from(state, &record)?;
    }

    if sources.is_empty() {
        clear_region(cli, state)?;
        config::save(state, &config::Config { sources })?;
        if !cli.quiet {
            println!("no sources configured; region removed");
        }
        return Ok(());
    }

    apply(cli, state, &sources, true)?;
    if !cli.quiet {
        println!("{message}");
    }
    Ok(())
}

fn validate_source(source: &str) -> Result<(), Box<dyn Error>> {
    if catalog::get(source).is_some()
        || source.starts_with("http://")
        || source.starts_with("https://")
        || Path::new(source).exists()
    {
        return Ok(());
    }
    Err(format!(
        "unknown source {source}; known ids: {}",
        catalog::ids().join(", ")
    )
    .into())
}

fn dedupe(sources: Vec<String>) -> Vec<String> {
    let mut out = Vec::new();
    for source in sources {
        if !out.contains(&source) {
            out.push(source);
        }
    }
    out
}

fn clear_region(cli: &Cli, state: &Path) -> Result<(), Box<dyn Error>> {
    let contents = read_hosts(&cli.hosts)?;
    let Some(parsed) = region::parse(&contents)? else {
        return Ok(());
    };
    require_root(&cli.hosts)?;
    backup(&cli.hosts)?;
    let next = region::replace(&contents, Some(&parsed), "");
    write_hosts(&cli.hosts, &next)?;
    let _ = fs::remove_file(state::path(state));
    pause::clear(state);
    Ok(())
}

fn status(cli: &Cli, state: &Path, json: bool) -> Result<(), Box<dyn Error>> {
    restore_expired(state)?;

    let contents = read_hosts(&cli.hosts)?;
    let parsed = region::parse(&contents)?;
    let record = pause::load(state)?;

    if json {
        return status_json(cli, state, &contents, parsed.as_ref(), record.as_ref());
    }

    if let Some(update) = state::load(state)? {
        if !cli.quiet {
            println!("last update: {}", local_datetime(update.last_update));
        }
    }

    match &parsed {
        Some(parsed) => {
            let total: usize = parsed
                .sections
                .iter()
                .map(|section| section.entries.len())
                .sum();
            if !cli.quiet {
                println!(
                    "{}: {total} entries in {} sources",
                    cli.hosts.display(),
                    parsed.sections.len()
                );
                for section in &parsed.sections {
                    println!("  {}: {}", section.name, section.entries.len());
                }
            }
        }
        None => {
            if !cli.quiet {
                println!("{}: no block region", cli.hosts.display());
            }
        }
    }

    if let Some(record) = record {
        if !cli.quiet {
            let names = record.paused.join(", ");
            println!("paused: {names} {}", describe(&record));
        }
    }
    Ok(())
}

fn status_json(
    cli: &Cli,
    state: &Path,
    contents: &str,
    parsed: Option<&region::Parsed>,
    record: Option<&pause::Pause>,
) -> Result<(), Box<dyn Error>> {
    let sources: Vec<(String, usize)> = parsed
        .map(|parsed| {
            parsed
                .sections
                .iter()
                .map(|section| (section.name.clone(), section.entries.len()))
                .collect()
        })
        .unwrap_or_default();
    let total: usize = sources.iter().map(|(_, count)| count).sum();
    let last = state::load(state)?.map(|update| local_datetime(update.last_update));
    let warnings = status_warnings(state, contents, parsed)?;

    let pause_json = match record {
        None => "null".to_string(),
        Some(record) => format!(
            "{{\"sources\":[{}],\"until\":{}}}",
            record
                .paused
                .iter()
                .map(|source| json_string(source))
                .collect::<Vec<_>>()
                .join(","),
            match record.until {
                Some(until) => json_string(&local_datetime(until)),
                None => "null".to_string(),
            }
        ),
    };

    println!(
        "{{\"hosts\":{},\"active\":{},\"last_update\":{},\"sources\":[{}],\"total_entries\":{},\"pause\":{},\"warnings\":[{}]}}",
        json_string(&cli.hosts.display().to_string()),
        parsed.is_some(),
        match last {
            Some(value) => json_string(&value),
            None => "null".to_string(),
        },
        sources
            .iter()
            .map(|(name, count)| format!(
                "{{\"name\":{},\"entries\":{}}}",
                json_string(name),
                count
            ))
            .collect::<Vec<_>>()
            .join(","),
        total,
        pause_json,
        warnings
            .iter()
            .map(|warning| json_string(warning))
            .collect::<Vec<_>>()
            .join(",")
    );
    Ok(())
}

fn status_warnings(
    state: &Path,
    contents: &str,
    parsed: Option<&region::Parsed>,
) -> Result<Vec<String>, Box<dyn Error>> {
    let mut warnings = Vec::new();
    if contents.contains("# Block List") {
        warnings.push("legacy-block-list".to_string());
    }
    if let (Some(parsed), Some(update)) = (parsed, state::load(state)?) {
        let region = &contents[parsed.begin..parsed.end];
        if state::hash_region(region) != update.region_hash {
            warnings.push("region-drift".to_string());
        }
    }
    if let Some(record) = pause::load(state)? {
        if record.is_expired(pause::now()) {
            warnings.push("pause-expired".to_string());
        }
    }
    for source in config::load(state)?.sources {
        if catalog::get(&source).is_some() {
            let slug = source.replace('/', "-");
            if !state.join("cache").join(slug).exists() {
                warnings.push(format!("cache-missing:{source}"));
            }
        }
    }
    Ok(warnings)
}

fn json_string(value: &str) -> String {
    let mut out = String::from("\"");
    for c in value.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\t' => out.push_str("\\t"),
            _ => out.push(c),
        }
    }
    out.push('"');
    out
}

fn test_domain(cli: &Cli, domain: &str) -> Result<(), Box<dyn Error>> {
    let contents = read_hosts(&cli.hosts)?;
    let parsed = region::parse(&contents)?;

    if let Some(parsed) = &parsed {
        for section in &parsed.sections {
            for entry in &section.entries {
                if entry.host.eq_ignore_ascii_case(domain) {
                    println!("blocked by blk: {} {}", entry.ip, entry.host);
                    return Ok(());
                }
            }
        }
    }

    let outside = match &parsed {
        Some(parsed) => format!("{}{}", &contents[..parsed.begin], &contents[parsed.end..]),
        None => contents.clone(),
    };
    for line in outside.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if let Some(entries) = region::entries_of(line) {
            if entries
                .iter()
                .any(|entry| entry.host.eq_ignore_ascii_case(domain))
            {
                println!("blocked by another line: {line}");
                return Ok(());
            }
        }
    }

    println!("not blocked: {domain}");
    Ok(())
}

fn doctor(cli: &Cli, state: &Path) -> Result<(), Box<dyn Error>> {
    let mut problems = 0;
    if !can_write() {
        println!("root: not running as root; writes would need sudo");
    }

    let contents = read_hosts(&cli.hosts)?;
    let parsed = match region::parse(&contents) {
        Ok(parsed) => parsed,
        Err(err) => {
            problems += 1;
            println!("region: {err}");
            None
        }
    };
    match &parsed {
        Some(parsed) => println!("region: ok, {} sections", parsed.sections.len()),
        None => println!("region: none"),
    }

    for source in config::load(state)?.sources {
        let Some(entry) = catalog::get(&source) else {
            continue;
        };
        let slug = source.replace('/', "-");
        let path = state.join("cache").join(&slug);
        if !path.exists() {
            problems += 1;
            println!("cache: missing for {source}");
            continue;
        }
        println!("cache: {source} present");
        if entry.format == catalog::Format::Abp {
            if let Some((kept, cosmetic, other)) = cached_abp_counts(&path) {
                println!(
                    "  abp: kept {kept}, dropped {cosmetic} cosmetic and {other} path or option rules"
                );
            }
        }
    }

    if parsed.is_some() && !state.join("hosts.bak").exists() {
        println!("backup: none yet");
    }
    if contents.contains("# Block List") {
        println!("warning: a legacy # Block List is present; blk leaves it alone");
    }
    if let (Some(parsed), Some(update)) = (&parsed, state::load(state)?) {
        let region = &contents[parsed.begin..parsed.end];
        if state::hash_region(region) == update.region_hash {
            println!("drift: none");
        } else {
            problems += 1;
            println!("drift: region changed since the last update");
        }
    }

    if problems > 0 {
        Err(format!("doctor found {problems} problems").into())
    } else {
        println!("doctor: ok");
        Ok(())
    }
}

fn cached_abp_counts(path: &Path) -> Option<(usize, usize, usize)> {
    let files = if path.is_dir() {
        fs::read_dir(path)
            .ok()?
            .filter_map(|entry| entry.ok().map(|entry| entry.path()))
            .filter(|path| path.extension().map(|ext| ext != "etag").unwrap_or(true))
            .collect::<Vec<_>>()
    } else {
        vec![path.to_path_buf()]
    };

    let mut kept = 0;
    let mut cosmetic = 0;
    let mut other = 0;
    let mut any = false;
    for file in files {
        let text = fs::read_to_string(file).ok()?;
        let parsed = catalog::parse(&text, catalog::Format::Abp);
        kept += parsed.kept;
        cosmetic += parsed.cosmetic;
        other += parsed.other;
        any = true;
    }
    any.then_some((kept, cosmetic, other))
}

fn config_command(state: &Path) -> Result<(), Box<dyn Error>> {
    fs::create_dir_all(state)?;
    let path = config::path(state);
    if !path.exists() {
        config::save(state, &config::Config::default())?;
    }
    let editor = std::env::var("EDITOR").unwrap_or_else(|_| "vi".to_string());
    let status = std::process::Command::new(editor).arg(&path).status()?;
    if !status.success() {
        return Err("editor exited with an error".into());
    }
    Ok(())
}

fn off(cli: &Cli, state: &Path) -> Result<(), Box<dyn Error>> {
    end_expired_pause(state)?;
    if let Some(record) = pause::load(state)? {
        return Err(format!(
            "already paused ({}); run blk resume first",
            describe(&record)
        )
        .into());
    }

    let contents = read_hosts(&cli.hosts)?;
    let Some(parsed) = region::parse(&contents)? else {
        if !cli.quiet {
            println!("{}: no block region", cli.hosts.display());
        }
        return Ok(());
    };

    if cli.dry_run {
        if !cli.quiet {
            println!("would pause every source in {}", cli.hosts.display());
        }
        return Ok(());
    }

    let snapshot = contents[parsed.begin..parsed.end].to_string();
    let record = pause::Pause {
        paused: parsed
            .sections
            .iter()
            .map(|section| section.name.clone())
            .collect(),
        hosts_path: cli.hosts.display().to_string(),
        started: pause::now(),
        until: None,
    };

    require_root(&cli.hosts)?;
    backup(&cli.hosts)?;
    pause::save(state, &record, &snapshot)?;
    let next = region::replace(&contents, Some(&parsed), &region::render(&[]));
    if let Err(err) = write_hosts(&cli.hosts, &next) {
        pause::clear(state);
        return Err(err);
    }
    if !cli.quiet {
        println!("Block List paused (off). Run `blk resume` to restore it.");
    }
    Ok(())
}

fn pause_command(cli: &Cli, state: &Path, args: &[String]) -> Result<(), Box<dyn Error>> {
    let (sources, until) = split_args(args)?;
    end_expired_pause(state)?;
    if let Some(record) = pause::load(state)? {
        return Err(format!(
            "already paused ({}); run blk resume first",
            describe(&record)
        )
        .into());
    }

    let contents = read_hosts(&cli.hosts)?;
    let Some(parsed) = region::parse(&contents)? else {
        return Err(format!("{} has no block region to pause", cli.hosts.display()).into());
    };

    let known: Vec<String> = parsed
        .sections
        .iter()
        .map(|section| section.name.clone())
        .collect();
    let targets = if sources.is_empty() {
        known.clone()
    } else {
        sources
    };
    for target in &targets {
        if !known.contains(target) {
            return Err(format!(
                "unknown source {target}; known sources: {}",
                known.join(", ")
            )
            .into());
        }
    }

    let kept: Vec<(String, Vec<Entry>)> = parsed
        .sections
        .iter()
        .filter(|section| !targets.contains(&section.name))
        .map(|section| (section.name.clone(), section.entries.clone()))
        .collect();
    let snapshot = contents[parsed.begin..parsed.end].to_string();
    let record = pause::Pause {
        paused: targets,
        hosts_path: cli.hosts.display().to_string(),
        started: pause::now(),
        until,
    };

    if cli.dry_run {
        if !cli.quiet {
            println!(
                "would pause {} {}",
                record.paused.join(", "),
                describe(&record)
            );
        }
        return Ok(());
    }

    require_root(&cli.hosts)?;
    backup(&cli.hosts)?;
    pause::save(state, &record, &snapshot)?;
    let next = region::replace(&contents, Some(&parsed), &region::render(&kept));
    if let Err(err) = write_hosts(&cli.hosts, &next) {
        pause::clear(state);
        return Err(err);
    }

    if !cli.quiet {
        println!("paused {} {}", record.paused.join(", "), describe(&record));
    }
    hold(state, &record)
}

fn resume(state: &Path) -> Result<(), Box<dyn Error>> {
    let Some(record) = pause::load(state)? else {
        println!("not paused");
        return Ok(());
    };
    resume_from(state, &record)?;
    println!("Block List resumed");
    Ok(())
}

/// Wait out a pause in the foreground, or report that nobody will restore it.
fn hold(state: &Path, record: &pause::Pause) -> Result<(), Box<dyn Error>> {
    if !std::io::stdin().is_terminal() {
        println!("nothing will restore this pause automatically; run `blk resume`");
        return Ok(());
    }

    install_signal_handlers();
    if record.until.is_none() {
        println!("Ctrl-C, closing the terminal, or `blk resume` restores the region");
    }

    let mut shown = u64::MAX;
    loop {
        if INTERRUPTED.load(Ordering::SeqCst) {
            break;
        }
        let Some(until) = record.until else {
            std::thread::sleep(Duration::from_secs(1));
            continue;
        };
        let now = pause::now();
        if now >= until {
            break;
        }
        let minutes = (until - now).div_ceil(60);
        if minutes != shown {
            shown = minutes;
            println!("{minutes}m left, until {}", hhmm(until));
        }
        std::thread::sleep(Duration::from_secs(1));
    }

    resume_from(state, record)?;
    println!("Block List resumed");
    Ok(())
}

/// Restore the exact region saved at pause time.
fn resume_from(state: &Path, record: &pause::Pause) -> Result<(), Box<dyn Error>> {
    let hosts = PathBuf::from(&record.hosts_path);
    let Some(snapshot) = pause::load_snapshot(state) else {
        return Err(format!(
            "the saved region for {} is missing; run `blk update` to rebuild it",
            hosts.display()
        )
        .into());
    };
    let contents = read_hosts(&hosts)?;
    let parsed = region::parse(&contents)?;
    require_root(&hosts)?;
    backup(&hosts)?;
    let next = region::replace(&contents, parsed.as_ref(), &snapshot);
    write_hosts(&hosts, &next)?;
    pause::clear(state);
    Ok(())
}

/// A later command takes over an expired pause. Without root, report it instead.
fn end_expired_pause(state: &Path) -> Result<(), Box<dyn Error>> {
    let Some(record) = pause::load(state)? else {
        return Ok(());
    };
    if !record.is_expired(pause::now()) {
        return Ok(());
    }
    if !can_write() {
        eprintln!("warning: pause expired; run sudo blk resume");
        return Ok(());
    }
    resume_from(state, &record)?;
    println!("pause expired; region restored");
    Ok(())
}

fn restore_expired(state: &Path) -> Result<(), Box<dyn Error>> {
    if let Err(err) = end_expired_pause(state) {
        eprintln!("warning: {err}");
    }
    Ok(())
}

fn split_args(args: &[String]) -> Result<(Vec<String>, Option<u64>), Box<dyn Error>> {
    let mut sources = args.to_vec();
    let mut until = None;
    if let Some(last) = sources.last() {
        if let Ok(seconds) = pause::parse_duration(last) {
            sources.pop();
            until = Some(pause::now() + seconds);
        }
    }
    Ok((sources, until))
}

fn describe(record: &pause::Pause) -> String {
    match record.until {
        Some(until) => {
            let left = until.saturating_sub(pause::now());
            format!("until {} ({} left)", hhmm(until), human(left))
        }
        None => "indefinitely".to_string(),
    }
}

fn human(seconds: u64) -> String {
    let minutes = seconds.div_ceil(60);
    if minutes >= 60 {
        format!("{}h{}m", minutes / 60, minutes % 60)
    } else {
        format!("{minutes}m")
    }
}

fn hhmm(epoch: u64) -> String {
    let local = local_time(epoch);
    format!("{:02}:{:02}", local.tm_hour, local.tm_min)
}

fn local_datetime(epoch: u64) -> String {
    let local = local_time(epoch);
    format!(
        "{:04}-{:02}-{:02} {:02}:{:02}",
        local.tm_year + 1900,
        local.tm_mon + 1,
        local.tm_mday,
        local.tm_hour,
        local.tm_min
    )
}

fn local_time(epoch: u64) -> libc::tm {
    let time = epoch as libc::time_t;
    let mut local: libc::tm = unsafe { std::mem::zeroed() };
    unsafe {
        libc::localtime_r(&time, &mut local);
    }
    local
}

extern "C" fn handle_signal(_: libc::c_int) {
    INTERRUPTED.store(true, Ordering::SeqCst);
}

fn install_signal_handlers() {
    let handler = handle_signal as extern "C" fn(libc::c_int);
    unsafe {
        libc::signal(libc::SIGINT, handler as libc::sighandler_t);
        libc::signal(libc::SIGTERM, handler as libc::sighandler_t);
        libc::signal(libc::SIGHUP, handler as libc::sighandler_t);
    }
}

fn catalog_command(id: Option<&str>) -> Result<(), Box<dyn Error>> {
    match id {
        None => {
            for source in catalog::SOURCES {
                let count = source.urls.len();
                println!(
                    "{:<24} {:<9} {:>2} {:<4} {}",
                    source.id,
                    source.format.name(),
                    count,
                    if count == 1 { "url" } else { "urls" },
                    source.licence
                );
            }
        }
        Some(id) => {
            let source = catalog::get_or_error(id)?;
            println!(
                "{} ({}, {})",
                source.id,
                source.format.name(),
                source.licence
            );
            for url in source.urls {
                println!("  {url}");
            }
        }
    }
    Ok(())
}

fn read_hosts(path: &Path) -> Result<String, Box<dyn Error>> {
    fs::read_to_string(path).map_err(|err| format!("cannot read {}: {err}", path.display()).into())
}

/// Writing the hosts file needs root. BLK_DIR marks a test run, where the target is a temp file.
fn can_write() -> bool {
    std::env::var_os("BLK_DIR").is_some() || unsafe { libc::geteuid() } == 0
}

fn require_root(hosts: &Path) -> Result<(), Box<dyn Error>> {
    if can_write() {
        return Ok(());
    }
    Err(format!("writing {} requires root; run with sudo", hosts.display()).into())
}

fn state_dir() -> PathBuf {
    std::env::var_os("BLK_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("/etc/blk"))
}

/// Keep the first version of the hosts file this tool ever modifies.
fn backup(hosts: &Path) -> Result<(), Box<dyn Error>> {
    let dir = state_dir();
    fs::create_dir_all(&dir).map_err(|err| format!("cannot create {}: {err}", dir.display()))?;
    let backup = dir.join("hosts.bak");
    if !backup.exists() {
        fs::copy(hosts, &backup)
            .map_err(|err| format!("cannot write backup {}: {err}", backup.display()))?;
    }
    Ok(())
}

fn write_hosts(path: &Path, contents: &str) -> Result<(), Box<dyn Error>> {
    region::write_atomic(path, contents).map_err(|err| {
        let message = if err.kind() == std::io::ErrorKind::PermissionDenied {
            format!(
                "permission denied writing {}; run with sudo",
                path.display()
            )
        } else {
            format!("cannot write {}: {err}", path.display())
        };
        Box::<dyn Error>::from(message)
    })
}
