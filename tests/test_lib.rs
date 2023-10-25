use s3_filesystem::OpenOptions;
use tokio::io::AsyncReadExt;

#[tokio::test]
async fn test_open_file() {
    let bucket = "my-test-bucket".to_string();

    let open_options = OpenOptions::new(bucket, None)
        .await
        .mount_path("data/test/")
        .force_download(true);

    let mut file = open_options
        .open_s3("my_test_folder/my_test_file.csv")
        .await
        .unwrap();

    let mut string = String::new();

    // read the whole file
    file.read_to_string(&mut string).await.unwrap();

    println!("String: {}", string);
}

#[tokio::test]
async fn test_walk_dir() {
    let bucket = "my-test-bucket".to_string();

    let open_options = OpenOptions::new(bucket, None).await;

    let data = open_options.walkdir("android").await.unwrap();
    for dat in data {
        println!("Data: {:?}", dat);
    }
}

#[tokio::test]
async fn combine_walkdir_and_download() {
    let bucket = "my-test-bucket".to_string();

    let open_options = OpenOptions::new(bucket, None)
        .await
        .mount_path("data/test/")
        .force_download(false);

    let data = open_options.walkdir("my_test_folder").await.unwrap();

    for entry in data {
        if entry.folder {
            continue;
        }

        let _s3_stuff = open_options.open_s3(&entry.path).await.unwrap();

        println!("entry: {:?} downloaded", entry.path);
    }
}
