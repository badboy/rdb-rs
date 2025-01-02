use std::io::Write;

use indexmap::IndexMap;

pub use self::json::JSON;
pub use self::nil::Nil;
pub use self::plain::Plain;
pub use self::protocol::Protocol;

use super::types::RdbValue;

pub mod json;
pub mod nil;
pub mod plain;
pub mod protocol;

pub fn write_str<W: Write>(out: &mut W, data: &str) {
    out.write(data.as_bytes()).unwrap();
}

#[allow(unused_variables)]
pub trait Formatter {
    fn start_rdb(&mut self) {}
    fn end_rdb(&mut self) {}
    fn checksum(&mut self, checksum: &[u8]) {}

    fn start_database(&mut self, db_index: u32) {}
    fn end_database(&mut self, db_index: u32) {}

    fn resizedb(&mut self, db_size: u32, expires_size: u32) {}
    fn aux_field(&mut self, key: &[u8], value: &[u8]) {}

    fn string(&mut self, key: &Vec<u8>, value: &Vec<u8>, expiry: &Option<u64>) {}

    fn hash(&mut self, key: &Vec<u8>, values: &IndexMap<Vec<u8>, Vec<u8>>, expiry: &Option<u64>) {}

    fn set(&mut self, key: &Vec<u8>, values: &Vec<Vec<u8>>, expiry: &Option<u64>) {}

    fn list(&mut self, key: &Vec<u8>, values: &Vec<Vec<u8>>, expiry: &Option<u64>) {}

    fn sorted_set(&mut self, key: &Vec<u8>, values: &Vec<(f64, Vec<u8>)>, expiry: &Option<u64>) {}

    fn format(&mut self, value: &RdbValue) -> std::io::Result<()> {
        match value {
            RdbValue::Set {
                key,
                members,
                expiry,
            } => {
                self.set(key, members, expiry);
                Ok(())
            }
            RdbValue::Hash {
                key,
                values,
                expiry,
            } => {
                self.hash(key, values, expiry);
                Ok(())
            }
            RdbValue::List {
                key,
                values,
                expiry,
            } => {
                self.list(key, values, expiry);
                Ok(())
            }
            RdbValue::SortedSet {
                key,
                values,
                expiry,
            } => {
                self.sorted_set(key, values, expiry);
                Ok(())
            }
            RdbValue::String { key, value, expiry } => {
                self.string(key, value, expiry);
                Ok(())
            }
            RdbValue::SelectDb(db_number) => {
                self.start_database(*db_number);
                Ok(())
            }
            RdbValue::ResizeDb {
                db_size,
                expires_size,
            } => Ok(()),
            RdbValue::AuxField { key, value } => {
                self.aux_field(key, value);
                Ok(())
            }
            RdbValue::Checksum(checksum) => {
                self.checksum(checksum);
                Ok(())
            }
        }
    }
}

pub enum FormatterType {
    Json(JSON),
    Plain(Plain),
    Nil(Nil),
    Protocol(Protocol),
}

impl Formatter for FormatterType {
    fn format(&mut self, value: &RdbValue) -> std::io::Result<()> {
        match self {
            Self::Json(f) => f.format(value),
            Self::Plain(f) => f.format(value),
            Self::Nil(f) => f.format(value),
            Self::Protocol(f) => f.format(value),
        }
    }
}
