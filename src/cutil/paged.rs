use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Default, Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct Paged<T> {
  pub items: Vec<T>,
  pub page_index: u64,
  pub page_size: u64,
  pub page_count: u64,
  pub item_count: u64,
}
