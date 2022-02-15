use super::{VideoFile, VideoRepository};
use crate::config;
use anyhow::Result;
use async_trait::async_trait;
use chrono::DateTime;
use futures::stream::Stream;
use serde::Serialize;
use std::path::{Path, PathBuf};
use std::pin::Pin;
use std::sync::Arc;
use tokio::fs;
use tokio::fs::ReadDir;
use tokio_stream::wrappers::ReadDirStream;
use tokio_stream::StreamExt;

pub struct LocalVideoRepository {
    path: PathBuf,
}

impl LocalVideoRepository {
    pub fn new() -> Self {
        let config = config::load_config(None);

        Self {
            path: Path::new(&config.storage.path).to_path_buf(),
        }
    }
}

#[async_trait]
impl VideoRepository for LocalVideoRepository {
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

    async fn stream_files_by_label(&self, label: String) -> Pin<Arc<dyn Stream<Item = VideoFile>>> {
        Arc::pin(
            ReadDirStream::new(fs::read_dir(&self.path).await.unwrap())
                .filter(move |entry| match entry {
                    Ok(e) => e.file_name().to_string_lossy().contains(&label),
                    Err(_) => false,
                })
                .map(|entry| VideoFile {
                    file_name: entry.unwrap().file_name().into_string().unwrap(),
                }),
        )
        //     match fs::read_dir(&self.path).await {
        //         Ok(e) => Ok(Arc::pin(
        //             ReadDirStream::new(e)
        //                 .filter(move |entry| match entry {
        //                     Ok(e) => e.file_name().to_string_lossy().contains(&label),
        //                     Err(_) => false,
        //                 })
        //                 .map(|entry| VideoFile {
        //                     file_name: entry.unwrap().file_name().into_string().unwrap(),
        //                 }),
        //         )),
        //         Err(e) => Err(anyhow::Error::from(e)),
        //     }
        // }
    }
}
