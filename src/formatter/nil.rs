use crate::formatter::Formatter;
use crate::types::RdbValue;
use std::io;
use std::io::Write;
use std::path::PathBuf;

pub struct Nil {
    _out: Box<dyn Write>,
}

impl Nil {
    pub fn new(file_path: Option<PathBuf>) -> Nil {
        let _out: Box<dyn Write> = match file_path {
            Some(path) => match std::fs::File::create(path) {
                Ok(file) => Box::new(file),
                Err(_) => Box::new(io::stdout()),
            },
            None => Box::new(io::stdout()),
        };
        Nil { _out }
    }
}

impl Formatter for Nil {
    fn format(&mut self, _value: &RdbValue) -> std::io::Result<()> {
        Ok(())
    }
}
