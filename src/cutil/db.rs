use std::sync::Arc;
use std::time::{Duration, Instant};

use crate::cutil::meta::{Meta, R};
use crate::meta;
use di::injectable;
use sea_orm::{ConnectOptions, Database, DatabaseConnection, DatabaseTransaction, TransactionTrait};
use tokio::sync::{OnceCell as AsyncOnceCell, RwLock};
use tracing::{debug, error, info, warn};

// =============================================================================
// DbClient - 数据库客户端
// =============================================================================

#[injectable]
pub struct DbClient {
  connection: RwLock<Option<Arc<DatabaseConnection>>>,
  options: AsyncOnceCell<ConnectOptions>,
  last_health_check: RwLock<Option<Instant>>,
  health_check_interval: Duration,
}

impl Default for DbClient {
  fn default() -> Self {
    Self {
      connection: RwLock::new(None),
      options: AsyncOnceCell::new(),
      last_health_check: RwLock::new(None),
      health_check_interval: Duration::from_secs(30),
    }
  }
}

impl DbClient {
  pub fn with_options(options: ConnectOptions) -> Self {
    let client = Self::default();
    let _ = client.options.set(options);
    client
  }

  pub fn with_health_check_interval(options: ConnectOptions, interval: Duration) -> Self {
    let mut client = Self::default();
    client.health_check_interval = interval;
    let _ = client.options.set(options);
    client
  }

  async fn create_connection(&self) -> R<Arc<DatabaseConnection>> {
    let options = self.options.get().ok_or_else(|| -> Meta {
      error!("Database options not configured");
      meta!("db_options_not_configured", "Database connection options not configured")
    })?;

    match Database::connect(options.clone()).await {
      Ok(conn) => {
        info!("Database connection established successfully");
        Ok(Arc::new(conn))
      }
      Err(err) => {
        error!("Database connection failed: {}", err);
        Err(meta!("connection_failed", "Failed to establish database connection"))
      }
    }
  }

  fn should_health_check(&self, last_check: Option<Instant>) -> bool {
    match last_check {
      Some(last) => last.elapsed() >= self.health_check_interval,
      None => true,
    }
  }

  async fn ensure_connection(&self) -> R<Arc<DatabaseConnection>> {
    {
      let conn_guard = self.connection.read().await;
      if let Some(conn) = conn_guard.as_ref() {
        let last_check = *self.last_health_check.read().await;

        if !self.should_health_check(last_check) {
          return Ok(conn.clone());
        }

        match conn.ping().await {
          Ok(_) => {
            *self.last_health_check.write().await = Some(Instant::now());
            return Ok(conn.clone());
          }
          Err(_) => {
            warn!("Existing connection is invalid, will recreate");
          }
        }
      }
    }

    let mut conn_guard = self.connection.write().await;

    if let Some(conn) = conn_guard.as_ref() {
      let last_check = *self.last_health_check.read().await;

      if !self.should_health_check(last_check) {
        return Ok(conn.clone());
      }

      match conn.ping().await {
        Ok(_) => {
          *self.last_health_check.write().await = Some(Instant::now());
          return Ok(conn.clone());
        }
        Err(_) => {}
      }
    }

    info!("Creating new database connection");
    let new_connection = self.create_connection().await?;
    *conn_guard = Some(new_connection.clone());

    *self.last_health_check.write().await = Some(Instant::now());

    Ok(new_connection)
  }

  pub async fn get(&self) -> R<Arc<DatabaseConnection>> {
    self.ensure_connection().await
  }

  pub async fn ping(&self) -> R<()> {
    let conn = self.get().await?;
    conn.ping().await.map_err(|e| {
      error!("Database ping failed: {}", e);
      meta!("ping_failed", "Database ping failed")
    })
  }

  pub async fn reset_connection(&self) -> R<()> {
    info!("Resetting database connection");

    let mut conn_guard = self.connection.write().await;

    if let Some(conn) = conn_guard.take() {
      if let Ok(conn) = Arc::try_unwrap(conn) {
        if let Err(e) = conn.close().await {
          warn!("Failed to close existing connection during reset: {}", e);
        }
      }
    }

    *self.last_health_check.write().await = None;

    info!("Database connection reset completed");
    Ok(())
  }

  pub async fn is_connected(&self) -> bool {
    let conn_guard = self.connection.read().await;
    conn_guard.is_some()
  }

  pub fn is_configured(&self) -> bool {
    self.options.get().is_some()
  }

