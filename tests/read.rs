/// The read tests in this file use REDASA COVID-19 Open Data stored on S3 as part of the AWS open data sponsorship program.
///
/// It is stored on eu-west2 - if your AWS client is not connected to eu-west2 it will fail.
/// TODO -  use a specified region.
use s3_filesystem::OpenOptions;
use tokio::io::AsyncReadExt;

// eu-west2 public data.
const BUCKET: &'static str = "pansurg-curation-workflo-kendraqueryresults50d0eb-open-data";

#[tokio::test]
async fn test_open_file() {
    let bucket = BUCKET.to_string();

    let open_options = OpenOptions::new(bucket, None)
        .await
        .mount_path("data/test/")
        .force_download(true);

    let mut file = open_options
        .open_s3("redasa1-Q1-20/manifest.txt")
        .await
        .unwrap();

    let mut string = String::new();

    // read the whole file
    file.read_to_string(&mut string).await.unwrap();

    println!("String: {}", string);
}

#[tokio::test]
async fn test_walk_dir() {
    let bucket = BUCKET.to_string();

    let open_options = OpenOptions::new(bucket, None).await;

    let data = open_options.walkdir("redasa1-Q1-20").await.unwrap();
    for dat in data {
        println!("Data: {:?}", dat);
    }
}

#[tokio::test]
async fn combine_walkdir_and_download() {
    let bucket = BUCKET.to_string();

    let open_options = OpenOptions::new(bucket, None)
        .await
        .mount_path("data/test/")
        .force_download(false);

    let data = open_options.walkdir("redasa1-Q1-20").await.unwrap();

    for entry in data {
        if entry.folder {
            continue;
        }

        let _s3_stuff = open_options.open_s3(&entry.path).await.unwrap();
        println!("entry: {:?} downloaded", entry.path);
    }
}
