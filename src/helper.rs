use nom::{
    IResult,

    le_u8,
    le_u16,
    le_u32,
};

pub fn int_to_vec(number: i32) -> Vec<u8> {
  number.to_string().into_bytes()
}

#[macro_export]
macro_rules! bitpeek (
  ($i:expr, $mask: expr, $bits: expr) => (
    {
      #[inline(always)]
      fn upperbit_mask(n: u8) -> u8 {
              !((1<<(8-n))-1)
      }

      if $i.len() < 1{
        nom::IResult::Incomplete(nom::Needed::Size(1))
      } else if ($i[0] & upperbit_mask($bits))>>(8-$bits) == $mask {
        nom::IResult::Done(&$i, ())
      } else {
        nom::IResult::Error(nom::Err::Position(nom::ErrorCode::Tag as u32, $i))
      }
    }
  );
);


pub fn le_i8<'a>(i:&'a [u8]) -> IResult<&'a [u8], i8> {
  map!(i, le_u8, | x | { x as i8 })
}

pub fn le_i16<'a>(i:&'a [u8]) -> IResult<&'a [u8], i16> {
  map!(i, le_u16, | x | { x as i16 })
}

pub fn le_i32<'a>(i:&'a [u8]) -> IResult<&'a [u8], i32> {
  map!(i, le_u32, | x | { x as i32 })
}
