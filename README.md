# rdb-rs - RDB parsing, formatting, analyzing. All in one library

Inspired and based on [redis-rdb-tools][].


## Build

```
cargo build --release
```

## Usage

```rust
extern crate rdb;

fn main() {
    let file = File::open(&Path::new("dump.rdb"));
    let mut reader = BufferedReader::new(file);

    let mut formatter = PlainFormatter::new();

    rdb::parse(&mut reader, &mut formatter)
}

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
