/// 脱密，仅保留前 1/3 和后 1/3，剩余使用 `*` 替代
pub fn mask_string(text: String) -> String {
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
      result.extend(std::iter::repeat_n('*', middle_len));
    }
  }

  result
}

pub fn mask_str(text: &str) -> String {
  mask_string(text.to_string())
}

/// 递归处理，将 JSON 值中的字符串字段进行脱敏处理
pub fn mask_value(value: serde_json::Value) -> serde_json::Value {
  match value {
    serde_json::Value::String(s) => serde_json::Value::String(mask_string(s)),
    serde_json::Value::Object(map) => {
      let mut masked_map = serde_json::Map::new();
      for (key, val) in map {
        masked_map.insert(key, mask_value(val));
      }
      serde_json::Value::Object(masked_map)
    }
    serde_json::Value::Array(arr) => {
      let masked_arr: Vec<serde_json::Value> = arr.into_iter().map(mask_value).collect();
      serde_json::Value::Array(masked_arr)
    }
    // 对于其他类型（Number, Bool, Null），直接返回原值
    _ => value,
  }
}

#[cfg(test)]
mod tests {
  use super::{mask_str, mask_value};

  #[test]
  fn test_empty_string() {
    assert_eq!(mask_str(""), "");
  }

  #[test]
  fn test_short_strings() {
    assert_eq!(mask_str("a"), "a");
    assert_eq!(mask_str("ab"), "ab");
    assert_eq!(mask_str("abc"), "abc");
  }

  #[test]
  fn test_regular_ascii() {
    // length = 9, front=3, back=3, middle=3
    assert_eq!(mask_str("abcdefghi"), "abc***ghi");
    // length = 4, front=1, back=1, middle=2
    assert_eq!(mask_str("abcd"), "a**d");
    assert_eq!(mask_str("abcde"), "a***e");
  }

  #[test]
  fn test_unicode_chars() {
    // 5个字符：前1 后1 中间3
    assert_eq!(mask_str("你好世界啊"), "你***啊");
  }

  #[test]
  fn test_mask_value_string() {
    let value = serde_json::Value::String("abcdefghi".to_string());
    let masked = mask_value(value);
    assert_eq!(masked, serde_json::Value::String("abc***ghi".to_string()));
  }

  #[test]
  fn test_mask_value_object() {
    let json_str = r#"{"name": "abcdefghi", "age": 30, "active": true}"#;
    let value: serde_json::Value = serde_json::from_str(json_str).unwrap();
    let masked = mask_value(value);

    let expected_str = r#"{"name": "abc***ghi", "age": 30, "active": true}"#;
    let expected: serde_json::Value = serde_json::from_str(expected_str).unwrap();
    assert_eq!(masked, expected);
  }

  #[test]
  fn test_mask_value_array() {
    let json_str = r#"["abcdefghi", "hello", 123, true]"#;
    let value: serde_json::Value = serde_json::from_str(json_str).unwrap();
    let masked = mask_value(value);

    let expected_str = r#"["abc***ghi", "h***o", 123, true]"#;
    let expected: serde_json::Value = serde_json::from_str(expected_str).unwrap();
    assert_eq!(masked, expected);
  }

  #[test]
  fn test_mask_value_nested() {
    let json_str = r#"{"user": {"name": "abcdefghi", "email": "test@example.com"}, "items": ["password123", "secret456"]}"#;
    let value: serde_json::Value = serde_json::from_str(json_str).unwrap();
    let masked = mask_value(value);

    let expected_str = r#"{"user": {"name": "abc***ghi", "email": "test@******e.com"}, "items": ["pas*****123", "sec***456"]}"#;
    let expected: serde_json::Value = serde_json::from_str(expected_str).unwrap();
    assert_eq!(masked, expected);
  }

  #[test]
  fn test_mask_value_non_string() {
    let json_str = r#"{"number": 42, "boolean": false, "null_value": null}"#;
    let value: serde_json::Value = serde_json::from_str(json_str).unwrap();
    let masked = mask_value(value.clone());

    // 非字符串值应该保持不变
    assert_eq!(masked, value);
  }
}
