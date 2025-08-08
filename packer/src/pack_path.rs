//! Pack path helpers. Contains [from_file_base_relative_path] that creates pack
//! paths from fs paths.

use crate::common::pack_path::PackPath;
use anyhow::{Error, anyhow, ensure};
use std::{
    iter,
    path::{Component, Path},
};

/// Creates pack path (eg. "/dir1/dir2/file.html") from relative fs path (eg.
/// "workdir\\dir1\\dir2\\file.html").
///
/// # Examples
///
/// ```
/// # use anyhow::Error;
/// # use std::path::PathBuf;
/// # use web_static_pack_packer::{
/// #    common::pack_path::PackPath, pack_path::from_file_base_relative_path,
/// # };
/// #
/// # fn main() -> Result<(), Error> {
/// #
/// assert_eq!(
///     from_file_base_relative_path(&PathBuf::from("path\\to\\file.txt"))?,
///     PackPath::from_string("/path/to/file.txt".to_owned()),
/// );
/// #
/// # Ok(())
/// # }
/// ```
pub fn from_file_base_relative_path(file_base_relative_path: &Path) -> Result<PackPath, Error> {
    assert!(file_base_relative_path.is_relative());

    // list of path components, eg. ["dir1", "dir2", "file.bin"]
    let file_base_relative_path_components = file_base_relative_path
        .components()
        .map(|component| {
            // we cannot handle things like '/' or '.' or '..' here
            ensure!(
                matches!(component, Component::Normal(_)),
                "relative path must not contain only standard path items, got {:?}",
                component
            );

            component
                .as_os_str()
                .to_str()
                .ok_or_else(|| anyhow!("cannot convert path component to string"))
        })
        .collect::<Result<Vec<_>, Error>>()?;

    // we add empty element at the beginning to have path starting with /
    let pack_path_string = itertools::join(
        iter::once("").chain(file_base_relative_path_components),
        "/",
    );

    // convert into pack path
    let pack_path = PackPath::from_string(pack_path_string);

    Ok(pack_path)
}

#[cfg(test)]
mod test {
    use super::from_file_base_relative_path;
    use crate::common::pack_path::PackPath;
    use std::path::{Path, PathBuf};
    use test_case::test_case;

    #[test_case(
        &PathBuf::from("somefile"),
        &PackPath::from_string("/somefile".to_owned());
        "base file path without prefix"
    )]
    #[test_case(
        &PathBuf::from("linux/like/relative/path.html"),
        &PackPath::from_string("/linux/like/relative/path.html".to_owned());
        "linux like relative path"
    )]
    #[test_case(
        &PathBuf::from("Project\\MyApp\\Application.js"),
        &PackPath::from_string("/Project/MyApp/Application.js".to_owned());
        "windows relative path"
    )]
    fn from_file_base_relative_path_returns_expected(
        path: &Path,
        expected: &PackPath,
    ) {
        assert_eq!(&from_file_base_relative_path(path).unwrap(), expected);
    }
}
