use futures::future::join_all;
use std::{collections::HashMap, time::Duration};

use async_trait::async_trait;
use common::{EventInput, KafkaRoutingConfig, PipelineConfig};
use rdkafka::{
    ClientConfig,
    error::KafkaError,
    producer::{FutureProducer, FutureRecord, future_producer::Delivery},
};

use crate::{EventSink, SinkError};

pub struct KafkaSink {
    producer: FutureProducer,
    timeout: Duration,
    routing: HashMap<String, String>,
    default_topic: String,
}

impl KafkaSink {
    pub fn new(
        pipeline_config: &PipelineConfig,
        routing_config: &KafkaRoutingConfig,
    ) -> Result<Self, KafkaError> {
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
        tracing::info!("Kafka sink initialized");
        Ok(Self {
            producer: producer,
            timeout: Duration::from_millis(
                pipeline_config.kafka_timeout_ms + pipeline_config.kafka_timeout_slack_ms,
            ),
            routing: routing_config.topic_mapping.clone(),
            default_topic: routing_config.default_topic.clone(),
        })
    }
    pub async fn write_event(
        &self,
        input: EventInput,
        producer: FutureProducer,
    ) -> Result<Delivery, SinkError> {
        let payload = input.payload.get().as_bytes();
        let topic = self
            .routing
            .get(&input.event_type)
            .unwrap_or(&self.default_topic);
        let key = input.event_type.into_bytes();
        let record = FutureRecord::to(&topic).payload(payload).key(&key);
        producer
            .send(record, Duration::ZERO)
            .await
            .map_err(|(e, _)| SinkError::KafkaPublish(e.to_string()))
    }
}

#[async_trait]
impl EventSink for KafkaSink {
    async fn send_batch(&self, batch: Vec<EventInput>) -> Result<(), SinkError> {
        let futures: Vec<_> = batch
            .into_iter()
            .map(|input| {
                let producer = self.producer.clone();
                async move { Self::write_event(&self, input, producer).await }
            })
            .collect();

        let results = tokio::time::timeout(self.timeout, join_all(futures))
            .await
            .map_err(|_| {
                tracing::error!("Kafka batch send timeout");
                SinkError::Timeout
            })?;
        let errors: Vec<_> = results
            .into_iter()
            .enumerate()
            .filter_map(|(idx, res)| res.err().map(|e| (idx, e)))
            .collect();
        if !errors.is_empty() {
            for (idx, err) in &errors {
                tracing::warn!(event_idx=idx, error=%err, "Kafka publish failed");
            }
            return Err(errors.into_iter().next().unwrap().1);
        }
        Ok(())
    }
}
