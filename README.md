# rdb-rs - RDB parsing, formatting, analyzing. All in one library

Inspired and based on [redis-rdb-tools][].

## Documentation

Online at [rdb.fnordig.de/doc/rdb/](http://rdb.fnordig.de/doc/rdb/).


## Build

```
cargo build --release
```

## Basic operation

rdb-rs exposes just one important method: `parse`.
This methods takes care of reading the RDB from a stream,
parsing the containted data and calling the provided formatter with already-parsed values.

```rust
use std::io::{BufferedReader, File};

let file = File::open(&Path::new("dump.rdb"));
let reader = BufferedReader::new(file);
rdb::parse(reader, rdb::JSONFormatter::new())
```

### Formatter

rdb-rs brings 4 pre-defined formatters, which can be used:

* `PlainFormatter`: Just plain output for testing
* `JSONFormatter`: JSON-encoded output
* `NilFormatter`: Surpresses all output
* `ProtocolFormatter`: Formats the data in [RESP](http://redis.io/topics/protocol),
the Redis Serialization Protocol

These formatters adhere to the `RdbParseFormatter` trait
and supply a method for each possible datatype or opcode.
Its up to the formatter to correctly handle all provided data such as
lists, sets, hashes, expires and metadata.

### Command-line

rdb-rs brings a Command Line application as well.

This application will take a RDB file as input and format it in the specified format (JSON by
default).

Example:

```
$ rdb --format json dump.rdb
[{"key":"value"}]
$ rdb --format protocol dump.rdb
*2
$6
SELECT
$1
0
*3
$3
SET
$3
key
$5
value
```

## Tests

Run tests with:

```
cargo test
```

## Contribute

If you find bugs or want to help otherwise, please [open an issue](https://github.com/badboy/rdb-rs/issues).

## License

BSD. See [LICENSE](LICENSE).

[redis-rdb-tools]: https://github.com/sripathikrishnan/redis-rdb-tools
