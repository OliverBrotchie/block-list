# Provider catalog research

Facts for issue #9, measured on 2026-10-31 between 15:00 and 15:30 UTC. Each URL was fetched with `curl -sSL`. A conditional request reused the `ETag` and `Last-Modified` values from the first 200 response. Counts come from the fetched copy at that time.

## Summary

Brave publishes ABP filter lists, not hosts files. Its default setup is a bundle of 22 sources named in `list_catalog.json`, and aggressive is a Shields mode that reuses the same engine, not a second list. There is no Brave hosts derivative.

The remaining providers split into three groups. StevenBlack, 1Hosts Lite and AdAway ship hosts format. OISD, hagezi and Peter Lowe ship other formats, with hosts available only from hagezi's legacy repo and not at all from OISD. hagezi and OISD update hourly or daily. 1Hosts last updated on 2026-09-03 and AdAway's list has not changed since 2023.

URLhaus still runs, but its first party hosts download is gone (410), and its API now needs an Auth-Key. The maintained hosts alternative is the third party `malware-filter` mirror, which Brave itself uses in ABP form.

Every URL tested answered a conditional request with 304 except the dead URLhaus download. OISD returned 503 to rapid repeated requests; its 304 answers came from requests spaced apart.

## Brave

Brave does not keep one file per mode. The browser ships a CRX component called "Brave Default Adblock Filters" (`uuid` `default`, hidden, enabled by default) whose sources are listed in the adblock resources catalog:

```
https://raw.githubusercontent.com/brave/adblock-resources/master/filter_lists/list_catalog.json
```

Fetched 2026-10-31: 75,270 bytes, ETag present, conditional GET 304. Last commit touching the file was 2026-09-01 (`gh api repos/brave/adblock-resources/commits?path=filter_lists/list_catalog.json&per_page=1`). The catalog has 62 entries and 100 sources; all 100 declare `"format": "Standard"`, meaning ABP or EasyList syntax. No entry or source describes hosts format.

The catalog entry named `default` has 22 sources. Sixteen are upstream lists (uBlock Origin filters, EasyList, the URLhaus AGH list) and six are Brave files. Brave also has separate default enabled catalog entries for privacy lists (`EasyPrivacy`, uBO privacy) and first party filters. The Brave files in the default entry, all MPL-2.0 and all ABP syntax:

| file | raw URL | rules | last commit |
| --- | --- | ---: | --- |
| `brave-unbreak.txt` at repo root | `https://raw.githubusercontent.com/brave/adblock-lists/master/brave-unbreak.txt` | 897 | 2026-10-30 |
| `brave-lists/brave-specific.txt` | `https://raw.githubusercontent.com/brave/adblock-lists/master/brave-lists/brave-specific.txt` | 81 | 2026-08-04 |
| `brave-lists/brave-social.txt` | `https://raw.githubusercontent.com/brave/adblock-lists/master/brave-lists/brave-social.txt` | 8 | 2023-06-23 |
| `brave-lists/brave-android-specific.txt` | `https://raw.githubusercontent.com/brave/adblock-lists/master/brave-lists/brave-android-specific.txt` | 1 | 2023-06-26 |
| `brave-lists/brave-sugarcoat.txt` | `https://raw.githubusercontent.com/brave/adblock-lists/master/brave-lists/brave-sugarcoat.txt` | 2 | 2023-06-23 |
| `brave-lists/brave-unbreak.txt` | `https://raw.githubusercontent.com/brave/adblock-lists/master/brave-lists/brave-unbreak.txt` | 0 (24 bytes) | 2020-11-03 |

Rule counts exclude comment and blank lines; see the reproduce section. The whole 22 source bundle held 111,150 rules on 2026-10-31. The repo was last pushed 2026-10-30.

Licence is MPL-2.0 for the `brave/adblock-lists` and `brave/adblock-resources` repositories, from the GitHub licence field on 2026-10-31. The bundle mixes upstream licences; for example EasyList has its own terms, so the MPL does not cover the whole bundle.

Aggressive is not a list. The iOS shield enum maps aggressive and standard to the same `BraveShields.AdBlockMode` values at `https://github.com/brave/brave-core/blob/master/ios/brave-ios/Sources/BraveShields/ShieldLevel.swift` (checked 2026-10-31). On desktop, `shouldStartRequest` receives an `aggressive_blocking` flag and applies it to the result of the same default engine at `https://github.com/brave/brave-core/blob/master/components/brave_shields/content/browser/ad_block_engine_wrapper.cc` (checked 2026-10-31). No catalog entry contains "aggressive" in its title or UUID.

