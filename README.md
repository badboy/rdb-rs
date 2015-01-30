# rdb-rs - RDB parsing, formatting, analyzing. All in one library

Inspired and based on [redis-rdb-tools][].

## Documentation

Online at [rdb.fnordig.de/doc/rdb/][doc].


## Build

```
cargo build --release
```

## Install

```
make install
```

You can change the path by setting `PREFIX`. Defaults to `/usr`.

## Basic operation

rdb-rs exposes just one important method: `parse`.
This methods takes care of reading the RDB from a stream,
parsing the containted data and calling the provided formatter with already-parsed values.

```rust
use std::old_io::{BufferedReader, File};

let file = File::open(&Path::new("dump.rdb"));
let reader = BufferedReader::new(file);
rdb::parse(reader, rdb::formatter::JSON::new(), rdb::filter::Simple::new());
```

### Formatter

rdb-rs brings 4 pre-defined formatters, which can be used:

* `Plain`: Just plain output for testing
* `JSON`: JSON-encoded output
* `Nil`: Surpresses all output
* `Protocol`: Formats the data in [RESP][],
the Redis Serialization Protocol

These formatters adhere to the `Formatter` trait and supply a method for each possible datatype or opcode.
Its up to the formatter to correctly handle all provided data such as lists, sets, hashes, expires and metadata.

### Command-line

rdb-rs brings a Command Line application as well.

This application will take a RDB file as input and format it in the specified format (JSON by default).

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
make test
```

This will run the code tests with cargo as well as checking that it can parse all included dump files.

## Contribute

If you find bugs or want to help otherwise, please [open an issue][issues].

## License

BSD. See [LICENSE](LICENSE).

[redis-rdb-tools]: https://github.com/sripathikrishnan/redis-rdb-tools
[RESP]: http://redis.io/topics/protocol
[issues]: https://github.com/badboy/rdb-rs/issues
[doc]: http://rdb.fnordig.de/doc/rdb/
