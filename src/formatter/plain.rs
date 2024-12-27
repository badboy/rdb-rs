#![allow(unused_must_use)]
use super::write_str;
use crate::formatter::Formatter;
use indexmap::IndexMap;
use serialize::hex::ToHex;
use std::io;
use std::io::Write;
use std::path::PathBuf;

pub struct Plain {
    out: Box<dyn Write + 'static>,
    dbnum: u32,
}

impl Plain {
    pub fn new(file_path: Option<PathBuf>) -> Plain {
        let out: Box<dyn Write> = match file_path {
            Some(path) => match std::fs::File::create(path) {
                Ok(file) => Box::new(file),
                Err(_) => Box::new(io::stdout()),
            },
            None => Box::new(io::stdout()),
        };

        Plain { out, dbnum: 0 }
    }

    fn write_line_start(&mut self) {
        write_str(&mut self.out, &format!("db={} ", self.dbnum));
    }

    fn hash_element(&mut self, key: &[u8], field: &[u8], value: &[u8]) {
        self.write_line_start();

        self.out.write_all(key);
        write_str(&mut self.out, " . ");
        self.out.write_all(field);
        write_str(&mut self.out, " -> ");
        self.out.write_all(value);
        write_str(&mut self.out, "\n");
        self.out.flush();
    }

    fn set_element(&mut self, key: &[u8], member: &[u8]) {
        self.write_line_start();

        self.out.write_all(key);
        write_str(&mut self.out, " { ");
        self.out.write_all(member);
        write_str(&mut self.out, " } ");
        write_str(&mut self.out, "\n");
        self.out.flush();
    }

    fn list_element(&mut self, index: usize, key: &[u8], value: &[u8]) {
        self.write_line_start();

        self.out.write_all(key);
        write_str(&mut self.out, &format!("[{}]", index));
        write_str(&mut self.out, " -> ");
        self.out.write_all(value);
        write_str(&mut self.out, "\n");
        self.out.flush();
    }

    fn sorted_set_element(&mut self, index: usize, key: &[u8], score: f64, member: &[u8]) {
        self.write_line_start();

        self.out.write_all(key);
        write_str(&mut self.out, &format!("[{}]", index));
        write_str(&mut self.out, " -> {");
        self.out.write_all(member);
        write_str(&mut self.out, &format!(", score={}", score));
        write_str(&mut self.out, "}\n");
        self.out.flush();
    }
}

impl Formatter for Plain {
    fn string(&mut self, key: &Vec<u8>, value: &Vec<u8>, _expiry: &Option<u64>) {
        self.write_line_start();
        self.out.write_all(key);
        write_str(&mut self.out, " -> ");
        self.out.write_all(value);
        write_str(&mut self.out, "\n");
        self.out.flush();
    }

    fn hash(&mut self, key: &Vec<u8>, values: &IndexMap<Vec<u8>, Vec<u8>>, _expiry: &Option<u64>) {
        for (field, value) in values {
            self.hash_element(key, field, value);
        }
    }

    fn set(&mut self, key: &Vec<u8>, values: &Vec<Vec<u8>>, _expiry: &Option<u64>) {
        for value in values {
            self.set_element(key, value);
        }
    }

    fn list(&mut self, key: &Vec<u8>, values: &Vec<Vec<u8>>, _expiry: &Option<u64>) {
        for (i, value) in values.iter().enumerate() {
            self.list_element(i, key, value);
        }
    }

    fn sorted_set(&mut self, key: &Vec<u8>, values: &Vec<(f64, Vec<u8>)>, _expiry: &Option<u64>) {
        for (i, (score, member)) in values.iter().enumerate() {
            self.sorted_set_element(i, key, *score, member);
        }
    }

    fn checksum(&mut self, checksum: &[u8]) {
        if checksum.len() != 0 {
            write_str(&mut self.out, "checksum ");
            write_str(&mut self.out, &checksum.to_hex());
            write_str(&mut self.out, "\n");
        }
    }

    fn start_database(&mut self, db_number: u32) {
        self.dbnum = db_number;
    }

    fn aux_field(&mut self, key: &[u8], value: &[u8]) {
        write_str(&mut self.out, "aux ");
        self.out.write_all(key);
        write_str(&mut self.out, " -> ");
        self.out.write_all(value);
        write_str(&mut self.out, "\n");
        self.out.flush();
    }
}
