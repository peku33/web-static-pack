//! File helpers. Contains [build_from_path] and [build_from_content] functions
//! to create a [File] from fs / memory content.

use crate::common::{cache_control::CacheControl, file::File};
use anyhow::Error;
use brotli::enc::BrotliEncoderParams;
use flate2::{write::GzEncoder, Compression};
use sha3::{Digest, Sha3_256};
use std::{
    fs,
    io::{Cursor, Write},
    path::Path,
};

/// Options when preparing file in [build_from_path].
///
/// If not sure what to set here, use [Default].
#[derive(Debug)]
pub struct BuildFromPathOptions {
    /// Try adding gzipped version of file. If set to true, it may still not be
    /// added (ex. in case gzipped version is larger than raw).
    pub use_gzip: bool,
    /// Try adding brotli version of file. If set to true, it may still not be
    /// added (ex. in case gzipped version is larger than raw).
    pub use_brotli: bool,

    /// Override `content-type` header for this file.
    pub content_type_override: Option<String>,
    /// Override [CacheControl] for this file.
    pub cache_control_override: Option<CacheControl>,
}
impl Default for BuildFromPathOptions {
    fn default() -> Self {
        Self {
            use_gzip: true,
            use_brotli: true,
            content_type_override: None,
            cache_control_override: None,
        }
    }
}

/// Creates a [File] by reading file from fs, specified by `path`.
///
/// Inside file will be read, `content-type` determined
/// from extension and then passed to [build_from_content].
///
/// # Examples
///
/// ```
/// # use anyhow::{anyhow, Error};
/// # use std::path::PathBuf;
/// # use web_static_pack_packer::file::{build_from_path, BuildFromPathOptions};
/// #
/// # fn main() -> Result<(), Error> {
/// #
/// let file = build_from_path(
///     &PathBuf::from(env!("CARGO_MANIFEST_DIR"))
///         .parent()
///         .ok_or_else(|| anyhow!("missing parent"))?
///         .join("tests")
///         .join("data")
///         .join("vcard-personal-portfolio")
///         .join("index.html"),
///     &BuildFromPathOptions::default(),
/// )?;
/// assert_eq!(file.content_type, "text/html; charset=utf-8");
/// #
/// # Ok(())
/// # }
/// ```
pub fn build_from_path(
    path: &Path,
    options: &BuildFromPathOptions,
) -> Result<File, Error> {
    // read content
    let content = content_from_path(path)?;

    // use user provided content type if set, otherwise guess from path
    let content_type = if let Some(content_type) = &options.content_type_override {
        content_type.clone()
    } else {
        content_type_from_path(path)
    };

    // pass to inner builder
    let file = build_from_content(
        content,
        content_type,
        &BuildFromContentOptions {
            use_gzip: options.use_gzip,
            use_brotli: options.use_brotli,
            cache_control_override: options.cache_control_override,
        },
    );

    Ok(file)
}

/// Options when preparing file in [build_from_content].
///
/// If not sure what to set here, use [Default].
#[derive(Debug)]
pub struct BuildFromContentOptions {
    /// Try adding gzipped version of content. If set to true, it may still not
    /// be added (ex. in case gzipped version is larger than raw).
    pub use_gzip: bool,
    /// Try adding brotli version of content. If set to true, it may still not
    /// be added (ex. in case gzipped version is larger than raw).
    pub use_brotli: bool,

    /// Override [CacheControl] for this file.
    pub cache_control_override: Option<CacheControl>,
}
impl Default for BuildFromContentOptions {
    fn default() -> Self {
        Self {
            use_gzip: true,
            use_brotli: true,
            cache_control_override: None,
        }
    }
}

