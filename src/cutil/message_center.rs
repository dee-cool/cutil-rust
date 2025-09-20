use std::sync::Arc;

use async_trait::async_trait;
use axum::http::{HeaderMap, HeaderValue};
use chrono::Local;
use serde::{Deserialize, Serialize};
use serde_json::{Value, from_str, to_string};
use tracing::info;
use utoipa::ToSchema;

use crate::cutil::message_broker::{MessageBroker, MessageBrokerImpl, MessageBrokerOptions, Qos};
use crate::cutil::meta::{Meta, R};
use crate::cutil::{message_broker, message_center};
use crate::meta;

#[async_trait]
pub trait MessageCenter: Send + Sync {
  async fn subscribe(&self, topics: Vec<String>, qos: Qos) -> R<()>;

  async fn unsubscribe(&self, topics: Vec<String>) -> R<()>;

  async fn listen(&self, handler: Arc<dyn Fn(Message) -> R<()> + Send + Sync>) -> R<()>;

  async fn shutdown(&self) -> R<()>;

  async fn publish(&self, qos: Qos, retain: bool, message: Message) -> R<()>;

  async fn publish_delay(&self, qos: Qos, retain: bool, message: Message) -> R<()>;
}

#[derive(Debug, Serialize, Deserialize, Clone, ToSchema)]
pub struct Message {
  pub id: String,
  pub name: String,
  pub created: i64,
  pub arrival: i64,
  pub body: Value,
}

impl Default for Message {
  fn default() -> Self {
    Self {
      id: "".to_string(),
      name: "".to_string(),
      created: Local::now().timestamp(),
      arrival: Local::now().timestamp(),
      body: Default::default(),
    }
  }
}

#[derive(Debug, Serialize, Deserialize, Clone, ToSchema)]
pub struct MessageCenterOptions {
  pub publish_url: String,
  pub publish_token: String,
}

pub struct MessageCenterImpl {
  broker: Arc<dyn MessageBroker>,
  options: MessageCenterOptions,
}

impl MessageCenterImpl {
  pub fn new(options: MessageCenterOptions, message_broker_options: MessageBrokerOptions) -> R<Self> {
    let broker = MessageBrokerImpl::new(message_broker_options)?;
    Ok(Self {
      broker: Arc::new(broker),
      options,
    })
  }
}

#[async_trait]
impl MessageCenter for MessageCenterImpl {
  async fn subscribe(&self, topics: Vec<String>, qos: Qos) -> R<()> {
    self.broker.subscribe(topics, qos).await
  }

  async fn unsubscribe(&self, topics: Vec<String>) -> R<()> {
    self.broker.unsubscribe(topics).await
  }

  async fn listen(&self, handler: Arc<dyn Fn(message_center::Message) -> R<()> + Send + Sync>) -> R<()> {
    let wrapped_handler = Arc::new(move |message_b: message_broker::Message| -> R<()> {
      if let Ok(mut message) = from_str::<message_center::Message>(&message_b.body) {
        handler(message)?;
      }
      Ok(())
    });

    self.broker.listen(wrapped_handler).await
  }

  async fn shutdown(&self) -> R<()> {
    self.broker.shutdown().await
  }

  async fn publish(&self, qos: Qos, retain: bool, message: message_center::Message) -> R<()> {
    let message_b = message_broker::Message {
      name: message.name.clone(),
      qos,
      retain,
      body: to_string(&message)?,
    };
    self.broker.publish(message_b).await
  }

  async fn publish_delay(&self, qos: Qos, retain: bool, message: message_center::Message) -> R<()> {
    let mut headers = HeaderMap::new();
    headers.insert(
      "Authorization",
      format!("token {}", self.options.publish_token.clone()).parse::<HeaderValue>()?,
    );

    let message_json = to_string(&message)?;
    info!("publish message: {}", message_json);

    let url = format!("{}?qos={:?}&retain={}", self.options.publish_url.clone(), qos, retain);
    let client = reqwest::Client::new();
    let res = client.post(url).headers(headers).json(&message).send().await?;
    if !res.status().is_success() {
      return meta!("publish_failed");
    }
    Ok(())
  }
}
