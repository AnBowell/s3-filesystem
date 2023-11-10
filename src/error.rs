use aws_sdk_s3::{error::SdkError, primitives::ByteStreamError};
use std::{fmt::Debug, io};

#[derive(Debug)]
/// Container for errors that can occur due to AWS or local I/O.
pub enum S3FilesystemError<E, R> {
    /// Occurs when a request to S3 is unsuccessful - for instance when a non-existent object is requested.
    S3(SdkError<E, R>),
    /// Occurs when a reading or writing to/from a ByteStream (used for S3 downloads/uploads).
    ByteStream(ByteStreamError),
    /// Occurs when there are issues with the local file system - for instance, creating a file with an invalid character in the filename.
    Io(io::Error),
}

impl<E, R> From<io::Error> for S3FilesystemError<E, R> {
    fn from(err: io::Error) -> Self {
        Self::Io(err)
    }
}

impl<E, R> From<SdkError<E, R>> for S3FilesystemError<E, R> {
    fn from(err: SdkError<E, R>) -> Self {
        Self::S3(err)
    }
}

impl<E, R> From<ByteStreamError> for S3FilesystemError<E, R> {
    fn from(err: ByteStreamError) -> Self {
        Self::ByteStream(err)
    }
}
impl<E, R> std::fmt::Display for S3FilesystemError<E, R> {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            S3FilesystemError::S3(s3_err) => write!(f, "S3 Error: {}", s3_err),
            S3FilesystemError::Io(io_err) => write!(f, "IO Error: {}", io_err),
            S3FilesystemError::ByteStream(bytestream_error) => {
                write!(f, "ByteStream error: {}", bytestream_error)
            }
        }
    }
}
impl<E, R> std::error::Error for S3FilesystemError<E, R>
where
    E: std::error::Error + 'static,
    R: Debug,
{
}
