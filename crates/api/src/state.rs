use common::EventInput;
use tokio::sync::mpsc::Sender;

pub struct AppState {
    pub tx: Sender<EventInput>,
}

impl AppState {
    pub fn new(tx: Sender<EventInput>) -> Self {
        Self { tx }
    }
}
