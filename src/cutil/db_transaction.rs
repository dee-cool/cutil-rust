use std::sync::Arc;

use crate::cutil::db_client::DbClient;
use crate::cutil::meta::{Meta, R};
use di::injectable;
use sea_orm::{DatabaseTransaction, TransactionTrait};
use tracing::debug;

tokio::task_local! {
    static TX_CONTEXT: Option<Arc<DatabaseTransaction>>;
}

fn get_tx_from_context() -> Option<Arc<DatabaseTransaction>> {
  TX_CONTEXT.try_with(|tx| tx.clone()).ok().flatten()
}

async fn with_tx<F, Fut>(tx: Option<Arc<DatabaseTransaction>>, f: F) -> Fut::Output
where
  F: FnOnce() -> Fut,
  Fut: std::future::Future,
{
  TX_CONTEXT.scope(tx, f()).await
}

#[injectable]
pub struct DbTransaction {
  client: Arc<DbClient>,
}

impl DbTransaction {
  pub async fn with_transaction<F, Fut, T>(&self, f: F) -> R<T>
  where
    F: FnOnce() -> Fut + Send,
    Fut: std::future::Future<Output = R<T>> + Send,
  {
    if let Some(existing_tx) = get_tx_from_context() {
      debug!("检测到已存在的事务上下文，复用现有事务（嵌套事务）");
      return with_tx(Some(existing_tx), f).await;
    }

    let conn = self.client.get().await?;
    let txn = conn.begin().await?;
    let txn_arc = Arc::new(txn);

    let result = with_tx(Some(txn_arc.clone()), f).await;

    match result {
      Ok(value) => {
        Arc::try_unwrap(txn_arc)
          .map_err(|_| Meta {
            name: "transaction_in_use".to_string(),
            message: "".to_string(),
            data: None,
          })?
          .commit()
          .await?;
        debug!("事务提交成功");
        Ok(value)
      }
      Err(e) => {
        Arc::try_unwrap(txn_arc)
          .map_err(|_| Meta {
            name: "transaction_in_use".to_string(),
            message: "".to_string(),
            data: None,
          })?
          .rollback()
          .await?;
        debug!("事务回滚: {:?}", e);
        Err(e)
      }
    }
  }
}
