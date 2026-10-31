# blk

A hosts-based block list manager for macOS and Linux. `blk` keeps one managed region inside your hosts file, leaves the rest of the file alone, and can pause blocking for a set time without losing your settings.

- Blocking applies to every program that uses the system resolver.
- The managed region is visible in the file, between `# BEGIN blk` and `# END blk`.
- Lists refresh with conditional requests, so a daily run downloads nothing when nothing changed.
- Purpose lists bundle sources for social media, streaming, piracy, gambling, adult content, malware, and ads.
- Pausing restores the exact previous region on expiry, Ctrl-C, or terminal close.

## Install

```shell
cargo install block-list
```

The package is `block-list` and the binary is `blk`.

## Quick start

```shell
sudo blk sources      # choose providers interactively
blk lists             # show purpose lists and their state
sudo blk lists toggle "Social media"
sudo blk update       # fetch and write /etc/hosts
blk status            # show what is active
sudo blk doctor       # check the file, cache, backup, and drift
```

Running `blk` with no arguments on a terminal opens a grouped menu: Blocking, Sources, Lists, Tools, and Quit. Each screen prints a one-line key hint, and the header shows a lock or unlock glyph for the current state.

## Commands

| command | what it does |
| --- | --- |
| `blk status [--json]` | show the region, source counts, last update, pause, and warnings |
| `blk update [SOURCE...]` | fetch the configured sources and rewrite the region |
| `blk sources` | choose sources from the catalog, interactive |
| `blk add <SOURCE>...` | add a source and apply |
| `blk rm <SOURCE>...` | remove a source and rebuild |
| `blk pause [SOURCE...] [DURATION]` | pause all or some sources, for example `30m` |
| `blk resume` | restore a paused region |
| `blk off` | pause every source with no deadline |
| `blk test <DOMAIN>` | report whether a domain is blocked |
| `blk doctor` | check markers, cache, backup, and drift; exit 1 on problems |
| `blk config` | open the config file in `$EDITOR` |
| `blk catalog [SOURCE]` | list the built-in sources, or print one source's URLs |
| `blk lists` | show the purpose lists and whether each is on |
| `blk lists toggle <NAME>` | turn a purpose list on or off and apply it |

## Options

| option | meaning |
| --- | --- |
| `--hosts <PATH>` | hosts file to edit, default `/etc/hosts` |
| `--offline` | rebuild from the cache, no network |
| `--dry-run` | print what would change, write nothing |
| `--yes` | skip the confirmation prompt |
| `--quiet` | suppress normal output |

## Purpose lists

A list is a named bundle of sources. Turning one on adds every member source. Turning one off removes them. A list is on when all its members are configured.

| list | sources |
| --- | --- |
| Ads and trackers | `stevenblack/hosts`, `1hosts/lite` |
| Social media | `blk/social` |
| Media streaming | `blk/streaming`, `blocklistproject/youtube` |
| Anime and piracy | `blocklistproject/piracy`, `blocklistproject/torrent` |
| Gambling | `stevenblack/gambling` |
| Adult content | `stevenblack/porn` |
| Malware and phishing | `malwarefilter/urlhaus`, `hagezi/pro` |

`blk/social` and `blk/streaming` are curated domain lists shipped with the binary. They work offline, need no cache, and change only with a new release. The social list contains no WhatsApp domains. Streaming lists block the services themselves, not just their ads, so use them when you want whole-service blocking.

Turning a list off keeps a source that another fully enabled list still needs. Editing an enabled custom list follows the same rule. Preset names are reserved and presets are immutable. Create, edit, and delete custom lists in the menu: run `blk`, choose Lists. Deleting a list keeps its sources configured.

## Pausing

```shell
sudo blk pause stevenblack/social 30m   # pause one source for 30 minutes
sudo blk pause                          # pause every source until Ctrl-C
sudo blk off                            # pause every source indefinitely
sudo blk resume                         # restore now
```

The pause holds the terminal and shows the time left. Expiry, Ctrl-C, SIGTERM, and closing the terminal all restore the previous region. Without a terminal, `pause` exits immediately and says nothing will restore it automatically. If the process is killed with SIGKILL, the record survives and the next command reports the live pause.

## Sources

`blk catalog` prints each source with its format, URL or domain count, and licence. The first few:

```
brave/default            abp       22 urls    MPL-2.0 plus upstream terms
stevenblack/hosts        hosts      1 url     MIT
stevenblack/social       hosts      1 url     MIT
blocklistproject/youtube hosts      1 url     Unlicense
blk/streaming            domains   28 domains curated, MIT
blk/social               domains   29 domains curated, MIT
```

Some sources ship inside the binary as curated domain lists. They need no network and no cache, and `doctor` reports them as inline.

Add a URL or a local list with `blk add`, or append it with `blk update --from`. Both save the source:

```shell
sudo blk add https://example.com/list.hosts
sudo blk update --from /path/to/list.hosts
```

## ABP lists

`brave/default` is an ABP bundle, not a hosts file, so it is converted. The converter keeps `||domain^` rules, subtracts domains named by `@@||domain^` exceptions, and drops cosmetic, path, option, regex, and `$badfilter` rules. It prints how many rules were kept and dropped the first time it fetches a list.

The conversion is approximate. A hosts file cannot express exceptions, paths, options, or cosmetic rules, and it blocks exact hostnames only. See the limits below.

## Files

`blk` keeps its state in `/etc/blk`:

| file | contents |
| --- | --- |
| `config.toml` | the ordered source list and custom purpose lists |
| `state.toml` | last update, region hash, per-source counts |
| `pause.toml` | the current pause record |
| `pause.region` | the region as it was when the pause started |
| `hosts.bak` | a copy of the hosts file from before the first change |
| `cache/` | downloaded lists, with `.etag` and `.src` sidecars |

Set `BLK_DIR` to use a different directory. The test suite uses it.

## How the hosts file changes

`blk` writes one region:

```
# BEGIN blk
# list: stevenblack/social
0.0.0.0 ads.example
# list: hagezi/normal
0.0.0.0 tracker.example
# END blk
```

Only the region is rewritten. Anything before or after it stays as it was. Writes go through a temp file in the same directory, an fsync, and a rename, and the original mode is kept.

## Limits

- A hosts entry blocks one exact hostname. Subdomains are blocked only when the list includes them. ABP's `||example.com^` covers subdomains, which a hosts file cannot.
- ABP conversion is lossy for the reasons above. Brave's bundle is mostly cosmetic and path rules, so only part of it becomes hosts entries.
- A legacy `# Block List` section from version 1 is never read, migrated, or removed. Remove it by hand after upgrading.
- The curated `blk/streaming` and `blk/social` lists only change with a release. They are not refreshed by `blk update`.

## Scheduled updates

Add this to root's crontab to refresh every night at 22:00:

```txt
0 22 * * * blk update
```

Conditional requests mean the run is cheap when no list changed.

## Upgrading from 1.x

- The binary is now `blk`, not `block-list`.
- The hosts path is no longer a positional argument. It defaults to `/etc/hosts`; use `--hosts` to override.
- Lists come from the catalog. Run `blk sources` or `blk add`.
- The region format changed. An old `# Block List` tail is left untouched.
- State lives in `/etc/blk`.

## Development

```shell
cargo test
cargo clippy --all-targets -- -D warnings
cargo fmt --check
```

Network tests are marked ignored:

```shell
cargo test -- --ignored
```
