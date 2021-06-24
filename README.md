<img src="https://external-content.duckduckgo.com/iu/?u=https%3A%2F%2Fcdn4.iconfinder.com%2Fdata%2Ficons%2Fweb-design-and-development-2-11%2F66%2F80-512.png&f=1&nofb=1" alt="mosaic" title="Mosaic" width="200" height="200" />


# Block List

A simple host block-list updater built in Rust.

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
# Blocks ads and social media (use hosts if you just want to block adds)
sudo block-list /etc/hosts social
```

Pipe in a custom list:

```
cat someFile.txt | sudo block-list /etc/hosts
```
