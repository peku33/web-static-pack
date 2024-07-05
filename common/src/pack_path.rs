//! Pack path contains custom type for representing path inside a `pack`.

use rkyv::{Archive, Serialize};
use std::{borrow::Borrow, ops::Deref};

/// [PackPath] represents path inside a `pack`. It will correspond to http
/// request path (aka. uri/url) directly.
///
/// Custom type is used to enforce some rules, eg. starts with "/", contains
/// only valid characters, etc.
#[derive(Archive, Serialize, PartialEq, Eq, Hash, Debug)]
#[archive(archived = "PackPathArchived")]
#[archive_attr(derive(PartialEq, Eq, Hash, Debug))]
pub struct PackPath {
    inner: String,
}
impl PackPath {
    /// Construct path from string representation. Refer to [self] for details.
    /// Providing invalid path won't result in catastrophic failure, but
    /// such will will never be resolved.
    pub fn from_string(inner: String) -> Self {
        Self { inner }
    }
}

// to allow searching in HashMap directly by http path (which is str)
impl Deref for PackPath {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}
impl Borrow<str> for PackPath {
    fn borrow(&self) -> &str {
        self.inner.as_str()
    }
}

impl Deref for PackPathArchived {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}
impl Borrow<str> for PackPathArchived {
    fn borrow(&self) -> &str {
        self.inner.as_str()
    }
}
