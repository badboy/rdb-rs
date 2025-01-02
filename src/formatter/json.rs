#![allow(unused_must_use)]
use super::write_str;
use crate::formatter::Formatter;
use indexmap::IndexMap;
use rustc_serialize::json;
use std::io;
use std::io::Write;
use std::path::PathBuf;
use std::str;

pub struct JSON {
    out: Box<dyn Write + 'static>,
    is_first_db: bool,
    has_databases: bool,
    is_first_key_in_db: bool,
    elements_in_key: u32,
    element_index: u32,
}

impl JSON {
    pub fn new(file_path: Option<PathBuf>) -> JSON {
        let out: Box<dyn Write> = match file_path {
            Some(path) => match std::fs::File::create(path) {
                Ok(file) => Box::new(file),
                Err(_) => Box::new(io::stdout()),
            },
            None => Box::new(io::stdout()),
        };

        JSON {
            out,
            is_first_db: true,
            has_databases: false,
            is_first_key_in_db: true,
            elements_in_key: 0,
            element_index: 0,
        }
    }

    fn start_key(&mut self, length: u32) {
        if !self.is_first_key_in_db {
            write_str(&mut self.out, ",");
        }

        self.is_first_key_in_db = false;
        self.elements_in_key = length;
        self.element_index = 0;
    }

    fn end_key(&mut self) {}

    fn write_comma(&mut self) {
        if self.element_index > 0 {
            write_str(&mut self.out, ",");
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

fn encode_to_ascii(value: &[u8]) -> String {
    match str::from_utf8(value) {
        Ok(s) => json::encode(&s).unwrap(),
        Err(_) => {
            let s: String = value
                .iter()
                .map(|&b| {
                    if (32..127).contains(&b) {
                        // ASCII printable characters
                        (b as char).to_string()
                    } else {
                        format!("\\u{:04x}", b as u16)
                    }
                })
                .collect();
            format!("\"{}\"", s)
        }
    }
}

impl Formatter for JSON {
    fn start_rdb(&mut self) {
        write_str(&mut self.out, "[");
    }

    fn end_rdb(&mut self) {
        if self.has_databases {
            write_str(&mut self.out, "}");
        }
        write_str(&mut self.out, "]\n");
    }

    fn start_database(&mut self, _db_number: u32) {
        if !self.is_first_db {
            write_str(&mut self.out, "},");
        }

        write_str(&mut self.out, "{");
        self.is_first_db = false;
        self.has_databases = true;
        self.is_first_key_in_db = true;
    }

    fn string(&mut self, key: &[u8], value: &[u8], _expiry: &Option<u64>) {
        self.start_key(0);
        self.write_key(key);
        write_str(&mut self.out, ":");
        self.write_value(value);
    }

    fn hash(&mut self, key: &[u8], values: &IndexMap<Vec<u8>, Vec<u8>>, _expiry: &Option<u64>) {
        self.start_key(values.len() as u32);
        self.write_key(key);
        write_str(&mut self.out, ":{");
        for (field, value) in values {
            self.write_comma();
            self.write_key(field);
            write_str(&mut self.out, ":");
            self.write_value(value);
        }
        self.end_key();
        write_str(&mut self.out, "}");
    }

    fn set(&mut self, key: &[u8], values: &[Vec<u8>], _expiry: &Option<u64>) {
        self.start_key(values.len() as u32);
        self.write_key(key);
        write_str(&mut self.out, ":[");
        for value in values {
            self.write_comma();
            self.write_value(value);
        }
        self.end_key();
        write_str(&mut self.out, "]");
    }

    fn list(&mut self, key: &[u8], values: &[Vec<u8>], _expiry: &Option<u64>) {
        self.start_key(values.len() as u32);
        self.write_key(key);
        write_str(&mut self.out, ":[");
        for value in values {
            self.write_comma();
            self.write_value(value);
        }
        self.end_key();
        write_str(&mut self.out, "]");
    }

    fn sorted_set(&mut self, key: &[u8], values: &[(f64, Vec<u8>)], _expiry: &Option<u64>) {
        self.start_key(values.len() as u32);
        self.write_key(key);
        write_str(&mut self.out, ":{");
        for (score, member) in values {
            self.write_comma();
            self.write_key(member);
            write_str(&mut self.out, ":");
            self.write_value(score.to_string().as_bytes());
        }
        self.end_key();
        write_str(&mut self.out, "}");
    }
}
