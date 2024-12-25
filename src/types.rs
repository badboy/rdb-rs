use std::collections::HashSet;
use std::io::Error as IoError;

use indexmap::IndexMap;

use crate::constants::encoding_type;

pub type RdbError = IoError;

pub type RdbResult<T> = Result<T, RdbError>;

pub type RdbOk = RdbResult<()>;

#[derive(Debug, PartialEq)]
pub enum Type {
    String,
    List,
    Set,
    SortedSet,
    Hash,
    Stream, // New type for streams
    Module, // New type for module data
}

impl Type {
    pub fn from_encoding(enc_type: u8) -> Type {
        match enc_type {
            encoding_type::STRING => Type::String,
            encoding_type::HASH
            | encoding_type::HASH_ZIPMAP
            | encoding_type::HASH_ZIPLIST
            | encoding_type::HASH_LIST_PACK => Type::Hash,
            encoding_type::LIST
            | encoding_type::LIST_ZIPLIST
            | encoding_type::LIST_QUICKLIST
            | encoding_type::LIST_QUICKLIST_2 => Type::List,
            encoding_type::SET | encoding_type::SET_INTSET | encoding_type::SET_LIST_PACK => {
                Type::Set
            }
            encoding_type::ZSET
            | encoding_type::ZSET_ZIPLIST
            | encoding_type::ZSET_2
            | encoding_type::ZSET_LIST_PACK => Type::SortedSet,
            encoding_type::STREAM_LIST_PACKS
            | encoding_type::STREAM_LIST_PACKS_2
            | encoding_type::STREAM_LIST_PACKS_3 => Type::Stream,
            encoding_type::MODULE | encoding_type::MODULE_2 => Type::Module,
            _ => {
                panic!("Unknown encoding type: {}", enc_type)
            }
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
    Quicklist2,    // New Quicklist v2 encoding
    ZSet2,         // New ZSet2 encoding
    ListPack(u64), // New ListPack encoding
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
    Skipped,
}
