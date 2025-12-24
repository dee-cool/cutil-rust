use crate::cutil::meta::R;
use async_trait::async_trait;
use idgenerator_thin::{IdGeneratorOptions, YitIdHelper};
use once_cell::sync::Lazy;
use rand::distr::Alphanumeric;
use rand::Rng;
use ulid::Ulid;
use uuid::Uuid;

#[async_trait]
pub trait Generator: Send + Sync {
  async fn next_id(&self) -> R<i64>;
}

pub struct GeneratorImpl {}

impl GeneratorImpl {
  pub fn new() -> Self {
    Self {}
  }
}

impl GeneratorImpl {
  fn configure(&self) {
    let mut options = IdGeneratorOptions::new(1);
    options.worker_id_bit_length = 1;
    options.base_time = 1710034500000;
    YitIdHelper::set_id_generator(options);
  }
}

#[async_trait]
impl Generator for GeneratorImpl {
  async fn next_id(&self) -> R<i64> {
    Ok(YitIdHelper::next_id())
  }
}

static SHARED: Lazy<GeneratorImpl> = Lazy::new(|| {
  let generator = GeneratorImpl::new();
  generator.configure();
  generator
});

pub async fn gen_id() -> R<i64> {
  SHARED.next_id().await
}

pub fn gen_string(length: usize) -> String {
  let rand_string: String = rand::rng().sample_iter(&Alphanumeric).take(length).map(char::from).collect();
  rand_string
}

pub fn gen_ulid() -> String {
  Ulid::new().to_string().to_lowercase()
}

pub fn gen_uuid() -> Uuid {
  let ulid = Ulid::new();
  ulid.into()
}
