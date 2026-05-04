#![allow(dead_code)]
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
    topic: String,
    timeout: Duration,
}

impl KafkaSink {
    pub fn new(pipeline_config: &PipelineConfig) -> Result<Self, KafkaError> {
        let producer = ClientConfig::new()
            .set("bootstrap.servers", pipeline_config.kafka_brokers.clone())
            .set(
                "compression.type",
                pipeline_config.kafka_compression.clone(),
            )
            .set("batch.num.messages", pipeline_config.batch_size.to_string())
            .set(
                "queue.buffering.max.ms",
                pipeline_config.flush_timeout_ms.to_string(),
            )
            .set(
                "message.timeout.ms",
                pipeline_config.kafka_timeout_ms.to_string(),
            )
            .create()?;

        Ok(Self {
            producer: producer,
            topic: pipeline_config.kafka_topic.clone(),
            timeout: Duration::from_millis(pipeline_config.kafka_timeout_ms),
        })
    }
}

#[async_trait]
impl EventSink for KafkaSink {
    async fn send_batch(&self, batch: Vec<EventInput>) -> Result<(), SinkError> {
        let result = tokio::time::timeout(self.timeout, async {
            for input in batch {
                let field_key = input.event_type.as_bytes();
                let payload = serde_json::to_vec(&input.payload)?;
                let record = FutureRecord::to(&self.topic)
                    .payload(&payload)
                    .key(field_key);
                let delivery_status = self.producer.send(record, Duration::from_millis(128)).await;

                match delivery_status {
                    Err((e, _)) => tracing::warn!("Kafka delivering failed: {}", e),
                    _ => continue,
                };
            }
            Ok::<_, SinkError>(())
        })
        .await;
        match result {
            Ok(Ok(())) => Ok(()),
            Ok(Err(e)) => Err(e),
            Err(_) => Err(SinkError::Timeout),
        }
    }
}
