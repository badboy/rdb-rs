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

#[allow(unused_variables)]
pub trait Formatter {
    fn format(&mut self, value: &RdbValue) -> std::io::Result<()>;
}
