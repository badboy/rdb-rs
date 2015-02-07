#![allow(unused_must_use)]

use formatter::Formatter;
use std::old_io;
use std::str;
use serialize::json;
use types::EncodingType;

pub struct JSON {
    out: Box<Writer+'static>,
    is_first_db: bool,
    has_databases: bool,
    is_first_key_in_db: bool,
    elements_in_key: u32,
    element_index: u32
}

impl JSON {
    pub fn new() -> JSON {
        let out = Box::new(old_io::stdout());
        JSON {
            out: out,
            is_first_db: true,
            has_databases: false,
            is_first_key_in_db: true,
            elements_in_key: 0,
            element_index: 0
        }
    }
}

fn encode_to_ascii(value: &[u8]) -> String {
    let s = unsafe{str::from_utf8_unchecked(value)};
    json::encode(&s).unwrap()
}

impl JSON {
    fn start_key(&mut self, length: u32) {
        if !self.is_first_key_in_db {
            self.out.write_str(",");
        }

        self.is_first_key_in_db = false;
        self.elements_in_key = length;
        self.element_index = 0;
    }

    fn end_key(&mut self) { }

    fn write_comma(&mut self) {
        if self.element_index > 0 {
            self.out.write_str(",");
        }
        self.element_index += 1;
    }

    fn write_key(&mut self, key: &[u8]) {
        self.out.write_all(encode_to_ascii(key).as_bytes());
    }
    fn write_value(&mut self, value: &[u8]) {
        self.out.write_all(encode_to_ascii(value).as_bytes());
    }
}

impl Formatter for JSON {
    fn start_rdb(&mut self) {
        self.out.write_str("[");
    }

    fn end_rdb(&mut self) {
        if self.has_databases {
            self.out.write_str("}");
        }
        self.out.write_str("]\n");
    }

    fn start_database(&mut self, _db_number: u32) {
        if !self.is_first_db {
            self.out.write_str("},");
        }

        self.out.write_str("{");
        self.is_first_db = false;
        self.has_databases = true;
        self.is_first_key_in_db = true;
    }

    fn set(&mut self, key: &[u8], value: &[u8], _expiry: Option<u64>) {
        self.start_key(0);
        self.write_key(key);
        self.out.write_str(":");
        self.write_value(value);
    }

    fn start_hash(&mut self, key: &[u8], length: u32,
                  _expiry: Option<u64>, _info: EncodingType) {
        self.start_key(length);
        self.write_key(key);
        self.out.write_str(":{");
        self.out.flush();
    }

    fn end_hash(&mut self, _key: &[u8]) {
        self.end_key();
        self.out.write_str("}");
        self.out.flush();
    }

    fn hash_element(&mut self, _key: &[u8], field: &[u8], value: &[u8]) {
        self.write_comma();
        self.write_key(field);
        self.out.write_str(":");
        self.write_value(value);
        self.out.flush();
    }

    fn start_set(&mut self, key: &[u8], cardinality: u32,
                 _expiry: Option<u64>, _info: EncodingType) {
        self.start_key(cardinality);
        self.write_key(key);
        self.out.write_str(":[");
        self.out.flush();
    }

    fn end_set(&mut self, _key: &[u8]) {
        self.end_key();
        self.out.write_str("]");
    }

    fn set_element(&mut self, _key: &[u8], member: &[u8]) {
        self.write_comma();
        self.write_value(member);
    }

    fn start_list(&mut self, key: &[u8], length: u32,
                  _expiry: Option<u64>, _info: EncodingType) {
        self.start_key(length);
        self.write_key(key);
        self.out.write_str(":[");
    }

    fn end_list(&mut self, _key: &[u8]) {
        self.end_key();
        self.out.write_str("]");
    }

    fn list_element(&mut self, _key: &[u8], value: &[u8]) {
        self.write_comma();
        self.write_value(value);
    }

    fn start_sorted_set(&mut self, key: &[u8], length: u32,
                        _expiry: Option<u64>, _info: EncodingType) {
        self.start_key(length);
        self.write_key(key);
        self.out.write_str(":{");
    }

    fn end_sorted_set(&mut self, _key: &[u8]) {
        self.end_key();
        self.out.write_str("}");
    }

    fn sorted_set_element(&mut self, _key: &[u8],
                          score: f64, member: &[u8]) {
        self.write_comma();
        self.write_key(member);
        self.out.write_str(":");
        self.write_value(score.to_string().as_bytes());
    }

}
