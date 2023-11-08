#![doc = include_str!("../README.md")]
#![deny(missing_docs, unused_imports)]

mod fs;
mod error;

pub use crate::fs::DirEntry;
pub use crate::fs::OpenOptions;
