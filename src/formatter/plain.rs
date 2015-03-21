#![allow(unused_must_use)]
use formatter::Formatter;
use std::io;
use std::io::Write;
use serialize::hex::ToHex;
use types::EncodingType;
use super::write_str;

pub struct Plain {
    out: Box<Write+'static>,
    dbnum: u32,
    index: u32
}

impl Plain {
    pub fn new() -> Plain {
        let out = Box::new(io::stdout());
        Plain { out: out, dbnum: 0, index: 0 }
    }

    fn write_line_start(&mut self) {
        write_str(&mut self.out, format!("db={} ", self.dbnum).as_slice());
    }
}

impl Formatter for Plain {
    fn checksum(&mut self, checksum: &[u8]) {
        write_str(&mut self.out, "checksum ");
        write_str(&mut self.out, checksum.to_hex().as_slice());
        write_str(&mut self.out, "\n");
    }

    fn start_database(&mut self, db_number: u32) {
        self.dbnum = db_number;
    }

    fn set(&mut self, key: &[u8], value: &[u8], _expiry: Option<u64>) {
        self.write_line_start();
        self.out.write_all(key.as_slice());
        write_str(&mut self.out, " -> ");

        self.out.write_all(value.as_slice());
        write_str(&mut self.out, "\n");
        self.out.flush();
    }

    fn aux_field(&mut self, key: &[u8], value: &[u8]) {
        write_str(&mut self.out, "aux ");
        self.out.write_all(key.as_slice());
        write_str(&mut self.out, " -> ");
        self.out.write_all(value.as_slice());
        write_str(&mut self.out, "\n");
        self.out.flush();
    }

    fn hash_element(&mut self, key: &[u8], field: &[u8], value: &[u8]) {
        self.write_line_start();

        self.out.write_all(key.as_slice());
        write_str(&mut self.out, " . ");
        self.out.write_all(field.as_slice());
        write_str(&mut self.out, " -> ");
        self.out.write_all(value.as_slice());
        write_str(&mut self.out, "\n");
        self.out.flush();
    }

    fn set_element(&mut self, key: &[u8], member: &[u8]) {
        self.write_line_start();

        self.out.write_all(key.as_slice());
        write_str(&mut self.out, " { ");
        self.out.write_all(member.as_slice());
        write_str(&mut self.out, " } ");
        write_str(&mut self.out, "\n");
        self.out.flush();
    }

    fn start_list(&mut self, _key: &[u8], _length: u32,
                  _expiry: Option<u64>, _info: EncodingType) {
        self.index = 0;
    }
    fn list_element(&mut self, key: &[u8], value: &[u8]) {
        self.write_line_start();

        self.out.write_all(key.as_slice());
        write_str(&mut self.out, format!("[{}]", self.index).as_slice());
        write_str(&mut self.out, " -> ");
        self.out.write_all(value.as_slice());
        write_str(&mut self.out, "\n");
        self.out.flush();
        self.index += 1;
    }

    fn start_sorted_set(&mut self, _key: &[u8], _length: u32,
                        _expiry: Option<u64>, _info: EncodingType) {
        self.index = 0;
    }

    fn sorted_set_element(&mut self, key: &[u8],
                          score: f64, member: &[u8]) {
        self.write_line_start();

        self.out.write_all(key.as_slice());
        write_str(&mut self.out, format!("[{}]", self.index).as_slice());
        write_str(&mut self.out, " -> {");
        self.out.write_all(member.as_slice());
        write_str(&mut self.out, format!(", score={}", score).as_slice());
        write_str(&mut self.out, "}\n");
        self.out.flush();
        self.index += 1;
    }
}
