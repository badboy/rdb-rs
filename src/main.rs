extern crate rdb;
extern crate getopts;
extern crate regex;
use std::env;
use std::io::{BufReader,Write};
use std::fs::File;
use std::path::Path;
use getopts::Options;
use regex::Regex;

fn print_usage(program: &str, opts: Options) {
    let brief = format!("Usage: {} [options] dump.rdb", program);
    print!("{}", opts.usage(&brief));

}

pub fn main() {
    let mut args = env::args();
    let program = args.next().unwrap();
    let mut opts = Options::new();

    opts.optopt("f", "format", "Format to output. Valid: json, hex-keys, plain, nil, protocol", "FORMAT");
    opts.optopt("k", "keys", "Keys to show. Can be a regular expression", "KEYS");
    opts.optmulti("d", "databases", "Database to show. Can be specified multiple times", "DB");
    opts.optmulti("t", "type", "Type to show. Can be specified multiple times", "TYPE");
    opts.optflag("h", "help", "print this help menu");


    let matches = match opts.parse(args) {
        Ok(m) => { m  }
        Err(e) => {
            println!("{}\n", e);
            print_usage(&program, opts);
            return;
        }
    };

    if matches.opt_present("h") {
         print_usage(&program, opts);
         return;
    }

    let mut filter = rdb::filter::Simple::new();

    for db in matches.opt_strs("d").iter() {
        filter.add_database(db.parse().unwrap());
    }

    for t in matches.opt_strs("t").iter() {
        let typ = match &t[..] {
            "string" => rdb::Type::String,
            "list" => rdb::Type::List,
            "set" => rdb::Type::Set,
            "sortedset" | "sorted-set" | "sorted_set" => rdb::Type::SortedSet,
            "hash" => rdb::Type::Hash,
            _ => {
                println!("Unknown type: {}\n", t);
                print_usage(&program, opts);
                return;
            }
        };
        filter.add_type(typ);
    }

    if let Some(k) = matches.opt_str("k") {
        let re = match Regex::new(&k) {
            Ok(re) => re,
            Err(err) => {
                println!("Incorrect regexp: {:?}\n", err);
                print_usage(&program, opts);
                return;
            }
        };
        filter.add_keys(re);
    }

    if matches.free.is_empty() {
        print_usage(&program, opts);
        return;
    }

    let path = matches.free[0].clone();
    let file = File::open(&Path::new(&*path)).unwrap();
    let reader = BufReader::new(file);

    let mut res = Ok(());

    if let Some(f) = matches.opt_str("f") {
        match &f[..] {
            "json" => {
                res = rdb::parse(reader, rdb::formatter::JSON::new(), filter);
            },
            "hex-keys" => {
                res = rdb::parse(reader, rdb::formatter::HexKeys::new(), filter);
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
                print_usage(&program, opts);
            }
        }
    } else {
        res = rdb::parse(reader, rdb::formatter::JSON::new(), filter);
    }

    match res {
        Ok(()) => {},
        Err(e) => {
            println!("");
            let mut stderr = std::io::stderr();

            let out = format!("Parsing failed: {}\n", e);
            stderr.write(out.as_bytes()).unwrap();
        }
    }
}
