extern crate rdb;
extern crate regex;

#[macro_use]
extern crate clap;
 
use std::io::{BufReader,Write};
use std::fs::File;
use std::path::Path;
 
use regex::Regex;
use clap::App;

use rdb::Type;
use rdb::formatter::{Plain, JSON, Nil, Protocol};
 
arg_enum!{
    pub enum FormatType {
        Json,
        Nil,
        Plain,
        Protocol
    }
}
 
pub fn main() {
    let matches = App::new("rdb")
                 // Use the version in your Cargo.toml
                 .version(&*format!("v{}", crate_version!()))
                 .about("CLI tool for parsing RDB dumps")
                 .author("Jan-Erik Rediger")
                 .args_from_usage(
                    "<dump_file>               'Path to the RDB dump file'
                     -k, --keys [keys]         'Keys to show. Can be a regular expression'
                     -d, --databases [dbs]...  'Databases to show. Can be specified multiple times'
                     -t, --type [type]...      'Type to show. Can be specified multiple times{n}\
                                                [valid values: hash, list, set, sortedset, string]'
                     -f, --format [format]     'Format to output (Defaults to JSON when omittied){n}\
                                                [valid values: json, nil, plain, protocol]'")
                 .get_matches();
    
    let mut filter = rdb::filter::Simple::new();
 
    if matches.is_present("dbs") {
        for db in value_t_or_exit!(matches.values_of("dbs"), u32) {
            filter.add_database(db);
        }
    }
 
    if matches.is_present("type") {
        for t in value_t_or_exit!(matches.values_of("type"), Type) {
            filter.add_type(t);
        }
    }
 
    if matches.is_present("keys") {
        let re = value_t_or_exit!(matches.value_of("keys"), Regex);
        filter.add_keys(re);
    }
 
    let path = matches.value_of("dump_file").unwrap();
    let file = File::open(&Path::new(path)).unwrap();
    let reader = BufReader::new(file);
 
    let res = match value_t!(matches.value_of("format"), FormatType).unwrap_or(FormatType::Json) {
        FormatType::Json     => rdb::parse(reader, JSON::new(), filter),
        FormatType::Plain    => rdb::parse(reader, Plain::new(), filter),
        FormatType::Nil      => rdb::parse(reader, Nil::new(), filter),
        FormatType::Protocol => rdb::parse(reader, Protocol::new(), filter)
    };
    
    if let Err(e) = res {
        println!("");
        let mut stderr = std::io::stderr();
        let out = format!("Parsing failed: {}\n", e);
        stderr.write(out.as_bytes()).unwrap();
    }
}