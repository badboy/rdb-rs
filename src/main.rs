#![feature(box_syntax)]
#![feature(core)]
#![feature(io)]
#![feature(collections)]
#![feature(path)]
extern crate rdb;
extern crate getopts;
extern crate regex;
use std::os;
use std::old_io::{BufferedReader, File};
use getopts::Options;
use regex::Regex;

fn print_usage(program: &str, opts: Options) {
    let brief = format!("Usage: {} [options] dump.rdb", program);
    print!("{}", opts.usage(brief.as_slice()));

}

pub fn main() {
    let args = os::args();
    let program = args[0].clone();
    let mut opts = Options::new();

    opts.optopt("f", "format", "Format to output. Valid: json, plain, nil, protocol", "FORMAT");
    opts.optopt("k", "keys", "Keys to show. Can be a regular expression", "KEYS");
    opts.optmulti("d", "databases", "Database to show. Can be specified multiple times", "DB");
    opts.optmulti("t", "type", "Type to show. Can be specified multiple times", "TYPE");
    opts.optflag("h", "help", "print this help menu");


    let matches = match opts.parse(args.tail()) {
        Ok(m) => { m  }
        Err(e) => {
            println!("{}\n", e);
            print_usage(program.as_slice(), opts);
            return;
        }
    };

    if matches.opt_present("h") {
         print_usage(program.as_slice(), opts);
         return;
    }

    let mut filter = rdb::filter::Simple::new();

    for db in matches.opt_strs("d").iter() {
        filter.add_database(db.parse().unwrap());
    }

    for t in matches.opt_strs("t").iter() {
        let typ = match t.as_slice() {
            "string" => rdb::Type::String,
            "list" => rdb::Type::List,
            "set" => rdb::Type::Set,
            "sortedset" | "sorted-set" | "sorted_set" => rdb::Type::SortedSet,
            "hash" => rdb::Type::Hash,
            _ => {
                println!("Unknown type: {}\n", t);
                print_usage(program.as_slice(), opts);
                return;
            }
        };
        filter.add_type(typ);
    }

    if let Some(k) = matches.opt_str("k") {
        let re = match Regex::new(k.as_slice()) {
            Ok(re) => re,
            Err(err) => {
                println!("Incorrect regexp: {:?}\n", err);
                print_usage(program.as_slice(), opts);
                return;
            }
        };
        filter.add_keys(re);
    }

    if matches.free.is_empty() {
        print_usage(program.as_slice(), opts);
        return;
    }

    let path = matches.free[0].clone();
    let file = File::open(&Path::new(path));
    let reader = BufferedReader::new(file);

    let mut res = Ok(());

    if let Some(f) = matches.opt_str("f") {
        match f.as_slice() {
            "json" => {
                res = rdb::parse(reader, rdb::formatter::JSON::new(), filter);
            },
            "plain" => {
                res = rdb::parse(reader, rdb::formatter::Plain::new(), filter);
            },
            "nil" => {
                res = rdb::parse(reader, rdb::formatter::Nil::new(), filter);
            },
            "protocol" => {
                res = rdb::parse(reader, rdb::formatter::Protocol::new(), filter);

            },
            _ => {
                println!("Unknown format: {}\n", f);
                print_usage(program.as_slice(), opts);
            }
        }
    } else {
        res = rdb::parse(reader, rdb::formatter::JSON::new(), filter);
    }

    match res {
        Ok(()) => {},
        Err(e) => {
            println!("");
            let mut stderr = std::old_io::stderr();

            stderr.write_str(format!("Parsing failed: {}\n", e.desc).as_slice()).unwrap();

            if let Some(detail) = e.detail {
                stderr.write_str(format!("Details: {}\n", detail).as_slice()).unwrap();
            }
        }
    }
}
