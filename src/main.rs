#![allow(unstable)]
#![feature(box_syntax)]
extern crate rdb;
extern crate getopts;
extern crate regex;
use std::os;
use std::io::{BufferedReader, File};
use getopts::{optopt,optflag,getopts,OptGroup,usage};
use regex::Regex;

fn print_usage(program: &str, opts: &[OptGroup]) {
    let brief = format!("Usage: {} [options] dump.rdb", program);
    print!("{}", usage(brief.as_slice(), opts));

}

pub fn main() {
    let args = os::args();
    let program = args[0].clone();

    let opts = &[
        optopt("f", "format", "Format to output. Valid: json, plain, nil, protocol", "FORMAT"),
        optopt("k", "keys", "Keys to show. Can be a regular expression", "KEYS"),
        optopt("d", "databases", "Database to show", "DB"),
        optopt("t", "type", "Type to show", "TYPE"),
        optflag("h", "help", "print this help menu")
    ];

    let matches = match getopts(args.tail(), opts) {
        Ok(m) => { m  }
        Err(f) => { panic!(f.to_string())  }
    };

    if matches.opt_present("h") {
         print_usage(program.as_slice(), opts);
         return;
    }

    let mut filter = rdb::filter::Simple::new();

    if let Some(d) = matches.opt_str("d") {
        filter.add_database(d.parse().unwrap());
    }

    if let Some(t) = matches.opt_str("t") {
        let typ = match t.as_slice() {
            "string" => rdb::Type::String,
            "list" => rdb::Type::List,
            "set" => rdb::Type::Set,
            "sortedset" | "sorted-set" | "sorted_set" => rdb::Type::SortedSet,
            "hash" => rdb::Type::Hash,
            _ => {
                println!("Unknown type: {}", t);
                print_usage(program.as_slice(), opts);
                return;
            }
        };
        filter.add_type(typ);
    }

    if let Some(k) = matches.opt_str("k") {
        let re = match Regex::new(k.as_slice()) {
            Ok(re) => re,
            Err(err) => panic!("{}", err)
        };
        filter.add_keys(re);
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
                let _ = rdb::parse(reader, rdb::formatter::JSON::new(), filter);
            },
            "plain" => {
                let _ = rdb::parse(reader, rdb::formatter::Plain::new(), filter);
            },
            "nil" => {
                let _ = rdb::parse(reader, rdb::formatter::Nil::new(), filter);
            }
            "protocol" => {
                let _ = rdb::parse(reader, rdb::formatter::Protocol::new(), filter);
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
        let _ = rdb::parse(reader, rdb::formatter::JSON::new(), filter);
    }
}
