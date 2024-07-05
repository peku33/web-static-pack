//! Cache control related types. Provides [CacheControl].

use crate::common::cache_control::{CacheControl as CacheControl_, CacheControlArchived};
use http::HeaderValue;

/// Cache control enumeration, used to generate `cache-control` header content.
/// This will be usually built from [CacheControlArchived] or [CacheControl_]
/// (via [From]).
#[derive(Debug)]
pub enum CacheControl {
    /// Prevents caching file, by setting `no-cache` in `cache-control`.
    NoCache,
    /// Sets value to make resource cached for as long as possible.
    MaxCache,
}
impl CacheControl {
    /// Creates http [HeaderValue] from [self].
    pub fn cache_control(&self) -> HeaderValue {
        match self {
            CacheControl::NoCache => HeaderValue::from_static("no-cache"),
            CacheControl::MaxCache => HeaderValue::from_static("max-age=31536000, immutable"),
        }
    }
}
impl From<CacheControl_> for CacheControl {
    fn from(value: CacheControl_) -> Self {
        match value {
            CacheControl_::NoCache => Self::NoCache,
            CacheControl_::MaxCache => Self::MaxCache,
        }
    }
}
impl From<CacheControlArchived> for CacheControl {
    fn from(value: CacheControlArchived) -> Self {
        match value {
            CacheControlArchived::NoCache => Self::NoCache,
            CacheControlArchived::MaxCache => Self::MaxCache,
        }
    }
}
