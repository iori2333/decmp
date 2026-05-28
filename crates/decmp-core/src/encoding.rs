use encoding_rs::Encoding;

use crate::error::{DecmpError, Result};

const COMMON_ENCODINGS: &[&str] = &["gbk", "gb18030", "big5", "shift_jis", "euc-jp", "euc-kr"];

pub fn decode_filename(raw: &[u8], encoding_name: &str) -> Result<String> {
  let encoding = Encoding::for_label(encoding_name.as_bytes())
    .ok_or_else(|| DecmpError::EncodingError(format!("unknown encoding: {encoding_name}")))?;

  let (decoded, _encoding_used, had_errors) = encoding.decode(raw);
  if had_errors {
    return Err(DecmpError::EncodingError(format!(
      "failed to decode with {encoding_name}"
    )));
  }

  Ok(decoded.into_owned())
}

pub fn encode_filename(name: &str, encoding_name: &str) -> Result<Vec<u8>> {
  let encoding = Encoding::for_label(encoding_name.as_bytes())
    .ok_or_else(|| DecmpError::EncodingError(format!("unknown encoding: {encoding_name}")))?;

  let (encoded, _encoding_used, had_errors) = encoding.encode(name);
  if had_errors {
    return Err(DecmpError::EncodingError(format!(
      "failed to encode with {encoding_name}"
    )));
  }

  Ok(encoded.into_owned())
}

pub fn try_decode_utf8(raw: &[u8]) -> String {
  String::from_utf8_lossy(raw).into_owned()
}

pub fn is_utf8(raw: &[u8]) -> bool {
  std::str::from_utf8(raw).is_ok()
}

pub fn auto_detect_encoding(raw_names: &[&[u8]]) -> Option<&'static str> {
  if raw_names.is_empty() {
    return None;
  }

  if raw_names.iter().all(|raw| is_utf8(raw)) {
    return None;
  }

  for &encoding_name in COMMON_ENCODINGS {
    if let Some(encoding) = Encoding::for_label(encoding_name.as_bytes()) {
      let ok_ratio = raw_names
        .iter()
        .filter(|raw| {
          let (_, _, had_errors) = encoding.decode(raw);
          !had_errors
        })
        .count() as f64
        / raw_names.len() as f64;

      if ok_ratio > 0.5 {
        return Some(encoding_name);
      }
    }
  }

  None
}

pub fn common_encoding_names() -> &'static [&'static str] {
  COMMON_ENCODINGS
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_decode_filename_utf8() {
    let raw = "hello.txt".as_bytes();
    assert_eq!(decode_filename(raw, "utf-8").unwrap(), "hello.txt");
  }

  #[test]
  fn test_decode_filename_gbk() {
    let raw = &[0xC4, 0xE3, 0xBA, 0xC3]; // "你好" in GBK
    let result = decode_filename(raw, "gbk").unwrap();
    assert_eq!(result, "你好");
  }

  #[test]
  fn test_decode_filename_shift_jis() {
    let raw = &[0x82, 0xB1, 0x82, 0xF1]; // "こん" in Shift_JIS
    let result = decode_filename(raw, "shift_jis").unwrap();
    assert_eq!(result, "こん");
  }

  #[test]
  fn test_decode_filename_unknown_encoding() {
    let result = decode_filename(b"test", "unknown-encoding");
    assert!(result.is_err());
  }

  #[test]
  fn test_encode_filename_utf8() {
    let encoded = encode_filename("hello.txt", "utf-8").unwrap();
    assert_eq!(encoded, b"hello.txt");
  }

  #[test]
  fn test_encode_filename_gbk() {
    let encoded = encode_filename("你好", "gbk").unwrap();
    assert_eq!(encoded, &[0xC4, 0xE3, 0xBA, 0xC3]);
  }

  #[test]
  fn test_encode_filename_unknown_encoding() {
    let result = encode_filename("test", "unknown-encoding");
    assert!(result.is_err());
  }

  #[test]
  fn test_encode_decode_roundtrip_gbk() {
    let original = "测试文件.txt";
    let encoded = encode_filename(original, "gbk").unwrap();
    let decoded = decode_filename(&encoded, "gbk").unwrap();
    assert_eq!(decoded, original);
  }

  #[test]
  fn test_encode_decode_roundtrip_shift_jis() {
    let original = "テスト.txt";
    let encoded = encode_filename(original, "shift_jis").unwrap();
    let decoded = decode_filename(&encoded, "shift_jis").unwrap();
    assert_eq!(decoded, original);
  }

  #[test]
  fn test_try_decode_utf8_valid() {
    assert_eq!(try_decode_utf8(b"hello"), "hello");
  }

  #[test]
  fn test_try_decode_utf8_invalid() {
    let raw = &[0xFF, 0xFE];
    let result = try_decode_utf8(raw);
    assert!(result.contains('\u{FFFD}'));
  }

  #[test]
  fn test_is_utf8_valid() {
    assert!(is_utf8(b"hello.txt"));
    assert!(is_utf8("你好.txt".as_bytes()));
  }

  #[test]
  fn test_is_utf8_invalid() {
    assert!(!is_utf8(&[0xFF, 0xFE, 0x00]));
  }

  #[test]
  fn test_decode_empty() {
    assert_eq!(decode_filename(b"", "utf-8").unwrap(), "");
  }

  #[test]
  fn test_encode_empty() {
    assert_eq!(encode_filename("", "utf-8").unwrap(), b"");
  }

  #[test]
  fn test_auto_detect_encoding_empty() {
    assert_eq!(auto_detect_encoding(&[]), None);
  }

  #[test]
  fn test_auto_detect_encoding_all_utf8() {
    let names: &[&[u8]] = &[b"hello.txt", b"readme.md"];
    assert_eq!(auto_detect_encoding(names), None);
  }

  #[test]
  fn test_auto_detect_encoding_gbk() {
    // "你好" in GBK
    let names: &[&[u8]] = &[&[0xC4, 0xE3, 0xBA, 0xC3]];
    let detected = auto_detect_encoding(names);
    assert!(detected.is_some());
  }

  #[test]
  fn test_auto_detect_encoding_shift_jis() {
    // "こん" in Shift_JIS — these bytes are also valid GBK,
    // so either may be detected; we just verify detection succeeds.
    let names: &[&[u8]] = &[&[0x82, 0xB1, 0x82, 0xF1]];
    let detected = auto_detect_encoding(names);
    assert!(detected.is_some());
  }

  #[test]
  fn test_auto_detect_encoding_mixed_utf8_and_gbk() {
    let names: &[&[u8]] = &[
      b"README.txt",
      &[0xC4, 0xE3, 0xBA, 0xC3, 0x2E, 0x74, 0x78, 0x74], // "你好.txt" in GBK
    ];
    // Not all are valid UTF-8, so should detect an encoding
    let detected = auto_detect_encoding(names);
    assert!(detected.is_some());
  }

  #[test]
  fn test_auto_detect_encoding_unrecognizable() {
    // These bytes are invalid in all common CJK encodings
    let names: &[&[u8]] = &[&[0xC0, 0xC1, 0xF5, 0xF6, 0xFE, 0xFF]];
    assert_eq!(auto_detect_encoding(names), None);
  }

  #[test]
  fn test_common_encoding_names() {
    let names = common_encoding_names();
    assert!(names.contains(&"gbk"));
    assert!(names.contains(&"shift_jis"));
  }
}
