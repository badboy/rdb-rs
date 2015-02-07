use std::old_io::IoError;

use constants::encoding_type;

#[derive(Debug,Clone)]
pub enum ZiplistEntry {
    String(Vec<u8>),
    Number(i64),
}

pub type RdbError = IoError;

pub type RdbResult<T> = Result<T, RdbError>;

pub type RdbOk = RdbResult<()>;

#[derive(Debug,Copy,PartialEq)]
pub enum Type {
    String,
    List,
    Set,
    SortedSet,
    Hash
}

impl Type {
    pub fn from_encoding(enc_type: u8) -> Type {
        match enc_type {
            encoding_type::STRING => Type::String,
            encoding_type::HASH | encoding_type::HASH_ZIPMAP | encoding_type::HASH_ZIPLIST => Type::Hash,
            encoding_type::LIST | encoding_type::LIST_ZIPLIST => Type::List,
            encoding_type::SET | encoding_type::SET_INTSET => Type::Set,
            encoding_type::ZSET | encoding_type::ZSET_ZIPLIST => Type::SortedSet,
            _ => { panic!("Unknown encoding type: {}", enc_type) }
        }
    }
}

#[derive(Copy)]
pub enum EncodingType {
    String,
    LinkedList,
    Hashtable,
    Skiplist,
    Intset(u64),
    Ziplist(u64),
    Zipmap(u64),
    Quicklist
}
