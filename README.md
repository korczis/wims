# wims - Where Is My Space?

Find where is your space.

## Prerequisites

- [rust](https://www.rust-lang.org)

## Getting started

```
$ git clone https://github.com/korczis/wims.git wims
$ cd wims
```

## Building

```
$ cargo install
```

## Usage

```
$ wims --help
Where Is My Space? 0.1.0
Tomas Korcak <korczis@gmail.com>
Disk Usage Information

USAGE:
    wims [FLAGS] [OPTIONS] [DIR]...

FLAGS:
        --cache             Cache items to disk
        --help              Prints help information
    -h, --human             Human readable sizes
    -p, --progress          Show progress
    -s, --stats             Print overall stats at exit
    -t, --tree              Show FS tree
        --tree-only-dirs    Print only directories in tree
    -V, --version           Prints version information
    -v, --verbose           Verbose mode

OPTIONS:
    -c, --progress-count <progress-count>      Progress count [default: 10000]
    -f, --progress-format <progress-format>    Progress format [default: path]  [values: dot, path, raw]
    -d, --tree-depth <tree-depth>              Show only N first tree levels [default: 0]

ARGS:
    <DIR>...    Directories to process
```
