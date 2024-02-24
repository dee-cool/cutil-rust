use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::error::Error;
use std::fmt;
use utoipa::ToSchema;

#[macro_export]
macro_rules! meta {
  ($name: expr) => {
    Meta {
      name: $name.to_string(),
      message: "".to_string(),
      data: None,
    }
    .into()
  };
  ($name: expr, $message: expr) => {
    Meta {
      name: $name.to_string(),
      message: $message.to_string(),
      data: None,
    }
    .into()
  };
  ($name: expr, $message: expr, $data: expr) => {
    Meta {
      name: $name.to_string(),
      message: $message.to_string(),
      data: Some($data),
    }
    .into()
  };
}

pub type R<T> = std::result::Result<T, Meta>;

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct Meta {
  pub name: String,
  pub message: String,
  pub data: Option<Value>,
}

unsafe impl Send for Meta {}

unsafe impl Sync for Meta {}

impl Error for Meta {}

impl fmt::Display for Meta {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    write!(f, "Meta: name={}, message={}, data: {:?}", self.name, self.message, self.data)
  }
}
