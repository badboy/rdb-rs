pub fn int_to_vec(number: i32) -> Vec<u8> {
  let number = number.to_string();
  let mut result = Vec::with_capacity(number.len());
  for &c in number.as_bytes().iter() {
    result.push(c);
  }
  result
}