  pub async fn close(&self) -> R<()> {
    info!("Closing database connection");

    let mut conn_guard = self.connection.write().await;

    if let Some(conn) = conn_guard.take() {
      if let Ok(conn) = Arc::try_unwrap(conn) {
        conn.close().await.map_err(|e: sea_orm::DbErr| -> Meta {
          error!("Failed to close database connection: {}", e);
          meta!("close_failed", "Failed to close database connection")
        })?;
      }
      info!("Database connection closed successfully");
    }

    Ok(())
  }
}

// =============================================================================
// DbTransaction - 数据库事务
// =============================================================================

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

// =============================================================================
// CRUD操作宏和自动更新工具
// =============================================================================

use sea_orm::*;

/// 通用的自动更新trait - 为实体提供自动字段设置功能
pub trait ImplAutoUpdate<Model, ActiveModel, Domain>
where
  Model: ModelTrait + Into<Domain> + Send + Sync,
  ActiveModel: ActiveModelTrait<Entity = Self> + Send + Sync,
  Domain: Into<Model> + Clone + Send + Sync,
  Self: EntityTrait<Model = Model>,
{
  fn set_all_fields_for_update(domain: Domain) -> ActiveModel;
}

/// 通用的Repository更新扩展trait
pub trait ImplRepositoryAutoUpdate {
  async fn auto_update<Entity, Model, ActiveModel, Domain>(&self, _entity: Domain) -> R<Domain>
  where
    Entity: EntityTrait<Model = Model> + ImplAutoUpdate<Model, ActiveModel, Domain>,
    Model: ModelTrait + Into<Domain> + Send + Sync + IntoActiveModel<ActiveModel>,
    ActiveModel: ActiveModelTrait<Entity = Entity> + ActiveModelBehavior + Send + Sync,
    Domain: Into<Model> + Clone + Send + Sync,
  {
    unimplemented!("This trait method should be implemented in specific repositories using the macro")
  }
}

/// 通用更新工具类 - 提供静态方法进行自动更新
pub struct ImplUniversalUpdater;

impl ImplUniversalUpdater {
  pub async fn update_all_fields<Entity, Model, ActiveModel, Domain>(conn: &DatabaseConnection, entity: Domain) -> R<Domain>
  where
    Entity: EntityTrait<Model = Model> + ImplAutoUpdate<Model, ActiveModel, Domain>,
    Model: ModelTrait + Into<Domain> + IntoActiveModel<ActiveModel> + Send + Sync,
    ActiveModel: ActiveModelTrait<Entity = Entity> + ActiveModelBehavior + Send + Sync,
    Domain: Into<Model> + Clone + Send + Sync,
  {
    let active_model = Entity::set_all_fields_for_update(entity.clone());
    let updated_model = active_model.update(conn).await?;
    Ok(updated_model.into())
  }

  pub async fn batch_update_all_fields<Entity, Model, ActiveModel, Domain>(
    conn: &DatabaseConnection,
    entities: Vec<Domain>,
  ) -> R<Vec<Domain>>
  where
    Entity: EntityTrait<Model = Model> + ImplAutoUpdate<Model, ActiveModel, Domain>,
    Model: ModelTrait + Into<Domain> + IntoActiveModel<ActiveModel> + Send + Sync,
    ActiveModel: ActiveModelTrait<Entity = Entity> + ActiveModelBehavior + Send + Sync,
    Domain: Into<Model> + Clone + Send + Sync,
  {
    let mut results = Vec::new();

    for entity in entities {
      let result = Self::update_all_fields::<Entity, Model, ActiveModel, Domain>(conn, entity).await?;
      results.push(result);
    }

    Ok(results)
  }
}

// =============================================================================
// 自动更新宏定义
// =============================================================================

#[macro_export]
macro_rules! impl_auto_update {
    ($entity:ident, $model:ty, $active_model:ty, $domain:ty, [$($field:ident),+ $(,)?]) => {
        impl $crate::cutil::db::ImplAutoUpdate<$model, $active_model, $domain> for $entity {
            fn set_all_fields_for_update(domain: $domain) -> $active_model {
                let model: $model = domain.into();
                let mut active_model = <$active_model>::from(model);

                $(
                    active_model.$field = sea_orm::ActiveValue::Set(active_model.$field.unwrap());
                )+

                active_model
            }
        }
    };
}

