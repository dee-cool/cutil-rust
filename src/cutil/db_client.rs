use std::sync::Arc;
use std::time::{Duration, Instant};

use crate::cutil::meta::{Meta, R};
use crate::meta;
use di::injectable;
use sea_orm::{ConnectOptions, Database, DatabaseConnection};
use tokio::sync::{OnceCell as AsyncOnceCell, RwLock};
use tracing::{error, info, warn};

#[injectable]
pub struct DbClient {
  // 使用 RwLock 支持连接重置，比 OnceCell 更灵活
  connection: RwLock<Option<Arc<DatabaseConnection>>>,
  options: AsyncOnceCell<ConnectOptions>,
  // 最后一次健康检查的时间
  last_health_check: RwLock<Option<Instant>>,
  // 健康检查间隔 (默认30秒)
  health_check_interval: Duration,
}

impl Default for DbClient {
  fn default() -> Self {
    Self {
      connection: RwLock::new(None),
      options: AsyncOnceCell::new(),
      last_health_check: RwLock::new(None),
      // 默认30秒检查一次，平衡性能和可靠性
      health_check_interval: Duration::from_secs(30),
    }
  }
}

impl DbClient {
  /// 使用提供的 ConnectOptions 创建 DbClient 实例
  pub fn with_options(options: ConnectOptions) -> Self {
    let client = Self::default();
    // 忽略错误，因为这是新创建的实例
    let _ = client.options.set(options);
    client
  }

  /// 使用自定义健康检查间隔创建 DbClient 实例
  pub fn with_health_check_interval(options: ConnectOptions, interval: Duration) -> Self {
    let mut client = Self::default();
    client.health_check_interval = interval;
    let _ = client.options.set(options);
    client
  }

  /// 创建新的数据库连接
  async fn create_connection(&self) -> R<Arc<DatabaseConnection>> {
    let options = self.options.get().unwrap();

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

  /// 检查是否需要进行健康检查
  fn should_health_check(&self, last_check: Option<Instant>) -> bool {
    match last_check {
      Some(last) => last.elapsed() >= self.health_check_interval,
      None => true, // 从未检查过，需要检查
    }
  }

  /// 获取或创建数据库连接 (优化版本)
  async fn ensure_connection(&self) -> R<Arc<DatabaseConnection>> {
    // 首先尝试读锁检查现有连接
    {
      let conn_guard = self.connection.read().await;
      if let Some(conn) = conn_guard.as_ref() {
        // 检查是否需要健康检查
        let last_check = *self.last_health_check.read().await;

        if !self.should_health_check(last_check) {
          // 距离上次检查时间不长，直接返回连接
          return Ok(conn.clone());
        }

        // 需要健康检查
        match conn.ping().await {
          Ok(_) => {
            // 更新最后检查时间
            *self.last_health_check.write().await = Some(Instant::now());
            return Ok(conn.clone());
          }
          Err(_) => {
            // 连接无效，需要重新创建
            warn!("Existing connection is invalid, will recreate");
          }
        }
      }
    }

    // 需要创建新连接，获取写锁
    let mut conn_guard = self.connection.write().await;

    // 双重检查，可能在等待写锁期间其他线程已经更新了连接
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
          // 连接无效，继续创建新连接
        }
      }
    }

    // 创建新连接
    info!("Creating new database connection");
    let new_connection = self.create_connection().await?;
    *conn_guard = Some(new_connection.clone());

    // 更新健康检查时间
    *self.last_health_check.write().await = Some(Instant::now());

    Ok(new_connection)
  }

  /// 获取数据库连接的公共方法（保持向后兼容）
  pub async fn get(&self) -> R<Arc<DatabaseConnection>> {
    self.ensure_connection().await
  }

  /// 测试数据库连接是否正常
  pub async fn ping(&self) -> R<()> {
    let conn = self.get().await?;
    conn.ping().await.map_err(|e| {
      error!("Database ping failed: {}", e);
      meta!("ping_failed", "Database ping failed")
    })
  }

  /// 强制重置连接
  pub async fn reset_connection(&self) -> R<()> {
    info!("Resetting database connection");

    let mut conn_guard = self.connection.write().await;

    // 关闭现有连接
    if let Some(conn) = conn_guard.take() {
      if let Ok(conn) = Arc::try_unwrap(conn) {
        if let Err(e) = conn.close().await {
          warn!("Failed to close existing connection during reset: {}", e);
        }
      }
    }

    // 重置健康检查时间
    *self.last_health_check.write().await = None;

    info!("Database connection reset completed");
    Ok(())
  }

  /// 检查连接是否已建立
  pub async fn is_connected(&self) -> bool {
    let conn_guard = self.connection.read().await;
    conn_guard.is_some()
  }

  /// 检查选项是否已配置
  pub fn is_configured(&self) -> bool {
    self.options.get().is_some()
  }

  /// 关闭数据库连接
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
