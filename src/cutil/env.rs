use std::env;

/// 获取环境变量，不区分大小写
pub fn get_env_var_case_insensitive(key: &str) -> Option<String> {
  if let Ok(val) = env::var(key) {
    return Some(val);
  }
  let upper = key.to_ascii_uppercase();
  if upper != key {
    if let Ok(val) = env::var(&upper) {
      return Some(val);
    }
  }
  let lower = key.to_ascii_lowercase();
  if lower != key && lower != upper {
    if let Ok(val) = env::var(&lower) {
      return Some(val);
    }
  }
  None
}