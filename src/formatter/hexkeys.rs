#![allow(unused_must_use)]

use formatter::Formatter;
use std::io;
use std::io::Write;
use super::write_str;
use serialize::hex::{ToHex};

pub struct HexKeys {
    out: Box<Write+'static>,
}

impl HexKeys {
    pub fn new() -> HexKeys {
        let out = Box::new(io::stdout());
        HexKeys {
            out: out,
        }
    }
}

impl Formatter for HexKeys {
    fn should_read_objects(&mut self) -> bool { false }

    fn matched_key(&mut self, key: &[u8]) {
        self.out.write_all(key.to_hex().as_bytes());
        write_str(&mut self.out, "\n");
    }
}
