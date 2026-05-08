use common::EventInput;
use tokio::sync::{mpsc::Sender, watch::Receiver};

pub struct AppState {
    pub tx: Sender<EventInput>,
    pub shutdown_rx: Receiver<bool>,
}

impl AppState {
    pub fn new(tx: Sender<EventInput>, shutdown_rx: Receiver<bool>) -> Self {
        Self {
            tx,
            shutdown_rx: shutdown_rx,
        }
    }
}
