<img src="https://external-content.duckduckgo.com/iu/?u=https%3A%2F%2Fcdn4.iconfinder.com%2Fdata%2Ficons%2Fweb-design-and-development-2-11%2F66%2F80-512.png&f=1&nofb=1" alt="mosaic" title="Mosaic" width="200" height="200" />


# Block List

A minimalist host-based block list tool built in Rust.

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
# Just use 'hosts' if you only want ad-blocking.
sudo block-list /etc/hosts social
```

Pipe in a custom list:

```
cat someFile.txt | sudo block-list /etc/hosts
# OR
cat someFile.txt | sudo block-list /etc/hosts social
```
