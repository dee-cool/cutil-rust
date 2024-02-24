pub fn is_empty_string(value: Option<String>) -> Option<String> {
  if let Some(v) = value {
    if !v.is_empty() {
      return Some(v);
    }
  }
  None
}

pub fn is_empty_string_ref(value: &Option<String>) -> Option<&String> {
  if let Some(v) = value {
    if !v.is_empty() {
      return Some(v);
    }
  }
  None
}

/// 脱密，仅保留前 1/3 和后 1/3，剩余使用 `*` 替代
pub fn mask_sensitive(text: &str) -> String {
  let length = text.chars().count();
  if length == 0 {
    return String::new();
  }
  if length <= 3 {
    return text.to_string();
  }

  let front_len = length / 3;
  let back_len = length / 3;
  let middle_len = length - front_len - back_len;

  // 预留容量：按字节长度预估，避免多次分配
  let mut result = String::with_capacity(text.len());

  for (i, ch) in text.chars().enumerate() {
    if i < front_len || i >= length - back_len {
      result.push(ch);
    } else if i == front_len {
      result.extend(std::iter::repeat('*').take(middle_len));
    }
  }

  result
}

#[cfg(test)]
mod tests {
  use super::mask_sensitive;

  #[test]
  fn test_empty_string() {
    assert_eq!(mask_sensitive(""), "");
  }

  #[test]
  fn test_short_strings() {
    assert_eq!(mask_sensitive("a"), "a");
    assert_eq!(mask_sensitive("ab"), "ab");
    assert_eq!(mask_sensitive("abc"), "abc");
  }

  #[test]
  fn test_regular_ascii() {
    // length = 9, front=3, back=3, middle=3
    assert_eq!(mask_sensitive("abcdefghi"), "abc***ghi");
    // length = 4, front=1, back=1, middle=2
    assert_eq!(mask_sensitive("abcd"), "a**d");
    assert_eq!(mask_sensitive("abcde"), "a***e");
  }

  #[test]
  fn test_unicode_chars() {
    // 5个字符：前1 后1 中间3
    assert_eq!(mask_sensitive("你好世界啊"), "你***啊");
  }
}
