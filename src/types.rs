use std::io::Error as IoError;

use constants::encoding_type;

#[derive(Debug, Clone)]
pub enum ZiplistEntry {
    String(Vec<u8>),
    Number(i64),
}

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
    Stream,       // New type for streams
    Module,       // New type for module data
}

impl Type {
    pub fn from_encoding(enc_type: u8) -> Type {
        match enc_type {
            encoding_type::STRING => Type::String,
            encoding_type::HASH | encoding_type::HASH_ZIPMAP | encoding_type::HASH_ZIPLIST | encoding_type::HASH_LIST_PACK => Type::Hash,
            encoding_type::LIST | encoding_type::LIST_ZIPLIST | encoding_type::LIST_QUICKLIST | encoding_type::LIST_QUICKLIST_2 => Type::List,
            encoding_type::SET | encoding_type::SET_INTSET | encoding_type::SET_LIST_PACK => Type::Set,
            encoding_type::ZSET | encoding_type::ZSET_ZIPLIST | encoding_type::ZSET_2 | encoding_type::ZSET_LIST_PACK => Type::SortedSet,
            encoding_type::STREAM_LIST_PACKS | encoding_type::STREAM_LIST_PACKS_2 | encoding_type::STREAM_LIST_PACKS_3 => Type::Stream,
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
    Quicklist2,         // New Quicklist v2 encoding
    ZSet2,              // New ZSet2 encoding
    ListPack(u64),      // New ListPack encoding
}
