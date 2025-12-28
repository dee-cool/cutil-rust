use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Default, Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct Paged<T> {
  pub items: Vec<T>,
  pub page_index: u64,
  pub page_size: u64,
  pub total_pages: u64,
  pub total_items: u64,
}

impl<T> Paged<T> {
  pub fn new(items: Vec<T>, page_index: u64, page_size: u64, total_pages: u64, total_items: u64) -> Self {
    Self {
      items,
      page_index,
      page_size,
      total_pages,
      total_items,
    }
  }
}
