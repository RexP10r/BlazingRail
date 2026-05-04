#![allow(dead_code)]
use rdkafka::producer::FutureProducer;

pub struct KafkaSink {
    producer: FutureProducer,
    topic: String,
    key_field: Option<String>,
}
