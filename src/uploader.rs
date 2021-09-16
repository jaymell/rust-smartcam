use aws_sdk_s3::{ByteStream, Client, Region};
use aws_types::{config::Config, Credentials};
use log::{debug, info};
use std::fs;
use std::path::Path;

const BUCKET: &str = &"FIXME";

pub async fn upload_file(p: &Path) -> () {
    let config = aws_config::load_from_env().await;
    let client = Client::new(&config);

    let content_length = fs::metadata(p).unwrap().len();
    let body = ByteStream::from_path(Path::new(p)).await.unwrap();
    let key = p.file_name().unwrap();
    // let key = "my-test-key.mkv";

    debug!(
        "Uplading path {:?} to bucket'{:?}' key {:?}",
        p, BUCKET, key
    );

    client
        .put_object()
        .BUCKET(BUCKET)
        .body(body)
        // .content_length(content_length as _)
        .key(key.to_str().unwrap())
        // .key(key)
        .content_type("video/x-matroska")
        .send()
        .await
        .unwrap();
    // .content_md5(@#$@#$)

    info!(
        "Successfully uploaded path {:?} to BUCKET {:?} key {:?}",
        p, BUCKET, key
    );
}
