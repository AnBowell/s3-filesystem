use aws_sdk_s3::{primitives::ByteStream, Client};
use std::{
    io,
    path::{Path, PathBuf},
};
use tokio::{
    fs::File,
    io::{AsyncSeekExt, AsyncWriteExt},
};

pub const DEFAULT_DATA_STORE: &'static str = "target/temp";

/// Holds configuration data for opening a file from S3.
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
    /// This function should be used to create a new option configuration for
    /// using an S3 bucket as if it were local to disk. A bucket is required and
    /// if data is needed from another bucket, a new OpenOptions should be created.
    ///
    /// Client is an optional argument - if it exists that will be the client used,
    /// and if it doesn't, this function will automatically create an S3 client
    /// from your environment (the AWS CLI).
    ///
    /// If non default mount paths are wanted, the function [OpenOptions::mount_path] can be
    /// used, and if you wish to re-download data each time, [OpenOptions::force_download] can
    /// be used.
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
    /// Open a file from S3 as if it were local.
    ///
    /// This function will find the S3 file, download it and return a [tokio::fs::File]
    /// which can be used as it normally would be. I've opted for read/write to disk so that
    /// large files can be downloaded and read in a chunked way and do not have to be read in their entirety
    /// into memory.
    ///
    /// Files will be placed in the `mount_path` and all folder structure is retained. Folders will be created
    /// if they do not exist already.
    ///
    pub async fn open_s3<P>(&self, path: P) -> io::Result<File>
    where
        P: AsRef<Path>,
    {
        let full_data_path = self.mount_path.join(&self.bucket).join(&path);

        let s3_data_path = match path.as_ref().to_str() {
            Some(path) => path.replace("\\", "/"),
            None => {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    "Invalid File Path",
                ))
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
                return Err(io::Error::new(io::ErrorKind::Other, e)); // TODO:  Error handling. Maybe a custom error?
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
    /// Enter the path, relative to the bucket, and this function will create a
    /// file in S3. It will return the file that has been written to.
    ///
    pub async fn write_s3<P>(&self, path: P, buf: &[u8]) -> io::Result<File>
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
                return Err(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    "Invalid File Path",
                ))
            }
        };

        let mut file = tokio::fs::OpenOptions::new()
            .read(true) // TODO - Do I want to return an empty file?
            .write(true)
            .create(true)
            .open(&full_data_path)
            .await?;

        file.write_all(buf).await?;

        let byte_stream = ByteStream::from_path(full_data_path).await?;

        let put_object_builder = self.s3_client.put_object().bucket(&self.bucket);
        return match put_object_builder
            .key(s3_data_path)
            .body(byte_stream)
            .send()
            .await
        {
            Ok(_) => Ok(file),
            Err(e) => {
                return Err(io::Error::new(io::ErrorKind::Other, e)); // TODO:  Error handling. Maybe a custom error?
            }
        };
    }

    /// Return a vector of Directories/Files in a WalkDir order.
    ///
    /// This function returns the files and folders in the bucket defined in [OpenOptions].
    ///
    /// It returns their path, size, and whether or not they're a directory.
    pub async fn walkdir<P>(&self, path: P) -> Result<Vec<DirEntry>, io::Error>
    where
        P: AsRef<Path>,
    {
        let mut obj_req = self.s3_client.list_objects_v2().bucket(&self.bucket);

        match path.as_ref().to_str() {
            Some(path) => obj_req = obj_req.prefix(path),
            None => {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    "Invalid filepath. Please ensure it's UTF-8 only.",
                ))
            }
        }

        let objects_res = match obj_req.send().await {
            Ok(x) => x,
            Err(e) => return Err(io::Error::new(io::ErrorKind::Other, e)),
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
