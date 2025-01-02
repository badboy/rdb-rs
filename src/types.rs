use thiserror::Error;

use indexmap::IndexMap;

use crate::constants::encoding_type;

#[cfg(feature = "python")]
use pyo3::prelude::*;
#[cfg(feature = "python")]
use pyo3::types::PyDict;

#[derive(Error, Debug)]
pub enum RdbError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("No value found after {0}")]
    MissingValue(&'static str),
    #[error("Unknown encoding type: {0}")]
    UnknownEncoding(u8),
    #[error("Parsing error in {context}: {message}")]
    ParsingError {
        context: &'static str,
        message: String,
    },
}
pub type RdbResult<T> = Result<T, RdbError>;

pub type RdbOk = RdbResult<()>;

#[derive(Debug, PartialEq)]
pub enum Type {
    String,
    List,
    Set,
    SortedSet,
    Hash,
    Stream,
    Module,
}

impl Type {
    pub fn from_encoding(enc_type: u8) -> RdbResult<Type> {
        match enc_type {
            encoding_type::STRING => Ok(Type::String),
            encoding_type::HASH
            | encoding_type::HASH_ZIPMAP
            | encoding_type::HASH_ZIPLIST
            | encoding_type::HASH_LIST_PACK => Ok(Type::Hash),
            encoding_type::LIST
            | encoding_type::LIST_ZIPLIST
            | encoding_type::LIST_QUICKLIST
            | encoding_type::LIST_QUICKLIST_2 => Ok(Type::List),
            encoding_type::SET | encoding_type::SET_INTSET | encoding_type::SET_LIST_PACK => {
                Ok(Type::Set)
            }
            encoding_type::ZSET
            | encoding_type::ZSET_ZIPLIST
            | encoding_type::ZSET_2
            | encoding_type::ZSET_LIST_PACK => Ok(Type::SortedSet),
            encoding_type::STREAM_LIST_PACKS
            | encoding_type::STREAM_LIST_PACKS_2
            | encoding_type::STREAM_LIST_PACKS_3 => Ok(Type::Stream),
            encoding_type::MODULE | encoding_type::MODULE_2 => Ok(Type::Module),
            _ => Err(RdbError::UnknownEncoding(enc_type)),
        }
    }
}

#[derive(Debug, PartialEq)]
pub enum EncodingType {
    String,
    LinkedList,
    Hashtable,
    Skiplist,
    Intset(u64),
    Ziplist(u64),
    Zipmap(u64),
    Quicklist,
    Quicklist2,
    ZSet2,
    ListPack(u64),
}

#[derive(Debug)]
pub enum RdbValue {
    SelectDb(u32),
    ResizeDb {
        db_size: u32,
        expires_size: u32,
    },
    AuxField {
        key: Vec<u8>,
        value: Vec<u8>,
    },
    Checksum(Vec<u8>),
    String {
        key: Vec<u8>,
        value: Vec<u8>,
        expiry: Option<u64>,
    },
    Hash {
        key: Vec<u8>,
        values: IndexMap<Vec<u8>, Vec<u8>>,
        expiry: Option<u64>,
    },
    Set {
        key: Vec<u8>,
        members: Vec<Vec<u8>>,
        expiry: Option<u64>,
    },
    List {
        key: Vec<u8>,
        values: Vec<Vec<u8>>,
        expiry: Option<u64>,
    },
    SortedSet {
        key: Vec<u8>,
        values: Vec<(f64, Vec<u8>)>, // (score, member)
        expiry: Option<u64>,
    },
}

#[cfg(feature = "python")]
impl<'py> IntoPyObject<'py> for RdbValue {
    type Target = PyDict;
    type Output = Bound<'py, PyDict>;
    type Error = PyErr;

    fn into_pyobject(self, py: Python<'py>) -> Result<Self::Output, Self::Error> {
        match self {
            RdbValue::Hash {
                key,
                values,
                expiry,
            } => {
                let dict = PyDict::new(py);
                let values_dict = PyDict::new(py);
                for (k, v) in values {
                    values_dict.set_item(k, v)?;
                }
                dict.set_item("type", "hash")?;
                dict.set_item("key", key)?;
                dict.set_item("values", values_dict)?;
                dict.set_item("expiry", expiry)?;
                Ok(dict)
            }
            RdbValue::List {
                key,
                values,
                expiry,
            } => {
                let dict = PyDict::new(py);
                dict.set_item("type", "list")?;
                dict.set_item("key", key)?;
                dict.set_item("values", values)?;
                dict.set_item("expiry", expiry)?;
                Ok(dict)
            }
            RdbValue::Set {
                key,
                members,
                expiry,
            } => {
                let dict = PyDict::new(py);
                dict.set_item("type", "set")?;
                dict.set_item("key", key)?;
                dict.set_item("members", members)?;
                dict.set_item("expiry", expiry)?;
                Ok(dict)
            }
            RdbValue::SortedSet {
                key,
                values,
                expiry,
            } => {
                let dict = PyDict::new(py);
                dict.set_item("type", "sorted_set")?;
                dict.set_item("key", key)?;
                dict.set_item("values", values)?;
                dict.set_item("expiry", expiry)?;
                Ok(dict)
            }
            RdbValue::String { key, value, expiry } => {
                let dict = PyDict::new(py);
                dict.set_item("type", "string")?;
                dict.set_item("key", key)?;
                dict.set_item("value", value)?;
                dict.set_item("expiry", expiry)?;
                Ok(dict)
            }
            RdbValue::SelectDb(db) => {
                let dict = PyDict::new(py);
                dict.set_item("type", "select_db")?;
                dict.set_item("db", db)?;
                Ok(dict)
            }
            RdbValue::ResizeDb {
                db_size,
                expires_size,
            } => {
                let dict = PyDict::new(py);
                dict.set_item("type", "resize_db")?;
                dict.set_item("db_size", db_size)?;
                dict.set_item("expires_size", expires_size)?;
                Ok(dict)
            }
            RdbValue::AuxField { key, value } => {
                let dict = PyDict::new(py);
                dict.set_item("type", "aux_field")?;
                dict.set_item("key", key)?;
                dict.set_item("value", value)?;
                Ok(dict)
            }
            RdbValue::Checksum(checksum) => {
                let dict = PyDict::new(py);
                dict.set_item("type", "checksum")?;
                dict.set_item("checksum", checksum)?;
                Ok(dict)
            }
        }
    }
}
