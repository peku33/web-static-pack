//! Pack path (file and pack path combined) helpers. Contains [FilePackPath], a
//! combination of [File] and [PackPath].

use crate::{
    common::{file::File, pack_path::PackPath},
    file, pack_path,
};
use anyhow::{Context, Error};
use std::path::Path;

/// [File] (describing the content) + its [PackPath] (describing uri that the
/// file will be accessible at).
///
/// This is the main item added to the `pack`.
///
/// It can be manually created by passing `file` and `pack_path` fields or using
/// associated helpers methods.
#[derive(Debug)]
pub struct FilePackPath {
    /// The file.
    pub file: File,

    /// The path inside the `pack`, corresponding to http path parameter.
    pub pack_path: PackPath,
}
impl FilePackPath {
    /// Creates [self] by reading file relative to given base directory.
    ///
    /// Given file path (`path`) to read and base directory path creates a
    /// [self] by preparing:
    /// - [File] with [file::build_from_path] using
    ///   [file::BuildFromPathOptions].
    /// - [PackPath] with [pack_path::from_file_base_relative_path] (as relative
    ///   path between `path` and `base_directory_path`).
    ///
    /// # Examples
    ///
    /// ```
    /// # use anyhow::{anyhow, Error};
    /// # use std::path::PathBuf;
    /// # use web_static_pack_packer::{file::BuildFromPathOptions, file_pack_path::FilePackPath};
    /// #
    /// # fn main() -> Result<(), Error> {
    /// #
    /// // base directory
    /// let base_directory = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
    ///     .parent()
    ///     .ok_or_else(|| anyhow!("missing parent"))?
    ///     .join("tests")
    ///     .join("data")
    ///     .join("vcard-personal-portfolio");
    ///
    /// // file in base/directory/path/index.html should end up as /index.html
    /// let file_pack_path_1 = FilePackPath::build_from_path(
    ///     &base_directory.join("index.html"),
    ///     &base_directory,
    ///     &BuildFromPathOptions::default(),
    /// )?;
    /// assert_eq!(
    ///     file_pack_path_1.file.content_type,
    ///     "text/html; charset=utf-8"
    /// );
    /// assert_eq!(&*file_pack_path_1.pack_path, "/index.html");
    ///
    /// // file in base/directory/path/website-demo-image/desktop.png should end up as /website-demo-image/desktop.png
    /// let file_pack_path_2 = FilePackPath::build_from_path(
    ///     &base_directory.join("website-demo-image").join("desktop.png"),
    ///     &base_directory,
    ///     &BuildFromPathOptions::default(),
    /// )?;
    /// assert_eq!(
    ///     file_pack_path_2.file.content_type,
    ///     "image/png"
    /// );
    /// assert_eq!(&*file_pack_path_2.pack_path, "/website-demo-image/desktop.png");
    /// #
    /// # Ok(())
    /// # }
    /// ```
    pub fn build_from_path(
        path: &Path,
        base_directory_path: &Path,
        file_options: &file::BuildFromPathOptions,
    ) -> Result<Self, Error> {
        // strip prefix, so entry_path is relative to search root
        let file_base_relative_path = path
            .strip_prefix(base_directory_path)
            .context("resolve file_base_relative_path")?;

        // create path prefix
        let pack_path = pack_path::from_file_base_relative_path(file_base_relative_path)?;

        // read and build file
        let file = file::build_from_path(path, file_options)?;

        Ok(Self { file, pack_path })
    }
}
