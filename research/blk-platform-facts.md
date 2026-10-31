# Platform facts for blk 2.0

Checks ran on 2026-10-31 on a Mac running macOS 26.6.2, build 25G83, Darwin kernel 25.6.0, arm64. The ticket calls this macOS 25.6; 25.6.0 is the Darwin kernel version.

Tools used: rustc 1.97.1, curl 8.7.1 (x86_64-apple-darwin25.0).

## ureq 2.5.0

The local `Cargo.lock` pins ureq 2.5.0, published 2022-07-10 (`https://crates.io/api/v1/crates/ureq/2.5.0`). That lock file is not tracked by git. `Cargo.toml` says `ureq = "2.5.0"`, which is a caret requirement, so a fresh resolve can pick a later 2.x.

### TLS backend and default features

The crate's `Cargo.toml.orig` at version 2.5.0:

```toml
[features]
default = ["tls", "gzip"]
tls = ["webpki", "webpki-roots", "rustls"]
```

The default build does use rustls, with a dependency requirement of 0.20.1, and webpki-roots. native-tls is optional, is never picked up by the crate-level `ureq::get` and friends, and the `native-certs` feature only replaces the trust store with the OS store.

Reproduce with:

```sh
curl -sSLO https://static.crates.io/crates/ureq/ureq-2.5.0.crate
tar xzf ureq-2.5.0.crate
sed -n '/^\[features\]/,/^$/p' ureq-2.5.0/Cargo.toml.orig
```

### Setting request headers

`Request::set(name, value)` adds a header and returns the request, so calls chain. `Request::header(name)` reads a header back during building. Header names are matched case-insensitively.

```rust
let resp = ureq::get(url)
    .set("If-None-Match", &etag)
    .call()?;
```

### Reading response headers

`Response::header(name) -> Option<&str>` returns the first matching value and compares names case-insensitively (`src/header.rs:153`, `is_name` uses `eq_ignore_ascii_case`). `Response::all(name)` returns every value for repeated headers. ETag and Last-Modified read as `resp.header("etag")` and `resp.header("last-modified")`.

### Timeouts

Defaults in `AgentBuilder::new`, `src/agent.rs:251`:

```rust
timeout_connect: Some(Duration::from_secs(30)),
timeout_read: None,
timeout_write: None,
timeout: None,
```

`Request::timeout(d)` sets an overall deadline for that request. It overrides the agent's overall timeout but not the connect timeout. The docs on `AgentBuilder::timeout` state that DNS resolution cannot be interrupted by the deadline, so a slow DNS lookup can exceed it. `timeout_read` and `timeout_write` apply to each read and write call.

### Redirects

`AgentBuilder::redirects(n)` sets how many redirects to follow. The default is 5. Setting 0 disables following and returns the 3xx response as `Ok`. Exceeding the limit returns the transport error kind `TooManyRedirects`. A 307 or 308 that has a body is not followed.

### Non-200 responses

`Request::do_call` in `src/request.rs:169`:

```rust
if response.status() >= 400 {
    Err(Error::Status(response.status(), response))
} else {
    Ok(response)
}
```

So a 2xx is `Ok(Response)`. A 4xx or 5xx, including 404, is `Err(Error::Status(code, Response))`. A 3xx is `Ok` when redirects are disabled, and is followed otherwise. A 304 is `Ok` because it is below 400; it is bodyless, and `Response::stream_to_reader` treats 204 and 304 as having no body. The probe below printed `Err::Status code=404 resp_status=404` for a missing file.

### ETag depends on content coding

The gzip feature is on by default and adds `accept-encoding: gzip` unless the caller already set `accept-encoding` or `range` (`src/request.rs:93`).

raw.githubusercontent.com returns a different ETag for each content coding. The identity response has a strong ETag and the gzip response a weak one:

```
identity: "c67d7efa0279a993dd9a8848ec00d652da15056d5eef93333c3aa22ef3f26031"
gzip:     W/"2436c305c9b5dd03e1d87edfa73f8981d1703a3e9f84088d4057734257956336"
```

