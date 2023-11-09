use aws_sdk_s3::{
    operation::{
        get_object::GetObjectError, list_objects_v2::ListObjectsV2Error, put_object::PutObjectError,
    },
    primitives::ByteStream,
    Client,
};
use aws_smithy_runtime_api::client::orchestrator::HttpResponse;
use std::{
    io,
    path::{Path, PathBuf},
};
use tokio::{
    fs::File,
    io::{AsyncSeekExt, AsyncWriteExt},
};

use crate::error::S3FilesystemError;

pub const DEFAULT_DATA_STORE: &'static str = "target/temp";

/// Holds configuration data for syncing S3 objects.
///
/// Bucket will specify the bucket which is mounted at mount_path. It will
/// download the file from the bucket to the path maintaining the same folder
/// structure. If force_download is set to true, it will always download the files
/// from S3. If it's false, it will use whatever is found on disk at that location.
#[derive(Debug, Clone)]
pub struct OpenOptions {
    s3_client: Client,
    bucket: String,
    mount_path: PathBuf,
    force_download: bool,
}

impl OpenOptions {
    /// Create a new OpenOptions struct.
    ///
    /// This function should be used to create a new option configuration for your S3 bucket and
    /// filesystem. If data is needed from another bucket, a new OpenOptions should be created.
    ///
    /// Client is an optional argument - if it exists that will be the client used
    /// and if it doesn't, this function will automatically create an S3 client
    /// from your environment (the AWS CLI).
    ///
    /// If non default mount paths are wanted, the function [OpenOptions::mount_path] can be
    /// used, and if you wish to re-download data each time, [OpenOptions::force_download] can
    /// be used.
    ///
    /// # Examples
    ///
    ///```no_run
    /// use s3_filesystem::OpenOptions;
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///  let open_options = OpenOptions::new(bucket, None)
    ///     .await
    ///     .mount_path("data/test/")
    ///     .force_download(true);
    /// }
    /// ```
    pub async fn new(bucket: String, client: Option<Client>) -> Self {
        let s3_client = match client {
            Some(x) => x,
            None => {
                let config = aws_config::load_from_env().await;
                aws_sdk_s3::Client::new(&config)
            }
        };

        OpenOptions {
            s3_client,
            bucket: bucket,
            mount_path: DEFAULT_DATA_STORE.into(),
            force_download: false,
        }
    }

    /// Attach a custom mount path.
    ///
    /// By default any data downloaded from S3 is found in target/temp. This can
    /// be changed by using this function!
    pub fn mount_path<P>(mut self, folder_path: P) -> Self
    where
        P: Into<PathBuf>,
    {
        self.mount_path = folder_path.into();
        self
    }

    /// Enforce downloading the data every time
    ///
    /// Cache is supported by default - if a file with the same name is found on disk
    /// then it is read in. Pass `download` = true if you wish to disable this behavior.
    pub fn force_download(mut self, download: bool) -> Self {
        self.force_download = download;
        self
    }
}

impl OpenOptions {
    /// Open a file from S3.
    ///
    /// This function will find the S3 file, download it and return a [tokio::fs::File] ready to be read. Doing it this way
    /// enables large files to be downloaded in chunks as well as local caching..
    ///
    /// Files will be placed in the `mount_path` and all folder structure is retained. Folders will be created
    /// if they do not exist already.
    ///
    /// # Arguments
    /// * `path`: The path, including filename, of the file to be downloaded and opened.
    ///```no_run
    /// use s3_filesystem::OpenOptions;
    /// use tokio::io::AsyncReadExt;
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///  let open_options = OpenOptions::new(bucket, None)
    ///     .await
    ///     .mount_path("data/test/")
    ///     .force_download(true);
    ///
    /// let mut file = open_options
    ///     .open_s3("redasa1-Q1-20/manifest.txt")
    ///     .await
    ///     .unwrap();
    ///
    ///  let mut string = String::new();
    ///
    ///  file.read_to_string(&mut string).await.unwrap();
    ///
    ///  println!("String: {}", string);
    /// }
    /// ```
    pub async fn open_s3<P>(
        &self,
        path: P,
    ) -> Result<File, S3FilesystemError<GetObjectError, HttpResponse>>
    where
        P: AsRef<Path>,
    {
        let full_data_path = self.mount_path.join(&self.bucket).join(&path);

        let s3_data_path = match path.as_ref().to_str() {
            Some(path) => path.replace("\\", "/"),
            None => {
                return Err(io::Error::new(io::ErrorKind::InvalidInput, "Invalid File Path").into())
            }
        };

        let exists = std::fs::metadata(&full_data_path).is_ok();

        if exists && !self.force_download {
            return Ok(tokio::fs::OpenOptions::new()
                .read(true)
                .open(&full_data_path)
                .await?);
        }

        match full_data_path.parent() {
            Some(parent_path) => std::fs::create_dir_all(parent_path)?,
            None => (),
        }

        let mut file = tokio::fs::OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .open(&full_data_path)
            .await?;

        let get_object_builder = self.s3_client.get_object().bucket(&self.bucket);

        let mut object = match get_object_builder.key(s3_data_path).send().await {
            Ok(x) => x,
            Err(e) => {
                tokio::fs::remove_file(&full_data_path).await?;
                return Err(e.into());
            }
        };

        while let Some(bytes) = object.body.try_next().await? {
            file.write(&bytes).await?;
        }

        file.seek(io::SeekFrom::Start(0)).await?;

        return Ok(file);
    }

