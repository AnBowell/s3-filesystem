use s3_filesystem::OpenOptions;

use tokio::fs;

// eu-west2 public data.
const BUCKET: &'static str = "test-bucket";

#[tokio::test]
async fn test_write_file() {
    let bucket = BUCKET.to_string();

    let open_options = OpenOptions::new(bucket, None)
        .await
        .mount_path("data/test/")
        .force_download(true);

    let data = fs::read("data/manifest.txt").await.unwrap();

    open_options.write_s3("manifest.txt", &data).await.unwrap();

    println!("Data uploaded successfully");
}
