//! File represents single item of a Pack, accessible under specific path.

use crate::cache_control::CacheControl;
use rkyv::{Archive, Serialize};

/// [File] represents an original file from filesystem with all fields
/// precalculated. It contains `gzip` / `brotli` compressed content,
/// precalculated http headers, like `content-type`, `ETag` and `cache-control`.
///
/// [File] is created in packing phase (once) to allow fast loading in loader
/// without need to perform expensive computations (like calculating compressed
/// forms) in runtime.
#[derive(Archive, Serialize, Debug)]
#[archive(archived = "FileArchived")]
#[archive_attr(derive(Debug))]
pub struct File {
    /// Raw (not compressed) file contents.
    pub content: Box<[u8]>,
    /// Gzip compressed file contents, if provided, otherwise None.
    pub content_gzip: Option<Box<[u8]>>,
    /// Brotli compressed file contents, if provided, otherwise None.
    pub content_brotli: Option<Box<[u8]>>,

    /// `content-type` header contents for the file, eg. `text/html;
    /// charset=utf-8` or `image/webp`.
    pub content_type: String,
    /// `ETag` header contents for the file, eg. checksum of `content`.
    pub etag: String,
    /// `cache-control` options for the file.
    pub cache_control: CacheControl,
}
