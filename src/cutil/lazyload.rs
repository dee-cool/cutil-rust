use crate::cutil::meta::R;
use std::{future::Future, pin::Pin, sync::Arc};
use tokio::sync::OnceCell;

type AsyncLoader<T> = Arc<dyn Fn() -> Pin<Box<dyn Future<Output = R<T>> + Send>> + Send + Sync>;

/// 异步懒加载器，用于延迟加载 Account 信息
pub struct Lazyload<T> {
  data: OnceCell<T>,
  loader: AsyncLoader<T>,
}

impl<T> std::fmt::Debug for Lazyload<T> {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    f.debug_struct("Lazyload").field("loaded", &self.loaded()).finish()
  }
}

impl<T> Lazyload<T> {
  /// 创建新的异步懒加载器
  pub fn new<F, Fut>(loader: F) -> Self
  where
    F: Fn() -> Fut + Send + Sync + 'static,
    Fut: std::future::Future<Output = R<T>> + Send + 'static,
  {
    Self {
      data: OnceCell::new(),
      loader: Arc::new(move || Box::pin(loader())),
    }
  }

  /// 获取数据，如果尚未加载则异步加载
  pub async fn get(&self) -> R<&T> {
    self.data.get_or_try_init(|| (self.loader)()).await
  }

  /// 检查数据是否已加载
  pub fn loaded(&self) -> bool {
    self.data.get().is_some()
  }
}
