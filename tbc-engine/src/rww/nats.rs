use super::{memory::MemoryStore, RwwMessage};
use async_nats::jetstream::consumer::{pull, AckPolicy};
use async_nats::jetstream::stream::{Config as StreamConfig, RetentionPolicy};
use futures_util::StreamExt;
use std::sync::mpsc::{self, RecvTimeoutError};
use std::sync::Arc;
use std::time::Duration;

struct Outbound {
  msg: RwwMessage,
  durable: bool,
}

pub struct NatsBridge {
  outbound: mpsc::Sender<Outbound>,
}

impl Clone for NatsBridge {
  fn clone(&self) -> Self {
    Self {
      outbound: self.outbound.clone(),
    }
  }
}

impl NatsBridge {
  pub fn connect(url: &str, stream_name: &str, store: Arc<MemoryStore>) -> Result<Self, String> {
    let (out_tx, out_rx) = mpsc::channel();
    let url = url.to_string();
    let stream_name = stream_name.to_string();

    std::thread::Builder::new()
      .name("tbc-rww-nats".into())
      .spawn(move || {
        let rt = tokio::runtime::Builder::new_multi_thread()
          .enable_all()
          .thread_name("tbc-rww-nats-async")
          .build()
          .expect("nats runtime");

        rt.block_on(async move {
          if let Err(e) = run_bridge(url, stream_name, store, out_rx).await {
            tracing::error!("NATS RWW bridge stopped: {}", e);
          }
        });
      })
      .map_err(|e| e.to_string())?;

    Ok(Self { outbound: out_tx })
  }

  pub fn publish(&self, msg: RwwMessage, durable: bool) {
    let _ = self.outbound.send(Outbound { msg, durable });
  }
}

async fn run_bridge(
  url: String,
  stream_name: String,
  store: Arc<MemoryStore>,
  out_rx: mpsc::Receiver<Outbound>,
) -> Result<(), String> {
  let client = async_nats::connect(url.clone())
    .await
    .map_err(|e| format!("connect {}: {}", url, e))?;

  let jetstream = async_nats::jetstream::new(client.clone());

  jetstream
    .get_or_create_stream(StreamConfig {
      name: stream_name.clone(),
      subjects: vec!["rww.>".into()],
      retention: RetentionPolicy::Limits,
      max_messages: 10_000,
      ..Default::default()
    })
    .await
    .map_err(|e| format!("stream: {}", e))?;

  tracing::info!(
    "RWW NATS connected to {} stream={}",
    url,
    stream_name
  );

  let consumer_name = format!("tbc-rww-{}", ulid::Ulid::new());
  let consumer = jetstream
    .create_consumer_on_stream(
      pull::Config {
        name: Some(consumer_name),
        filter_subject: "rww.>".into(),
        ack_policy: AckPolicy::Explicit,
        ..Default::default()
      },
      stream_name.clone(),
    )
    .await
    .map_err(|e| format!("consumer: {}", e))?;

  let store_in = store.clone();
  tokio::spawn(async move {
    loop {
      match consumer.fetch().max_messages(32).messages().await {
        Ok(mut batch) => {
          while let Some(msg) = batch.next().await {
            match msg {
              Ok(msg) => {
                if let Ok(rww) = serde_json::from_slice::<RwwMessage>(&msg.payload) {
                  store_in.insert(rww, true);
                }
                msg.ack().await.ok();
              }
              Err(e) => tracing::warn!("RWW consumer error: {}", e),
            }
          }
        }
        Err(e) => {
          tracing::warn!("RWW fetch error: {}", e);
          tokio::time::sleep(Duration::from_millis(500)).await;
        }
      }
    }
  });

  loop {
    match out_rx.recv_timeout(Duration::from_millis(200)) {
      Ok(job) => {
        let wire = serde_json::to_vec(&job.msg).map_err(|e| e.to_string())?;
        if job.durable {
          jetstream
            .publish(job.msg.subject.clone(), wire.into())
            .await
            .map_err(|e| format!("jetstream publish: {}", e))?
            .await
            .map_err(|e| format!("jetstream ack: {}", e))?;
        } else {
          client
            .publish(job.msg.subject.clone(), wire.into())
            .await
            .map_err(|e| format!("core publish: {}", e))?;
        }
      }
      Err(RecvTimeoutError::Timeout) => {}
      Err(RecvTimeoutError::Disconnected) => break,
    }
  }

  Ok(())
}
