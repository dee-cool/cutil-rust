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
