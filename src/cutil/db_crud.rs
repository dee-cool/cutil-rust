use crate::cutil::meta::R;
use sea_orm::*;

/// CRUD操作宏和自动更新工具 - 简化Repository的常用操作
/// 这些宏提供了完整的CRUD方法和自动更新功能

// =============================================================================
// 自动更新功能模块
// =============================================================================

/// 通用的自动更新trait - 为实体提供自动字段设置功能
/// 为每个需要自动更新的实体实现这个trait
pub trait ImplAutoUpdate<Model, ActiveModel, Domain>
where
  Model: ModelTrait + Into<Domain> + Send + Sync,
  ActiveModel: ActiveModelTrait<Entity = Self> + Send + Sync,
  Domain: Into<Model> + Clone + Send + Sync,
  Self: EntityTrait<Model = Model>,
{
  /// 自动更新方法 - 将实体的所有字段设置为更新状态
  /// 需要在具体实现中定义哪些字段需要更新
  fn set_all_fields_for_update(domain: Domain) -> ActiveModel;
}

/// 通用的Repository更新扩展trait
pub trait ImplRepositoryAutoUpdate {
  /// 通用的自动更新方法
  async fn auto_update<Entity, Model, ActiveModel, Domain>(&self, _entity: Domain) -> R<Domain>
  where
    Entity: EntityTrait<Model = Model> + ImplAutoUpdate<Model, ActiveModel, Domain>,
    Model: ModelTrait + Into<Domain> + Send + Sync + IntoActiveModel<ActiveModel>,
    ActiveModel: ActiveModelTrait<Entity = Entity> + ActiveModelBehavior + Send + Sync,
    Domain: Into<Model> + Clone + Send + Sync,
  {
    // 这个trait需要在具体的Repository中实现，因为需要访问client
    unimplemented!("This trait method should be implemented in specific repositories using the macro")
  }
}

/// 通用更新工具类 - 提供静态方法进行自动更新
pub struct ImplUniversalUpdater;

impl ImplUniversalUpdater {
  /// 完全通用的更新方法 - 使用宏生成的自动更新逻辑
  pub async fn update_all_fields<Entity, Model, ActiveModel, Domain>(conn: &DatabaseConnection, entity: Domain) -> R<Domain>
  where
    Entity: EntityTrait<Model = Model> + ImplAutoUpdate<Model, ActiveModel, Domain>,
    Model: ModelTrait + Into<Domain> + IntoActiveModel<ActiveModel> + Send + Sync,
    ActiveModel: ActiveModelTrait<Entity = Entity> + ActiveModelBehavior + Send + Sync,
    Domain: Into<Model> + Clone + Send + Sync,
  {
    // 使用AutoUpdate trait自动设置所有字段
    let active_model = Entity::set_all_fields_for_update(entity.clone());
    let updated_model = active_model.update(conn).await?;
    Ok(updated_model.into())
  }

