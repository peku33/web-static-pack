//! Pack is the root entity, a collection of files.

use crate::{file::File, pack_path::PackPath};
use rkyv::{Archive, Serialize};
use std::collections::HashMap;

/// Pack represents a group of files distinguished by their path.
///
/// [Pack] is a bit like a `zip`, single entity containing directory/file tree.
/// [Pack] will usually be built with
/// [web-static-pack-packer](https://crates.io/crates/web-static-pack-packer)
/// crate, either with command line tool or by a script/build.rs. After [Pack]
/// is built, it will be serialized (called "Archived" by [rkyv] we use for that
/// purpose) into zero-copy deserializable representation - [PackArchived]. This
/// representation will be then included by your target program into binary (or
/// read/mmaped from fs) and served with
/// [web-static-pack](https://crates.io/crates/web-static-pack)
/// crate.
#[derive(Archive, Serialize, Debug)]
#[archive(archived = "PackArchived")]
#[archive_attr(derive(Debug))]
pub struct Pack {
    /// List of contained files by their paths.
    pub files_by_path: HashMap<PackPath, File>,
}