/// Creates a [File] from provided raw content and `content-type`.
///
/// Inside compressed versions will be created (according to options), `ETag`
/// calculated and [CacheControl] set.
///
/// When setting `content_type` remember to set charset for text files, eg.
/// `text/plain; charset=utf-8`.
///
/// # Examples
///
/// ```
/// # use anyhow::Error;
/// # use std::path::PathBuf;
/// # use web_static_pack_packer::file::{build_from_content, BuildFromContentOptions};
/// #
/// # fn main() -> Result<(), Error> {
/// #
/// let file = build_from_content(
///     Box::new(*b"<html>Hello World!</html>"),
///     "text/html; charset=utf-8".to_owned(),
///     &BuildFromContentOptions::default(),
/// );
/// assert!(file.content_gzip.is_none()); // too short for gzip
/// assert!(file.content_brotli.is_none()); // too short for gzip
/// assert_eq!(&*file.content, b"<html>Hello World!</html>");
/// assert_eq!(file.content_type, "text/html; charset=utf-8");
/// #
/// # Ok(())
/// # }
/// ```
pub fn build_from_content(
    content: Box<[u8]>,
    content_type: String,
    options: &BuildFromContentOptions,
) -> File {
    let content_gzip = if options.use_gzip {
        content_gzip_from_content(&content)
    } else {
        None
    };
    let content_brotli = if options.use_brotli {
        content_brotli_from_content(&content)
    } else {
        None
    };

    let etag = etag_from_content(&content);
    let cache_control = if let Some(cache_control) = &options.cache_control_override {
        *cache_control
    } else {
        // we assume, that content is "static" and provide max caching opportunity
        CacheControl::MaxCache
    };

    File {
        content,
        content_gzip,
        content_brotli,
        content_type,
        etag,
        cache_control,
    }
}

/// Builds content by reading given file.
fn content_from_path(path: &Path) -> Result<Box<[u8]>, Error> {
    let content = fs::read(path)?.into_boxed_slice();

    Ok(content)
}
/// Builds gzip compressed version of `content`.
///
/// Returns [None] if there is no sense in having compressed version in `pack`
/// (eg. compressed is larger than raw).
fn content_gzip_from_content(content: &[u8]) -> Option<Box<[u8]>> {
    // no sense in compressing empty files
    if content.is_empty() {
        return None;
    }

    let mut content_gzip = GzEncoder::new(Vec::new(), Compression::best());
    content_gzip.write_all(content).unwrap();
    let content_gzip = content_gzip.finish().unwrap().into_boxed_slice();

    // if gzip is longer then original value - it makes no sense to store it
    if content_gzip.len() >= content.len() {
        return None;
    }

    Some(content_gzip)
}
/// Builds brotli compressed version of `content`.
///
/// Returns [None] if there is no sense in having compressed version in `pack`
/// (eg. compressed is larger than raw).
fn content_brotli_from_content(content: &[u8]) -> Option<Box<[u8]>> {
    // no sense in compressing empty files
    if content.is_empty() {
        return None;
    }

    let mut content_cursor = Cursor::new(content);
    let mut content_brotli = Vec::new();
    let content_brotli_length = brotli::BrotliCompress(
        &mut content_cursor,
        &mut content_brotli,
        &BrotliEncoderParams::default(),
    )
    .unwrap();
    let content_brotli = content_brotli.into_boxed_slice();
    assert!(content_brotli.len() == content_brotli_length);

    // if brotli is longer then original value - it makes no sense to store it
    if content_brotli.len() >= content.len() {
        return None;
    }

    Some(content_brotli)
}

/// Guesses `content-type` from file path.
///
/// Only path is used, file content is not read. If file type cannot be guessed,
/// returns "application/octet-stream". For text files (eg. plain, html, css,
/// js, etc) it assumes utf-8 encoding.
fn content_type_from_path(path: &Path) -> String {
    let mut content_type = mime_guess::from_path(path)
        .first_or_octet_stream()
        .as_ref()
        .to_owned();

    // NOTE: temporary workaround for https://github.com/abonander/mime_guess/issues/90
    if content_type == "application/javascript" {
        content_type = "text/javascript".to_owned();
    }

    if content_type.starts_with("text/") {
        content_type.push_str("; charset=utf-8");
    }
    content_type
}
/// Calculates `ETag` header from file contents.
fn etag_from_content(content: &[u8]) -> String {
    let mut etag = Sha3_256::new();
    etag.update(content);
    let etag = etag.finalize();
    let etag = format!("\"{:x}\"", &etag); // `ETag` as "quoted" hex sha3. Quote is required by standard
    etag
}

#[cfg(test)]
mod test {
    use super::{
        build_from_content, content_brotli_from_content, content_gzip_from_content,
        content_type_from_path, etag_from_content, BuildFromContentOptions,
    };
    use crate::common::file::File;
    use std::path::{Path, PathBuf};
    use test_case::test_case;

