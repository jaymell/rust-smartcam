mod local;

use crate::config;
use anyhow::Result;
use async_trait::async_trait;
use chrono::DateTime;
use futures::stream::Stream;
use local::LocalVideoRepository;
use once_cell::sync::Lazy;
use serde::Serialize;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use std::pin::Pin;
use tokio_stream::wrappers::ReadDirStream;
use tokio_stream::StreamExt;

static GLOBAL_DATA: Lazy<Arc<dyn VideoRepository + Send + Sync>> =
    Lazy::new(|| Arc::new(LocalVideoRepository::new()));

pub fn load() -> Arc<dyn VideoRepository + Send + Sync> {
    Arc::clone(&GLOBAL_DATA)
}

#[derive(Serialize, Clone, Debug)]
pub struct VideoFile {
    // timestamp: DateTime<Utc>,
    // label: String,
    // link: String,
    pub file_name: String,
    // length?
    // resolution?
}

#[async_trait]
pub trait VideoRepository {
    async fn list_files_by_label(&self, label: &str) -> Result<Vec<VideoFile>>;
    async fn stream_files_by_label(&self, label: String) -> Pin<Arc<dyn Stream<Item = VideoFile>>>;
    // async fn list_files_by_label_since_time(&self, label: &str, since_time: &str) -> Vec<VideoFile>;
    // async fn list_files_by_label_before_time(&self, label: &str, before_time: &str) -> Vec<VideoFile>;
    // async fn list_files_by_label_between_times(
    //     &self,
    //     label: &str,
    //     begin_time: &str,
    //     end_time: &str,
    // ) -> Vec<VideoFile>;
}