No Brave hosts derivative was found. The `brave/adblock-lists` tree contains no hosts file (checked with `gh api repos/brave/adblock-lists/git/trees/master?recursive=1` on 2026-10-31), and all catalog sources are `Standard` ABP. This is a negative result from inspection, not a statement from Brave.

Conditional GET on the six Brave files and the catalog returned 304. `raw.githubusercontent.com` sends an ETag and no `Last-Modified` header, so date information has to come from the commit history.

## StevenBlack

Repository is `https://github.com/StevenBlack/hosts`, MIT licence (`license.txt`, checked 2026-10-31). The unified file and the three requested alternates are hosts format, with entries such as `0.0.0.0 example.com`. All four files were last changed by commit `2026-10-29T15:33:52Z` ("Updates from KADhosts.", via `gh api repos/StevenBlack/hosts/commits?path=hosts&per_page=1`).

| list | raw URL | entries | conditional GET |
| --- | --- | ---: | --- |
| unified | `https://raw.githubusercontent.com/StevenBlack/hosts/master/hosts` | 76,750 | 304 |
| social | `https://raw.githubusercontent.com/StevenBlack/hosts/master/alternates/social/hosts` | 80,904 | 304 |
| gambling | `https://raw.githubusercontent.com/StevenBlack/hosts/master/alternates/gambling/hosts` | 83,839 | 304 |
| porn | `https://raw.githubusercontent.com/StevenBlack/hosts/master/alternates/porn/hosts` | 153,210 | 304 |

Entry counts were taken on 2026-10-31 with `curl -sSL <url> | awk 'NF && $1 !~ /^[#!]/ {n++} END {print n}'`.

There is no domains-only variant. The repo root has one list file, `hosts`, and the alternates directories contain only `hosts`, a readme and stats. The `-only` directory names such as `social-only` mean "social category only", not "domain lines only". All published variants use hosts syntax.

## OISD

The small and big lists are served from the list host, not from GitHub, and use wildcard domain syntax (`*.example.com` blocks the domain and its subdomains). Headers in the files state the syntax, entry count and licence.

| list | raw URL | format | entries | last modified | conditional GET |
| --- | --- | --- | ---: | --- | --- |
| small | `https://small.oisd.nl/domainswild` | domains wildcard, one `*.domain` per line | 60,320 | 2026-10-31T14:05:39+0000 | 304 |
| big | `https://big.oisd.nl/domainswild` | domains wildcard, one `*.domain` per line | 235,269 | 2026-10-31T14:04:49+0000 | 304 |

Fetched sizes were 1,302,490 and 5,098,476 bytes. The file header links the licence at `https://github.com/sjhgvr/oisd/blob/main/LICENSE`, which is GPL-3.0, matching the GitHub licence field for `sjhgvr/oisd` on 2026-10-31.

Both files returned 304 with the server's ETag and Last-Modified values at 15:11 UTC. Later requests within the same minute returned 503, so the server rate limits bursts; retry with a delay.

OISD dropped hosts and domains syntax on 2024-01-01. The announcement is on `https://oisd.nl/` (checked 2026-10-31), and `https://small.oisd.nl/hosts` and `https://small.oisd.nl/domains` both return 404. `https://small.oisd.nl/abp`, `/dnsmasq` and `/rpz` return 200, so ABP is the closest equivalent for a converter that accepts ABP.

## hagezi

The main repo is `https://github.com/hagezi/dns-blocklists`, GPL-3.0. The current normal and pro lists are published under `wildcard/` as plain-domain and wildcard variants. The hosts and subdomain formats moved to `https://github.com/hagezi/dns-blocklists-legacy`, also GPL-3.0.

| list | raw URL | format | entries | last modified | conditional GET |
| --- | --- | --- | ---: | --- | --- |
| normal | `https://raw.githubusercontent.com/hagezi/dns-blocklists/main/wildcard/multi-onlydomains.txt` | plain domains, one per line | 175,278 | 2026-10-31 09:21 UTC | 304 |
| pro | `https://raw.githubusercontent.com/hagezi/dns-blocklists/main/wildcard/pro-onlydomains.txt` | plain domains, one per line | 213,292 | 2026-10-31 09:21 UTC | 304 |
| normal, hosts | `https://raw.githubusercontent.com/hagezi/dns-blocklists-legacy/main/hosts/multi.txt` | hosts, `0.0.0.0 domain` | 373,391 lines | commit 2026-10-31T10:33:20Z | 304 |
| pro, hosts | `https://raw.githubusercontent.com/hagezi/dns-blocklists-legacy/main/hosts/pro.txt` | hosts, `0.0.0.0 domain` | 479,277 lines | commit 2026-10-31T10:33:20Z | 304 |

