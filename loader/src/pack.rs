//! Pack related types. Provides [Pack] trait.

use crate::{
    common::{
        file::{File as File_, FileArchived},
        pack::{Pack as Pack_, PackArchived},
    },
    file::File,
};

/// Trait representing Pack, a container for files identified by path.
///
/// Most users will use [PackArchived] implementation, returned by
/// [crate::loader::load].
/// This trait is also implemented for non-archived [Pack_], mostly for testing
/// purposes.
pub trait Pack {
    /// File type returned by this `pack`.
    type File: File;

    /// Given `pack` relative path, eg. `/dir1/dir2/file.html` returns file
    /// associated with this path. Returns [None] if file for given path
    /// does not exist.
    fn get_file_by_path(
        &self,
        path: &str,
    ) -> Option<&Self::File>;
}
impl Pack for Pack_ {
    type File = File_;

    fn get_file_by_path(
        &self,
        path: &str,
    ) -> Option<&Self::File> {
        let file = self.files_by_path.get(path)?;
        Some(file)
    }
}
impl Pack for PackArchived {
    type File = FileArchived;

    fn get_file_by_path(
        &self,
        path: &str,
    ) -> Option<&Self::File> {
        let file = self.files_by_path.get(path)?;
        Some(file)
    }
}