Both responses carry `vary: Authorization,Accept-Encoding`. A conditional request made with the other representation's ETag returns 200. A client must therefore reuse the ETag from the same configuration. With ureq's default features that is the weak gzip ETag, and the probe confirmed a 304 with it.

Probe built in `/tmp/ureqprobe` with `ureq = "=2.5.0"`:

```rust
let url = "https://raw.githubusercontent.com/StevenBlack/hosts/master/hosts";
let first = ureq::get(url).call().expect("first request");
println!("first: status={} etag={:?} last_modified={:?}",
    first.status(), first.header("etag"), first.header("last-modified"));
let etag = first.header("etag").unwrap().to_string();
match ureq::get(url).set("If-None-Match", &etag).call() {
    Ok(r) => println!("second: Ok status={}", r.status()),
    Err(e) => println!("second: Err {:?}", e),
}
```

Output:

```
first: status=200 etag=Some("W/\"2436c305c9b5dd03e1d87edfa73f8981d1703a3e9f84088d4057734257956336\"") last_modified=None
second: Ok status=304 etag=Some("W/\"2436c305c9b5dd03e1d87edfa73f8981d1703a3e9f84088d4057734257956336\"")
missing: Err::Status code=404 resp_status=404
```

This also shows the 304 keeps its ETag header and is not an error, and that Last-Modified is absent.

### Minimum Rust version

No MSRV is declared. The 2.5.0 `Cargo.toml` has no `rust-version` field, the crates.io version metadata has `"rust_version": null`, the README and CHANGELOG do not state one, and `.github/workflows/test.yml` at tag 2.5.0 tests only `toolchain: stable`. Treat this as unconfirmed. Building the probe with rustc 1.97.1 worked but says nothing about older compilers.

## /etc/hosts edits and resolver caches

### macOS

`man 5 hosts` says the file "is read by mDNSResponder(8) and used to supply results for calls to getaddrinfo(3), getnameinfo(3), etc." So the daemon owns hosts lookups, not a separate NSS files module.

The installed daemon reports version mDNSResponder-2881.160.4 (`strings /usr/sbin/mDNSResponder | grep mDNSResponder-`). Its strings show that it watches the file and re-parses it:

```
mDNSMacOSXGetEtcHostsFD: /etc/hosts being monitored, and not etc
mDNSMacOSXGetEtcHostsFD: /etc/hosts changed 0x%x
mDNSMacOSXUpdateEtcHosts: Parsing /etc/hosts fd %d
```