The canonical library also has `https://raw.githubusercontent.com/hagezi/dns-blocklists/main/wildcard/multi.txt` and `pro.txt`, which write `*.domain` instead of bare domains. All four raw URLs returned 304 on 2026-10-31; the jsDelivr mirrors used in the hagezi README also returned 304 but send only a weak ETag.

The entry counts are the `Number of entries` values in each file header. The hosts files expand subdomains, so their line counts are larger than the domain counts. The legacy README says hosts and subdomain formats have coverage gaps and that some hagezi lists do not exist in that format.

## 1Hosts

Repo is `https://github.com/badmojr/1Hosts`, MPL-2.0. Only Lite and Xtra remain; the Pro directory was deleted by commit `2025-10-22T14:16:42Z` ("Delete Pro directory", via `gh api repos/badmojr/1Hosts/commits?path=Pro&per_page=3`). The README now says 1Hosts offers two variants.

| list | raw URL | format | entries | last update | conditional GET |
| --- | --- | --- | ---: | --- | --- |
| Lite | `https://raw.githubusercontent.com/badmojr/1Hosts/master/Lite/hosts.txt` | hosts, `0.0.0.0 domain` | 202,968 | 2026-09-03T12:23:55Z | 304 |
| Lite | `https://raw.githubusercontent.com/badmojr/1Hosts/master/Lite/domains.txt` | plain domains | 202,954 | 2026-09-03T12:23:55Z | 304 |
| Xtra | `https://github.com/badmojr/1Hosts/releases/download/latest/1hosts-Xtra_hosts.txt` | hosts | not counted (31,302,396 bytes) | release 2026-09-03T12:28:13Z | 304 |

The Lite file header states "Count: 202,954 rules" and "Licence: MPLv2". The hosts file has 14 more non-comment lines than the domains file. The repo has commits on consecutive days until 2026-09-03 and none after that at the time of checking, so the list has been static for about eight weeks.

Xtra is described in the README as the aggressive beta variant and is distributed through release assets, not through a stable path under `Lite/` or `Pro/`. The release asset answered a conditional request with 304 and sent `Last-Modified: Thu, 03 Sep 2026 12:28:28 GMT`. There is no raw Pro URL to point at.

## AdAway

`https://adaway.org/hosts.txt`, hosts format, 243,454 bytes and 6,542 entries on 2026-10-31. The file header states "License: CC Attribution 3.0". Conditional GET returned 304 with the server's ETag and Last-Modified.

The list content last changed on 2023-04-29 (`gh api repos/AdAway/adaway.github.io/commits?path=hosts.txt&per_page=1`). The HTTP `Last-Modified` is 2025-12-28, but the two commits from that day are dependency and site maintenance ("fix: update dependencies to address security vulnerabilities", "Improvements"), and `hosts.txt` itself was not touched. Treat this list as stale. It is unmaintained as a list even though the repository received maintenance commits.

## Peter Lowe

`https://pgl.yoyo.org/adservers/serverlist.php?hostformat=hosts&showintro=0&mimetype=plaintext`, hosts format, 3,562 entries and 95,321 bytes on 2026-10-31. The file header and the page at `https://pgl.yoyo.org/adservers/` state "last updated: Thu, 29 Oct 2026 21:57:19 GMT" and "last validated: Fri, 30 Oct 2026 12:02:07", so the list is current.

No licence statement was found on the list page, the about page, the policy page or the formats page on 2026-10-31. The site asks for Patreon support and states no reuse terms. Treat the licence as unstated.

The server sends `Last-Modified` and no ETag. A request with `If-Modified-Since` returned 304.

## URLhaus

The first party hosts download is gone. `https://urlhaus.abuse.ch/downloads/hostfile/` returned HTTP 410 on 2026-10-31 with the body "The requested resource is no longer available on this server and there is no forwarding address". `https://urlhaus.abuse.ch/downloads/` also returns 410.

The URLhaus site and database are still running, but data access moved behind the API, which requires an Auth-Key (`https://urlhaus.abuse.ch/api/`, checked 2026-10-31). The abuse.ch terms at `https://abuse.ch/terms-of-use/` limit file exports to volumes expected for non-commercial or non-profit purposes, and say CC0 applies to Feodo Tracker, SSL Blacklist and YARAify, not URLhaus.

The maintained hosts form comes from the `malware-filter` project:

| list | raw URL | format | last update | conditional GET |
| --- | --- | --- | --- | --- |
| hosts | `https://malware-filter.gitlab.io/malware-filter/urlhaus-filter-hosts-online.txt` | hosts, `0.0.0.0 domain` | 2026-10-31T12:09:56Z in header | 304 |
| AGH | `https://malware-filter.gitlab.io/malware-filter/urlhaus-filter-agh-online.txt` | AdGuard Home ABP rules | 2026-10-31T12:09:56Z in header | 304 |

