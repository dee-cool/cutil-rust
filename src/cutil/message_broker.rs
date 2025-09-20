use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use rumqttc::Outgoing;
use rumqttc::Transport;
use rumqttc::v5::mqttbytes::QoS;
use rumqttc::v5::mqttbytes::v5::Packet;
use rumqttc::v5::{AsyncClient, Event, EventLoop, MqttOptions};
use serde::{Deserialize, Serialize};
use tokio::sync::{Mutex, watch};
use tokio::time::sleep;
use tracing::{error, info};
use utoipa::ToSchema;

use crate::cutil::generator::gen_string;
use crate::cutil::meta::R;

#[async_trait]
pub trait MessageBroker: Send + Sync {
  async fn subscribe(&self, names: Vec<String>, qos: Qos) -> R<()>;

  async fn unsubscribe(&self, names: Vec<String>) -> R<()>;

  async fn listen(&self, handler: Arc<dyn Fn(Message) -> R<()> + Send + Sync>) -> R<()>;

  async fn publish(&self, message: Message) -> R<()>;

  async fn shutdown(&self) -> R<()>;
}

#[derive(Debug, Serialize, Deserialize, Clone, ToSchema)]
pub struct Message {
  pub name: String,
  pub body: String,
  pub qos: Qos,
  pub retain: bool,
}

impl Default for Message {
  fn default() -> Self {
    Self {
      name: "".to_string(),
      body: "".to_string(),
      qos: Qos::AtMostOnce,
      retain: false,
    }
  }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Serialize, Deserialize, ToSchema)]
pub enum Qos {
  AtMostOnce = 0,
  AtLeastOnce = 1,
  ExactlyOnce = 2,
}

#[derive(Clone, Debug)]
pub struct MessageBrokerOptions {
  pub host: String,
  pub port: u16,
  pub username: String,
  pub password: String,
  pub client_id: String,
  pub keep_alive: u64,
  pub max_reconnect_delay: u64,
}

impl Default for MessageBrokerOptions {
  fn default() -> Self {
    Self {
      host: String::default(),
      port: 1883,
      username: String::default(),
      password: String::default(),
      client_id: gen_string(16),
      keep_alive: 60,
      max_reconnect_delay: 300,
    }
  }
}

pub struct MessageBrokerImpl {
  client: Arc<Mutex<AsyncClient>>,
  eventloop: Arc<Mutex<EventLoop>>,
  shutdown: watch::Receiver<bool>,
  shutdown_tx: watch::Sender<bool>,
  options: MessageBrokerOptions,
}

impl MessageBrokerImpl {
  pub fn new(options: MessageBrokerOptions) -> R<MessageBrokerImpl> {
    let (client, eventloop) = Self::create_mqtt_client(&options)?;
    let (shutdown_tx, shutdown) = watch::channel(false);

    Ok(MessageBrokerImpl {
      client: Arc::new(Mutex::new(client)),
      eventloop: Arc::new(Mutex::new(eventloop)),
      shutdown,
      shutdown_tx,
      options,
    })
  }

  fn create_mqtt_client(options: &MessageBrokerOptions) -> R<(AsyncClient, EventLoop)> {
    let mut mqttoptions = MqttOptions::new(&options.client_id, &options.host, options.port);
    mqttoptions.set_credentials(&options.username, &options.password);
    mqttoptions.set_keep_alive(Duration::from_secs(options.keep_alive));
    mqttoptions.set_clean_start(true);
    mqttoptions.set_transport(Transport::Tcp);

    Ok(AsyncClient::new(mqttoptions, 10))
  }

  async fn handle_reconnect(&self, reconnect_delay: u64) -> R<()> {
    error!("Attempting to reconnect in {} seconds...", reconnect_delay);
    sleep(Duration::from_secs(reconnect_delay)).await;

    let (new_client, new_eventloop) = Self::create_mqtt_client(&self.options).unwrap();
    *self.client.lock().await = new_client;
    *self.eventloop.lock().await = new_eventloop;
    Ok(())
  }
}

#[async_trait]
impl MessageBroker for MessageBrokerImpl {
  async fn subscribe(&self, names: Vec<String>, qos: Qos) -> R<()> {
    for name in &names {
      self
        .client
        .lock()
        .await
        .subscribe(
          name,
          match qos {
            Qos::AtMostOnce => QoS::AtMostOnce,
            Qos::AtLeastOnce => QoS::AtLeastOnce,
            Qos::ExactlyOnce => QoS::ExactlyOnce,
          },
        )
        .await?;
    }
    Ok(())
  }

  async fn unsubscribe(&self, names: Vec<String>) -> R<()> {
    for name in &names {
      self.client.lock().await.unsubscribe(name).await?;
    }
    Ok(())
  }

  async fn listen(&self, handler: Arc<dyn Fn(Message) -> R<()> + Send + Sync>) -> R<()> {
    let mut reconnect_delay = 1;
    let mut shutdown_rx = self.shutdown.clone();

    loop {
      let mut eventloop = self.eventloop.lock().await;

      tokio::select! {
        Ok(event) = eventloop.poll() => match event {
          Event::Incoming(Packet::Publish(msg)) => {
            let message = Message {
              name: String::from_utf8(msg.topic.to_vec()).unwrap_or("".to_string()),
              qos: match msg.qos {
                QoS::AtMostOnce => Qos::AtMostOnce,
                QoS::AtLeastOnce => Qos::AtLeastOnce,
                QoS::ExactlyOnce => Qos::ExactlyOnce,
              },
              retain: msg.retain,
              body: String::from_utf8(msg.payload.to_vec()).unwrap_or("".to_string()),
            };
            if let Err(e) = handler(message) {
              error!("Handler error: {}", e);
            }
            reconnect_delay = 1;
          }
          Event::Outgoing(Outgoing::Disconnect) => {
            drop(eventloop);
            self.handle_reconnect(reconnect_delay).await?;
            reconnect_delay = (reconnect_delay * 2).min(self.options.max_reconnect_delay);
          }
          _ => {}
        },
        _ = shutdown_rx.changed() => break,
      }
    }

    info!("MQTT listener shutdown complete");
    Ok(())
  }

  async fn shutdown(&self) -> R<()> {
    self.shutdown_tx.send(true)?;
    tokio::time::sleep(Duration::from_millis(100)).await;
    self.client.lock().await.disconnect().await?;
    Ok(())
  }

  async fn publish(&self, message: Message) -> R<()> {
    self
      .client
      .lock()
      .await
      .publish(
        message.name,
        match message.qos {
          Qos::AtMostOnce => QoS::AtMostOnce,
          Qos::AtLeastOnce => QoS::AtLeastOnce,
          Qos::ExactlyOnce => QoS::ExactlyOnce,
        },
        message.retain,
        message.body,
      )
      .await?;
    Ok(())
  }
}
