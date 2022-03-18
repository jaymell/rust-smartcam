use aws_sdk_s3::{ByteStream, Client, Region};
use aws_types::config::Config;
use log::{debug, info};
use std::error::Error;
use std::fs;
use std::path::Path;

use crate::config;

mod error;

pub use error::UploadError;

pub async fn upload_file(path: &str) -> Result<(), Box<dyn Error>> {
    let p = Path::new(path);
    let app_config = config::load_config(None);
    let env_config = aws_config::load_from_env().await;
    let mut aws_config_builder = Config::builder();
    aws_config_builder.set_credentials_provider(env_config.credentials_provider().cloned());
    if let Some(r) = env_config.region() {
        aws_config_builder.set_region(r.clone());
    } else {
        if let Some(r) = &app_config.cloud.region {
            aws_config_builder.set_region(Region::new(r.clone()));
        } else {
            return Err(Box::new(UploadError::new("Region not set")));
        }
    }
    let bucket = &app_config.cloud.bucket;
    let client = Client::new(&aws_config_builder.build());

    let _content_length = fs::metadata(p).unwrap().len();
    let body = ByteStream::from_path(Path::new(p)).await.unwrap();
    let key = p.file_name().unwrap();

    debug!(
        "Uplading path {:?} to bucket'{:?}' key {:?}",
        p, bucket, key
    );

    match client
        .put_object()
        .bucket(bucket)
        .body(body)
        // .content_length(content_length as _)
        .key(key.to_str().unwrap())
        .content_type("video/x-matroska")
        // .content_md5(@#$@#$)
        .send()
        .await
    {
        Ok(_) => {
            info!(
                "Successfully uploaded path {:?} to BUCKET {:?} key {:?}",
                p, bucket, key
            );
            Ok(())
        }
        Err(e) => Err(Box::new(e)),
    }
}