#[macro_export]
macro_rules! impl_smart_update {
  ($repo:ident, $domain:ty, $entity_mod:ident, { $($field:ident),+ }) => {
    impl $repo {
      pub async fn update_smart(&self, entity: $domain) -> R<$domain> {
        let conn = self.client.get().await?;
        use sea_orm::ActiveModelTrait;
        let model: $entity_mod::Model = entity.clone().into();
        let mut active_model = $entity_mod::ActiveModel::from(model);

        $(
          active_model.$field = sea_orm::ActiveValue::Set(active_model.$field.unwrap());
        )+

        let updated_model = active_model.update(&*conn).await?;
        Ok(updated_model.into())
      }
    }
  };
}

#[macro_export]
macro_rules! impl_auto_update_to_repo {
  ($repo:ident) => {
    impl $repo {
      pub async fn update_all_auto<Entity, Model, ActiveModel, Domain>(&self, entity: Domain) -> R<Domain>
      where
        Entity: $crate::cutil::db::ImplAutoUpdate<Model, ActiveModel, Domain> + sea_orm::EntityTrait<Model = Model>,
        Model: sea_orm::ModelTrait + Into<Domain> + sea_orm::IntoActiveModel<ActiveModel> + Send + Sync,
        ActiveModel: sea_orm::ActiveModelTrait<Entity = Entity> + sea_orm::ActiveModelBehavior + Send + Sync,
        Domain: Into<Model> + Clone + Send + Sync,
      {
        let conn = self.client.get().await?;
        $crate::cutil::db::ImplUniversalUpdater::update_all_fields::<Entity, Model, ActiveModel, Domain>(&*conn, entity)
          .await
      }
    }
  };
}

// =============================================================================
// Repository结构定义宏
// =============================================================================

#[macro_export]
macro_rules! impl_repository_struct {
  ($repo:ident) => {
    pub struct $repo {
      client: std::sync::Arc<$crate::cutil::db::DbClient>,
    }

    impl $repo {
      pub fn new(client: std::sync::Arc<$crate::cutil::db::DbClient>) -> Self {
        Self { client }
      }
    }
  };
}

// =============================================================================
// 核心CRUD宏定义
// =============================================================================

#[macro_export]
macro_rules! impl_batch_delete_helper {
  () => {
    async fn batch_delete_internal<Entity, PrimaryKey>(conn: &sea_orm::DatabaseConnection, ids: Vec<PrimaryKey>) -> R<u64>
    where
      Entity: sea_orm::EntityTrait,
      PrimaryKey: Copy + Send + Sync + Into<sea_orm::Value>,
    {
      if ids.is_empty() {
        return Ok(0);
      }

      use sea_orm::*;

      let txn = conn.begin().await?;
      let mut total_deleted = 0u64;

      for chunk in ids.chunks(1000) {
        let delete_result = Entity::delete_many()
          .filter(
            Entity::find().filter(Condition::any().add(chunk.iter().fold(Condition::any(), |cond, &id| {
              cond.add(Entity::find_by_id(id).build(sea_orm::DatabaseBackend::Postgres).clone())
            }))),
          )
          .exec(&txn)
          .await?;

        total_deleted += delete_result.rows_affected;
      }

      txn.commit().await?;
      Ok(total_deleted)
    }
  };
}

