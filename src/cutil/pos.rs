use std::collections::HashMap;
use std::str::FromStr;

use crate::cutil::extract::Extract;
use crate::cutil::meta::R;
use crate::cutil::paged::Paged;
use sea_orm::{DatabaseConnection, EntityTrait, FromQueryResult, PaginatorTrait, QueryOrder, Select};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct Pos {
  pub page_index: u64,
  pub page_size: u64,
  pub order_by: Vec<String>,
}

impl Default for Pos {
  fn default() -> Self {
    Pos {
      page_index: 0,
      page_size: 10,
      order_by: vec![],
    }
  }
}

impl Pos {
  pub fn first() -> Self {
    Pos {
      page_index: 0,
      page_size: 1,
      order_by: vec![],
    }
  }

  pub fn new(page_index: u64, page_size: u64, order_by: Vec<String>) -> Self {
    Pos {
      page_index,
      page_size,
      order_by,
    }
  }
  pub fn from(params: HashMap<String, String>) -> Self {
    let page_index = params.extract_u64_default("page_index", 0);
    let page_size = params.extract_u64_default("page_size", 10);
    let order_by: Vec<String> = params.extract_vec("order_by");

    Pos {
      page_index,
      page_size,
      order_by,
    }
  }

  pub async fn paged<M, E, D>(&self, conn: &DatabaseConnection, mut query: Select<E>) -> R<Paged<D>>
  where
    M: FromQueryResult + Send + Sync,
    E: EntityTrait<Model = M>,
    D: FromQueryResult + Send + Sync,
  {
    for item in &self.order_by {
      let list = item.split('.').collect::<Vec<_>>();
      if list.len() != 2 {
        continue;
      }

      let (column_name, sort_direction) = (list[0], list[1]);

      if let Ok(column) = E::Column::from_str(column_name) {
        query = match sort_direction.to_lowercase().as_str() {
          "asc" => query.order_by_asc(column),
          _ => query.order_by_desc(column),
        };
      }
    }
    let page_size = if self.page_size == 0 { 1 } else { self.page_size };
    let query = query.into_model::<D>();
    let paginator = query.paginate(conn, page_size);

    let items: Vec<D> = paginator.fetch_page(self.page_index).await?;

    let num = paginator.num_items_and_pages().await?;

    let paged = Paged {
      page_index: self.page_index,
      page_size,
      page_count: num.number_of_pages,
      items,
      item_count: num.number_of_items,
    };
    Ok(paged)
  }
}
