const NULL_BYTE: u8 = b'\0';
const BACKSLASH: u8 = b'\\';
const SLASH: char = '/';

pub(crate) fn clean_path(input: &[u8]) -> String {
  let mut output = String::new();

  for c in input {
    if c == &NULL_BYTE {
      return output;
    }

    if c == &BACKSLASH {
      output.push(SLASH);
    } else {
      output.push(*c as char)
    }
  }

  output
}