#[macro_export]
macro_rules! impl_core_crud_operations {
  ($repo:ident, $domain:ty, $entity_mod:ident, $primary_key:ty, $method_suffix:expr) => {
    paste::paste! {
      impl $repo {
        pub async fn [<create $method_suffix>](&self, entity: $domain) -> R<$domain> {
          let conn = self.client.get().await?;
          use sea_orm::EntityTrait;
          let model: $entity_mod::Model = entity.clone().into();
          let active_model = $entity_mod::ActiveModel::from(model);

          let txn = conn.begin().await?;
          let result = $entity_mod::Entity::insert(active_model).exec(&txn).await;

          match result {
            Ok(_) => {
              txn.commit().await?;
              Ok(entity)
            },
            Err(e) => {
              txn.rollback().await?;
              Err(e.into())
            }
          }
        }

        pub async fn [<get $method_suffix>](&self, id: $primary_key) -> R<Option<$domain>> {
          let conn = self.client.get().await?;
          use sea_orm::EntityTrait;
          let result = $entity_mod::Entity::find_by_id(id).one(&*conn).await?;
          Ok(result.map(Into::into))
        }

        pub async fn [<update_with $method_suffix>]<F>(&self, entity: $domain, field_setter: F) -> R<$domain>
        where
          F: FnOnce(&mut $entity_mod::ActiveModel),
        {
          let conn = self.client.get().await?;
          use sea_orm::ActiveModelTrait;
          let model: $entity_mod::Model = entity.clone().into();
          let mut active_model = $entity_mod::ActiveModel::from(model);

          field_setter(&mut active_model);

          let txn = conn.begin().await?;
          let result = active_model.update(&txn).await;

          match result {
            Ok(updated_model) => {
              txn.commit().await?;
              Ok(updated_model.into())
            },
            Err(e) => {
              txn.rollback().await?;
              Err(e.into())
            }
          }
        }

        pub async fn [<delete $method_suffix>](&self, id: $primary_key) -> R<()> {
          let conn = self.client.get().await?;
          use sea_orm::EntityTrait;

          let txn = conn.begin().await?;
          let result = $entity_mod::Entity::delete_by_id(id).exec(&txn).await;

          match result {
            Ok(delete_result) => {
              if delete_result.rows_affected == 0 {
                txn.rollback().await?;
                return Err(sea_orm::DbErr::RecordNotFound("要删除的实体不存在".to_string()).into());
              }
              txn.commit().await?;
              Ok(())
            },
            Err(e) => {
              txn.rollback().await?;
              Err(e.into())
            }
          }
        }

        pub async fn [<create_many $method_suffix>](&self, entities: Vec<$domain>) -> R<Vec<$domain>> {
          let conn = self.client.get().await?;
          use sea_orm::EntityTrait;
          if entities.is_empty() {
            return Ok(vec![]);
          }

          let active_models: Vec<$entity_mod::ActiveModel> = entities
            .iter()
            .map(|e| {
              let model: $entity_mod::Model = e.clone().into();
              $entity_mod::ActiveModel::from(model)
            })
            .collect();

          let txn = conn.begin().await?;
          let result = $entity_mod::Entity::insert_many(active_models).exec(&txn).await;

          match result {
            Ok(_) => {
              txn.commit().await?;
              Ok(entities)
            },
            Err(e) => {
              txn.rollback().await?;
              Err(e.into())
            }
          }
        }

        pub async fn [<delete_many $method_suffix>](&self, ids: Vec<$primary_key>) -> R<u64> {
          let conn = self.client.get().await?;
          use sea_orm::*;
          if ids.is_empty() {
            return Ok(0);
          }

          let txn = conn.begin().await?;
          let mut total_deleted = 0u64;

          for chunk in ids.chunks(500) {
            for &id in chunk {
              match $entity_mod::Entity::delete_by_id(id).exec(&txn).await {
                Ok(result) => total_deleted += result.rows_affected,
                Err(e) => {
                  txn.rollback().await?;
                  return Err(e.into());
                }
              }
            }
          }

          txn.commit().await?;
          Ok(total_deleted)
        }

        pub async fn [<exists $method_suffix>](&self, id: $primary_key) -> R<bool> {
          let conn = self.client.get().await?;
          use sea_orm::EntityTrait;
          let result = $entity_mod::Entity::find_by_id(id).one(&*conn).await?;
          Ok(result.is_some())
        }

        pub async fn [<count $method_suffix>](&self) -> R<u64> {
          let conn = self.client.get().await?;
          use sea_orm::EntityTrait;
          let count = $entity_mod::Entity::find().count(&*conn).await?;
          Ok(count)
        }

        pub async fn [<find_all $method_suffix>](&self) -> R<Vec<$domain>> {
          let conn = self.client.get().await?;
          use sea_orm::EntityTrait;
          let results = $entity_mod::Entity::find().all(&*conn).await?;
          Ok(results.into_iter().map(Into::into).collect())
        }

        pub async fn [<find_paged $method_suffix>](&self, pos: $crate::cutil::paged::Pos) -> R<$crate::cutil::paged::Paged<$domain>> {
          let conn = self.client.get().await?;
          use sea_orm::EntityTrait;
          let query = $entity_mod::Entity::find();
          pos
            .paged::<$entity_mod::Model, $entity_mod::Entity, $domain>(&*conn, query)
            .await
        }
      }
    }
  };
}

#[macro_export]
macro_rules! impl_crud_methods {
  ($repo:ident, $domain:ty, $entity_mod:ident, $primary_key:ty) => {
    impl_core_crud_operations!($repo, $domain, $entity_mod, $primary_key, "_entity");
  };
}

#[macro_export]
macro_rules! impl_standard_crud {
  ($repo:ident, $domain:ty, $entity_mod:ident, $primary_key:ty) => {
    impl_core_crud_operations!($repo, $domain, $entity_mod, $primary_key, "");
  };
}

