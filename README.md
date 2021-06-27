<div align="center">

  <h1><code>block-list</code></h1>

  <strong>A minimalist hosts-based tool for managing block lists and ad-blocking.</strong>

</div>


# About

A minimalist hosts-based tool for managing block lists and ad-blocking.

This project uses the excellent and regularly updated [Unified Hosts](https://github.com/StevenBlack/hosts) lists from Steven Black.



## Why should you use Block List?

- It is an extensible, small and simple tool; consisting of only fifty-five lines of code!
- It will block this traffic across all programs running on the system. 
- It is written in Rust and hence can be depended upon to be fast and stable. 
- It is designed to be easily incorporated into shell scripts to automate or customize its behaviour to your liking.
- A hosts-based block list such as this uses minimal system resources to block unwanted traffic in comparison to a browser extension.

## Installation

Using Cargo (Rust)

```shell
cargo install block-list
```

More options will come in future!

## Usage

Just use 'hosts' as the desired list if you only want ad-blocking.

```shell
sudo block-list /path/to/hosts your-desired-list
```

## Examples

**Basic usage:**

```shell
sudo block-list /etc/hosts social
```

<br>

**Pipe in a custom list:**

```shell
cat someFile.txt | sudo block-list /etc/hosts
```

**OR**

```shell
cat someFile.txt | sudo block-list /etc/hosts social
```

<br>

**Automatically update Block List at 10pm every day:**

Open the root user's crontab file

```shell
sudo crontab -e
```

Then add the following line

```txt
0 22 * * * block-list /etc/hosts hosts
```

<br>

**Use Block List to re-route a url to a different address:**

Find the address using dig

```shell
dig duckduckgo.com
```

Coppy the IP address listed in 'Answer Section' to a file

```txt
# A custom Block List

# Re-route all requests from Google to DuckDuckGo
52.142.124.215 www.google.com
52.142.124.215 google.com
```

Pipe in the custom list to Block List

```shell
cat someFile.txt | sudo block-list /etc/hosts
```


## Output

If successful, this command will append a new list or replace the current list at the tail of the supplied hosts file, eg:

```shell
Block List updated! 🔒
```

```txt
##
# Host Database
##
127.0.0.1	localhost
255.255.255.255	broadcasthost
::1             localhost


# Your list of addresses
1.2.3.4 somehost

... 

# Block List

0.0.0.0 some.custom.address.net
0.0.0.0 another.custom.address.xyz

0.0.0.0 some.malware.address.com
0.0.0.0 another.blocked.address.xxx
```
