extern crate lzf;
extern crate rustc_serialize as serialize;
extern crate regex;
extern crate byteorder;

#[macro_use]
extern crate nom;

macro_rules! errprint {
    ($($arg:tt)*) => (
        std::io::stderr().write_fmt(format_args!($($arg)*))
        );
}

macro_rules! errln {
    ($fmt:expr) => (errprint!(concat!($fmt, "\n")));
    ($fmt:expr, $($arg:tt)*) => (errprint!(concat!($fmt, "\n"), $($arg)*));
}

#[allow(unused_imports)] use std::io::Write;


use nom::IResult::*;
use nom::Err::*;
use nom::{
    IResult,
    be_u8,
    be_u32,
};

#[macro_use]
mod helper;

mod constants;
use constants::{
    constant,
    encoding,
};
#[derive(Debug,PartialEq)]
pub struct RdbVersion(u32);

named!(pub rdb_magic<&[u8],&[u8]>, tag!("REDIS"));

named!(pub rdb_version < &[u8],RdbVersion >,
       alt!(
           tag!("0001") => { |_| RdbVersion(1) } |
           tag!("0002") => { |_| RdbVersion(2) } |
           tag!("0003") => { |_| RdbVersion(3) } |
           tag!("0004") => { |_| RdbVersion(4) } |
           tag!("0005") => { |_| RdbVersion(5) } |
           tag!("0006") => { |_| RdbVersion(6) } |
           tag!("0007") => { |_| RdbVersion(7) }
           )
      );

named!(pub parse_length_with_encoding < &[u8], (u32, bool) >,
       alt!(
           chain!(
               bitpeek!(constant::RDB_6BITLEN, 2) ~
               len: be_u8 ,
               || ((len as u32) & 0x3F, false)
               ) |

           chain!(
               bitpeek!(constant::RDB_14BITLEN, 2) ~
               len: be_u8 ~
               len2: be_u8,
               || {
                   (((len as u32) & 0x3F) << 8 | len2 as u32, false)
               }
               ) |

           chain!(
               bitpeek!(constant::RDB_32BITLEN, 2) ~
               be_u8 ~
               len: be_u32,
               || (len, false)
               ) |

           chain!(
               bitpeek!(constant::RDB_ENCVAL, 2) ~
               len: be_u8 ,
               || ((len as u32) & 0x3F, true)
               )
         )
      );

named!(pub parse_length < &[u8], u32 >, map!(parse_length_with_encoding, |(l,_)| l));

named!(parse_i8  < &[u8], Vec<u8> >, map!(helper::le_i8,  |i| helper::int_to_vec(i as i32)));
named!(parse_i16 < &[u8], Vec<u8> >, map!(helper::le_i16, |i| helper::int_to_vec(i as i32)));
named!(parse_i32 < &[u8], Vec<u8> >, map!(helper::le_i32, |i| helper::int_to_vec(i as i32)));

fn read_exact(i: &[u8], n: usize) -> IResult<&[u8],&[u8]> {
    take!(i, n)
}

named!(pub read_lzf_string < &[u8], Vec<u8> >,
       chain!(
           compressed_length: parse_length ~
           real_length: parse_length ~
           data: apply!(read_exact, compressed_length as usize) ,
           ||{
               lzf::decompress(data, real_length as usize).unwrap()
           })
      );

fn parse_value(i: &[u8], l_e: (u32, bool)) -> IResult<&[u8], Vec<u8>> {
    match l_e {
        (encoding::INT8, true) => parse_i8(i),
        (encoding::INT16, true) => parse_i16(i),
        (encoding::INT32, true) => parse_i32(i),
        (encoding::LZF, true) => read_lzf_string(i),
        (length, false) => map!(i, take!(length as usize), From::from),
        (l, e) => { panic!("Unknown encoding: {} (encoded: {})", l, e) }
    }
}

named!(pub parse_blob < &[u8], Vec<u8> >,
    chain!(
        length_encoded: parse_length_with_encoding ~
        value: apply!(parse_value, length_encoded),
        ||{
            value
        })
);


#[cfg(test)]
mod tests {
    extern crate lzf;

    #[allow(unused_imports)] use std;
    #[allow(unused_imports)] use std::io::Write;

    use nom::IResult::*;
    use nom::Err::*;
    use nom::Needed;

    use super::*;

    #[test]
    fn test_parse_blob_easy() {
        assert_eq!(Done(&[][..], vec![102,111,111]), parse_blob(&[3, 102, 111, 111]));
    }

    #[test]
    fn test_parse_blob_large() {
        let mut bytes  = vec![0x40, 0x40];
        let data = vec![97, 97, 97, 97, 97, 97, 97, 97, 97, 97, 97, 97, 97, 97, 97, 97, 97, 97, 97, 97, 97, 97, 97, 97, 97, 97, 97, 97, 97, 97, 97, 97, 97, 97, 97, 97, 97, 97, 97, 97, 97, 97, 97, 97, 97, 97, 97, 97, 97, 97, 97, 97, 97, 97, 97, 97, 97, 97, 97, 97, 97, 97, 97, 97];
        bytes.extend(data.iter());

        assert_eq!(Done(&[][..], data), parse_blob(&bytes));
    }

