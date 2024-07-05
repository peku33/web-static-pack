//! Single file related types. Provides [File] trait.

use crate::{
    cache_control::CacheControl,
    common::file::{File as File_, FileArchived},
};
use http::HeaderValue;

/// Trait for single file inside a `pack`. Consists of body in different encodings
/// (`identity` aka `normal`, `gzip`, `brotli`), some precomputed header values
/// etc.
///
/// Most users will indirectly use [FileArchived] implementation, obtained from
/// [crate::pack::Pack::get_file_by_path] (implemented by
/// [crate::common::pack::PackArchived]).
/// This trait is also implemented for non-archived [File_], mostly for testing
/// purposes.
pub trait File {
    // content with different types
    /// Accesses file content in original (`identity`) encoding.
    fn content(&self) -> &[u8];
    /// Accesses file content in `gzip` encoding if available.
    fn content_gzip(&self) -> Option<&[u8]>;
    /// Accesses file content in `brotli` encoding if available.
    fn content_brotli(&self) -> Option<&[u8]>;

    // headers
    /// Accesses `content-type` header contents for this file.
    fn content_type(&self) -> HeaderValue;
    /// Accesses `ETag` header contents for this file.
    fn etag(&self) -> HeaderValue;
    /// Accesses [CacheControl] for this file.
    fn cache_control(&self) -> CacheControl;
}
impl File for File_ {
    fn content(&self) -> &[u8] {
        &self.content
    }
    fn content_gzip(&self) -> Option<&[u8]> {
        self.content_gzip.as_deref()
    }
    fn content_brotli(&self) -> Option<&[u8]> {
        self.content_brotli.as_deref()
    }

    fn content_type(&self) -> HeaderValue {
        HeaderValue::from_str(&self.content_type).unwrap()
    }
    fn etag(&self) -> HeaderValue {
        HeaderValue::from_str(&self.etag).unwrap()
    }
    fn cache_control(&self) -> CacheControl {
        CacheControl::from(self.cache_control)
    }
}
impl File for FileArchived {
    fn content(&self) -> &[u8] {
        &self.content
    }
    fn content_gzip(&self) -> Option<&[u8]> {
        self.content_gzip.as_deref()
    }
    fn content_brotli(&self) -> Option<&[u8]> {
        self.content_brotli.as_deref()
    }

    fn content_type(&self) -> HeaderValue {
        HeaderValue::from_str(&self.content_type).unwrap()
    }
    fn etag(&self) -> HeaderValue {
        HeaderValue::from_str(&self.etag).unwrap()
    }
    fn cache_control(&self) -> CacheControl {
        CacheControl::from(self.cache_control)
    }
}
