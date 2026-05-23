use std::path::PathBuf;

use tokio::sync::mpsc;

use crate::ocr_engine::error::PipelineError;

#[derive(Clone, Debug)]
pub struct IngestItem {
    pub job_id: String,
    pub path: PathBuf,
    pub output_path: PathBuf,
}

pub async fn enqueue_files(
    tx: mpsc::Sender<IngestItem>,
    items: Vec<IngestItem>,
) -> Result<(), PipelineError> {
    for item in items {
        tx.send(item)
            .await
            .map_err(|_| PipelineError::Channel("ingest channel closed".to_string()))?;
    }
    Ok(())
}
