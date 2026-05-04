#![allow(dead_code)]
use std::time::Duration;

use common::{EventInput, PipelineConfig};
use rdkafka::{ClientConfig, error::KafkaError, producer::FutureProducer};

use crate::{EventSink, SinkError};

pub struct KafkaSink {
    producer: FutureProducer,
    topic: String,
    timeout: Duration,
    key_field: String,
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
            key_field: pipeline_config.kafka_key_field.clone(),
        })
    }
}

impl EventSink for KafkaSink {
    async fn send_batch(&self, _batch: Vec<EventInput>) -> Result<(), SinkError> {
        todo!()
    }
}
