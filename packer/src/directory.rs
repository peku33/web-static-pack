//! Directory helpers. Contains [search] function, used to gather files from
//! directory recursively.

use crate::{file, file_pack_path};
use anyhow::{Context, Error};
use std::path::Path;
use walkdir::WalkDir;

/// Settings for [search] function.
///
/// If not sure what to set here, use [Default].
#[derive(Debug)]
pub struct SearchOptions {
    /// Whether to follow links while traversing directories.
    pub follow_links: bool,
}
impl Default for SearchOptions {
    fn default() -> Self {
        Self { follow_links: true }
    }
}

/// Searches fs recursively and builds [file_pack_path::FilePackPath] for each
/// file.
///
/// Traverses directory specified in `path` using [SearchOptions]. Builds all
/// found files as [file_pack_path::FilePackPath] using
/// [file::BuildFromPathOptions]. Paths are created by stripping `path` from
/// full file path.
///
/// # Examples
///
/// ```
/// # use anyhow::Error;
/// # use std::{collections::HashMap, path::PathBuf};
/// # use web_static_pack_packer::{
/// #     directory::{search, SearchOptions},
/// #     file::BuildFromPathOptions,
/// # };
/// #
/// # fn main() -> Result<(), Error> {
/// #
/// // traverse directory from tests
/// let files = search(
///     &PathBuf::from(env!("CARGO_MANIFEST_DIR"))
///         .parent()
///         .unwrap()
///         .join("tests")
///         .join("data")
///         .join("vcard-personal-portfolio")
///         .join("assets"),
///     &SearchOptions::default(),
///     &BuildFromPathOptions::default(),
/// )?;
///
/// // group files into hashmap {pack_path: file}
/// let files_by_path = files
///     .into_vec()
///     .into_iter()
///     .map(|file_pack_path| (file_pack_path.pack_path, file_pack_path.file))
///     .collect::<HashMap<_, _>>();
///
/// // verify necessary files were added
/// assert!(files_by_path.contains_key("/css/style.css"));
/// assert!(files_by_path.contains_key("/js/script.js"));
/// assert!(!files_by_path.contains_key("/index.html"));
/// #
/// # Ok(())
/// # }
/// ```
pub fn search(
    path: &Path,
    options: &SearchOptions,
    file_build_options: &file::BuildFromPathOptions,
) -> Result<Box<[file_pack_path::FilePackPath]>, Error> {
    let file_paths = WalkDir::new(path)
        .follow_links(options.follow_links)
        .into_iter()
        .map(|file_entry| {
            // detect search errors
            let file_entry = file_entry?;

            // we are interested in files only
            // if follow_links is true, this will be resolved as link target
            if !file_entry.file_type().is_file() {
                return Ok(None);
            }

            // build file
            let file_pack_path = file_pack_path::FilePackPath::build_from_path(
                file_entry.path(),
                path,
                file_build_options,
            )
            .with_context(|| file_entry.path().to_string_lossy().into_owned())?;

            // yield for processing
            Ok(Some(file_pack_path))
        })
        .filter_map(|entry_result| entry_result.transpose()) // strips Ok(None)
        .collect::<Result<Box<[_]>, Error>>()?;

    Ok(file_paths)
}
