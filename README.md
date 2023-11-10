# s3-filesystem 
This crate is a simple wrapper for file interactions with S3. It enables you to open a file, write a file, and perform a walk through the objects in an S3 bucket. Both reading and writing will create a copy on your machine to make S3 feel as much like your local file system as possible. 


This crate utilises the official AWS SDK for S3 operations and uses Tokio for local IO. 

When using the crate, be careful! Writing files can overwrite data previously there and using S3 will incur costs.

# Usage
First create an OpenOptions struct containing the bucket you wish to connect to and where you wish files to be cached to. 

There are then three functions available 
- **[crate::OpenOptions::open_s3]**: Downloads the file and opens it and returns a Tokio File for data to be read from as standard.
- **[crate::OpenOptions::write_s3]**: Writes the file to disk and to S3, returning the Tokio File.
- **[crate::OpenOptions::walkdir]**: Walks through the objects in the S3 bucket, with an optional path to walk through a subset of objects.


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

## Write a file
```rust no_run
use s3_filesystem::OpenOptions;
use tokio::fs;

const BUCKET: &'static str = "test-bucket";

#[tokio::main]
async fn main() {
    let bucket = BUCKET.to_string();

    let open_options = OpenOptions::new(bucket, None)
        .await
        .mount_path("data/test/")
        .force_download(true);

    let data = fs::read("data/manifest.txt").await.unwrap();

    open_options.write_s3("manifest.txt", &data).await.unwrap();

    println!("Data uploaded successfully");
}
```

## Walkdir
```rust no_run
use s3_filesystem::OpenOptions;

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
## Walkdir and download 

```rust no_run
use s3_filesystem::OpenOptions;

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
- Add feature flags for automatic decompression?
- Look for changes in the file? If bytes is different download, if not read from cache. Beats generic force download config.

Test on more operating systems with more edge cases - currently little testing has occurred.