The mDNSResponder source adds hosts entries as local-only authoritative records and has a restart path whose comment says it "is called when /etc/hosts changes" (`mDNSCore/mDNS.c:6437` in https://github.com/apple-oss-distributions/mDNSResponder/tree/mDNSResponder-2881.120.11). So an edit is picked up by the daemon itself, and no flush is needed for a new lookup to see it.

One limit on that answer: this task forbids writing /etc/hosts, so the end-to-end check (edit, resolve, observe) was not run. The conclusion rests on the daemon binary and source plus the man page.

The flush command, if you want to drop cached DNS answers anyway, is the one Apple documents at https://support.apple.com/en-us/101481 (archived, published 2023-10-19). For OS X 10.10.4 and later it gives:

```sh
sudo killall -HUP mDNSResponder
```

The daemon binary contains the string `SIGHUP: Purge cache`, which matches that command. `man 1 dscacheutil` describes `-flushcache` as flushing the Directory Service cache, and Apple's article lists it only for Snow Leopard, so the combined `sudo dscacheutil -flushcache; sudo killall -HUP mDNSResponder` recipe adds a flush of a different cache. The article does not name macOS 26, so its coverage of this release is unconfirmed, though the command is the same one used since 10.10.4.

Reproduce the binary evidence with:

```sh
strings /usr/sbin/mDNSResponder | grep -i 'etc/hosts\|SIGHUP'
```

### systemd-resolved

systemd-resolved reads /etc/hosts itself. `man 5 systemd-resolved.service` says "This resolver reads and caches /etc/hosts internally" and that "Entries in /etc/hosts have highest priority". The man page for nss-resolve adds that resolved supports /etc/hosts internally "but with caching".

The cache is validated against the file. In `src/resolve/resolved-etc-hosts.c` (checked at tags v249, v250, v252, v254, v256, v257, v258, v259 and main):

```c
#define ETC_HOSTS_RECHECK_USEC (2*USEC_PER_SEC)   /* line 25 */
...
if (m->etc_hosts_last != USEC_INFINITY && m->etc_hosts_last + ETC_HOSTS_RECHECK_USEC > ts)
        return 0;                                 /* line 346 */
...
/* Did the mtime or ino/dev change? If not, there's no point in re-reading the file. */
if (stat_inode_unmodified(&m->etc_hosts_stat, &st))
        return 0;                                 /* lines 360-362 */
```

`manager_etc_hosts_lookup` calls `manager_etc_hosts_read` on each lookup, so the file is stat'ed at most once every 2 seconds and re-read when mtime or inode changed. Hosts answers are tried before the normal lookup, with a source comment saying this "is done *before* the normal lookup" so hosts "takes precedence over the network" (`src/resolve/resolved-dns-query.c:886`, call at line 920, v258). A cached upstream answer therefore does not shadow a hosts entry.

So no flush is needed. An edit can take up to about 2 seconds to be seen if a lookup ran just before it. `sudo resolvectl flush-caches` (documented as flushing "all DNS resource record caches", `man 1 resolvectl`, and equivalent to SIGUSR2) does not control the hosts re-read, so it is not the lever for hosts changes. systemd 263, unreleased at the time of this check, adds re-reading of /etc/hosts on reload (`NEWS`, "Changes in systemd-resolved": "/etc/hosts entries are now re-read on reload"), which implies earlier releases did not re-read on reload.

All of this only applies when nsswitch sends lookups to resolved, through `resolve` in `/etc/nsswitch.conf`.

### glibc

The glibc files module opens /etc/hosts for each by-name lookup. In `nss/nss_files/files-XXX.c` the comment says "For getXXent operations the stream needs to be held open across calls, the other getXXbyYY operations all use their own stream", and `internal_setent` calls `__nss_files_fopen` when the stream is NULL. `_nss_files_gethostbyname3_r` and `_nss_files_gethostbyname4_r` in `nss/nss_files/files-hosts.c` declare a fresh `FILE *stream = NULL` and then call `internal_setent`, so each lookup opens the file again. Sources at tag glibc-2.42:

```
https://raw.githubusercontent.com/bminor/glibc/glibc-2.42/nss/nss_files/files-XXX.c
https://raw.githubusercontent.com/bminor/glibc/glibc-2.42/nss/nss_files/files-hosts.c
```

There is no cache, so nothing needs flushing. The exception is nscd: `man 8 nscd` describes it as a cache for the hosts database, says it auto-detects changes to the database files (glibc 2.9 and later), and documents `nscd -i hosts` to invalidate after a change. nscd is not enabled by default on current distributions.

### musl

musl opens /etc/hosts on each lookup and has no cache. In `src/network/lookup_name.c`, `name_from_hosts` starts with:

```c
FILE _f, *f = __fopen_rb_ca("/etc/hosts", &_f, _buf, sizeof _buf);
```

(`https://git.musl-libc.org/cgit/musl/plain/src/network/lookup_name.c?h=v1.2.5`, line 55). Nothing needs flushing.

## Conditional requests against raw.githubusercontent.com

URL shapes tested (branch `master`):

```
https://raw.githubusercontent.com/StevenBlack/hosts/master/hosts
https://raw.githubusercontent.com/StevenBlack/hosts/master/alternates/social/hosts
https://raw.githubusercontent.com/StevenBlack/hosts/master/alternates/gambling/hosts
https://raw.githubusercontent.com/StevenBlack/hosts/master/alternates/porn/hosts
```

All four return 200 with an ETag and without Last-Modified, and `If-None-Match` with the returned ETag returns 304. The exact marker line `# End of custom host records.` appears once in each file, at line 33 in the unified file and line 34 in the three alternates.

Results from this run:

| File | GET | ETag | Last-Modified | If-None-Match | Marker line |
|---|---|---|---|---|---|
| hosts | 200 | `"c67d7efa0279a993dd9a8848ec00d652da15056d5eef93333c3aa22ef3f26031"` | absent | 304 | 33 |
| alternates/social/hosts | 200 | `"eed2eebae46ac926fcfe592b04cfa6227fbfe757ac86e3d3b337c70057bf4f00"` | absent | 304 | 34 |
| alternates/gambling/hosts | 200 | `"cb96d9d703caa4e2bab38c767d45e31f1a0534d21841632a077cab772255832d"` | absent | 304 | 34 |
| alternates/porn/hosts | 200 | `"51bdf8a250973a42547853e972b95001490f9dc152b5cbfd8f20c06bb4bdc4d0"` | absent | 304 | 34 |

Reproduce:

```sh
for f in hosts alternates/social/hosts alternates/gambling/hosts alternates/porn/hosts; do
  url="https://raw.githubusercontent.com/StevenBlack/hosts/master/$f"
  code=$(curl -sS -o /tmp/f.txt -D /tmp/f.h -w '%{http_code}' "$url")
  et=$(grep -i '^etag:' /tmp/f.h | tr -d '\r' | cut -d' ' -f2)
  lm=$(grep -i '^last-modified:' /tmp/f.h | tr -d '\r' | cut -d' ' -f2-)
  m=$(grep -c '^# End of custom host records\.$' /tmp/f.txt)
  echo "$f code=$code etag=$et last_modified=[$lm] marker_lines=$m"
  curl -sS -o /dev/null -w "conditional=%{http_code}\n" -H "If-None-Match: $et" "$url"
done
```

Observed details:

- Last-Modified is absent on every response, so date validation is unavailable. `If-Modified-Since: Sat, 31 Oct 2026 00:00:00 GMT` on the unified file returned 200, not 304.
- The identity ETag is a strong validator. With `accept-encoding: gzip` the ETag is weak and different, as covered under ureq above. The ureq probe with default gzip features got a 304 using the weak ETag.
- An unknown path, `https://raw.githubusercontent.com/StevenBlack/hosts/master/nope.txt`, returns 404.
- The responses carry `cache-control: max-age=300` and `vary: Authorization,Accept-Encoding`.
- The marker line is LF terminated, the file is ASCII, and the matched text is exactly `# End of custom host records.` with a trailing period and no trailing spaces.

The ETag values will change when the upstream files change; only the presence and conditional behaviour are stable facts.

## std::io::IsTerminal

`std::io::IsTerminal` was stabilized in Rust 1.70.0, released 2023-06-01. The release notes say: "This newly-stabilized trait has a single method, `is_terminal`, to determine if a given file descriptor or handle represents a terminal or TTY. This is another case of standardizing functionality that existed in external crates, like atty and is-terminal." See `https://blog.rust-lang.org/2023/06/01/Rust-1.70.0/#isterminal`.

The 1.70.0 standard library page shows the trait as version 1.70.0 and lists implementors for `Stdin`, `StdinLock`, `Stdout`, `StdoutLock`, `Stderr`, `StderrLock`, `File`, `BorrowedFd`, `OwnedFd`, plus `BorrowedHandle` and `OwnedHandle` on Windows (`https://doc.rust-lang.org/1.70.0/std/io/trait.IsTerminal.html`, Implementors section). So stdin, stdout, and stderr are all covered.

The atty README now says the crate "is no longer maintained" and points users to `std::io::IsTerminal`, introduced in Rust 1.70.0 (`https://github.com/softprops/atty`). Dropping atty and raising the crate's `rust-version` to 1.70 is supported by both sources.

## Unconfirmed

- The minimum supported Rust version for ureq 2.5.0. No declaration exists in the crate, its metadata, its docs, or its CI.
- An end-to-end macOS test of editing /etc/hosts and resolving without a flush. Writing /etc/hosts was out of bounds for this ticket, so the answer rests on the daemon's file watcher (binary strings and source comments) and the hosts(5) man page.
- Whether Apple's archived flush article is still considered current for macOS 26. It does not list that version, and the command is unchanged since 10.10.4.
