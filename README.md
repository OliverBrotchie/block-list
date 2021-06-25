<img src="https://external-content.duckduckgo.com/iu/?u=https%3A%2F%2Fcdn4.iconfinder.com%2Fdata%2Ficons%2Fweb-design-and-development-2-11%2F66%2F80-512.png&f=1&nofb=1" alt="mosaic" title="Mosaic" width="200" height="200" />


# Block List

A minimalist hosts-based tool for managing block lists and ad-blocking.

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

Basic usage:

```shell
# Just use 'hosts' as the desired list if you only want ad-blocking.
sudo block-list /etc/hosts social
```

Pipe in a custom list:

```
cat someFile.txt | sudo block-list /etc/hosts
```
**OR**
```
cat someFile.txt | sudo block-list /etc/hosts social
```
**OR**
```
cat someFile.txt | sudo block-list /etc/hosts social
```
