#![feature(globs)]

extern crate rdb;
use std::os;
use std::io::{BufferedReader, File};

use rdb::*;

fn main() {
    let args = os::args();
    if args.len() == 1 {
        println!("Usage: {} [list of files]", args[0]);
        panic!();
    }

    let file = File::open(&Path::new(args[1].to_string()));
    let mut reader = BufferedReader::new(file);

    let parser = RdbParser::new(&mut reader);

    parser.parse()
}