The `urlhaus-filter` repo at `https://gitlab.com/malware-filter/urlhaus-filter` is licensed CC0 1.0 and MIT, with last activity 2026-10-22. The HTTP `Last-Modified` for both mirror URLs was 2026-10-31T12:31:35Z. Brave's default bundle sources the AGH variant, which had 1,887 rules on 2026-10-31.

## What converts to hosts

I downloaded all 22 sources in the `default` catalog entry on 2026-10-31 and classified every non-comment, non-blank line. Cosmetic means the line contains `##`, `#@#`, `#?#`, `#$#`, `#%#` or a related cosmetic separator. Domain-only means the line matches `^\|\|[A-Za-z0-9._-]+\^$`. Exception means it starts with `@@`. Everything else is counted as path, options or pattern.

| class | rules | share |
| --- | ---: | ---: |
| domain-only, `||domain^` | 44,847 | 40.3% |
| exception, `@@` | 4,229 | 3.8% |
| cosmetic | 43,543 | 39.2% |
| path, options or pattern | 18,531 | 16.7% |
| total | 111,150 | 100% |

Within the path and options class, 15,424 rules carry a `$` option list, 2,439 are `||` rules without a terminal `^`, 617 are plain substring rules, 41 have a host and path without options, and 10 are regex rules. EasyList supplied 42,557 of the domain-only rules. The Brave authored files contributed only 7 domain-only rules out of 989, with 246 exceptions, 473 cosmetic rules and 263 path or option rules.

The main failure mode is that hosts files have no exception mechanism. An ABP list says "block these, except these" with `@@` rules; a hosts file can only block. Dropping 4,229 exception rules, or converting them into blocks, makes the output block domains the list deliberately allowed. This is not a rounding error: Brave's own `brave-unbreak.txt` is almost entirely exceptions and unbreak rules.

The second failure mode is over-blocking from host scoping. Path rules such as `||example.com/path/` and option rules such as `||example.com^$third-party` become whole-domain blocks if reduced to `||example.com^`, which blocks first party and unrelated paths. The reverse choice, dropping them, loses the blocking the list intended. Cosmetic rules have no DNS meaning and are simply dropped, which is correct for hosts but means a hosts file reproduces less than half of the bundle's blocking surface.

Pattern rules add smaller losses: wildcards and regex cannot be represented, negated domains (`~`) and `$badfilter` have no equivalent, and entries such as `||[::]^$third-party` are not hostnames. A converter that ignores `$badfilter` will re-enable rules the list deliberately disabled.

## Reproduce

Conditional request and status:

```
curl -sS -L -o /dev/null -w '%{http_code} %header{etag} %header{last-modified}\n' URL
curl -sS -L -o /dev/null -w '%{http_code}\n' -H 'If-None-Match: ETAG' URL
```

Hosts entry count:

```
curl -sSL URL | awk 'NF && $1 !~ /^[#!]/ {n++} END {print n}'
```

Rule count and classification, run over the 22 raw URLs in the catalog:

```
python3 - <<'EOF'
import re, subprocess
urls = [ ... ]  # jq -r '.[]|select(.uuid=="default")|.sources[].url' list_catalog.json
cosm = re.compile(r'##|#@#|#\?#|#\$#|#%#')
dom = re.compile(r'^\|\|[A-Za-z0-9._-]+\^$')
counts = dict(dom=0, exc=0, cosm=0, other=0)
for u in urls:
    text = subprocess.run(['curl','-sSL',u], capture_output=True, text=True).stdout
    for line in text.splitlines():
        s = line.strip()
        if not s or s.startswith('!') or s.startswith('['):
            continue
        if cosm.search(s): counts['cosm'] += 1
        elif s.startswith('@@'): counts['exc'] += 1
        elif dom.match(s): counts['dom'] += 1
        else: counts['other'] += 1
print(counts)
EOF
```

Last commit per file:

```
gh api 'repos/OWNER/REPO/commits?path=PATH&per_page=1' --jq '.[0].commit.committer.date'
```

## Unconfirmed

Brave has no published statement that it will never ship a hosts list; the absence is from inspecting the two repositories and the catalog on 2026-10-31.

AdAway's list may be regenerated elsewhere in future. Its repository shows no list changes since 2023 and no commits since 2025-12-28.

1Hosts had no commits between 2026-09-03 and 2026-10-31. It may be paused rather than retired.

OISD returned 503 to rapid repeated requests. The 304 results above come from requests spaced apart.

Peter Lowe states no licence on the pages checked. Do not redistribute that list until the author clarifies terms.
