mod catalog;
mod config;
mod error;
mod hosts;
mod lists;
mod pause;
mod region;
mod sources;
mod state;
mod timefmt;

use std::fs;
use std::io::IsTerminal;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use clap::{Parser, Subcommand};
use dialoguer::console::style;
use dialoguer::theme::ColorfulTheme;
use dialoguer::{Confirm, Input, MultiSelect, Select};

use error::{Error, Result};

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
    /// Show purpose lists, or toggle one.
    Lists {
        #[command(subcommand)]
        action: Option<ListsAction>,
    },
}

#[derive(Debug, Subcommand)]
enum ListsAction {
    /// Toggle a list on or off.
    Toggle {
        #[arg(value_name = "NAME")]
        name: String,
    },
}

fn main() {
    let cli = Cli::parse();
    if let Err(err) = run(&cli) {
        eprintln!("error: {err}");
        std::process::exit(1);
    }
}

fn run(cli: &Cli) -> Result<()> {
    let state = state_dir();
    match &cli.command {
        Some(Cmd::Update { sources, from }) => update(cli, &state, sources, from),
        Some(Cmd::Status { json }) => status(cli, &state, *json),
        Some(Cmd::Test { domain }) => test_domain(cli, &state, domain),
        Some(Cmd::Doctor) => doctor(cli, &state),
        Some(Cmd::Config) => config_command(&state),
        Some(Cmd::Off) => off(cli, &state),
        Some(Cmd::Pause { args }) => pause_command(cli, &state, args),
        Some(Cmd::Resume) => resume(cli, &state),
        Some(Cmd::Sources) => sources_command(cli, &state),
        Some(Cmd::Add { sources }) => add_command(cli, &state, sources),
        Some(Cmd::Rm { sources }) => rm_command(cli, &state, sources),
        Some(Cmd::Catalog { source }) => catalog_command(source.as_deref()),
        Some(Cmd::Lists { action }) => lists_command(cli, &state, action.as_ref()),
        None => bare(cli, &state),
    }
}

fn bare(cli: &Cli, state: &Path) -> Result<()> {
    if std::io::stdin().is_terminal() {
        return menu(cli, state);
    }
    status(cli, state, false)?;
    eprintln!(
        "usage: blk <status|update|sources|add|rm|pause|resume|off|test|doctor|config|catalog|lists>"
    );
    std::process::exit(2);
}

fn theme() -> ColorfulTheme {
    ColorfulTheme::default()
}

fn hint(text: &str) {
    println!("{}", style(text).dim().italic());
}

fn header(cli: &Cli, state: &Path) -> Result<()> {
    let record = pause::load(state)?.filter(|record| !record.is_expired(pause::now()));
    let lock = if record.is_some() { "🔓" } else { "🔒" };
    let summary = match hosts::Hosts::read_lenient(&cli.hosts) {
        Ok(hosts) => match hosts.parsed() {
            Ok(Some(parsed)) => {
                let count = parsed.sections.len();
                let noun = if count == 1 { "section" } else { "sections" };
                format!("{count} {noun}")
            }
            Ok(None) => "no region".to_string(),
            Err(_) => "region error".to_string(),
        },
        Err(_) => "hosts not readable".to_string(),
    };
    println!();
    println!("{}  {}", style(lock).bold(), style("blk").bold());
    println!(
        "{}  {}",
        style(cli.hosts.display()).dim(),
        style(summary).dim()
    );
    if let Some(record) = record {
        println!(
            "{}",
            style(format!("paused: {}", record.describe())).yellow()
        );
    }
    Ok(())
}

/// Keep the menu open after a failed action, but let a cancelled prompt abort.
fn attempt(outcome: Result<()>) -> Result<()> {
    match outcome {
        Ok(()) => Ok(()),
        Err(Error::Dialog(error)) => Err(Error::Dialog(error)),
        Err(error) => {
            eprintln!("{error}");
            Ok(())
        }
    }
}

fn submenu(prompt: &str, items: &[&str]) -> Result<Option<usize>> {
    hint("up/down move, enter select");
    let pick = Select::with_theme(&theme())
        .with_prompt(prompt)
        .items(items)
        .default(0)
        .interact()?;
    Ok((pick + 1 != items.len()).then_some(pick))
}

