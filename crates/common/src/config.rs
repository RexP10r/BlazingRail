use std::{collections::HashMap, net::IpAddr, path::PathBuf};

use clap::Parser;
use serde::Deserialize;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
pub struct AppConfig {
    #[arg(long, env = "SERVER_HOST", default_value = "127.0.0.1")]
    pub server_host: IpAddr,

    #[arg(long, env = "SERVER_PORT", default_value_t = 3000)]
    pub server_port: u16,

    #[arg(long, env = "CHANNEL_CAPACITY", default_value_t = 16384)]
    pub channel_capacity: usize,

    #[arg(long, env = "SOCKET_MAX_CONNECTIONS", default_value_t = 4096)]
    pub socket_max_connections: i32,

    #[arg(long, env = "SHUTDOWN_TIMEOUT_SECS", default_value_t = 4)]
    pub shutdown_timeout_secs: u64,
}

#[derive(Deserialize, Debug)]
pub struct KafkaRoutingConfig {
    pub default_topic: String,
    #[serde(default)]
    pub topic_mapping: HashMap<String, String>,
}

impl KafkaRoutingConfig {
    pub fn new(config_path: &PathBuf) -> Option<KafkaRoutingConfig> {
        let content = std::fs::read_to_string(config_path).ok()?;
        serde_yaml::from_str(&content).ok()
    }
}

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
pub struct PipelineConfig {
    #[arg(long, env = "BATCH_SIZE", default_value_t = 512)]
    pub batch_size: usize,

    #[arg(long, env = "BATCH_CAPACITY", default_value_t = 8192)]
    pub batch_capacity: usize,

    #[arg(long, env = "FLUSH_TIMEOUT_MS", default_value_t = 128)]
    pub flush_timeout_ms: u64,

    #[arg(long, env = "PRIME_PATH", default_value = "/dev/null")]
    pub prime_path: PathBuf,

    #[arg(long, env = "ENABLE_KAFKA", default_value_t = false)]
    pub enable_kafka: bool,

    #[arg(
        long,
        env = "KAFKA_ROUTING_CONF_PATH",
        default_value = "kafka_routing.yaml"
    )]
    pub kafka_routing_conf_path: PathBuf,

    #[arg(long, env = "KAFKA_BROKERS", default_value = "127.0.0.1:9092")]
    pub kafka_brokers: String,

    #[arg(long, env = "KAFKA_TIMEOUT_MS", default_value_t = 512)]
    pub kafka_timeout_ms: u64,

    #[arg(long, env = "KAFKA_TIMEOUT_SLACK_MS", default_value_t = 1024)]
    pub kafka_timeout_slack_ms: u64,

    #[arg(long, env = "KAFKA_COMPRESSION", default_value = "lz4")]
    pub kafka_compression: String,

    #[arg(long, env = "ENABLE_CIRCUIT_BREAKER", default_value_t = false)]
    pub enable_circuit_breaker: bool,

    #[arg(long, env = "CIRCUIT_BREAKER_THRESHOLD", default_value_t = 2)]
    pub circuit_breaker_threshold: usize,

    #[arg(long, env = "CIRCUIT_BREAKER_TIMEOUT", default_value_t = 2048)]
    pub circuit_breaker_timeout: u64,
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
