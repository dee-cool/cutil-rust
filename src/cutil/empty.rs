use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Debug, Serialize, Deserialize, Default, ToSchema)]
pub struct Empty {}

impl Empty {
  pub fn new() -> Self {
    Empty {}
  }
}

const SHARED: Empty = Empty {};

pub fn empty() -> Empty {
  SHARED
}
