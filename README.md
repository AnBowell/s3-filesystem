# S3-Filesystem 
A way to asynchronously interact with S3 files as if they were local on your disk. 

## Open a file
```rust no_run
use s3_filesystem::OpenOptions;
use tokio::io::AsyncReadExt;

#[tokio::main]
async fn main() {
    let bucket = "my_aws_s3_bucket".to_string();


    // If a custom S3 client is needed, replace None with Some(client).
    // Default behavior is to use AWS CLI env.
    let open_options = OpenOptions::new(bucket, None)
        .await
        .mount_path("data/test/")
        .force_download(true);

    let mut file = open_options.open_s3("some_folder/some_file.csv").await.unwrap();

    let mut string = String::new();

    // Read the example file and print it.
    file.read_to_string(&mut string).await.unwrap();

    println!("String: {}", string);

}
```


## Walkdir
```rust no_run
use s3_filesystem::OpenOptions;
use tokio::io::AsyncReadExt;

#[tokio::main]
async fn main() {
    let bucket = "my_aws_s3_bucket".to_string();

    let open_options = OpenOptions::new(bucket, None).await;
   
    let data = open_options.walkdir("").await.unwrap();

    for dat in data {
        println!("Data: {:?}", dat);
    }
}
```

```rust no_run
use s3_filesystem::OpenOptions;
use tokio::io::AsyncReadExt;

#[tokio::main]
async fn main() {
    let bucket = "my_aws_s3_bucket".to_string();

    let open_options = OpenOptions::new(bucket, None)
        .await
        .mount_path("data/test/")
        .force_download(false);

    let data = open_options

        .walkdir("some_bucket_sub_folder")
        .await
        .unwrap();

    // WARNING!
    // Downloads and opens every file in the sub folder!
    for entry in data {
        if entry.folder {
            continue;
        }

        let s3_stuff = open_options.open_s3(&entry.path).await.unwrap();

        println!("Entry: {:?} downloaded", entry.path);
    }
}
```



## TODOs 

- Introduce some nicer error handling. Currently using std io errors, but could re-export aws errors etc.
- Add feature flags for automatic decompression?
- Look for changes in the file? If bytes is different download, if not read from cache. Beats generic force download config.

Test on more operating systems with more edge cases - currently had very little testing!