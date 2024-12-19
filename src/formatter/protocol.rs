#![allow(unused_must_use)]
use super::write_str;
use crate::formatter::Formatter;
use crate::types::EncodingType;
use std::io;
use std::io::Write;

pub struct Protocol {
    out: Box<dyn Write + 'static>,
    last_expiry: Option<u64>,
}

impl Protocol {
    pub fn new(file_path: Option<&str>) -> Protocol {
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

    fn pre_expire(&mut self, expiry: Option<u64>) {
        self.last_expiry = expiry
    }

    fn post_expire(&mut self, key: &[u8]) {
        if let Some(expire) = self.last_expiry {
            let expire = expire.to_string();
            self.emit(vec!["PEXPIREAT".as_bytes(), key, expire.as_bytes()]);
            self.last_expiry = None;
        }
    }
}

impl Formatter for Protocol {
    fn start_rdb(&mut self) {}

    fn end_rdb(&mut self) {}

    fn start_database(&mut self, db_number: u32) {
        let db = db_number.to_string();
        self.emit(vec!["SELECT".as_bytes(), db.as_bytes()])
    }

    fn set(&mut self, key: &[u8], value: &[u8], expiry: Option<u64>) {
        self.pre_expire(expiry);
        self.emit(vec!["SET".as_bytes(), key, value]);
        self.post_expire(key);
    }

    fn start_hash(&mut self, _key: &[u8], _length: u32, expiry: Option<u64>, _info: EncodingType) {
        self.pre_expire(expiry);
    }
    fn end_hash(&mut self, key: &[u8]) {
        self.post_expire(key);
    }
    fn hash_element(&mut self, key: &[u8], field: &[u8], value: &[u8]) {
        self.emit(vec!["HSET".as_bytes(), key, field, value]);
    }

    fn start_set(
        &mut self,
        _key: &[u8],
        _cardinality: u32,
        expiry: Option<u64>,
        _info: EncodingType,
    ) {
        self.pre_expire(expiry);
    }
    fn end_set(&mut self, key: &[u8]) {
        self.post_expire(key);
    }
    fn set_element(&mut self, key: &[u8], member: &[u8]) {
        self.emit(vec!["SADD".as_bytes(), key, member]);
    }

    fn start_list(&mut self, _key: &[u8], _length: u32, expiry: Option<u64>, _info: EncodingType) {
        self.pre_expire(expiry);
    }
    fn end_list(&mut self, key: &[u8]) {
        self.post_expire(key);
    }
    fn list_element(&mut self, key: &[u8], value: &[u8]) {
        self.emit(vec!["RPUSH".as_bytes(), key, value]);
    }

    fn start_sorted_set(
        &mut self,
        _key: &[u8],
        _length: u32,
        expiry: Option<u64>,
        _info: EncodingType,
    ) {
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
