use futures::future::try_join_all;
use std::time::Duration;

use async_trait::async_trait;
use common::{EventInput, PipelineConfig};
use rdkafka::{
    ClientConfig,
    error::KafkaError,
    producer::{FutureProducer, FutureRecord},
};

use crate::{EventSink, SinkError};

pub struct KafkaSink {
    producer: FutureProducer,
    timeout: Duration,
}

impl KafkaSink {
    pub fn new(pipeline_config: &PipelineConfig) -> Result<Self, KafkaError> {
        let producer = ClientConfig::new()
            .set("bootstrap.servers", &pipeline_config.kafka_brokers)
            // --- Delivery ---
            .set(
                "message.timeout.ms",
                pipeline_config.kafka_timeout_ms.to_string(),
            )
            .set("request.required.acks", "1")
            .set("queue.buffering.max.ms", "0")
            // --- Compression ---
            .set("compression.type", &pipeline_config.kafka_compression)
            // --- Reliability ---
            .set("message.send.max.retries", "3")
            .set("retry.backoff.ms", "100")
            // --- Socket ---
            .set("socket.keepalive.enable", "true")
            .set("socket.nagle.disable", "true") 

            .create()?;
        tracing::info!("Kafka sink initializated");
        Ok(Self {
            producer: producer,
            timeout: Duration::from_millis(pipeline_config.kafka_timeout_ms + 1024),
        })
    }
}

#[async_trait]
impl EventSink for KafkaSink {
    async fn send_batch(&self, batch: Vec<EventInput>) -> Result<(), SinkError> {
        let futures: Vec<_> = batch
            .into_iter()
            .map(|input| {
                let producer = self.producer.clone();
                async move {
                    let payload = input.payload.get().as_bytes();
                    let topic = input.event_type.clone();
                    let key = input.event_type.into_bytes();
                    let record = FutureRecord::to(&topic).payload(payload).key(&key);
                    producer
                        .send(record, Duration::ZERO)
                        .await
                        .map_err(|(e, _)| SinkError::KafkaPublish(e.to_string()))
                }
            })
            .collect();

        tokio::time::timeout(self.timeout, try_join_all(futures))
            .await
            .map_err(|_| SinkError::Timeout)?
            .map(|_| ())
    }
}