// =============================================================================
// 高级功能宏定义
// =============================================================================

pub trait ImplExtractId<PrimaryKey> {
  fn extract_id(&self) -> PrimaryKey;
}

pub trait ImplSoftDelete {
  fn set_deleted(&mut self, deleted: bool);
  fn is_deleted(&self) -> bool;
}

pub trait ImplAuditFields {
  fn set_created(&mut self, timestamp: chrono::DateTime<chrono::Utc>);
  fn set_updated(&mut self, timestamp: chrono::DateTime<chrono::Utc>);
  fn set_created_by(&mut self, user_id: Option<String>);
  fn set_updated_by(&mut self, user_id: Option<String>);
}

#[macro_export]
macro_rules! impl_advanced_methods {
  ($repo:ident, $domain:ty, $entity_mod:ident, $primary_key:ty) => {
    impl $repo {
      pub async fn get_required(&self, id: $primary_key) -> R<$domain> {
        match self.get(id).await? {
          Some(entity) => Ok(entity),
          None => Err(sea_orm::DbErr::RecordNotFound(format!("实体不存在: {:?}", id)).into()),
        }
      }

      pub async fn get_many(&self, ids: Vec<$primary_key>) -> R<Vec<$domain>> {
        let mut results = Vec::new();
        for id in ids {
          if let Some(entity) = self.get(id).await? {
            results.push(entity);
          }
        }
        Ok(results)
      }

      pub async fn get_many_required(&self, ids: Vec<$primary_key>) -> R<Vec<$domain>> {
        let mut results = Vec::new();
        for id in ids {
          let entity = self.get_required(id).await?;
          results.push(entity);
        }
        Ok(results)
      }

      pub async fn exists_all(&self, ids: Vec<$primary_key>) -> R<bool> {
        for id in ids {
          if !self.exists(id).await? {
            return Ok(false);
          }
        }
        Ok(true)
      }

      pub async fn get_missing_ids(&self, ids: Vec<$primary_key>) -> R<Vec<$primary_key>>
      where
        $primary_key: Clone,
      {
        let mut missing = Vec::new();
        for id in ids {
          if !self.exists(id.clone()).await? {
            missing.push(id);
          }
        }
        Ok(missing)
      }
    }
  };
}

#[macro_export]
macro_rules! impl_soft_delete_methods {
  ($repo:ident, $domain:ty, $entity_mod:ident, $primary_key:ty) => {
    impl $repo {
      pub async fn soft_delete(&self, id: $primary_key) -> R<$domain>
      where
        $domain: $crate::cutil::db::ImplSoftDelete + $crate::cutil::db::ImplAuditFields,
      {
        let mut entity = self.get_required(id).await?;
        entity.set_deleted(true);
        entity.set_updated(chrono::Utc::now());
        self.update_smart(entity).await
      }

      pub async fn soft_delete_many(&self, ids: Vec<$primary_key>) -> R<Vec<$domain>>
      where
        $domain: $crate::cutil::db::ImplSoftDelete + $crate::cutil::db::ImplAuditFields,
      {
        let mut results = Vec::new();
        for id in ids {
          let result = self.soft_delete(id).await?;
          results.push(result);
        }
        Ok(results)
      }

      pub async fn restore_soft_deleted(&self, id: $primary_key) -> R<$domain>
      where
        $domain: $crate::cutil::db::ImplSoftDelete + $crate::cutil::db::ImplAuditFields,
      {
        let mut entity = self.get_required(id).await?;
        entity.set_deleted(false);
        entity.set_updated(chrono::Utc::now());
        self.update_smart(entity).await
      }

      pub async fn find_not_deleted(&self) -> R<Vec<$domain>>
      where
        $domain: $crate::cutil::db::ImplSoftDelete,
      {
        let all_entities = self.find_all().await?;
        Ok(all_entities.into_iter().filter(|e| !e.is_deleted()).collect())
      }

      pub async fn find_deleted(&self) -> R<Vec<$domain>>
      where
        $domain: $crate::cutil::db::ImplSoftDelete,
      {
        let all_entities = self.find_all().await?;
        Ok(all_entities.into_iter().filter(|e| e.is_deleted()).collect())
      }

      pub async fn purge_soft_deleted(&self) -> R<u64>
      where
        $domain: $crate::cutil::db::ImplSoftDelete + $crate::cutil::db::ImplExtractId<$primary_key>,
      {
        let deleted_entities = self.find_deleted().await?;
        let ids: Vec<$primary_key> = deleted_entities.iter().map(|e| e.extract_id()).collect();
        self.delete_many(ids).await
      }
    }
  };
}