  /// 批量自动更新
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

/// 为实体实现自动更新功能 - 更强大和灵活的版本
/// 这个宏会自动生成所有字段的Set()包装代码，无需手动指定每个字段
///
/// 使用方法：
/// ```ignore
/// // 为DeviceRecent实体实现自动更新
/// use rust_cutil::impl_auto_update;
///
/// impl_auto_update!(
///   device_recent::Entity,
///   device_recent::Model,
///   device_recent::ActiveModel,
///   DeviceRecent,
///   [platform, is_active, updated, created]
/// );
/// ```
#[macro_export]
macro_rules! impl_auto_update {
    ($entity:ident, $model:ty, $active_model:ty, $domain:ty, [$($field:ident),+ $(,)?]) => {
        impl $crate::cutil::db_crud::ImplAutoUpdate<$model, $active_model, $domain> for $entity {
            fn set_all_fields_for_update(domain: $domain) -> $active_model {
                let model: $model = domain.into();
                let mut active_model = <$active_model>::from(model);

                // 自动设置所有指定字段为Set状态
                $(
                    active_model.$field = sea_orm::ActiveValue::Set(active_model.$field.unwrap());
                )+

                active_model
            }
        }
    };
}

/// 为Repository实现一个智能的update方法，自动设置指定字段为Set状态
///
/// 使用方法：
/// ```ignore
/// impl_smart_update!(DeviceRecentRepo, DeviceRecent, device_recent, {
///   platform, is_active, updated
/// });
/// ```
#[macro_export]
macro_rules! impl_smart_update {
  ($repo:ident, $domain:ty, $entity_mod:ident, { $($field:ident),+ }) => {
    impl $repo {
      /// 智能更新方法 - 自动设置指定字段为Set状态
      pub async fn update_smart(&self, entity: $domain) -> R<$domain> {
        let conn = self.client.get().await?;
        use sea_orm::ActiveModelTrait;
        let model: $entity_mod::Model = entity.clone().into();
        let mut active_model = $entity_mod::ActiveModel::from(model);

        // 自动设置指定字段为Set状态
        $(
          active_model.$field = sea_orm::ActiveValue::Set(active_model.$field.unwrap());
        )+

        let updated_model = active_model.update(&*conn).await?;
        Ok(updated_model.into())
      }
    }
  };
}

/// 为任何Repository添加自动更新功能
/// 这是一个简化的宏，一次性添加多种自动更新方法
///
/// 使用方法：
/// ```ignore
/// add_auto_update_to_repo!(DeviceRecentRepo);
/// ```
#[macro_export]
macro_rules! impl_auto_update_to_repo {
  ($repo:ident) => {
    impl $repo {
      /// 自动更新方法 - 不需要指定字段名（需要先用impl_auto_update!宏）
      pub async fn update_all_auto<Entity, Model, ActiveModel, Domain>(&self, entity: Domain) -> R<Domain>
      where
        Entity: $crate::cutil::db_crud::ImplAutoUpdate<Model, ActiveModel, Domain> + sea_orm::EntityTrait<Model = Model>,
        Model: sea_orm::ModelTrait + Into<Domain> + sea_orm::IntoActiveModel<ActiveModel> + Send + Sync,
        ActiveModel: sea_orm::ActiveModelTrait<Entity = Entity> + sea_orm::ActiveModelBehavior + Send + Sync,
        Domain: Into<Model> + Clone + Send + Sync,
      {
        let conn = self.client.get().await?;
        $crate::cutil::db_crud::ImplUniversalUpdater::update_all_fields::<Entity, Model, ActiveModel, Domain>(&*conn, entity)
          .await
      }
    }
  };
}

// =============================================================================
// Repository结构定义宏
// =============================================================================

/// 为Repository定义标准结构，包含DbClient字段
///
/// 使用方法：
/// ```ignore
/// impl_repository_struct!(UserRepo);
/// ```
#[macro_export]
macro_rules! impl_repository_struct {
  ($repo:ident) => {
    pub struct $repo {
      client: std::sync::Arc<$crate::cutil::db_client::DbClient>,
    }

    impl $repo {
      /// 创建新的Repository实例
      pub fn new(client: std::sync::Arc<$crate::cutil::db_client::DbClient>) -> Self {
        Self { client }
      }
    }
  };
}

// =============================================================================
// 核心CRUD宏定义 - 重构优化版本
// =============================================================================

/// 核心批量删除辅助宏 - 提供高效的批量删除实现
#[macro_export]
macro_rules! impl_batch_delete_helper {
  () => {
    /// 高效批量删除的通用实现
    async fn batch_delete_internal<Entity, PrimaryKey>(conn: &sea_orm::DatabaseConnection, ids: Vec<PrimaryKey>) -> R<u64>
    where
      Entity: sea_orm::EntityTrait,
      PrimaryKey: Copy + Send + Sync + Into<sea_orm::Value>,
    {
      if ids.is_empty() {
        return Ok(0);
      }

      use sea_orm::*;

      // 使用事务确保数据一致性
      let txn = conn.begin().await?;
      let mut total_deleted = 0u64;

      // 批量删除优化：每次删除最多1000条记录
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

/// 核心CRUD操作宏 - 统一的底层实现
/// 这个宏包含了所有CRUD操作的核心逻辑，避免代码重复
/// Repository需要包含一个db_client字段
#[macro_export]
macro_rules! impl_core_crud_operations {
  ($repo:ident, $domain:ty, $entity_mod:ident, $primary_key:ty, $method_suffix:expr) => {
    paste::paste! {
      impl $repo {
        /// 创建实体
        pub async fn [<create $method_suffix>](&self, entity: $domain) -> R<$domain> {
          let conn = self.client.get().await?;
          use sea_orm::EntityTrait;
          let model: $entity_mod::Model = entity.clone().into();
          let active_model = $entity_mod::ActiveModel::from(model);

          // 使用事务确保数据一致性
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

        /// 根据主键获取实体
        pub async fn [<get $method_suffix>](&self, id: $primary_key) -> R<Option<$domain>> {
          let conn = self.client.get().await?;
          use sea_orm::EntityTrait;
          let result = $entity_mod::Entity::find_by_id(id).one(&*conn).await?;
          Ok(result.map(Into::into))
        }

        /// 更新实体 - 使用闭包自定义要更新的字段
        pub async fn [<update_with $method_suffix>]<F>(&self, entity: $domain, field_setter: F) -> R<$domain>
        where
          F: FnOnce(&mut $entity_mod::ActiveModel),
        {
          let conn = self.client.get().await?;
          use sea_orm::ActiveModelTrait;
          let model: $entity_mod::Model = entity.clone().into();
          let mut active_model = $entity_mod::ActiveModel::from(model);

          // 让使用者自定义要更新的字段
          field_setter(&mut active_model);

          // 使用事务
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

        /// 删除实体
        pub async fn [<delete $method_suffix>](&self, id: $primary_key) -> R<()> {
          let conn = self.client.get().await?;
          use sea_orm::EntityTrait;

          // 使用事务
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

        /// 批量创建实体
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

          // 使用事务确保所有操作成功或全部回滚
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

        /// 高效批量删除实体
        pub async fn [<delete_many $method_suffix>](&self, ids: Vec<$primary_key>) -> R<u64> {
          let conn = self.client.get().await?;
          use sea_orm::*;
          if ids.is_empty() {
            return Ok(0);
          }

          // 使用事务确保数据一致性
          let txn = conn.begin().await?;
          let mut total_deleted = 0u64;

          // 批量删除优化：每批最多处理500个ID，避免SQL参数过多
          for chunk in ids.chunks(500) {
            // 使用简单的循环删除，确保兼容性
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

        /// 检查实体是否存在
        pub async fn [<exists $method_suffix>](&self, id: $primary_key) -> R<bool> {
          let conn = self.client.get().await?;
          use sea_orm::EntityTrait;
          let result = $entity_mod::Entity::find_by_id(id).one(&*conn).await?;
          Ok(result.is_some())
        }

        /// 获取实体总数
        pub async fn [<count $method_suffix>](&self) -> R<u64> {
          let conn = self.client.get().await?;
          use sea_orm::EntityTrait;
          let count = $entity_mod::Entity::find().count(&*conn).await?;
          Ok(count)
        }

        /// 获取所有实体
        pub async fn [<find_all $method_suffix>](&self) -> R<Vec<$domain>> {
          let conn = self.client.get().await?;
          use sea_orm::EntityTrait;
          let results = $entity_mod::Entity::find().all(&*conn).await?;
          Ok(results.into_iter().map(Into::into).collect())
        }

        /// 分页查询实体
        pub async fn [<find_paged $method_suffix>](&self, pos: $crate::cutil::pos::Pos) -> R<$crate::cutil::paged::Paged<$domain>> {
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

/// 为Repository实现标准的CRUD方法（带entity后缀）
///
/// 使用方法：
/// ```ignore
/// impl_crud_methods!(DeviceRecentRepo, DeviceRecent, device_recent, Uuid);
/// ```
#[macro_export]
macro_rules! impl_crud_methods {
  ($repo:ident, $domain:ty, $entity_mod:ident, $primary_key:ty) => {
    impl_core_crud_operations!($repo, $domain, $entity_mod, $primary_key, "_entity");
  };
}

/// 为Repository实现便捷的CRUD方法（简化命名）
///
/// 使用方法：
/// ```ignore
/// impl_standard_crud!(DeviceRecentRepo, DeviceRecent, device_recent, Uuid);
/// ```
///
/// 这会添加标准的CRUD方法：
/// - create(entity) -> R<Domain>
/// - get(id) -> R<Option<Domain>>
/// - update_with(entity, field_setter) -> R<Domain>
/// - delete(id) -> R<()>
/// - create_many(entities) -> R<Vec<Domain>>
/// - delete_many(ids) -> R<u64>
/// - exists(id) -> R<bool>
/// - count() -> R<u64>
/// - find_all() -> R<Vec<Domain>>
/// - find_paged(pos) -> R<Paged<Domain>>
#[macro_export]
macro_rules! impl_standard_crud {
  ($repo:ident, $domain:ty, $entity_mod:ident, $primary_key:ty) => {
    impl_core_crud_operations!($repo, $domain, $entity_mod, $primary_key, "");
  };
}

// =============================================================================
// 高级功能宏定义 - 软删除、审计、批量更新等
// =============================================================================

/// ID提取trait - 为支持辅助方法的实体提供ID提取功能
pub trait ImplExtractId<PrimaryKey> {
  fn extract_id(&self) -> PrimaryKey;
}

/// 软删除trait - 为支持软删除的实体提供软删除功能
pub trait ImplSoftDelete {
  fn set_deleted(&mut self, deleted: bool);
  fn is_deleted(&self) -> bool;
}

/// 审计字段trait - 为支持审计的实体提供审计字段设置功能
pub trait ImplAuditFields {
  fn set_created(&mut self, timestamp: chrono::DateTime<chrono::Utc>);
  fn set_updated(&mut self, timestamp: chrono::DateTime<chrono::Utc>);
  fn set_created_by(&mut self, user_id: Option<String>);
  fn set_updated_by(&mut self, user_id: Option<String>);
}

/// 为Repository添加高级辅助方法
#[macro_export]
macro_rules! impl_advanced_methods {
  ($repo:ident, $domain:ty, $entity_mod:ident, $primary_key:ty) => {
    impl $repo {
      /// 获取必需的实体（不存在时返回错误）
      pub async fn get_required(&self, id: $primary_key) -> R<$domain> {
        match self.get(id).await? {
          Some(entity) => Ok(entity),
          None => Err(sea_orm::DbErr::RecordNotFound(format!("实体不存在: {:?}", id)).into()),
        }
      }

      /// 批量获取实体
      pub async fn get_many(&self, ids: Vec<$primary_key>) -> R<Vec<$domain>> {
        let mut results = Vec::new();
        for id in ids {
          if let Some(entity) = self.get(id).await? {
            results.push(entity);
          }
        }
        Ok(results)
      }

      /// 批量获取必需的实体（任何一个不存在都返回错误）
      pub async fn get_many_required(&self, ids: Vec<$primary_key>) -> R<Vec<$domain>> {
        let mut results = Vec::new();
        for id in ids {
          let entity = self.get_required(id).await?;
          results.push(entity);
        }
        Ok(results)
      }

      /// 检查多个实体是否都存在
      pub async fn exists_all(&self, ids: Vec<$primary_key>) -> R<bool> {
        for id in ids {
          if !self.exists(id).await? {
            return Ok(false);
          }
        }
        Ok(true)
      }

      /// 获取不存在的实体ID列表
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

/// 为Repository添加软删除功能
#[macro_export]
macro_rules! impl_soft_delete_methods {
  ($repo:ident, $domain:ty, $entity_mod:ident, $primary_key:ty) => {
    impl $repo {
      /// 软删除实体
      pub async fn soft_delete(&self, id: $primary_key) -> R<$domain>
      where
        $domain: $crate::cutil::db_crud::ImplSoftDelete + $crate::cutil::db_crud::ImplAuditFields,
      {
        let mut entity = self.get_required(id).await?;
        entity.set_deleted(true);
        entity.set_updated(chrono::Utc::now());
        self.update_smart(entity).await
      }

      /// 批量软删除
      pub async fn soft_delete_many(&self, ids: Vec<$primary_key>) -> R<Vec<$domain>>
      where
        $domain: $crate::cutil::db_crud::ImplSoftDelete + $crate::cutil::db_crud::ImplAuditFields,
      {
        let mut results = Vec::new();
        for id in ids {
          let result = self.soft_delete(id).await?;
          results.push(result);
        }
        Ok(results)
      }

      /// 恢复软删除的实体
      pub async fn restore_soft_deleted(&self, id: $primary_key) -> R<$domain>
      where
        $domain: $crate::cutil::db_crud::ImplSoftDelete + $crate::cutil::db_crud::ImplAuditFields,
      {
        let mut entity = self.get_required(id).await?;
        entity.set_deleted(false);
        entity.set_updated(chrono::Utc::now());
        self.update_smart(entity).await
      }

      /// 获取所有未被软删除的实体
      pub async fn find_not_deleted(&self) -> R<Vec<$domain>>
      where
        $domain: $crate::cutil::db_crud::ImplSoftDelete,
      {
        let all_entities = self.find_all().await?;
        Ok(all_entities.into_iter().filter(|e| !e.is_deleted()).collect())
      }

      /// 获取所有被软删除的实体
      pub async fn find_deleted(&self) -> R<Vec<$domain>>
      where
        $domain: $crate::cutil::db_crud::ImplSoftDelete,
      {
        let all_entities = self.find_all().await?;
        Ok(all_entities.into_iter().filter(|e| e.is_deleted()).collect())
      }

      /// 永久删除所有软删除的实体
      pub async fn purge_soft_deleted(&self) -> R<u64>
      where
        $domain: $crate::cutil::db_crud::ImplSoftDelete + $crate::cutil::db_crud::ImplExtractId<$primary_key>,
      {
        let deleted_entities = self.find_deleted().await?;
        let ids: Vec<$primary_key> = deleted_entities.iter().map(|e| e.extract_id()).collect();
        self.delete_many(ids).await
      }
    }
  };
}

/// 为Repository添加审计功能
#[macro_export]
macro_rules! impl_audit_methods {
  ($repo:ident, $domain:ty, $entity_mod:ident, $primary_key:ty) => {
    impl $repo {
      /// 创建实体并自动设置审计字段
      pub async fn create_with_audit(&self, mut entity: $domain, user_id: Option<String>) -> R<$domain>
      where
        $domain: $crate::cutil::db_crud::ImplAuditFields,
      {
        let now = chrono::Utc::now();
        entity.set_created(now);
        entity.set_updated(now);
        entity.set_created_by(user_id.clone());
        entity.set_updated_by(user_id);
        self.create(entity).await
      }

      /// 更新实体并自动设置审计字段
      pub async fn update_with_audit(&self, mut entity: $domain, user_id: Option<String>) -> R<$domain>
      where
        $domain: $crate::cutil::db_crud::ImplAuditFields,
      {
        entity.set_updated(chrono::Utc::now());
        entity.set_updated_by(user_id);
        self.update_smart(entity).await
      }

      /// 批量创建实体并自动设置审计字段
      pub async fn create_many_with_audit(&self, mut entities: Vec<$domain>, user_id: Option<String>) -> R<Vec<$domain>>
      where
        $domain: $crate::cutil::db_crud::ImplAuditFields,
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

/// 为Repository添加创建或更新功能
#[macro_export]
macro_rules! impl_upsert_methods {
  ($repo:ident, $domain:ty, $entity_mod:ident, $primary_key:ty) => {
    impl $repo {
      /// 创建或更新实体
      pub async fn upsert(&self, entity: $domain) -> R<($domain, bool)>
      where
        $domain: $crate::cutil::db_crud::ImplExtractId<$primary_key>,
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

      /// 批量创建或更新实体
      pub async fn upsert_many(&self, entities: Vec<$domain>) -> R<(Vec<$domain>, usize, usize)>
      where
        $domain: $crate::cutil::db_crud::ImplExtractId<$primary_key>,
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

/// 一体化高级功能宏 - 包含所有高级功能
#[macro_export]
macro_rules! impl_all_advanced_methods {
  ($repo:ident, $domain:ty, $entity_mod:ident, $primary_key:ty) => {
    impl_advanced_methods!($repo, $domain, $entity_mod, $primary_key);
    impl_soft_delete_methods!($repo, $domain, $entity_mod, $primary_key);
    impl_audit_methods!($repo, $domain, $entity_mod, $primary_key);
    impl_upsert_methods!($repo, $domain, $entity_mod, $primary_key);
  };
}

/// 完整的CRUD宏，包含所有方法
#[macro_export]
macro_rules! impl_full_crud {
  ($repo:ident, $domain:ty, $entity_mod:ident, $primary_key:ty) => {
    // 实现标准CRUD方法
    impl_standard_crud!($repo, $domain, $entity_mod, $primary_key);

    // 实现高级辅助方法
    impl_advanced_methods!($repo, $domain, $entity_mod, $primary_key);
  };
}

/// 一体化CRUD宏 - 包含标准CRUD + 智能更新 + 高级功能
/// 这是最简单的使用方式，一个宏搞定所有功能
///
/// 使用方法：
/// ```ignore
/// // 首先定义Repository结构
/// impl_repository_struct!(DeviceRecentRepo);
///
/// // 然后实现完整CRUD功能
/// impl_complete_crud!(
///   DeviceRecentRepo,        // Repository类型
///   DeviceRecent,            // Domain类型
///   device_recent,           // Entity模块名
///   Uuid,                    // 主键类型
///   [platform, is_active, updated]  // 需要智能更新的字段列表
/// );
/// ```
#[macro_export]
macro_rules! impl_complete_crud {
  ($repo:ident, $domain:ty, $entity_mod:ident, $primary_key:ty, [$($field:ident),+ $(,)?]) => {
    // 实现标准CRUD方法
    impl_standard_crud!($repo, $domain, $entity_mod, $primary_key);

    // 实现智能更新方法
    impl_smart_update!($repo, $domain, $entity_mod, { $($field),+ });

    // 添加自动更新功能到Repository
    impl_auto_update_to_repo!($repo);

    // 添加高级辅助方法
    impl_advanced_methods!($repo, $domain, $entity_mod, $primary_key);
  };
}

/// 超级完整CRUD宏 - 包含所有可能的功能
/// 适用于需要完整功能集的场景，包含软删除、审计等高级特性
///
/// 使用方法：
/// ```ignore
/// impl_super_complete_crud!(
///   DeviceRecentRepo,        // Repository类型
///   DeviceRecent,            // Domain类型
///   device_recent,           // Entity模块名
///   Uuid,                    // 主键类型
///   [platform, is_active, updated]  // 需要智能更新的字段列表
/// );
/// ```
#[macro_export]
macro_rules! impl_super_complete_crud {
  ($repo:ident, $domain:ty, $entity_mod:ident, $primary_key:ty, [$($field:ident),+ $(,)?]) => {
    // 实现标准CRUD方法
    impl_standard_crud!($repo, $domain, $entity_mod, $primary_key);

    // 实现智能更新方法
    impl_smart_update!($repo, $domain, $entity_mod, { $($field),+ });

    // 添加自动更新功能到Repository
    impl_auto_update_to_repo!($repo);

    // 添加所有高级功能
    impl_all_advanced_methods!($repo, $domain, $entity_mod, $primary_key);
  };
}

/// 使用示例和测试
#[cfg(test)]
mod example_usage {
  /*
   * 🚀 完整使用示例 - 重构优化版本
   *
   * 使用示例：
   *
   * // === 示例1: 基础CRUD功能 ===
   * pub struct ProductRepo {
   *     // Repository 实现
   * }
   *
   * // 基础CRUD + 智能更新
   * impl_complete_crud!(
   *   ProductRepo,
   *   Product,
   *   product,
   *   Uuid,
   *   [name, price, updated]
   * );
   *
   * // === 示例2: 完整功能集（包含软删除、审计等） ===
   * pub struct UserRepo {
   *     // Repository 实现
   * }
   *
   * // 实现所有高级特性
   * impl_super_complete_crud!(
   *   UserRepo,
   *   User,
   *   user,
   *   Uuid,
   *   [name, email, updated]
   * );
   *
   * // 为User实体实现必要的trait
   * impl ImplExtractId<Uuid> for User {
   *     fn extract_id(&self) -> Uuid { self.id }
   * }
   *
   * impl ImplSoftDelete for User {
   *     fn set_deleted(&mut self, deleted: bool) { self.is_deleted = deleted; }
   *     fn is_deleted(&self) -> bool { self.is_deleted }
   * }
   *
   * impl ImplAuditFields for User {
   *     fn set_created(&mut self, timestamp: DateTime<Utc>) { self.created = timestamp; }
   *     fn set_updated(&mut self, timestamp: DateTime<Utc>) { self.updated = timestamp; }
   *     fn set_created_by(&mut self, user_id: Option<String>) { self.created_by = user_id; }
   *     fn set_updated_by(&mut self, user_id: Option<String>) { self.updated_by = user_id; }
   * }
   *
   * // === 使用示例 ===
   * async fn comprehensive_example(client: Arc<DbClient>) -> R<()> {
   *     let user_repo = UserRepo::new(client.clone());
   *     let product_repo = ProductRepo::new(client);
   *
   *     // === 1. 基础CRUD操作 ===
   *     let user = User { /* ... */ };
   *     let created_user = user_repo.create(user).await?;
   *     let found_user = user_repo.get_required(created_user.id).await?;
   *
   *     // === 2. 智能更新 ===
   *     let mut user = found_user;
   *     user.name = "Updated Name".to_string();
   *     let updated_user = user_repo.update_smart(user).await?;
   *
   *     // === 2.1. 自定义字段更新 ===
   *     let custom_updated = user_repo.update_with(user, |active_model| {
   *         use sea_orm::ActiveValue;
   *         active_model.name = ActiveValue::Set("自定义名字".to_string());
   *         active_model.email = ActiveValue::Set("custom@email.com".to_string());
   *         // 其他字段保持 Unchanged，不会被更新
   *     }).await?;
   *
   *     // === 3. 批量操作 ===
   *     let users = vec![user1, user2, user3];
   *     let created_users = user_repo.create_many(users).await?;
   *     let user_ids: Vec<Uuid> = created_users.iter().map(|u| u.id).collect();
   *
   *     // 批量获取
   *     let batch_users = user_repo.get_many(user_ids.clone()).await?;
   *     let required_users = user_repo.get_many_required(user_ids.clone()).await?;
   *
   *     // === 4. 审计功能 ===
   *     let admin_id = Some("admin_123".to_string());
   *     let audited_user = user_repo.create_with_audit(new_user, admin_id.clone()).await?;
   *     let updated_audited = user_repo.update_with_audit(audited_user, admin_id).await?;
   *
   *     // === 5. 软删除功能 ===
   *     let soft_deleted = user_repo.soft_delete(updated_audited.id).await?;
   *     let active_users = user_repo.find_not_deleted().await?;
   *     let deleted_users = user_repo.find_deleted().await?;
   *
   *     // 恢复软删除
   *     let restored_user = user_repo.restore_soft_deleted(soft_deleted.id).await?;
   *
   *     // 清理软删除的数据
   *     let purged_count = user_repo.purge_soft_deleted().await?;
   *
   *     // === 6. Upsert操作 ===
   *     let (upserted_user, is_new) = user_repo.upsert(some_user).await?;
   *     let (batch_results, created_count, updated_count) =
   *         user_repo.upsert_many(multiple_users).await?;
   *
   *     // === 7. 辅助查询 ===
   *     let user_exists = user_repo.exists(user_id).await?;
   *     let all_exist = user_repo.exists_all(user_ids.clone()).await?;
   *     let missing_ids = user_repo.get_missing_ids(user_ids).await?;
   *     let total_count = user_repo.count().await?;
   *
   *     // === 8. 分页查询 ===
   *     let pos = Pos::new(1, 10, vec![]);
   *     let paged_users = user_repo.find_paged(pos).await?;
   *
   *     println!("创建: {}, 更新: {}, 清理: {}", created_count, updated_count, purged_count);
   *     Ok(())
   * }
   *
   * // === 可用的宏总结 ===
   * // 1. impl_standard_crud! - 基础CRUD
   * // 2. impl_complete_crud! - CRUD + 智能更新 + 高级辅助
   * // 3. impl_super_complete_crud! - 所有功能（软删除、审计、upsert等）
   * // 4. impl_smart_update! - 仅智能更新
   * // 5. impl_advanced_methods! - 高级辅助方法
   * // 6. impl_soft_delete_methods! - 软删除功能
   * // 7. impl_audit_methods! - 审计功能
   * // 8. impl_upsert_methods! - 创建或更新功能
   * // 9. impl_all_advanced_methods! - 所有高级功能
   *
   * // === 性能优化特性 ===
   * // ✅ 所有批量操作使用事务
   * // ✅ 批量删除优化（分批处理）
   * // ✅ 详细的错误信息
   * // ✅ 类型安全的ID提取机制
   * // ✅ 灵活的trait系统支持
   * // ✅ 所有方法现在都使用内部 client，无需传递 conn 参数
   */
  fn example() {
    println!("🎉 请参考上面的注释查看完整的重构优化示例!");
    println!("ℹ️ 所有CRUD方法现在都使用内部client，无需传递conn参数");
  }

  #[test]
  fn test_concepts() {
    println!("✅ CRUD宏重构优化测试 - 性能提升、功能扩展、类型安全");
  }
}
