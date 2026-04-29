use clap::Parser;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
pub struct AppConfig {

    #[arg(long, env = "SERVER_PORT", default_value_t = 3000)]
    pub server_port: u16,

    #[arg(long, env = "CHANNEL_CAPACITY", default_value_t = 4096)]
    pub channel_capacity: usize,
}

impl AppConfig {
    pub fn new() -> Self {
        AppConfig::parse()
    }
}
