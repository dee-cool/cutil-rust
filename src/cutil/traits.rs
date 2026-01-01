use std::collections::HashMap;
use uuid::Uuid;

use crate::cutil::meta::R;
use crate::meta;

pub trait Extract {
  fn extract_i64(&self, key: &str) -> Option<i64>;
  fn extract_u64(&self, key: &str) -> Option<u64>;
  fn extract_i32(&self, key: &str) -> Option<i32>;
  fn extract_u32(&self, key: &str) -> Option<u32>;
  fn extract_f32(&self, key: &str) -> Option<f32>;
  fn extract_f64(&self, key: &str) -> Option<f64>;
  fn extract_bool(&self, key: &str) -> Option<bool>;
  fn extract_string(&self, key: &str) -> Option<String>;
  fn extract_uuid(&self, key: &str) -> Option<Uuid>;

  fn extract_i64_default(&self, key: &str, default: i64) -> i64;
  fn extract_u64_default(&self, key: &str, default: u64) -> u64;
  fn extract_i32_default(&self, key: &str, default: i32) -> i32;
  fn extract_u32_default(&self, key: &str, default: u32) -> u32;
  fn extract_f32_default(&self, key: &str, default: f32) -> f32;
  fn extract_f64_default(&self, key: &str, default: f64) -> f64;
  fn extract_bool_default(&self, key: &str, default: bool) -> bool;
  fn extract_string_default(&self, key: &str, default: &str) -> String;
  fn extract_uuid_default(&self, key: &str, default: Uuid) -> Uuid;

  fn extract_vec(&self, key: &str) -> Vec<String>;
}

impl Extract for HashMap<String, String> {
  fn extract_i64(&self, key: &str) -> Option<i64> {
    self.get(key).map(|s| s.parse::<i64>().unwrap_or_default())
  }

  fn extract_u64(&self, key: &str) -> Option<u64> {
    self.get(key).map(|s| s.parse::<u64>().unwrap_or_default())
  }

  fn extract_i32(&self, key: &str) -> Option<i32> {
    self.get(key).map(|s| s.parse::<i32>().unwrap_or_default())
  }

  fn extract_u32(&self, key: &str) -> Option<u32> {
    self.get(key).map(|s| s.parse::<u32>().unwrap_or_default())
  }

  fn extract_f32(&self, key: &str) -> Option<f32> {
    self.get(key).map(|s| s.parse::<f32>().unwrap_or_default())
  }

  fn extract_f64(&self, key: &str) -> Option<f64> {
    self.get(key).map(|s| s.parse::<f64>().unwrap_or_default())
  }

  fn extract_bool(&self, key: &str) -> Option<bool> {
    self.get(key).map(|s| s.parse::<bool>().unwrap_or_default())
  }

  fn extract_string(&self, key: &str) -> Option<String> {
    self.get(key).map(|s| s.to_string())
  }

  fn extract_uuid(&self, key: &str) -> Option<Uuid> {
    self.get(key).map(|s| Uuid::parse_str(s).unwrap_or_default())
  }

  fn extract_i64_default(&self, key: &str, default: i64) -> i64 {
    self.get(key).unwrap_or(&default.to_string()).parse().unwrap_or_default()
  }

  fn extract_u64_default(&self, key: &str, default: u64) -> u64 {
    self.get(key).unwrap_or(&default.to_string()).parse().unwrap_or_default()
  }

  fn extract_i32_default(&self, key: &str, default: i32) -> i32 {
    self.get(key).unwrap_or(&default.to_string()).parse().unwrap_or_default()
  }

  fn extract_u32_default(&self, key: &str, default: u32) -> u32 {
    self.get(key).unwrap_or(&default.to_string()).parse().unwrap_or_default()
  }

  fn extract_f32_default(&self, key: &str, default: f32) -> f32 {
    self.get(key).unwrap_or(&default.to_string()).parse().unwrap_or_default()
  }

  fn extract_f64_default(&self, key: &str, default: f64) -> f64 {
    self.get(key).unwrap_or(&default.to_string()).parse().unwrap_or_default()
  }

  fn extract_bool_default(&self, key: &str, default: bool) -> bool {
    self.get(key).unwrap_or(&default.to_string()).parse().unwrap_or_default()
  }

  fn extract_string_default(&self, key: &str, default: &str) -> String {
    self.get(key).unwrap_or(&default.to_string()).to_string()
  }

  fn extract_uuid_default(&self, key: &str, default: Uuid) -> Uuid {
    self
      .get(key)
      .map(|s| Uuid::parse_str(s).unwrap_or_default())
      .unwrap_or(default)
  }

  fn extract_vec(&self, key: &str) -> Vec<String> {
    self
      .get(key)
      .map(|s| s.split(',').map(|s| s.to_string()).collect())
      .unwrap_or(vec![])
  }
}

pub trait Optionable {
  fn to_option(self) -> Option<String>;
}

impl Optionable for String {
  fn to_option(self) -> Option<String> {
    if self.is_empty() { None } else { Some(self) }
  }
}

pub trait Resultable<T> {
  fn to_result(self, name: &str, message: &str) -> R<T>;
}

impl<T> Resultable<T> for Option<T> {
  fn to_result(self, name: &str, message: &str) -> R<T> {
    match self {
      Some(value) => Ok(value),
      None => meta!(name, message),
    }
  }
}

