use crate::cutil::meta::R;
use crate::meta;

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
