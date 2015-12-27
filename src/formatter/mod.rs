use std::io::{Read, Write};

pub use self::nil::Nil;
pub use self::plain::Plain;
pub use self::json::JSON;
pub use self::protocol::Protocol;

use super::RdbParser;
use super::filter::Filter;
use super::iterator_type::RdbIteratorType::*;

mod nil;
mod plain;
mod json;
mod protocol;


pub fn write_str<W: Write>(out: &mut W, data: &str) {
    out.write(data.as_bytes()).unwrap();
}

#[allow(unused_variables)]
pub trait Formatter {
    fn start_rdb(&mut self) {}
    fn end_rdb(&mut self) {}
    fn checksum(&mut self, checksum: &[u8]) {}

    fn start_database(&mut self, db_index: u32) {}

    fn resizedb(&mut self, db_size: u32, expires_size: u32) {}
    fn aux_field(&mut self, key: &[u8], value: &[u8]) {}

    fn set(&mut self, key: &[u8], value: &[u8], expiry: Option<u64>) {}

    fn start_hash(&mut self, key: &[u8], length: u32, expiry: Option<u64>) {}
    fn end_hash(&mut self, key: &[u8]) {}
    fn hash_element(&mut self, key: &[u8], field: &[u8], value: &[u8]) {}


    fn start_set(&mut self, key: &[u8], cardinality: u32, expiry: Option<u64>) {}
    fn end_set(&mut self, key: &[u8]) {}
    fn set_element(&mut self, key: &[u8], member: &[u8]) {}

    fn start_list(&mut self, key: &[u8], length: u32, expiry: Option<u64>) {}
    fn end_list(&mut self, key: &[u8]) {}
    fn list_element(&mut self, key: &[u8], value: &[u8]) {}

    fn start_sorted_set(&mut self, key: &[u8], length: u32, expiry: Option<u64>) {}
    fn end_sorted_set(&mut self, key: &[u8]) {}
    fn sorted_set_element(&mut self, key: &[u8], score: f64, member: &[u8]) {}
}

pub fn print_formatted<R: Read, F: Filter>(parser: RdbParser<R, F>, fmt: &mut Formatter) {
    let mut key: Option<Vec<u8>> = None;
    let mut expire = None;

    fmt.start_rdb();

    for value in parser {
        let value = value.unwrap();
        match value {
            RdbEnd             => fmt.end_rdb(),
            StartDatabase(idx) => fmt.start_database(idx),
            ResizeDB(a, b)     => fmt.resizedb(a, b),
            AuxiliaryKey(a,b)  => fmt.aux_field(&a, &b),
            Checksum(cks)      => fmt.checksum(&cks),
            Key(k, exp)        => {
                key = Some(k);
                expire = exp;
            },

            Blob(blob)                    => fmt.set(&key.as_ref().unwrap(), &blob, expire),

            ListStart(len)                => fmt.start_list(&key.as_ref().unwrap(), len, expire),
            HashStart(len)                => fmt.start_hash(&key.as_ref().unwrap(), len, expire),
            SetStart(len)                 => fmt.start_set(&key.as_ref().unwrap(), len, expire),
            SortedSetStart(len)           => fmt.start_sorted_set(&key.as_ref().unwrap(), len, expire),

            ListEnd                       => fmt.end_list(&key.as_ref().unwrap()),
            HashEnd                       => fmt.end_hash(&key.as_ref().unwrap()),
            SetEnd                        => fmt.end_set(&key.as_ref().unwrap()),
            SortedSetEnd                  => fmt.end_sorted_set(&key.as_ref().unwrap()),

            ListElement(elem)             => fmt.list_element(&key.as_ref().unwrap(), &elem),
            HashElement(field, value)     => fmt.hash_element(&key.as_ref().unwrap(), &field, &value),
            SetElement(elem)              => fmt.set_element(&key.as_ref().unwrap(), &elem),
            SortedSetElement(score, elem) => fmt.sorted_set_element(&key.as_ref().unwrap(), score, &elem),


            val @ _ => {
                panic!("not implemented: {:?}", val);
            },
        };
    }
}
