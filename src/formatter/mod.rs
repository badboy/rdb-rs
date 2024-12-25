use std::io::Write;

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

pub trait Formatter {
    fn start_rdb(&mut self) {}
    fn end_rdb(&mut self) {}
    fn checksum(&mut self, checksum: &[u8]) {}

    fn start_database(&mut self, db_index: u32) {}
    fn end_database(&mut self, db_index: u32) {}

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
    fn format(&mut self, value: &RdbValue) -> std::io::Result<()>;
}
