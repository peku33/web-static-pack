//! Main loader module.
//! This is the part you should include in you target project if you want to read packs directly.
//! After creating a pack with cli packer tool, include this into your program with `include_bytes!` macro. Then pass it to `Loader::new()`.
//! Files may be retrieved using `get()` method.

use anyhow::{bail, Context, Error};
use std::{collections::HashMap, str};

/// File descriptor, retrieved from loader.
pub struct FileDescriptor {
    content_type: &'static str,
    etag: &'static str,
    content: &'static [u8],
    content_gzip: Option<&'static [u8]>,
}
impl FileDescriptor {
    /// Returns HTTP Content-Type.
    pub fn content_type(&self) -> &'static str {
        self.content_type
    }

    /// Returns quoted http ETag precalculated for this file.
    pub fn etag(&self) -> &'static str {
        self.etag
    }

    /// Returns original file content.
    pub fn content(&self) -> &'static [u8] {
        self.content
    }

    /// Returns gzipped file content (if available)
    pub fn content_gzip(&self) -> Option<&'static [u8]> {
        self.content_gzip
    }
}

/// Main loader. Create using `::new()` providing reference to result of `include_bytes!`.
/// Call `get()` to access files.
pub struct Loader {
    files: HashMap<&'static str, FileDescriptor>,
}
impl Loader {
    fn read_u8(rest: &mut &'static [u8]) -> Result<&'static [u8], Error> {
        #[allow(clippy::len_zero)]
        if rest.len() < 1 {
            bail!("Premature length termination");
        }
        let length = u8::from_ne_bytes(unsafe { [*rest.get_unchecked(0)] }) as usize;

        if rest.len() - 1 < length {
            bail!("Premature data termination");
        }
        let data = &rest[1..(1 + length)];

        *rest = &rest[1 + length..];

        Ok(data)
    }
    fn read_u16(rest: &mut &'static [u8]) -> Result<&'static [u8], Error> {
        if rest.len() < 2 {
            bail!("Premature length termination");
        }
        let length = u16::from_ne_bytes(unsafe { [*rest.get_unchecked(0), *rest.get_unchecked(1)] })
            as usize;

        if rest.len() - 2 < length {
            bail!("Premature data termination");
        }
        let data = &rest[2..(2 + length)];

        *rest = &rest[2 + length..];

        Ok(data)
    }
    fn read_u32(rest: &mut &'static [u8]) -> Result<&'static [u8], Error> {
        if rest.len() < 4 {
            bail!("Premature length termination");
        }
        let length = u32::from_ne_bytes(unsafe {
            [
                *rest.get_unchecked(0),
                *rest.get_unchecked(1),
                *rest.get_unchecked(2),
                *rest.get_unchecked(3),
            ]
        }) as usize;

        if rest.len() - 4 < length {
            bail!("Premature data termination");
        }
        let data = &rest[4..(4 + length)];

        *rest = &rest[4 + length..];

        Ok(data)
    }

    /// Creates a loader.
    /// Pass result of `include_bytes!` macro here.
    /// Create pack (for inclusion) with `web-static-pack-packer`.
    pub fn new(included_bytes: &'static [u8]) -> Result<Self, Error> {
        let mut rest = included_bytes;
        let mut files = HashMap::<&'static str, FileDescriptor>::new();
        while !rest.is_empty() {
            // Extract.
            let path =
                unsafe { str::from_utf8_unchecked(Self::read_u16(&mut rest).context("path")?) };
            let content_type = unsafe {
                str::from_utf8_unchecked(Self::read_u8(&mut rest).context("content_type")?)
            };
            let etag =
                unsafe { str::from_utf8_unchecked(Self::read_u8(&mut rest).context("etag")?) };
            let content = Self::read_u32(&mut rest).context("content")?;
            let content_gzip = Self::read_u32(&mut rest).context("content_gzip")?;
            let content_gzip = if !content_gzip.is_empty() {
                Some(content_gzip)
            } else {
                None
            };

            // Build FileDescriptor.
            let file_descriptor = FileDescriptor {
                content_type,
                etag,
                content,
                content_gzip,
            };

            // Push to collection.
            if files.insert(path, file_descriptor).is_some() {
                bail!("File corrupted, duplicated path: {}", path);
            }
        }
        log::info!("Loaded total {} files", files.len());
        Ok(Self { files })
    }

    /// Retrieves file from pack.
    /// The path should usually start with `/`, exactly as in URL.
    /// Returns `Some(&FileDescriptor)` if file is found, `None` otherwise.
    pub fn get(
        &self,
        path: &str,
    ) -> Option<&FileDescriptor> {
        self.files.get(path)
    }
}
