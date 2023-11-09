#![doc = include_str!("../README.md")]
#![deny(missing_docs, unused_imports)]

mod error;
mod fs;

pub use crate::error::S3FilesystemError;
pub use crate::fs::DirEntry;
pub use crate::fs::OpenOptions;
