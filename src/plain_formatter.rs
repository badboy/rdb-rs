use formatter::RdbParseFormatter;
use std::io;

pub struct PlainFormatter {
    out: Box<Writer+'static>
}

impl PlainFormatter {
    pub fn new() -> PlainFormatter {
        let out = box io::stdout() as Box<Writer>;
        PlainFormatter { out: out }
    }
}

impl RdbParseFormatter for PlainFormatter {
    fn start_rdb(&mut self) {
        println!("Start of RDB");
    }

    fn end_rdb(&mut self) {
        println!("End of RDB");
    }

    fn checksum(&mut self, checksum: Vec<u8>) {
        println!("Checksum: {}", checksum);
    }

    fn start_database(&mut self, db_number: u32) {
        println!("SELECTDB: {}", db_number);
    }

    fn end_database(&mut self, db_number: u32) {
        println!("END_DB: {}", db_number);
    }

    fn set(&mut self, key: Vec<u8>, value: Vec<u8>) {
        let _ = self.out.write(key[]);
        let _ = self.out.write_str(": ");

        let _ = self.out.write(value[]);
        let _ = self.out.write_str("\n");
        let _ = self.out.flush();
    }

    fn aux_field(&mut self, key: Vec<u8>, value: Vec<u8>) {
        let _ = self.out.write_str("[aux] ");
        let _ = self.out.write(key[]);
        let _ = self.out.write_str(": ");
        let _ = self.out.write(value[]);
        let _ = self.out.write_str("\n");
        let _ = self.out.flush();
    }
}
