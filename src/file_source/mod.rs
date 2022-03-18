use crate::config;
use anyhow::Result;
use async_trait::async_trait;
use once_cell::sync::Lazy;
use serde::Serialize;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::fs;

static GLOBAL_DATA: Lazy<Arc<dyn FileSource + Send + Sync>> =
    Lazy::new(|| Arc::new(LocalFileSource::new()));

pub fn load() -> Arc<dyn FileSource + Send + Sync> {
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
pub trait FileSource {
    async fn list_files_by_label(&self, label: &str) -> Result<Vec<VideoFile>>;
    // async fn list_files_by_label_since_time(&self, label: &str, since_time: &str) -> Vec<VideoFile>;
    // async fn list_files_by_label_before_time(&self, label: &str, before_time: &str) -> Vec<VideoFile>;
    // async fn list_files_by_label_between_times(
    //     &self,
    //     label: &str,
    //     begin_time: &str,
    //     end_time: &str,
    // ) -> Vec<VideoFile>;
}

struct LocalFileSource {
    path: PathBuf,
}

impl LocalFileSource {
    pub fn new() -> Self {
        let config = config::load_config(None);

        Self {
            path: Path::new(&config.storage.path).to_path_buf(),
        }
    }
}

#[async_trait]
impl FileSource for LocalFileSource {
    async fn list_files_by_label(&self, label: &str) -> Result<Vec<VideoFile>> {
        let mut v = Vec::new();
        let mut entries = fs::read_dir(&self.path).await?;

        while let Some(entry) = entries.next_entry().await? {
            println!("{:?}", entry.path());
            if entry.file_name().to_string_lossy().contains(label) {
                v.push(VideoFile {
                    file_name: entry.file_name().into_string().unwrap(),
                });
            }
        }
        Ok(v)
    }
}
