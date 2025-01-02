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
//! rdb = "*"
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
//! # use std::io::BufReader;
//! # use std::fs::File;
//! # use std::path::Path;
//! let file = File::open(&Path::new("dump.rdb")).unwrap();
//! let reader = BufReader::new(file);
//! rdb::parse(reader, rdb::formatter::JSON::new(None), rdb::filter::Simple::new());
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

#[cfg(feature = "python")]
use pyo3::exceptions::PyValueError;
#[cfg(feature = "python")]
use pyo3::prelude::*;

use std::io::Read;

#[doc(hidden)]
pub use types::{RdbError, RdbOk, RdbResult, Type};

pub mod constants;
pub mod decoder;
pub mod filter;
pub mod formatter;
pub mod types;

pub use decoder::RdbDecoder;
pub use filter::{Filter, Simple};
pub use formatter::{Formatter, FormatterType};

// Main entry point for parsing RDB files
pub struct RdbParser<R: Read, L: Filter, F: Formatter> {
    decoder: RdbDecoder<R, L>,
    formatter: Option<F>,
}

impl<R: Read, L: Filter, F: Formatter> RdbParser<R, L, F> {
    pub fn builder() -> RdbParserBuilder<R, L, F> {
        RdbParserBuilder {
            reader: None,
            filter: None,
            formatter: None,
        }
    }

    pub fn into_iter(self) -> RdbDecoder<R, L> {
        self.decoder
    }
}

#[derive(Default)]
pub struct RdbParserBuilder<R: Read, L: Filter, F: Formatter> {
    reader: Option<R>,
    filter: Option<L>,
    formatter: Option<F>,
}

impl<R: Read, L: Filter + Default, F: Formatter> RdbParserBuilder<R, L, F> {
    pub fn build(self) -> RdbParser<R, L, F> {
        let reader = self.reader.unwrap();
        let filter = self.filter.unwrap_or_default();
        let formatter = self.formatter;
        RdbParser {
            decoder: RdbDecoder::new(reader, filter).unwrap(),
            formatter: formatter,
        }
    }

    pub fn with_reader(mut self, reader: R) -> Self {
        self.reader = Some(reader);
        self
    }

    pub fn with_filter(mut self, filter: L) -> Self {
        self.filter = Some(filter);
        self
    }

    pub fn with_formatter(mut self, formatter: F) -> Self {
        self.formatter = Some(formatter);
        self
    }
}

impl<R: Read, L: Filter, F: Formatter> RdbParser<R, L, F> {
    pub fn parse(self) -> RdbResult<()> {
        if let Some(mut formatter) = self.formatter {
            formatter.start_rdb();
            for value in self.decoder {
                formatter.format(&value?)?;
            }
            formatter.end_rdb();
        }
        Ok(())
    }
}

pub fn parse<R: Read, L: Filter + Default, F: Formatter>(
    reader: R,
    formatter: F,
    filter: L,
) -> RdbResult<()> {
    let parser = RdbParser::builder()
        .with_reader(reader)
        .with_filter(filter)
        .with_formatter(formatter)
        .build();
    parser.parse()
}

#[cfg(feature = "python")]
#[pyclass(name = "RdbDecoder")]
pub struct PyRdbDecoder {
    decoder: RdbDecoder<std::fs::File, Simple>,
}

#[cfg(feature = "python")]
#[pymethods]
impl PyRdbDecoder {
    #[new]
    pub fn new(path: &str) -> PyResult<Self> {
        let file = std::fs::File::open(path)
            .map_err(|e| PyValueError::new_err(format!("Failed to open file: {}", e)))?;

        let mut filter = Simple::new();

        for t in [
            Type::Hash,
            Type::String,
            Type::List,
            Type::Set,
            Type::SortedSet,
        ] {
            filter.add_type(t);
        }

        let decoder = RdbDecoder::new(file, filter)
            .map_err(|e| PyValueError::new_err(format!("Failed to create decoder: {}", e)))?;

        Ok(PyRdbDecoder { decoder })
    }

    fn __iter__(slf: PyRef<'_, Self>) -> PyRef<'_, Self> {
        slf
    }

    fn __next__(mut slf: PyRefMut<'_, Self>) -> PyResult<Option<PyObject>> {
        match slf.decoder.next() {
            Some(Ok(value)) => Python::with_gil(|py| {
                value
                    .into_pyobject(py)
                    .map(|obj| Some(obj.into()))
                    .map_err(|e| PyValueError::new_err(format!("Conversion error: {}", e)))
            }),
            Some(Err(e)) => Err(PyValueError::new_err(format!("Parsing error: {}", e))),
            None => Ok(None),
        }
    }
}

#[cfg(feature = "python")]
#[pymodule(name = "rdb")]
fn rdb_py(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<PyRdbDecoder>()?;
    Ok(())
}