fn menu(cli: &Cli, state: &Path) -> Result<()> {
    // Choosing in the menu is the confirmation, so no second prompt.
    let cli = Cli {
        hosts: cli.hosts.clone(),
        quiet: cli.quiet,
        dry_run: cli.dry_run,
        offline: cli.offline,
        yes: true,
        command: None,
    };
    loop {
        header(&cli, state)?;
        hint("up/down move, enter open");
        let pick = Select::with_theme(&theme())
            .with_prompt("blk")
            .items(&["Blocking", "Sources", "Lists", "Tools", "Quit"])
            .default(0)
            .interact()?;
        let outcome = match pick {
            0 => blocking_menu(&cli, state),
            1 => sources_menu(&cli, state),
            2 => lists_menu(&cli, state),
            3 => tools_menu(&cli, state),
            _ => return Ok(()),
        };
        attempt(outcome)?;
    }
}

fn blocking_menu(cli: &Cli, state: &Path) -> Result<()> {
    loop {
        let Some(pick) = submenu(
            "Blocking",
            &[
                "Show status",
                "Update now",
                "Pause",
                "Resume",
                "Turn off",
                "Back",
            ],
        )?
        else {
            return Ok(());
        };
        let outcome = match pick {
            0 => status(cli, state, false),
            1 => update(cli, state, &[], &[]),
            2 => pause_menu(cli, state),
            3 => resume(cli, state),
            4 => off(cli, state),
            _ => Ok(()),
        };
        attempt(outcome)?;
    }
}

fn sources_menu(cli: &Cli, state: &Path) -> Result<()> {
    loop {
        let Some(pick) = submenu(
            "Sources",
            &[
                "Choose sources",
                "Add a source",
                "Remove a source",
                "Browse catalog",
                "Back",
            ],
        )?
        else {
            return Ok(());
        };
        match pick {
            0 => attempt(sources_command(cli, state))?,
            1 => attempt(add_source_menu(cli, state))?,
            2 => attempt(remove_sources_menu(cli, state))?,
            3 => attempt(catalog_command(None))?,
            _ => {}
        }
    }
}

fn add_source_menu(cli: &Cli, state: &Path) -> Result<()> {
    hint("catalog id, URL, or local path");
    let source: String = Input::with_theme(&theme())
        .with_prompt("Source")
        .interact_text()?;
    let source = source.trim();
    if source.is_empty() {
        return Ok(());
    }
    add_command(cli, state, &[source.to_string()])
}

fn remove_sources_menu(cli: &Cli, state: &Path) -> Result<()> {
    let config = config::load(state)?;
    if config.sources.is_empty() {
        println!("no sources configured");
        return Ok(());
    }
    hint("space toggle, a select all, enter confirm");
    let picked = MultiSelect::with_theme(&theme())
        .with_prompt("Remove which sources?")
        .items(&config.sources)
        .interact()?;
    if picked.is_empty() {
        return Ok(());
    }
    let chosen: Vec<String> = picked
        .iter()
        .map(|&index| config.sources[index].clone())
        .collect();
    rm_command(cli, state, &chosen)
}

fn tools_menu(cli: &Cli, state: &Path) -> Result<()> {
    loop {
        let Some(pick) = submenu(
            "Tools",
            &["Test a domain", "Run doctor", "Edit config", "Back"],
        )?
        else {
            return Ok(());
        };
        match pick {
            0 => attempt(test_menu(cli, state))?,
            1 => attempt(doctor(cli, state))?,
            2 => attempt(config_command(state))?,
            _ => {}
        }
    }
}

fn test_menu(cli: &Cli, state: &Path) -> Result<()> {
    hint("exact hostname, for example ads.example");
    let domain: String = Input::with_theme(&theme())
        .with_prompt("Domain")
        .interact_text()?;
    test_domain(cli, state, domain.trim())
}

