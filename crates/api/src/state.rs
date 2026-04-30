use std::sync::Arc;

use common::{AppConfig, EventInput};
use tokio::sync::mpsc::Sender;

pub struct AppState {
    pub config: Arc<AppConfig>,
    pub tx: Sender<EventInput>,
}

impl AppState {
    pub fn new(config: Arc<AppConfig>, tx: Sender<EventInput>) -> Self {
        Self {
            config,
            tx
        }
    }
}
