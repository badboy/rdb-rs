# rdb-rs - fast and efficient RDB parsing utility

## Introduction

> [Redis](http://redis.io) is an open source, BSD licensed, advanced key-value cache and store. It is often referred to as a data structure server since keys can contain strings, hashes, lists, sets, sorted sets, bitmaps and hyperloglogs.

Redis’ RDB file is a binary representation of the in-memory store. This binary file is sufficient to completely restore Redis’ state.

Optimizing for fast read/writes means the on-disk format should be as close as possible to the in-memory representation. This is the approach taken by the RDB file. As a consequence, you cannot parse the RDB file without some understanding of Redis’ in-memory representation of data structures.

`rdb-rs` is a library and tool to parse RDB and dump it into another format like JSON or the [Redis protocol](http://redis.io/topics/protocol).

It is based on Sripathi Krishnan's [redis-rdb-tools](https://github.com/sripathikrishnan/redis-rdb-tools) and compatible with the latest Redis RDB version 7.

## Getting started

`rdb-rs` is offered both as a library and as a stand-alone command line tool.

### Command-line tool

The command line tool can be used to dump an existing RDB file in one of the provided formats:

```bash
rdb --format json dump.rdb
# [{"key":"value"}]
rdb --format protocol dump.rdb
# *2
# $6
# SELECT
# $1
# 0
# *3
# $3
# SET
# $3
# key
# $5
# value
```

See the help output for more info how to use it:

```bash
rdb --help
# Usage: target/rdb [options] dump.rdb
#
# Options:
#     -f --format FORMAT  Format to output. Valid: json, plain, nil, protocol
#     -k --keys KEYS      Keys to show. Can be a regular expression
#     -d --databases DB   Database to show
#     -t --type TYPE      Type to show
#     -h --help           print this help menu
```

### Library

Using the library is as easy as calling the `rdb::parse` function and pass it a stream to read from and a formatter to use.

```rust
use std::io::{BufferedReader, File};

let file = File::open(&Path::new("dump.rdb"));
let reader = BufferedReader::new(file);
rdb::parse(reader, rdb::formatter::JSON::new(), rdb::filter::Simple::new());
```

`rdb-rs` brings 4 pre-defined formatters, which can be used:

* `Plain`: Just plain output for testing
* `JSON`: JSON-encoded output
* `Nil`: Surpresses all output
* `Protocol`: Formats the data in [RESP](http://redis.io/topics/protocol), the Redis Serialization Protocol

It's easy to build your own formatter. All you need to do is implementing the `Formatter` trait.

## Code

The code is available on GitHub: [github.com/badboy/rdb-rs](https://github.com/badboy/rdb-rs).  
Submit bugs, requests and improvements to the [issue tracker](https://github.com/badboy/rdb-rs/issues).
You can also contact me via [email](mailto:janerik@fnordig.de) or [twitter](https://twitter.com/badboy_).

## Documentation

The included documentation of the RDB format is largely based on
[RDB_File_Format.textile](https://github.com/sripathikrishnan/redis-rdb-tools/blob/d39c8e5127daf3e109c0f0e101af8ed0e5400493/docs/RDB_File_Format.textile)
and
[RDB_Version_History.textile](https://github.com/sripathikrishnan/redis-rdb-tools/blob/d39c8e5127daf3e109c0f0e101af8ed0e5400493/docs/RDB_Version_History.textile).
Thanks to Sripathi Krishnan and his work on the [redis-rdb-tools](https://github.com/sripathikrishnan/redis-rdb-tools).

### Crate documentation

The full code documentation is available [online](http://rdb.fnordig.de/doc/rdb/).

### Included documentation

* [File Format](file_format.html)
* [Version History](version_history.html)
