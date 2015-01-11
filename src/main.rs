#![allow(unstable)]
#![feature(box_syntax)]
extern crate rdb;
extern crate getopts;
use std::os;
use std::io::{BufferedReader, File};
use getopts::{optopt,optflag,getopts,OptGroup,usage};

fn print_usage(program: &str, opts: &[OptGroup]) {
    let brief = format!("Usage: {} [options] dump.rdb", program);
    print!("{}", usage(brief.as_slice(), opts));

}

pub fn main() {
    let args = os::args();
    let program = args[0].clone();

    let opts = &[
        optopt("f", "format", "Format to output. Valid: json, plain, nil, protocol", "FORMAT"),
        optflag("h", "help", "print this help menu")
    ];

    let matches = match getopts(args.tail(), opts) {
        Ok(m) => { m  }
        Err(f) => { panic!(f.to_string())  }
    };

    //let mut format : Box<rdb::RdbParseFormatter>;

    if matches.opt_present("h") {
         print_usage(program.as_slice(), opts);
         return;
    }


    if let Some(f) = matches.opt_str("f") {
        if matches.free.is_empty() {
            print_usage(program.as_slice(), opts);
            return;
        }
        let path = matches.free[0].clone();
        let file = File::open(&Path::new(path));
        let reader = BufferedReader::new(file);

        match f.as_slice() {
            "json" => {
                rdb::parse(reader, rdb::JSONFormatter::new())
            },
            "plain" => {
                rdb::parse(reader, rdb::PlainFormatter::new())
            },
            "nil" => {
                rdb::parse(reader, rdb::NilFormatter::new())
            }
            "protocol" => {
                rdb::parse(reader, rdb::ProtocolFormatter::new())
            }
            _ => {
                println!("Unknown format: {}", f);
                println!("");
                print_usage(program.as_slice(), opts);
            }
        }

        return
    } else {
        if matches.free.is_empty() {
            print_usage(program.as_slice(), opts);
            return;
        }
        let path = matches.free[0].clone();
        let file = File::open(&Path::new(path));
        let reader = BufferedReader::new(file);
        rdb::parse(reader, rdb::JSONFormatter::new())
    }
}
