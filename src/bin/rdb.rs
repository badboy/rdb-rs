extern crate rdb;
use std::io::BufReader;
use std::fs::File;
use std::path::Path;

fn main() {
    let path = "dump.rdb";
    let file = File::open(&Path::new(&*path)).unwrap();
    let reader = BufReader::new(file);
    let filter = rdb::filter::Simple::new();

    let parser = rdb::parse(reader, filter);

    for val in parser {
        match val {
            Ok(rdb::RdbIteratorType::EOF) => break,
            Ok(val) => println!("{:?}", val),
            Err(err) => {
                println!("ERR! -> {:?}", err);
                break;
            }
        }
    }
}
