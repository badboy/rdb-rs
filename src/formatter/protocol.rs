#![allow(unused_must_use)]
use indexmap::IndexMap;

use super::write_str;
use crate::formatter::Formatter;
use std::io;
use std::io::Write;
use std::path::PathBuf;

pub struct Protocol {
    out: Box<dyn Write + 'static>,
    last_expiry: Option<u64>,
}

impl Protocol {
    pub fn new(file_path: Option<PathBuf>) -> Protocol {
        let out: Box<dyn Write> = match file_path {
            Some(path) => match std::fs::File::create(path) {
                Ok(file) => Box::new(file),
                Err(_) => Box::new(io::stdout()),
            },
            None => Box::new(io::stdout()),
        };

        Protocol {
            out,
            last_expiry: None,
        }
    }
}

impl Protocol {
    fn emit(&mut self, args: Vec<&[u8]>) {
        write_str(&mut self.out, "*");
        self.out.write_all(args.len().to_string().as_bytes());
        write_str(&mut self.out, "\r\n");
        for arg in &args {
            write_str(&mut self.out, "$");
            self.out.write_all(arg.len().to_string().as_bytes());
            write_str(&mut self.out, "\r\n");
            self.out.write_all(arg);
            write_str(&mut self.out, "\r\n");
        }
    }

    fn pre_expire(&mut self, expiry: &Option<u64>) {
        self.last_expiry = expiry.clone();
    }

    fn post_expire(&mut self, key: &[u8]) {
        if let Some(expire) = self.last_expiry {
            let expire = expire.to_string();
            self.emit(vec!["PEXPIREAT".as_bytes(), key, expire.as_bytes()]);
            self.last_expiry = None;
        }
    }

    fn set(&mut self, key: &[u8], value: &[u8], expiry: &Option<u64>) {
        self.pre_expire(expiry);
        self.emit(vec!["SET".as_bytes(), key, value]);
        self.post_expire(key);
    }

    fn start_hash(&mut self, expiry: &Option<u64>) {
        self.pre_expire(expiry);
    }
    fn end_hash(&mut self, key: &[u8]) {
        self.post_expire(key);
    }
    fn hash_element(&mut self, key: &[u8], field: &[u8], value: &[u8]) {
        self.emit(vec!["HSET".as_bytes(), key, field, value]);
    }

    fn start_set(&mut self, expiry: &Option<u64>) {
        self.pre_expire(expiry);
    }
    fn end_set(&mut self, key: &[u8]) {
        self.post_expire(key);
    }
    fn set_element(&mut self, key: &[u8], member: &[u8]) {
        self.emit(vec!["SADD".as_bytes(), key, member]);
    }

    fn start_list(&mut self, expiry: &Option<u64>) {
        self.pre_expire(expiry);
    }
    fn end_list(&mut self, key: &[u8]) {
        self.post_expire(key);
    }
    fn list_element(&mut self, key: &[u8], value: &[u8]) {
        self.emit(vec!["RPUSH".as_bytes(), key, value]);
    }

    fn start_sorted_set(&mut self, expiry: &Option<u64>) {
        self.pre_expire(expiry);
    }
    fn end_sorted_set(&mut self, key: &[u8]) {
        self.post_expire(key);
    }
    fn sorted_set_element(&mut self, key: &[u8], score: f64, member: &[u8]) {
        let score = score.to_string();
        self.emit(vec!["ZADD".as_bytes(), key, score.as_bytes(), member]);
    }
}

impl Formatter for Protocol {
    fn string(&mut self, key: &Vec<u8>, value: &Vec<u8>, _expiry: &Option<u64>) {
        self.set(key, value, _expiry);
    }

    fn hash(&mut self, key: &Vec<u8>, values: &IndexMap<Vec<u8>, Vec<u8>>, expiry: &Option<u64>) {
        self.start_hash(expiry);
        for (field, value) in values {
            self.hash_element(key, field, value);
        }
        self.end_hash(key);
    }

    fn set(&mut self, key: &Vec<u8>, values: &Vec<Vec<u8>>, expiry: &Option<u64>) {
        self.start_set(expiry);
        for value in values {
            self.set_element(key, value);
        }
        self.end_set(key);
    }

    fn list(&mut self, key: &Vec<u8>, values: &Vec<Vec<u8>>, expiry: &Option<u64>) {
        self.start_list(expiry);
        for value in values {
            self.list_element(key, value);
        }
        self.end_list(key);
    }

    fn sorted_set(&mut self, key: &Vec<u8>, values: &Vec<(f64, Vec<u8>)>, expiry: &Option<u64>) {
        self.start_sorted_set(expiry);
        for (score, member) in values {
            self.sorted_set_element(key, *score, member);
        }
        self.end_sorted_set(key);
    }

    fn start_database(&mut self, db_number: u32) {
        let db = db_number.to_string();
        self.emit(vec!["SELECT".as_bytes(), db.as_bytes()])
    }
}
