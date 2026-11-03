# Changelog

## 2.0.1

Fixed:

- Curated inline domains now also block their `www.` and `m.` hosts. A hosts file matches exact names, so `netflix.com` alone did not block `www.netflix.com`.
- The streaming list includes YouTube front ends (`youtube.com`, `youtu.be`, `youtube-nocookie.com`, `youtubekids.com`, `ytimg.com`, `ytstatic.com`, and related domains). The YouTube source only covered CDN hosts.
- The streaming and social lists cover more services and provider domains.

## 2.0.0

Breaking changes:

- The binary is named `blk`. The package is still `block-list`. `cargo install` leaves a 1.x `block-list` binary in place, so run `cargo uninstall block-list` before installing 2.0.
- The hosts path defaults to `/etc/hosts`. The required path argument is gone; use `--hosts` to override.
- The block region is marked with `# BEGIN blk` and `# END blk`, with one `# list:` section per source. Re-applying writes the same bytes.
- A `# Block List` section from 1.x is left untouched. It is not migrated or removed.
- State, config, cache, and the backup live in `/etc/blk`.

Added:

- A provider catalog with `stevenblack/*`, `oisd/*`, `hagezi/*`, `1hosts/lite`, `peterlowe/hosts`, `malwarefilter/urlhaus`, and `brave/default`.
- Hosts, plain domain, wildcard domain, and strict ABP parsers.
- Conditional GET with ETag caches, `--offline` rebuilds, and `--dry-run`.
- Interactive source selection, `add`, and `rm`.
- Seven purpose lists with `blk lists` and `blk lists toggle <NAME>`, plus a menu manager for toggling, creating, editing, and deleting custom lists. Toggling reuses cached sources.
- Curated inline sources `blk/streaming` and `blk/social`, and Blocklist Project sources for YouTube, piracy, and torrent.
- A grouped TTY menu with a ColorfulTheme, one-line key hints, and a lock or unlock header.
- Pause accepts a bare number as minutes, and the menu pause picker groups enabled lists separately from individual sources.
- Menu screens clear between steps. Escape, blank input, and `c` cancel the current action without exiting.
- A state lock serialises writers. A second `blk` process reports that another process is running instead of racing.
- Pause with a duration, foreground countdown, signal restore, and lazy expiry.
- `status --json`, `test <domain>`, `doctor`, `completions <shell>`, and a menu on a bare invocation.
- Atomic hosts writes, a one-time `hosts.bak`, and a root check with a sudo hint.
- Integration tests on macOS and Ubuntu.
