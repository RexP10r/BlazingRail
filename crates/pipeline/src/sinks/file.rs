#![allow(dead_code)]

use std::{
    fs::File,
    io::{BufWriter, Error},
    sync::{Arc, Mutex},
};

use common::PipelineConfig;

pub struct FileSynk {
    writer: Arc<Mutex<BufWriter<File>>>,
}

impl FileSynk {
    pub fn new(pipeline_config: Arc<PipelineConfig>) -> Result<Self, Error> {
        let file = File::create(pipeline_config.dlq_path.clone())?;
        let buf_writer = BufWriter::with_capacity(pipeline_config.batch_size, file);
        Ok(Self {
            writer: Arc::new(Mutex::new(buf_writer)),
        })
    }
}