    /// Write a file to S3
    ///
    /// Enter a path relative to the bucket and this function will create a file in S3 and on your local system under
    /// the mount path chosen in [OpenOptions]. This will overwrite any files that exist with the same name and will
    /// return the file that has been written to.
    ///
    /// # Arguments
    /// * `path`: The path, including the filename, where you wish to store the data.
    /// * `buf`: The data you wish to store.
    ///
    /// # Examples
    /// ```no_run
    /// use s3_filesystem::OpenOptions;
    /// use tokio::fs;
    ///
    /// const BUCKET: &'static str = "test-bucket";
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///     let bucket = BUCKET.to_string();
    ///
    ///     let open_options = OpenOptions::new(bucket, None)
    ///         .await
    ///         .mount_path("data/test/")
    ///         .force_download(true);
    ///
    ///     let data = fs::read("data/manifest.txt").await.unwrap();
    ///
    ///     open_options.write_s3("manifest.txt", &data).await.unwrap();
    ///
    ///     println!("Data uploaded successfully");
    /// }
    pub async fn write_s3<P>(
        &self,
        path: P,
        buf: &[u8],
    ) -> Result<File, S3FilesystemError<PutObjectError, HttpResponse>>
    where
        P: AsRef<Path>,
    {
        let full_data_path = self.mount_path.join(&self.bucket).join(&path);
        match full_data_path.parent() {
            Some(parent_path) => std::fs::create_dir_all(parent_path)?,
            None => (),
        }
        let s3_data_path = match path.as_ref().to_str() {
            Some(path) => path.replace("\\", "/"),
            None => {
                return Err(io::Error::new(io::ErrorKind::InvalidInput, "Invalid File Path").into())
            }
        };

        let mut file = tokio::fs::OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .open(&full_data_path)
            .await?;

        file.write_all(buf).await?;

        let byte_stream = ByteStream::from_path(&full_data_path).await?;

        let put_object_builder = self.s3_client.put_object().bucket(&self.bucket);
        return match put_object_builder
            .key(s3_data_path)
            .body(byte_stream)
            .send()
            .await
        {
            Ok(_) => Ok(file),
            Err(e) => {
                tokio::fs::remove_file(&full_data_path).await?;
                return Err(e.into());
            }
        };
    }

    /// Return a list of S3 objects within the bucket
    ///
    /// This function returns the files and folders (S3 objects) in the bucket defined in [OpenOptions]. A sub path
    /// can be specified to return a subset of the items - for the entire bucket provide an empty string: "".
    ///
    /// It returns their path, size, and whether or not it's a directory, but be wary - directories do not exist in S3.
    /// This function will return any directories that have been created as a dummy object ending in "/" within S3. It is not
    /// guaranteed to find all directories. This may change in upcoming versions.
    ///
    /// # Arguments
    /// * `path`: A path to search within the S3 bucket. If you want the entire bucket, just specify an empty string: "".
    ///
    /// # Examples
    /// ```rust no_run
    /// use s3_filesystem::OpenOptions;
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///     let bucket = "my_aws_s3_bucket".to_string();
    ///
    ///     let open_options = OpenOptions::new(bucket, None).await;
    ///
    ///     let data = open_options.walkdir("").await.unwrap();
    ///
    ///     for dat in data {
    ///         println!("Data: {:?}", dat);
    ///     }
    /// }
    /// ```
    pub async fn walkdir<P>(
        &self,
        path: P,
    ) -> Result<Vec<DirEntry>, S3FilesystemError<ListObjectsV2Error, HttpResponse>>
    where
        P: AsRef<Path>,
    {
        let mut obj_req = self.s3_client.list_objects_v2().bucket(&self.bucket);

        match path.as_ref().to_str() {
            Some(path) => obj_req = obj_req.prefix(path),
            None => {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    "Invalid filepath for S3. Please ensure it's UTF-8 only.",
                )
                .into())
            }
        }

        let objects_res = match obj_req.send().await {
            Ok(x) => x,
            Err(e) => return Err(e.into()),
        };

        let mut data_to_return = Vec::new();

        for s3_object in objects_res.contents() {
            let filepath = match s3_object.key() {
                Some(x) => x.to_string(),
                None => continue,
            };

            data_to_return.push(DirEntry {
                path: PathBuf::from(&filepath),
                size: s3_object.size(),
                folder: filepath.ends_with("/"),
            });
        }

        return Ok(data_to_return);
    }
}

#[derive(Debug, Clone)]
/// Holds information describing a file or folder.
pub struct DirEntry {
    /// Path data is located at in S3.
    pub path: PathBuf,
    /// Size of the data in bytes. Folders = 0 bytes.
    pub size: i64,
    /// Whether the S3 object is a folder or not.
    pub folder: bool,
}
