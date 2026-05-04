use std::{net::IpAddr, path::PathBuf};

use clap::Parser;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
pub struct AppConfig {
    #[arg(long, env = "SERVER_HOST", default_value = "127.0.0.1")]
    pub server_host: IpAddr,

    #[arg(long, env = "SERVER_PORT", default_value_t = 3000)]
    pub server_port: u16,

    #[arg(long, env = "CHANNEL_CAPACITY", default_value_t = 4096)]
    pub channel_capacity: usize,
}

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
pub struct PipelineConfig {
    #[arg(long, env = "BATCH_SIZE", default_value_t = 256)]
    pub batch_size: usize,

    #[arg(long, env = "BATCH_CAPACITY", default_value_t = 4096)]
    pub batch_capacity: usize,

    #[arg(long, env = "FLUSH_TIMEOUT_MS", default_value_t = 64)]
    pub flush_timeout_ms: u64,

    #[arg(long, env = "PRIME_PATH", default_value = "/dev/null")]
    pub prime_path: PathBuf,
    
    #[arg(long, env = "ENABLE_KAFKA", default_value_t = false)]
    pub enable_kafka: bool,

    #[arg(long, env = "KAFKA_BROKERS", default_value = "localhost:9092")]
    pub kafka_brokers: String,

    #[arg(long, env = "KAFKA_TIMEOUT", default_value_t = 4096)]
    pub kafka_timeout_ms: u64,

    #[arg(long, env = "KAFKA_TOPIC", default_value = "blazingrail-events")]
    pub kafka_topic: String,

    #[arg(long, env = "KAFKA_KEY_FIELD", default_value = "")]
    pub kafka_key_field: String,  // Empty string → None
    
    #[arg(long, env = "KAFKA_COMPRESSION", default_value = "lz4")]
    pub kafka_compression: String,
}

#[derive(Parser, Debug)]
pub struct Config {
    #[command(flatten)]
    pub app: AppConfig,

    #[command(flatten)]
    pub pipeline: PipelineConfig,
}

impl Config {
    pub fn new() -> Self {
        Self::parse()
    }
}