    #[test]
    fn build_from_content_returns_expected() {
        let content_original = b"lorem ipsum lorem ipsum lorem ipsum lorem ipsum lorem ipsum";
        let content_type_original = "text/plain; charset=utf-8";

        let file = build_from_content(
            Box::new(*content_original),
            content_type_original.to_owned(),
            &BuildFromContentOptions::default(),
        );

        let File {
            content,
            content_gzip,
            content_brotli,
            content_type,
            // implementation dependant
            // etag,
            // cache_control,
            ..
        } = file;
        assert_eq!(&*content, content_original);
        assert_eq!(&*content_gzip.unwrap(), b"\x1f\x8b\x08\x00\x00\x00\x00\x00\x02\xff\x95\xc6\x41\x09\x00\x00\x08\x03\xc0\x2a\x2b\xe7\x43\xd8\x50\x14\xfb\x9b\x61\xbf\x63\x4d\x08\xd9\x7b\x02\x3d\x3f\x1e\x08\x7c\xb8\x3b\x00\x00\x00");
        assert_eq!(&*content_brotli.unwrap(), b"\x1b\x3a\x00\xf8\x1d\xa9\x53\x9f\xbb\x70\x9d\xc6\xf6\x06\xa7\xda\xe4\x1a\xa4\x6c\xae\x4e\x18\x15\x0b\x98\x56\x70\x03");
        assert_eq!(content_type, content_type_original);

        // implementation dependant
        // assert_eq!(etag, "");
        // assert_eq!(cache_control, CacheControl::MaxCache);
    }

    #[test]
    fn empty_should_not_be_compressed() {
        assert!(content_gzip_from_content(&[]).is_none());
        assert!(content_brotli_from_content(&[]).is_none());
    }

    #[test]
    fn content_gzip_from_content_returns_expected() {
        assert_eq!(
            content_gzip_from_content(b"lorem ipsum lorem ipsum lorem ipsum lorem ipsum lorem ipsum").as_deref(),
            Some(b"\x1f\x8b\x08\x00\x00\x00\x00\x00\x02\xff\x95\xc6\x41\x09\x00\x00\x08\x03\xc0\x2a\x2b\xe7\x43\xd8\x50\x14\xfb\x9b\x61\xbf\x63\x4d\x08\xd9\x7b\x02\x3d\x3f\x1e\x08\x7c\xb8\x3b\x00\x00\x00".as_slice())
        );
    }

    #[test]
    fn content_brotli_from_content_returns_expected() {
        assert_eq!(
            content_brotli_from_content(b"lorem ipsum lorem ipsum lorem ipsum lorem ipsum lorem ipsum").as_deref(),
            Some(b"\x1b\x3a\x00\xf8\x1d\xa9\x53\x9f\xbb\x70\x9d\xc6\xf6\x06\xa7\xda\xe4\x1a\xa4\x6c\xae\x4e\x18\x15\x0b\x98\x56\x70\x03".as_slice())
        );
    }

    #[test]
    fn etag_from_content_returns_expected() {
        // two identical payloads should produce identical `ETag`
        // two different payloads should produce different `ETag`

        assert_eq!(
            etag_from_content(b"lorem ipsum"),
            etag_from_content(b"lorem ipsum")
        );
        assert_ne!(
            etag_from_content(b"lorem ipsum"),
            etag_from_content(b"ipsum lorem")
        );
    }

    #[test_case(
        &PathBuf::from("a.html"),
        "text/html; charset=utf-8";
        "html file"
    )]
    #[test_case(
        &PathBuf::from("directory/styles.css"),
        "text/css; charset=utf-8";
        "css file in directory"
    )]
    #[test_case(
        &PathBuf::from("/root/dir/script.00ff00.js"),
        "text/javascript; charset=utf-8";
        "js file, full path, with some hex in stem"
    )]
    #[test_case(
        &PathBuf::from("C:\\Users\\example\\Images\\SomeImage.webp"),
        "image/webp";
        "webp image in windows style path format"
    )]
    fn content_type_from_path_returns_expected(
        path: &Path,
        expected: &str,
    ) {
        assert_eq!(content_type_from_path(path), expected);
    }
}