fn lists_menu(cli: &Cli, state: &Path) -> Result<()> {
    loop {
        let mut config = config::load(state)?;
        let views = lists::views(&config);
        let mut items: Vec<String> = views
            .iter()
            .map(|view| {
                let mark = if view.enabled { "[on]" } else { "[off]" };
                let kind = if view.custom { "custom" } else { "preset" };
                let count = view.sources.len();
                let unit = if count == 1 { "source" } else { "sources" };
                format!("{mark} {:<22} {kind:<6} {count} {unit}", view.name)
            })
            .collect();
        items.push("Create a list".to_string());
        items.push("Back".to_string());
        hint("up/down move, enter choose");
        let pick = Select::with_theme(&theme())
            .with_prompt("Lists")
            .items(&items)
            .default(0)
            .interact()?;
        if pick == views.len() {
            attempt(create_list_menu(cli, state, &mut config))?;
            continue;
        }
        if pick > views.len() {
            return Ok(());
        }
        attempt(list_menu(cli, state, &mut config, &views[pick]))?;
    }
}

fn list_menu(
    cli: &Cli,
    state: &Path,
    config: &mut config::Config,
    view: &lists::View,
) -> Result<()> {
    let toggle_label = if view.enabled { "Turn off" } else { "Turn on" };
    let mut items = vec![toggle_label];
    if view.custom {
        items.push("Edit sources");
        items.push("Delete list");
    }
    items.push("Back");
    let prompt = format!("List: {}", view.name);
    let Some(pick) = submenu(&prompt, &items)? else {
        return Ok(());
    };
    match pick {
        0 => {
            let result = lists::toggle(config, &view.name)?;
            let message = toggle_message(&view.name, &result);
            apply_lists(cli, state, config, &message)
        }
        1 if view.custom => edit_list_menu(cli, state, config, view),
        2 if view.custom => delete_list_menu(cli, state, config, view),
        _ => Ok(()),
    }
}

fn create_list_menu(cli: &Cli, state: &Path, config: &mut config::Config) -> Result<()> {
    hint("preset names are reserved");
    let name: String = Input::with_theme(&theme())
        .with_prompt("List name")
        .interact_text()?;
    let name = name.trim().to_string();
    if name.is_empty() {
        return Err(Error::EmptyListName);
    }
    let sources = pick_sources(&[])?;
    lists::create(config, &name, sources)?;
    if cli.dry_run {
        println!("would create list {name}");
        return Ok(());
    }
    config::save(state, config)?;
    if !cli.quiet {
        println!("list {name} created");
    }
    Ok(())
}

fn edit_list_menu(
    cli: &Cli,
    state: &Path,
    config: &mut config::Config,
    view: &lists::View,
) -> Result<()> {
    let before = config.sources.clone();
    let sources = pick_sources(&view.sources)?;
    lists::edit(config, &view.name, sources)?;
    if cli.dry_run {
        println!("would update list {}", view.name);
        return Ok(());
    }
    if config.sources == before {
        config::save(state, config)?;
        if !cli.quiet {
            println!("list {} updated", view.name);
        }
        return Ok(());
    }
    apply_lists(cli, state, config, &format!("list {} updated", view.name))
}

fn delete_list_menu(
    cli: &Cli,
    state: &Path,
    config: &mut config::Config,
    view: &lists::View,
) -> Result<()> {
    let confirmed = Confirm::with_theme(&theme())
        .with_prompt(format!("Delete {}? Its sources stay configured", view.name))
        .default(false)
        .interact()?;
    if !confirmed {
        return Ok(());
    }
    lists::delete(config, &view.name)?;
    if cli.dry_run {
        println!("would delete list {}", view.name);
        return Ok(());
    }
    config::save(state, config)?;
    if !cli.quiet {
        println!("list {} deleted", view.name);
    }
    Ok(())
}

fn apply_lists(cli: &Cli, state: &Path, config: &config::Config, message: &str) -> Result<()> {
    change_sources(
        cli,
        state,
        config.sources.clone(),
        config.lists.clone(),
        message,
    )
}

