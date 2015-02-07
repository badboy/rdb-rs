#![allow(unused_must_use)]
use formatter::Formatter;
use std::old_io;
use serialize::hex::ToHex;
use types::EncodingType;

pub struct Plain {
    out: Box<Writer+'static>,
    dbnum: u32,
    index: u32
}

impl Plain {
    pub fn new() -> Plain {
        let out = Box::new(old_io::stdout());
        Plain { out: out, dbnum: 0, index: 0 }
    }

    fn write_line_start(&mut self) {
        self.out.write_str(format!("db={} ", self.dbnum).as_slice());
    }
}

impl Formatter for Plain {
    fn checksum(&mut self, checksum: &[u8]) {
        self.out.write_str("checksum ");
        self.out.write_str(checksum.to_hex().as_slice());
        self.out.write_str("\n");
    }

    fn start_database(&mut self, db_number: u32) {
        self.dbnum = db_number;
    }

    fn set(&mut self, key: &[u8], value: &[u8], _expiry: Option<u64>) {
        self.write_line_start();
        self.out.write_all(key.as_slice());
        self.out.write_str(" -> ");

        self.out.write_all(value.as_slice());
        self.out.write_str("\n");
        self.out.flush();
    }

    fn aux_field(&mut self, key: &[u8], value: &[u8]) {
        self.out.write_str("aux ");
        self.out.write_all(key.as_slice());
        self.out.write_str(" -> ");
        self.out.write_all(value.as_slice());
        self.out.write_str("\n");
        self.out.flush();
    }

    fn hash_element(&mut self, key: &[u8], field: &[u8], value: &[u8]) {
        self.write_line_start();

        self.out.write_all(key.as_slice());
        self.out.write_str(" . ");
        self.out.write_all(field.as_slice());
        self.out.write_str(" -> ");
        self.out.write_all(value.as_slice());
        self.out.write_str("\n");
        self.out.flush();
    }

    fn set_element(&mut self, key: &[u8], member: &[u8]) {
        self.write_line_start();

        self.out.write_all(key.as_slice());
        self.out.write_str(" { ");
        self.out.write_all(member.as_slice());
        self.out.write_str(" } ");
        self.out.write_str("\n");
        self.out.flush();
    }

    fn start_list(&mut self, _key: &[u8], _length: u32,
                  _expiry: Option<u64>, _info: EncodingType) {
        self.index = 0;
    }
    fn list_element(&mut self, key: &[u8], value: &[u8]) {
        self.write_line_start();

        self.out.write_all(key.as_slice());
        self.out.write_str(format!("[{}]", self.index).as_slice());
        self.out.write_str(" -> ");
        self.out.write_all(value.as_slice());
        self.out.write_str("\n");
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
        self.out.write_str(format!("[{}]", self.index).as_slice());
        self.out.write_str(" -> {");
        self.out.write_all(member.as_slice());
        self.out.write_str(format!(", score={}", score).as_slice());
        self.out.write_str("}\n");
        self.out.flush();
        self.index += 1;
    }
}
