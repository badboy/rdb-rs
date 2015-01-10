#![allow(unused_must_use)]

use formatter::RdbParseFormatter;
use std::io;

pub struct JSONFormatter {
    out: Box<Writer+'static>,
    is_first_db: bool,
    has_databases: bool,
    is_first_key_in_db: bool,
    elements_in_key: u32,
    element_index: u32
}

impl JSONFormatter {
    pub fn new() -> JSONFormatter {
        let out = Box::new(io::stdout());
        JSONFormatter {
            out: out,
            is_first_db: true,
            has_databases: false,
            is_first_key_in_db: true,
            elements_in_key: 0,
            element_index: 0
        }
    }
}

fn encode_key<'a>(key: Vec<u8>) -> Vec<u8> {
    key
}

fn encode_value<'a>(value: Vec<u8>) -> Vec<u8> {
    value
}

impl JSONFormatter {
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
        if self.element_index > 0 && self.element_index < self.elements_in_key {
            self.out.write_str(",");
        }
        self.element_index += 1;
    }

    fn write_key(&mut self, key: &[u8]) {
        self.out.write_str("\"");
        self.out.write(key);
        self.out.write_str("\"");
    }
    fn write_value(&mut self, value: &[u8]) {
        self.out.write_str("\"");
        self.out.write(value);
        self.out.write_str("\"");
    }
}

impl RdbParseFormatter for JSONFormatter {
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
            self.out.write_str("}");
        }

        self.out.write_str("{");
        self.is_first_db = false;
        self.has_databases = true;
        self.is_first_key_in_db = true;
    }

    fn set(&mut self, key: &[u8], value: &[u8], _expiry: Option<u32>) {
        self.start_key(0);
        self.write_key(key.as_slice());
        self.out.write_str(":");
        self.write_key(value.as_slice());
    }

    fn start_hash(&mut self, key: &[u8], length: u32,
                  _expiry: Option<u32>, _info: Option<()>) {
        self.start_key(length);
        self.write_key(key.as_slice());
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
        self.write_key(field.as_slice());
        self.out.write_str(":");
        self.write_value(value.as_slice());
        self.out.flush();
    }


    fn start_set(&mut self, key: &[u8], cardinality: u32,
                 _expiry: Option<u32>, _info: Option<()>) {
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
        self.write_value(member.as_slice());
    }


    fn start_list(&mut self, key: &[u8], length: u32,
                  _expiry: Option<u32>, _info: Option<()>) {
        self.start_key(length);
        self.write_key(key.as_slice());
        self.out.write_str(":[");
    }
    fn end_list(&mut self, _key: &[u8]) {
        self.end_key();
        self.out.write_str("]");
    }
    fn list_element(&mut self, _key: &[u8], value: &[u8]) {
        self.write_comma();
        self.write_value(value.as_slice());
    }

    fn start_sorted_set(&mut self, key: &[u8], length: u32,
                        _expiry: Option<u32>, _info: Option<()>) {
        self.start_key(length);
        self.write_key(key.as_slice());
        self.out.write_str(":{");
    }
    fn end_sorted_set(&mut self, _key: &[u8]) {
        self.end_key();
        self.out.write_str("}");
    }
    fn sorted_set_element(&mut self, _key: &[u8],
                          score: f64, member: &[u8]) {
        self.write_comma();
        self.write_key(member.as_slice());
        self.out.write_str(":");
        self.out.write_str(score.to_string().as_slice());
    }

}