fn pick_sources(preselected: &[String]) -> Result<Vec<String>> {
    let ids = catalog::ids();
    let mut items: Vec<String> = ids.iter().map(|id| id.to_string()).collect();
    for source in preselected {
        if catalog::get(source).is_none() && !items.contains(source) {
            items.push(source.clone());
        }
    }
    let defaults: Vec<bool> = items
        .iter()
        .map(|item| preselected.iter().any(|source| source == item))
        .collect();
    hint("space toggle, a select all, enter confirm");
    let picked = MultiSelect::with_theme(&theme())
        .with_prompt("Sources")
        .items(&items)
        .defaults(&defaults)
        .interact()?;
    let mut chosen: Vec<String> = picked.iter().map(|&index| items[index].clone()).collect();
    hint("blank to skip");
    let extra: String = Input::with_theme(&theme())
        .with_prompt("Add a URL or path")
        .allow_empty(true)
        .interact_text()?;
    let extra = extra.trim();
    if !extra.is_empty() && !chosen.iter().any(|source| source == extra) {
        chosen.push(extra.to_string());
    }
    Ok(chosen)
}

fn toggle_message(name: &str, result: &lists::Toggle) -> String {
    if result.enabled {
        if result.kept_by.is_empty() {
            format!("{name} turned on")
        } else {
            format!(
                "{name} stays on; sources kept for {}",
                result.kept_by.join(", ")
            )
        }
    } else if result.kept_by.is_empty() {
        format!("{name} turned off")
    } else {
        format!(
            "{name} turned off; sources kept for {}",
            result.kept_by.join(", ")
        )
    }
}

fn pause_menu(cli: &Cli, state: &Path) -> Result<()> {
    let hosts = hosts::Hosts::read(&cli.hosts)?;
    let Some(parsed) = hosts.parsed()? else {
        println!("no block region");
        return Ok(());
    };
    let names: Vec<String> = parsed
        .sections
        .iter()
        .map(|section| section.name.clone())
        .collect();
    let defaults = vec![true; names.len()];
    hint("space toggle, a select all, enter confirm");
    let picked = MultiSelect::with_theme(&theme())
        .with_prompt("Pause which sources?")
        .items(&names)
        .defaults(&defaults)
        .interact()?;
    if picked.is_empty() {
        println!("nothing selected");
        return Ok(());
    }
    hint("30m, 2h, or blank to hold until Ctrl-C");
    let duration: String = Input::with_theme(&theme())
        .with_prompt("Duration")
        .allow_empty(true)
        .interact_text()?;
    let mut args: Vec<String> = picked.iter().map(|&index| names[index].clone()).collect();
    let duration = duration.trim();
    if !duration.is_empty() {
        args.push(duration.to_string());
    }
    pause_command(cli, state, &args)
}

fn update(cli: &Cli, state: &Path, args: &[String], from: &[String]) -> Result<()> {
    if !cli.dry_run {
        restore_expired(state)?;
    }
    let config = config::load(state)?;
    let mut sources = if args.is_empty() {
        config.sources
    } else {
        Vec::new()
    };
    sources.extend(args.iter().chain(from.iter()).cloned());
    let sources = dedupe(sources);
    let persist = if !args.is_empty() || !from.is_empty() {
        Some(config::Config {
            sources: sources.clone(),
            lists: config.lists,
        })
    } else {
        None
    };
    apply(cli, state, &sources, persist.as_ref())?;
    Ok(())
}

/// Fetch or read every source, then replace the region in one write.
fn apply(
    cli: &Cli,
    state: &Path,
    sources: &[String],
    persist: Option<&config::Config>,
) -> Result<bool> {
    if sources.is_empty() {
        return Err(Error::NoSourcesConfigured);
    }

    let hosts = hosts::Hosts::read(&cli.hosts)?;
    let loaded = sources::load_all(state, sources, cli.offline, !cli.dry_run)?;

    for source in &loaded {
        for warning in &source.warnings {
            eprintln!("warning: {}: {warning}", source.name);
        }
        if let Some(counts) = source.counts {
            if !cli.quiet {
                println!(
                    "{}: kept {}, dropped {} cosmetic, {} path or option, {} exceptions subtracted",
                    source.name, counts.kept, counts.cosmetic, counts.other, counts.exceptions
                );
            }
        }
    }

    let sections: Vec<region::Section> = loaded
        .iter()
        .map(|source| region::Section {
            name: source.name.clone(),
            entries: source.entries.clone(),
        })
        .collect();
    let total: usize = sections.iter().map(|section| section.entries.len()).sum();
    let rendered = region::render(&sections);
    let noun = if sources.len() == 1 {
        "source"
    } else {
        "sources"
    };

    if cli.dry_run {
        if !cli.quiet {
            for section in &sections {
                println!("{}: {} entries", section.name, section.entries.len());
            }
            println!(
                "would write {total} entries from {} {noun} to {}",
                sources.len(),
                cli.hosts.display()
            );
        }
        return Ok(false);
    }

    if !cli.yes && !cli.quiet && std::io::stdin().is_terminal() {
        let confirmed = Confirm::with_theme(&theme())
            .with_prompt(format!(
                "Write {total} entries from {} {noun} to {}?",
                sources.len(),
                cli.hosts.display()
            ))
            .default(false)
            .interact()?;
        if !confirmed {
            return Ok(false);
        }
    }

    hosts.commit(state, &rendered)?;

    state::save(
        state,
        &state::State {
            last_update: pause::now(),
            region_hash: state::hash_region(&rendered),
        },
    )?;
    if let Some(persist) = persist {
        config::save(state, persist)?;
    }
    pause::clear(state);

    if !cli.quiet {
        println!(
            "Block List updated: {total} entries from {} {noun}",
            sources.len()
        );
    }
    Ok(true)
}

