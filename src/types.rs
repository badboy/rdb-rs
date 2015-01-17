use std::error;
use constants::encoding_type;

pub enum Value {
    Blob(Vec<u8>),
    List(Vec<Vec<u8>>),
    IntegerList(Vec<i64>),
    Set(Vec<Vec<u8>>),
    SortedSet(Vec<(Vec<u8>,f64)>),
    Hash(Vec<(Vec<u8>,Vec<u8>)>)
}

#[derive(PartialEq, Eq, Clone, Show)]
pub struct RdbError;

pub type RdbResult<T> = Result<T, RdbError>;

#[derive(Copy,PartialEq)]
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
