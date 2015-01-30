use formatter::Formatter;
use std::old_io;

pub struct Plain {
    out: Box<Writer+'static>
}

impl Plain {
    pub fn new() -> Plain {
        let out = Box::new(old_io::stdout());
        Plain { out: out }
    }
}

impl Formatter for Plain {
    fn start_rdb(&mut self) {
        println!("Start of RDB");
    }

    fn end_rdb(&mut self) {
        println!("End of RDB");
    }

    fn checksum(&mut self, checksum: &[u8]) {
        let _ = self.out.write_str("Checksum: ");
        let _ = self.out.write(checksum.as_slice());
        let _ = self.out.write_str("\n");
    }

    fn start_database(&mut self, db_number: u32) {
        println!("SELECTDB: {}", db_number);
    }

    fn end_database(&mut self, db_number: u32) {
        println!("END_DB: {}", db_number);
    }

    fn set(&mut self, key: &[u8], value: &[u8], _expiry: Option<u64>) {
        let _ = self.out.write(key.as_slice());
        let _ = self.out.write_str(": ");

        let _ = self.out.write(value.as_slice());
        let _ = self.out.write_str("\n");
        let _ = self.out.flush();
    }

    fn aux_field(&mut self, key: &[u8], value: &[u8]) {
        let _ = self.out.write_str("[aux] ");
        let _ = self.out.write(key.as_slice());
        let _ = self.out.write_str(": ");
        let _ = self.out.write(value.as_slice());
        let _ = self.out.write_str("\n");
        let _ = self.out.flush();
    }
}
