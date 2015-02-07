//! rdb - Parse, analyze and dump RDB files
//!
//! A RDB file is a binary representation of the in-memory data of Redis.
//! This binary file is sufficient to completely restore Redis’ state.
//!
//! This library provides the methods to parse and analyze a RDB file
//! and to reformat and dump it in another format such as JSON or
//! RESP, the Redis Serialization.
//!
//! You can depend on this library via Cargo:
//!
//! ```ini
//! [dependencies]
//! rdb = "0.5.0"
//! ```
//!
//! # Basic operation
//!
//! rdb-rs exposes just one important method: `parse`.
//! This methods takes care of reading the RDB from a stream,
//! parsing the containted data and calling the provided formatter with already-parsed values.
//!
//! ```rust,no_run
//! # #![allow(unstable)]
//! # use std::old_io::{BufferedReader, File};
//! let file = File::open(&Path::new("dump.rdb"));
//! let reader = BufferedReader::new(file);
//! rdb::parse(reader, rdb::formatter::JSON::new(), rdb::filter::Simple::new());
//! ```
//!
//! # Formatter
//!
//! rdb-rs brings 4 pre-defined formatters, which can be used:
//!
//! * `PlainFormatter`: Just plain output for testing
//! * `JSONFormatter`: JSON-encoded output
//! * `NilFormatter`: Surpresses all output
//! * `ProtocolFormatter`: Formats the data in [RESP](http://redis.io/topics/protocol),
//! the Redis Serialization Protocol
//!
//! These formatters adhere to the `RdbParseFormatter` trait
//! and supply a method for each possible datatype or opcode.
//! Its up to the formatter to correctly handle all provided data such as
//! lists, sets, hashes, expires and metadata.
//!
//! # Command-line
//!
//! rdb-rs brings a Command Line application as well.
//!
//! This application will take a RDB file as input and format it in the specified format (JSON by
//! default).
//!
//! Example:
//!
//! ```shell,no_compile
//! $ rdb --format json dump.rdb
//! [{"key":"value"}]
//! $ rdb --format protocol dump.rdb
//! *2
//! $6
//! SELECT
//! $1
//! 0
//! *3
//! $3
//! SET
//! $3
//! key
//! $5
//! value
//! ```

#![feature(slicing_syntax)]
#![feature(io)]
#![feature(core)]

extern crate lzf;
extern crate "rustc-serialize" as serialize;
extern crate regex;

#[doc(hidden)]
pub use types::{
    ZiplistEntry,
    Type,

    /* error and result types */
    RdbError,
    RdbResult,
    RdbOk,
};

pub use parser::RdbParser;

use formatter::Formatter;
use filter::Filter;

mod macros;
mod constants;
mod helper;

pub mod types;
pub mod parser;
pub mod formatter;
pub mod filter;

pub fn parse<R: Reader, F: Formatter, T: Filter>(input: R, formatter: F, filter: T) -> RdbOk {
    let mut parser = RdbParser::new(input, formatter, filter);
    parser.parse()
}
