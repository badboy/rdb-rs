#![allow(unused_must_use)]
#![allow(unused_variables)]

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
}

impl RdbParseFormatter for JSONFormatter {
    fn start_rdb(&mut self) {
        self.out.write_str("[");
    }

    fn end_rdb(&mut self) {
        if self.has_databases {
            self.out.write_str("}");
        }
        self.out.write_str("]");
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

    fn set(&mut self, key: Vec<u8>, value: Vec<u8>, __expiry: Option<u32>) {
        self.start_key(0);
        self.out.write_str("\"");
        self.out.write(encode_key(key).as_slice());
        self.out.write_str("\":\"");
        self.out.write(encode_key(value).as_slice());
        self.out.write_str("\"");
    }

    fn start_hash(&mut self, key: Vec<u8>, length: u32,
                  _expiry: Option<u32>, _info: Option<()>) {}
    fn end_hash(&mut self, key: Vec<u8>) {}
    fn hash_element(&mut self, key: Vec<u8>, field: Vec<u8>, value: Vec<u8>) {}


    fn start_set(&mut self, key: Vec<u8>, cardinality: u32,
                 _expiry: Option<u32>, _info: Option<()>) {
        self.start_key(cardinality);
        self.out.write_str("\"");
        self.out.write(encode_key(key).as_slice());
        self.out.write_str("\":[");
    }
    fn end_set(&mut self, key: Vec<u8>) {
        self.end_key();
        self.out.write_str("]");
    }
    fn set_element(&mut self, key: Vec<u8>, member: Vec<u8>) {
        self.write_comma();
        self.out.write(encode_value(key).as_slice());
    }


    fn start_list(&mut self, key: Vec<u8>, length: u32,
                  _expiry: Option<u32>, _info: Option<()>) {}
    fn end_list(&mut self, key: Vec<u8>) {}
    fn list_element(&mut self, key: Vec<u8>, value: Vec<u8>) {}

    fn start_sorted_set(&mut self, key: Vec<u8>, length: u32,
                        _expiry: Option<u32>, _info: Option<()>) {}
    fn end_sorted_set(&mut self, key: Vec<u8>) {}
    fn sorted_set_element(&mut self, key: Vec<u8>,
                          score: f64, member: Vec<u8>) {

    }

    //fn aux_field(&mut self, key: Vec<u8>, value: Vec<u8>) {
        //let _ = self.out.write_str("[aux] ");
        //let _ = self.out.write(key.as_slice());
        //let _ = self.out.write_str(": ");
        //let _ = self.out.write(value.as_slice());
        //let _ = self.out.write_str("\n");
        //let _ = self.out.flush();
    //}
}