fn sources_command(cli: &Cli, state: &Path) -> Result<()> {
    if !cli.dry_run {
        restore_expired(state)?;
    }
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
    hint("space toggle, a select all, enter confirm");
    let picked = MultiSelect::with_theme(&theme())
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

    hint("blank to skip");
    let extra: String = Input::with_theme(&theme())
        .with_prompt("Add a URL or path")
        .allow_empty(true)
        .interact_text()?;
    let extra = extra.trim();
    if !extra.is_empty() && !chosen.iter().any(|source| source == extra) {
        chosen.push(extra.to_string());
    }

    change_sources(cli, state, chosen, config.lists, "sources selected")
}

fn lists_command(cli: &Cli, state: &Path, action: Option<&ListsAction>) -> Result<()> {
    if !cli.dry_run {
        restore_expired(state)?;
    }
    match action {
        None => {
            let config = config::load(state)?;
            for view in lists::views(&config) {
                let state = if view.enabled { "on" } else { "off" };
                let kind = if view.custom { "custom" } else { "preset" };
                let count = view.sources.len();
                let unit = if count == 1 { "source" } else { "sources" };
                println!("[{state}] {:<22} {kind:<7} {count} {unit}", view.name);
            }
            println!("toggle with `blk lists toggle <name>`; create and edit in `blk`");
            Ok(())
        }
        Some(ListsAction::Toggle { name }) => {
            let mut config = config::load(state)?;
            let result = lists::toggle(&mut config, name)?;
            let message = toggle_message(name, &result);
            change_sources(cli, state, config.sources, config.lists, &message)
        }
    }
}

fn add_command(cli: &Cli, state: &Path, sources: &[String]) -> Result<()> {
    for source in sources {
        validate_source(source)?;
    }
    let config = config::load(state)?;
    let mut chosen = config.sources;
    chosen.extend(sources.iter().cloned());
    change_sources(cli, state, chosen, config.lists, "sources added")
}

fn rm_command(cli: &Cli, state: &Path, sources: &[String]) -> Result<()> {
    let config = config::load(state)?;
    for source in sources {
        if !config.sources.contains(source) {
            return Err(Error::SourceNotConfigured {
                source: source.clone(),
            });
        }
    }
    let remaining = config
        .sources
        .into_iter()
        .filter(|source| !sources.contains(source))
        .collect();
    change_sources(cli, state, remaining, config.lists, "sources removed")
}

/// Save a new source list, then apply it unless a live pause holds the region.
fn change_sources(
    cli: &Cli,
    state: &Path,
    sources: Vec<String>,
    lists: Vec<config::List>,
    message: &str,
) -> Result<()> {
    let sources = dedupe(sources);
    let next = config::Config {
        sources: sources.clone(),
        lists,
    };

    if cli.dry_run {
        for source in &sources {
            println!("{source}");
        }
        if !cli.quiet {
            let noun = if sources.len() == 1 {
                "source"
            } else {
                "sources"
            };
            println!("would save and apply {} {noun}", sources.len());
        }
        return Ok(());
    }

    if let Some(record) = pause::load(state)? {
        if !record.is_expired(pause::now()) {
            config::save(state, &next)?;
            if !cli.quiet {
                println!("{message}; the pause is still in effect, run `blk resume`");
            }
            return Ok(());
        }
        pause::resume(state, &record)?;
    }

    if sources.is_empty() {
        clear_region(cli, state)?;
        config::save(state, &next)?;
        if !cli.quiet {
            println!("no sources configured; region removed");
            println!("{message}");
        }
        return Ok(());
    }

    let applied = apply(cli, state, &sources, Some(&next))?;
    if applied && !cli.quiet {
        println!("{message}");
    }
    Ok(())
}

