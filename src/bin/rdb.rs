extern crate rdb;
use std::io::BufReader;
use std::fs::File;
use std::path::Path;

fn main() {
    let path = "dump.rdb";
    let file = File::open(&Path::new(&*path)).unwrap();
    let reader = BufReader::new(file);
    let filter = rdb::filter::Simple::new();

    let mut parser = rdb::parse(reader, filter);

    loop {
        match parser.next() {
            Ok(rdb::RdbIteratorType::EOF) => break,
            Ok(val) => println!("{:?}", val),
            Err(err) => {
                println!("ERR! -> {:?}", err);
                break;
            }
        }
    }
}