#[macro_export]
macro_rules! impl_audit_methods {
  ($repo:ident, $domain:ty, $entity_mod:ident, $primary_key:ty) => {
    impl $repo {
      pub async fn create_with_audit(&self, mut entity: $domain, user_id: Option<String>) -> R<$domain>
      where
        $domain: $crate::cutil::db::ImplAuditFields,
      {
        let now = chrono::Utc::now();
        entity.set_created(now);
        entity.set_updated(now);
        entity.set_created_by(user_id.clone());
        entity.set_updated_by(user_id);
        self.create(entity).await
      }

      pub async fn update_with_audit(&self, mut entity: $domain, user_id: Option<String>) -> R<$domain>
      where
        $domain: $crate::cutil::db::ImplAuditFields,
      {
        entity.set_updated(chrono::Utc::now());
        entity.set_updated_by(user_id);
        self.update_smart(entity).await
      }

      pub async fn create_many_with_audit(&self, mut entities: Vec<$domain>, user_id: Option<String>) -> R<Vec<$domain>>
      where
        $domain: $crate::cutil::db::ImplAuditFields,
      {
        let now = chrono::Utc::now();
        for entity in &mut entities {
          entity.set_created(now);
          entity.set_updated(now);
          entity.set_created_by(user_id.clone());
          entity.set_updated_by(user_id.clone());
        }
        self.create_many(entities).await
      }
    }
  };
}

#[macro_export]
macro_rules! impl_upsert_methods {
  ($repo:ident, $domain:ty, $entity_mod:ident, $primary_key:ty) => {
    impl $repo {
      pub async fn upsert(&self, entity: $domain) -> R<($domain, bool)>
      where
        $domain: $crate::cutil::db::ImplExtractId<$primary_key>,
      {
        let id = entity.extract_id();
        let is_new = !self.exists(id).await?;

        let result = if is_new {
          self.create(entity).await?
        } else {
          self.update_smart(entity).await?
        };

        Ok((result, is_new))
      }

      pub async fn upsert_many(&self, entities: Vec<$domain>) -> R<(Vec<$domain>, usize, usize)>
      where
        $domain: $crate::cutil::db::ImplExtractId<$primary_key>,
      {
        let mut results = Vec::new();
        let mut created_count = 0;
        let mut updated_count = 0;

        for entity in entities {
          let (result, is_new) = self.upsert(entity).await?;
          results.push(result);

          if is_new {
            created_count += 1;
          } else {
            updated_count += 1;
          }
        }

        Ok((results, created_count, updated_count))
      }
    }
  };
}

#[macro_export]
macro_rules! impl_all_advanced_methods {
  ($repo:ident, $domain:ty, $entity_mod:ident, $primary_key:ty) => {
    impl_advanced_methods!($repo, $domain, $entity_mod, $primary_key);
    impl_soft_delete_methods!($repo, $domain, $entity_mod, $primary_key);
    impl_audit_methods!($repo, $domain, $entity_mod, $primary_key);
    impl_upsert_methods!($repo, $domain, $entity_mod, $primary_key);
  };
}

#[macro_export]
macro_rules! impl_full_crud {
  ($repo:ident, $domain:ty, $entity_mod:ident, $primary_key:ty) => {
    impl_standard_crud!($repo, $domain, $entity_mod, $primary_key);
    impl_advanced_methods!($repo, $domain, $entity_mod, $primary_key);
  };
}

#[macro_export]
macro_rules! impl_complete_crud {
  ($repo:ident, $domain:ty, $entity_mod:ident, $primary_key:ty, [$($field:ident),+ $(,)?]) => {
    impl_standard_crud!($repo, $domain, $entity_mod, $primary_key);
    impl_smart_update!($repo, $domain, $entity_mod, { $($field),+ });
    impl_auto_update_to_repo!($repo);
    impl_advanced_methods!($repo, $domain, $entity_mod, $primary_key);
  };
}

#[macro_export]
macro_rules! impl_super_complete_crud {
  ($repo:ident, $domain:ty, $entity_mod:ident, $primary_key:ty, [$($field:ident),+ $(,)?]) => {
    impl_standard_crud!($repo, $domain, $entity_mod, $primary_key);
    impl_smart_update!($repo, $domain, $entity_mod, { $($field),+ });
    impl_auto_update_to_repo!($repo);
    impl_all_advanced_methods!($repo, $domain, $entity_mod, $primary_key);
  };
}

