# Block List

A simple hosts blocklist updater built in Rust.
This project uses the excellent and regularly updated [Unified Hosts](https://github.com/StevenBlack/hosts) lists from Steven Black.

### Installation

```shell
cargo install block-list
```

### Usage

```shell
sudo block-list /path/to/hosts your-desired-list
```

### Examples

Basic example:

```shell
# Blocks adds and social media (use hosts if you just want to block adds)
sudo block-list /etc/hosts social
```

Pipe in a custom list:

```
cat someFile.txt | sudo block-list /etc/hosts
```
