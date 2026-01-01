use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use std::env;
use chrono::{DateTime, Local};
use crate::cutil::meta::R;
use async_trait::async_trait;
use idgenerator_thin::{IdGeneratorOptions, YitIdHelper};
use once_cell::sync::Lazy;
use rand::Rng;
use rand::distr::Alphanumeric;
use ulid::Ulid;
use uuid::Uuid;
use std::{future::Future, pin::Pin, sync::Arc};
use tokio::sync::OnceCell;

// =============================================================================
// Empty
// =============================================================================

#[derive(Debug, Serialize, Deserialize, Default, ToSchema)]
pub struct Empty {}

impl Empty {
  pub fn new() -> Self {
    Empty {}
  }
}

const EMPTY_SHARED: Empty = Empty {};

pub fn empty() -> Empty {
  EMPTY_SHARED
}

// =============================================================================
// String utilities
// =============================================================================

pub fn is_empty_string(value: Option<String>) -> Option<String> {
  value.filter(|v| !v.is_empty())
}

pub fn is_empty_string_ref(value: &Option<String>) -> Option<&String> {
  value.as_ref().filter(|v| !v.is_empty())
}

// =============================================================================
// Time utilities
// =============================================================================

pub fn now() -> DateTime<Local> {
  Local::now()
}

pub fn now_string() -> String {
  Local::now().to_rfc3339()
}

pub fn now_millis() -> i64 {
  Local::now().timestamp_millis()
}

pub fn now_secs() -> i64 {
  Local::now().timestamp()
}

// =============================================================================
// Mask utilities
// =============================================================================

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
    _ => value,
  }
}

#[cfg(test)]
mod mask_tests {
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
    assert_eq!(mask_str("abcdefghi"), "abc***ghi");
    assert_eq!(mask_str("abcd"), "a**d");
    assert_eq!(mask_str("abcde"), "a***e");
  }

  #[test]
  fn test_unicode_chars() {
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

    assert_eq!(masked, value);
  }
}

// =============================================================================
// Environment utilities
// =============================================================================

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

// =============================================================================
// Generator
// =============================================================================

#[async_trait]
pub trait Generator: Send + Sync {
  async fn next_id(&self) -> R<i64>;
}

pub struct GeneratorImpl {}

impl GeneratorImpl {
  pub fn new() -> Self {
    Self {}
  }

  fn configure(&self) {
    let mut options = IdGeneratorOptions::new(1);
    options.worker_id_bit_length = 1;
    options.base_time = 1710034500000;
    YitIdHelper::set_id_generator(options);
  }
}

#[async_trait]
impl Generator for GeneratorImpl {
  async fn next_id(&self) -> R<i64> {
    Ok(YitIdHelper::next_id())
  }
}

static GENERATOR_SHARED: Lazy<GeneratorImpl> = Lazy::new(|| {
  let generator = GeneratorImpl::new();
  generator.configure();
  generator
});

pub async fn gen_id() -> R<i64> {
  GENERATOR_SHARED.next_id().await
}

pub fn gen_string(length: usize) -> String {
  let rand_string: String = rand::rng().sample_iter(&Alphanumeric).take(length).map(char::from).collect();
  rand_string
}

pub fn gen_ulid() -> String {
  Ulid::new().to_string().to_lowercase()
}

pub fn gen_uuid() -> Uuid {
  let ulid = Ulid::new();
  ulid.into()
}

// =============================================================================
// Reqwest utilities
// =============================================================================

use tracing::info;

pub async fn create_reqwest_client() -> R<reqwest::Client> {
  let Some(http_proxy) = get_env_var_case_insensitive("http_proxy") else {
    let reqwest_client = reqwest::Client::new();
    return Ok(reqwest_client);
  };

  info!("create_client, using proxy: {}", http_proxy);
  let reqwest_proxy = reqwest::Proxy::all(http_proxy)?;
  let reqwest_client = reqwest::Client::builder().proxy(reqwest_proxy).build()?;

  Ok(reqwest_client)
}

// =============================================================================
// Lazyload
// =============================================================================

type AsyncLoader<T> = Arc<dyn Fn() -> Pin<Box<dyn Future<Output = R<T>> + Send>> + Send + Sync>;

pub struct Lazyload<T> {
  data: OnceCell<T>,
  loader: AsyncLoader<T>,
}

impl<T> std::fmt::Debug for Lazyload<T> {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    f.debug_struct("Lazyload").field("loaded", &self.loaded()).finish()
  }
}

impl<T> Lazyload<T> {
  pub fn new<F, Fut>(loader: F) -> Self
  where
    F: Fn() -> Fut + Send + Sync + 'static,
    Fut: std::future::Future<Output = R<T>> + Send + 'static,
  {
    Self {
      data: OnceCell::new(),
      loader: Arc::new(move || Box::pin(loader())),
    }
  }

  pub async fn get(&self) -> R<&T> {
    self.data.get_or_try_init(|| (self.loader)()).await
  }

  pub fn loaded(&self) -> bool {
    self.data.get().is_some()
  }
}

