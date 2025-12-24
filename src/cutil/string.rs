pub fn is_empty_string(value: Option<String>) -> Option<String> {
  value.filter(|v| !v.is_empty())
}

pub fn is_empty_string_ref(value: &Option<String>) -> Option<&String> {
  value.as_ref().filter(|v| !v.is_empty())
}
