use aws_sdk_s3::{error::SdkError, primitives::ByteStreamError};
use std::{io, fmt::Debug};


#[derive(Debug)]
pub enum S3FilesystemError<E,R>{
    S3(SdkError<E, R>),
    ByteStream(ByteStreamError),
    IO(io::Error)
}

impl <E,R>From<io::Error> for S3FilesystemError<E,R>{
    fn from(err: io::Error) -> Self {
        Self::IO(err)
    }
}

impl <E,R>From<SdkError<E,R>> for S3FilesystemError<E,R>{
    fn from(err: SdkError<E,R>) -> Self {
        Self::S3(err)
    }
}

impl <E,R>From<ByteStreamError> for S3FilesystemError<E,R>{
    fn from(err: ByteStreamError) -> Self {
        Self::ByteStream(err)
    }
}
impl <E,R>std::fmt::Display for S3FilesystemError<E,R> {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self{
            S3FilesystemError::S3(s3_err) => write!(f, "S3 Error {}", s3_err.to_string()),
            S3FilesystemError::IO(io_err) => write!(f, "IO Error: {}", io_err.to_string()),
            S3FilesystemError::ByteStream(bytestream_error) => write!(f, "Bytestream error: {}", bytestream_error.to_string())
        }
 
    }
}
impl <E,R>std::error::Error for S3FilesystemError<E,R> where
E: std::error::Error + 'static,
R: Debug
{
}