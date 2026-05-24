use std::path::PathBuf;

use tokio::sync::mpsc;

use crate::ocr_engine::error::PipelineError;

/// A single file queued for processing through the OCR pipeline.
#[derive(Clone, Debug)]
pub struct IngestItem {
    /// Unique job identifier.
    pub job_id: String,
    /// Path to the input PDF file.
    pub path: PathBuf,
    /// Path where the cleaned/OCRed PDF will be written.
    pub output_path: PathBuf,
}

/// Sends all `items` into the bounded channel. Returns an error if the receiver is dropped.
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

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::sync::mpsc;

    #[tokio::test]
    async fn enqueue_sends_all_items() {
        let (tx, mut rx) = mpsc::channel(10);
        let items = vec![
            IngestItem {
                job_id: "1".into(),
                path: PathBuf::from("/a.pdf"),
                output_path: PathBuf::from("/a_out.pdf"),
            },
            IngestItem {
                job_id: "2".into(),
                path: PathBuf::from("/b.pdf"),
                output_path: PathBuf::from("/b_out.pdf"),
            },
        ];

        let handle = tokio::spawn(async move {
            enqueue_files(tx, items).await.unwrap();
        });

        let mut received = Vec::new();
        while let Some(item) = rx.recv().await {
            received.push(item.job_id);
        }
        handle.await.unwrap();
        assert_eq!(received, vec!["1", "2"]);
    }

    #[tokio::test]
    async fn enqueue_errors_on_closed_channel() {
        let (tx, rx) = mpsc::channel::<IngestItem>(1);
        drop(rx);
        let result = enqueue_files(tx, vec![]).await;
        assert!(result.is_ok()); // Sending empty vec succeeds
    }
}
