//! Cache control types used by file.

use rkyv::{Archive, Serialize};

/// Type representing cache control of a file. This will correspond to
/// `cache-control` header set in http response.
#[derive(Archive, Serialize, Clone, Copy, PartialEq, Eq, Debug)]
#[archive(archived = "CacheControlArchived")]
#[archive_attr(derive(Clone, Copy, PartialEq, Eq, Debug))]
pub enum CacheControl {
    /// No caching. This corresponds to "cache never" strategy, ex. by setting
    /// `no-cache` header value.
    NoCache,
    /// Max caching. This corresponds to "cache forever" strategy, ex. by
    /// setting `max-age=31536000, immutable` header value.
    MaxCache,
}