fn validate_source(source: &str) -> Result<()> {
    if catalog::get(source).is_some()
        || source.starts_with("http://")
        || source.starts_with("https://")
        || Path::new(source).exists()
    {
        return Ok(());
    }
    Err(Error::unknown_source(
        source,
        catalog::ids().iter().map(|id| id.to_string()).collect(),
    ))
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

fn clear_region(cli: &Cli, state: &Path) -> Result<()> {
    let hosts = hosts::Hosts::read(&cli.hosts)?;
    if hosts.parsed()?.is_none() {
        return Ok(());
    }
    hosts.commit(state, "")?;
    let _ = fs::remove_file(state::path(state));
    pause::clear(state);
    Ok(())
}

fn status(cli: &Cli, state: &Path, json: bool) -> Result<()> {
    if !cli.dry_run {
        restore_expired(state)?;
    }

    let hosts = hosts::Hosts::read(&cli.hosts)?;
    let record = pause::load(state)?;

    if json {
        return status_json(cli, state, &hosts, record.as_ref());
    }

    if let Some(update) = state::load(state)? {
        if !cli.quiet {
            println!(
                "last update: {}",
                timefmt::local_datetime(update.last_update)
            );
        }
    }

    if !cli.quiet {
        for warning in status_warnings(state, &hosts)? {
            println!("warning: {warning}");
        }
    }

    match hosts.parsed()? {
        Some(parsed) => {
            let total: usize = parsed
                .sections
                .iter()
                .map(|section| section.entries.len())
                .sum();
            if !cli.quiet {
                let noun = if parsed.sections.len() == 1 {
                    "source"
                } else {
                    "sources"
                };
                println!(
                    "{}: {total} entries in {} {noun}",
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
            println!("paused: {names} {}", record.describe());
        }
    }
    Ok(())
}

fn status_json(
    cli: &Cli,
    state: &Path,
    hosts: &hosts::Hosts,
    record: Option<&pause::Pause>,
) -> Result<()> {
    let parsed = hosts.parsed()?;
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
    let last = state::load(state)?.map(|update| timefmt::local_datetime(update.last_update));
    let warnings = status_warnings(state, hosts)?;

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
                Some(until) => json_string(&timefmt::local_datetime(until)),
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

fn status_warnings(state: &Path, hosts: &hosts::Hosts) -> Result<Vec<String>> {
    let mut warnings = Vec::new();
    if hosts.contents().contains("# Block List") {
        warnings.push("legacy-block-list".to_string());
    }
    let record = pause::load(state)?;
    let now = pause::now();
    let paused = record
        .as_ref()
        .is_some_and(|record| !record.is_expired(now));
    if !paused {
        if let (Some(region), Some(update)) = (hosts.region()?, state::load(state)?) {
            if state::hash_region(region) != update.region_hash {
                warnings.push("region-drift".to_string());
            }
        }
    }
    if record.as_ref().is_some_and(|record| record.is_expired(now)) {
        warnings.push("pause-expired".to_string());
    }
    for source in config::load(state)?.sources {
        if let Some(entry) = catalog::get(&source) {
            if !entry.is_inline() && !sources::cache_path(state, &source).exists() {
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
            c if c.is_control() => out.push_str(&format!("\\u{:04x}", c as u32)),
            _ => out.push(c),
        }
    }
    out.push('"');
    out
}

fn test_domain(cli: &Cli, state: &Path, domain: &str) -> Result<()> {
    if !cli.dry_run {
        restore_expired(state)?;
    }
    let hosts = hosts::Hosts::read(&cli.hosts)?;

    if let Some(parsed) = hosts.parsed()? {
        for section in &parsed.sections {
            for entry in &section.entries {
                if entry.host.eq_ignore_ascii_case(domain) {
                    println!("blocked by blk: {} {}", entry.ip, entry.host);
                    return Ok(());
                }
            }
        }
    }

    for line in hosts.outside()?.lines() {
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

fn doctor(cli: &Cli, state: &Path) -> Result<()> {
    if !cli.dry_run {
        restore_expired(state)?;
    }
    let mut problems = 0;
    if !hosts::can_write() {
        println!("root: not running as root; writes would need sudo");
    }

    let hosts = hosts::Hosts::read_lenient(&cli.hosts)?;
    let parsed = match hosts.parsed() {
        Ok(parsed) => parsed,
        Err(err) => {
            problems += 1;
            println!("region: {err}");
            None
        }
    };
    match &parsed {
        Some(parsed) if parsed.sections.len() == 1 => {
            println!("region: ok, 1 section");
        }
        Some(parsed) => println!("region: ok, {} sections", parsed.sections.len()),
        None => println!("region: none"),
    }

    for source in config::load(state)?.sources {
        let Some(entry) = catalog::get(&source) else {
            continue;
        };
        if entry.is_inline() {
            println!("cache: {source} inline, {} domains", entry.inline.len());
            continue;
        }
        let path = sources::cache_path(state, &source);
        if !path.exists() {
            problems += 1;
            println!("cache: missing for {source}");
            continue;
        }
        println!("cache: {source} present");
        if entry.format == catalog::Format::Abp {
            if let Some(counts) = sources::cached_counts(state, &source) {
                println!(
                    "  abp: kept {}, dropped {} cosmetic and {} path or option rules",
                    counts.kept, counts.cosmetic, counts.other
                );
            }
        }
    }

    if parsed.is_some() && !state.join("hosts.bak").exists() {
        println!("backup: none yet");
    }
    if hosts.contents().contains("# Block List") {
        println!("warning: a legacy # Block List is present; blk leaves it alone");
    }
    let paused = pause::load(state)?
        .filter(|record| !record.is_expired(pause::now()))
        .is_some();
    if let (Some(_), Some(update)) = (&parsed, state::load(state)?) {
        let region = hosts.region()?.unwrap_or_default();
        if paused {
            println!("drift: skipped while paused");
        } else if state::hash_region(region) == update.region_hash {
            println!("drift: none");
        } else {
            problems += 1;
            println!("drift: region changed since the last update");
        }
    }

    if problems > 0 {
        Err(Error::DoctorProblems { count: problems })
    } else {
        println!("doctor: ok");
        Ok(())
    }
}

fn config_command(state: &Path) -> Result<()> {
    fs::create_dir_all(state)
        .map_err(|err| Error::io("create", state.display().to_string(), err))?;
    let path = config::path(state);
    if !path.exists() {
        config::save(state, &config::Config::default())?;
    }
    let editor = std::env::var("EDITOR").unwrap_or_else(|_| "vi".to_string());
    let status = std::process::Command::new(&editor)
        .arg(&path)
        .status()
        .map_err(|err| Error::io("start", editor.clone(), err))?;
    if !status.success() {
        return Err(Error::EditorFailed);
    }
    Ok(())
}

fn off(cli: &Cli, state: &Path) -> Result<()> {
    if !cli.dry_run {
        end_expired_pause(state)?;
    }
    if let Some(record) = pause::load(state)? {
        if !(cli.dry_run && record.is_expired(pause::now())) {
            return Err(Error::AlreadyPaused(record));
        }
    }

    let hosts = hosts::Hosts::read(&cli.hosts)?;
    let Some(parsed) = hosts.parsed()? else {
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

    let record = pause::Pause::plan(&cli.hosts, parsed, &[], None)?;
    pause::begin(state, &hosts, &record)?;
    if !cli.quiet {
        println!("Block List paused (off). Run `blk resume` to restore it.");
    }
    Ok(())
}

fn pause_command(cli: &Cli, state: &Path, args: &[String]) -> Result<()> {
    let (sources, until) = split_args(args);
    if !cli.dry_run {
        end_expired_pause(state)?;
    }
    if let Some(record) = pause::load(state)? {
        if !(cli.dry_run && record.is_expired(pause::now())) {
            return Err(Error::AlreadyPaused(record));
        }
    }

    let hosts = hosts::Hosts::read(&cli.hosts)?;
    let Some(parsed) = hosts.parsed()? else {
        return Err(Error::NoBlockRegion {
            hosts: cli.hosts.clone(),
        });
    };

    let record = pause::Pause::plan(&cli.hosts, parsed, &sources, until)?;

    if cli.dry_run {
        if !cli.quiet {
            println!(
                "would pause {} {}",
                record.paused.join(", "),
                record.describe()
            );
        }
        return Ok(());
    }

    pause::begin(state, &hosts, &record)?;

    if !cli.quiet {
        println!("paused {} {}", record.paused.join(", "), record.describe());
    }
    hold(state, &record)
}

fn resume(cli: &Cli, state: &Path) -> Result<()> {
    let Some(record) = pause::load(state)? else {
        println!("not paused");
        return Ok(());
    };
    if cli.dry_run {
        if !cli.quiet {
            println!(
                "would restore {} and resume {}",
                record.hosts_path.display(),
                record.paused.join(", ")
            );
        }
        return Ok(());
    }
    pause::resume(state, &record)?;
    println!("Block List resumed");
    Ok(())
}

/// Wait out a pause in the foreground, or report that nobody will restore it.
fn hold(state: &Path, record: &pause::Pause) -> Result<()> {
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
            println!("{minutes}m left, until {}", timefmt::local_hhmm(until));
        }
        std::thread::sleep(Duration::from_secs(1));
    }

    if pause::load(state)?.is_none() {
        println!("pause was already resumed elsewhere");
        return Ok(());
    }
    pause::resume(state, record)?;
    println!("Block List resumed");
    Ok(())
}

/// A later command takes over an expired pause. Without root, report it instead.
fn end_expired_pause(state: &Path) -> Result<()> {
    match pause::expire(state)? {
        pause::Expiry::None => {}
        pause::Expiry::Resumed => println!("pause expired; region restored"),
        pause::Expiry::NeedsRoot => {
            eprintln!("warning: pause expired; run sudo blk resume");
        }
    }
    Ok(())
}

fn restore_expired(state: &Path) -> Result<()> {
    if let Err(err) = end_expired_pause(state) {
        eprintln!("warning: {err}");
    }
    Ok(())
}

fn split_args(args: &[String]) -> (Vec<String>, Option<u64>) {
    let mut sources = args.to_vec();
    let mut until = None;
    if let Some(last) = sources.last() {
        if let Ok(seconds) = pause::parse_duration(last) {
            sources.pop();
            until = Some(pause::now() + seconds);
        }
    }
    (sources, until)
}

extern "C" fn handle_signal(_: libc::c_int) {
    INTERRUPTED.store(true, Ordering::SeqCst);
}

fn install_signal_handlers() {
    INTERRUPTED.store(false, Ordering::SeqCst);
    let handler = handle_signal as extern "C" fn(libc::c_int);
    unsafe {
        libc::signal(libc::SIGINT, handler as libc::sighandler_t);
        libc::signal(libc::SIGTERM, handler as libc::sighandler_t);
        libc::signal(libc::SIGHUP, handler as libc::sighandler_t);
    }
}

fn catalog_command(id: Option<&str>) -> Result<()> {
    match id {
        None => {
            for source in catalog::SOURCES {
                let count = if source.is_inline() {
                    source.inline.len()
                } else {
                    source.urls.len()
                };
                let unit = if source.is_inline() {
                    if count == 1 {
                        "domain"
                    } else {
                        "domains"
                    }
                } else if count == 1 {
                    "url"
                } else {
                    "urls"
                };
                println!(
                    "{:<24} {:<9} {:>2} {:<7} {}",
                    source.id,
                    source.format.name(),
                    count,
                    unit,
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
            if source.is_inline() {
                for domain in source.inline {
                    println!("  {domain}");
                }
            } else {
                for url in source.urls {
                    println!("  {url}");
                }
            }
        }
    }
    Ok(())
}

fn state_dir() -> PathBuf {
    std::env::var_os("BLK_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("/etc/blk"))
}
