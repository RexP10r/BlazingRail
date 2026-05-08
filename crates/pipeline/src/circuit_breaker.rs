use std::{
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
    time::Duration,
};

use crate::{EventSink, SinkError};
use async_trait::async_trait;
use common::{EventInput, PipelineConfig};

pub struct CircuitBreaker {
    primary_sink: Arc<dyn EventSink>,
    fallback_sink: Arc<dyn EventSink>,
    timeout: Duration,
    open_time: AtomicUsize,
    threshold: usize,
    error_count: AtomicUsize,
}

use std::time::{SystemTime, UNIX_EPOCH};

impl CircuitBreaker {
    pub fn new(
        primary_sink: Arc<dyn EventSink>,
        fallback_sink: Arc<dyn EventSink>,
        pipeline_config: &PipelineConfig,
    ) -> Self {
        tracing::info!("Circuit breaker initialized");
        Self {
            primary_sink,
            fallback_sink,
            timeout: Duration::from_millis(pipeline_config.circuit_breaker_timeout),
            open_time: AtomicUsize::new(0),
            threshold: pipeline_config.circuit_breaker_threshold,
            error_count: AtomicUsize::new(0),
        }
    }
    fn now_ms(&self) -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64
    }
    fn record_failure(&self) {
        let prev = self.error_count.fetch_add(1, Ordering::AcqRel);
        tracing::warn!(
            current_count = prev + 1,
            threshold = self.threshold,
            "circuit breaker failure recorded"
        );
        if prev + 1 >= self.threshold {
            self.open_circuit();
        }
    }

    fn is_open(&self) -> bool {
        let resume_at = self.open_time.load(Ordering::Acquire) as u64;
        self.now_ms() < resume_at
    }

    fn open_circuit(&self) {
        let resume_at = self.now_ms() + self.timeout.as_millis() as u64;
        assert!(resume_at <= usize::MAX as u64, "timestamp overflow");
        self.open_time.store(resume_at as usize, Ordering::Release);
        tracing::error!("Primary sink failed, circuit opened");
    }

    fn reset_circuit(&self) {
        let prev = self.open_time.swap(0, Ordering::Release);
        if prev != 0 {
            tracing::info!("Circuit breaker CLOSED — resumed primary sink");
        }
        self.error_count.store(0, Ordering::Release);
    }
}

#[async_trait]
impl EventSink for CircuitBreaker {
    async fn send_batch(&self, batch: Vec<EventInput>) -> Result<(), SinkError> {
        if self.is_open() {
            return self
                .fallback_sink
                .send_batch(batch)
                .await
                .inspect_err(|e| tracing::error!(error=%e, "fallback sink also failed"));
        }

        match self.primary_sink.send_batch(batch).await {
            Ok(()) => {
                self.reset_circuit();
                Ok(())
            }
            Err(e) => {
                self.record_failure();
                Err(e)
            }
        }
    }
}