    #[test]
    fn test_parse_blob_missing_bytes() {
        let mut bytes  = vec![0x40, 0x40];
        let data = vec![97, 97, 97];
        bytes.extend(data.iter());

        assert_eq!(Incomplete(Needed::Size(64)), parse_blob(&bytes));
    }

    #[test]
    fn test_parse_blob_lzf() {
        // LZF example
        let lorem = "Lorem ipsum dolor sit amet, consetetur sadipscing elitr, sed diam nonumy eirmod \
                 tempor invidunt ut labore et dolore magna aliquyam erat, sed diam voluptua. At \
                 vero eos et accusam et justo duo dolores et ea rebum. Stet clita kasd gubergren, \
                 no sea takimata sanctus est Lorem ipsum dolor sit amet. Lorem ipsum dolor sit \
                 amet, consetetur sadipscing elitr, sed diam nonumy eirmod tempor invidunt ut \
                 labore et dolore magna aliquyam erat, sed diam voluptua.";
        let compressed = lzf::compress(lorem.as_bytes()).unwrap();


        let compressed_length = [0x41, 0x10]; // 272
        let real_length = [0x41, 0xc3]; // 451

        let mut bytes = vec![0xC3];
        bytes.extend(compressed_length.iter());
        bytes.extend(real_length.iter());
        bytes.extend(compressed.iter());

        match parse_blob(&bytes) {
            Done(_, res) => {
                assert_eq!(lorem.as_bytes(), &res[..]);
            },
            _ => assert!(false)
        };
    }

    #[test]
    fn test_parse_blob_integers() {
        // INT8
        let bytes = [0xC0, 42];
        assert_eq!(Done(&[][..], "42".to_owned().into_bytes()), parse_blob(&bytes));

        // INT16
        let bytes = [0xC1, 0x8C, 0x3C];
        assert_eq!(Done(&[][..], "15500".to_owned().into_bytes()),
                   parse_blob(&bytes));
        let bytes = [0xC1, 0xDC, 0xFF];
        assert_eq!(Done(&[][..], "-36".to_owned().into_bytes()),
                   parse_blob(&bytes));

        // INT32
        let bytes = [0xC2, 0xFF, 0xFF, 0xFF, 0x7F];
        assert_eq!(Done(&[][..], "2147483647".to_owned().into_bytes()),
                   parse_blob(&bytes));
        let bytes = [0xC2, 0x0, 0x0, 0x0, 0x80];
        assert_eq!(Done(&[][..], "-2147483648".to_owned().into_bytes()),
                   parse_blob(&bytes));
    }

    #[test]
    fn test_lzf_parser() {
        let lorem = "Lorem ipsum dolor sit amet, consetetur sadipscing elitr, sed diam nonumy eirmod \
                 tempor invidunt ut labore et dolore magna aliquyam erat, sed diam voluptua. At \
                 vero eos et accusam et justo duo dolores et ea rebum. Stet clita kasd gubergren, \
                 no sea takimata sanctus est Lorem ipsum dolor sit amet. Lorem ipsum dolor sit \
                 amet, consetetur sadipscing elitr, sed diam nonumy eirmod tempor invidunt ut \
                 labore et dolore magna aliquyam erat, sed diam voluptua.";
        let compressed = lzf::compress(lorem.as_bytes()).unwrap();

        let compressed_length = [0x41, 0x10]; // 272
        let real_length = [0x41, 0xc3]; // 451

        let mut bytes = vec![];
        bytes.extend(compressed_length.iter());
        bytes.extend(real_length.iter());
        bytes.extend(compressed.iter());


        match read_lzf_string(&bytes) {
            Done(_, res) => {
                assert_eq!(lorem.as_bytes(), &res[..]);
            },
            _ => assert!(false)
        };
    }

    #[test]
    fn test_verify_correct_version() {
        let correct_version = b"0003";

        assert_eq!(
            Done(&[][..], RdbVersion(3)),
            rdb_version(correct_version));
    }

    #[test]
    fn test_verify_broken_version() {
        let broken_version  = b"0010";

        assert_eq!(
            Error(Position(0, &broken_version[..])),
            rdb_version(broken_version));
    }

    #[test]
    fn test_verify_correct_magic() {
        let correct_magic = b"REDIS";

        assert_eq!(
            Done(&[][..], &correct_magic[..]),
            rdb_magic(correct_magic));
    }

    #[test]
    fn test_verify_broken_magic() {
        let broken_magic  = b"FOOBA";

        assert_eq!(
            Error(Position(0, &broken_magic[..])),
            rdb_magic(broken_magic));
    }

    #[test]
    fn test_verify_length_encoding() {
        assert_eq!(
            Done(&[][..], (50,true)),
            parse_length_with_encoding(&[0xF2])
            );

        assert_eq!(
            Done(&[][..], (50,false)),
            parse_length_with_encoding(&[50])
            );

        assert_eq!(
            Done(&[][..], (15000, false)),
            parse_length_with_encoding(&[0x7A, 0x98])
            );

        assert_eq!(
            Done(&[][..], (4294967295, false)),
            parse_length_with_encoding(&[0xBF, 0xFF, 0xFF, 0xFF, 0xFF])
            );

        assert_eq!(
            Done(&[][..], 4294967295),
            parse_length(&[0xBF, 0xFF, 0xFF, 0xFF, 0xFF])
            );
    }
}
