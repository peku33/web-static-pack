use bytes::Bytes;
use failure::{err_msg, Error};
use libflate::gzip;
use sha3::{Digest, Sha3_256};
use std::collections::LinkedList;
use std::convert::TryInto;
use std::fs;
use std::fs::File;
use std::io::Write;
use std::path::{Component, Path, PathBuf};
use walkdir::WalkDir;

struct FileDescriptor {
    pack_path: String,
    content_type: String,
    etag: String,
    content: Bytes,
    content_gzip: Option<Bytes>,
}
impl FileDescriptor {
    fn serialize_into<W: Write>(&mut self, write: &mut W) -> Result<(), Error> {
        // pack_path
        let pack_path_bytes = self.pack_path.as_bytes();
        // pack_path length, u16, 2 bytes
        let pack_path_bytes_length: u16 = pack_path_bytes.len().try_into()?;
        write.write_all(&pack_path_bytes_length.to_ne_bytes())?;
        // pack_path, length as above
        write.write_all(pack_path_bytes)?;

        // content_type
        let content_type_bytes = self.content_type.as_bytes();
        // content_type length, u8, 1 byte
        let content_type_bytes_length: u8 = content_type_bytes.len().try_into()?;
        write.write_all(&content_type_bytes_length.to_ne_bytes())?;
        // content_type, length as above
        write.write_all(content_type_bytes)?;

        // etag
        let etag_bytes = self.etag.as_bytes();
        // etag length, u8, 1 byte
        let etag_bytes_length: u8 = etag_bytes.len().try_into()?;
        write.write_all(&etag_bytes_length.to_ne_bytes())?;
        // etag, length as above
        write.write_all(etag_bytes)?;

        // content
        // size as u32, should be enough
        let content_bytes_length: u32 = self.content.len().try_into()?;
        write.write_all(&content_bytes_length.to_ne_bytes())?;
        // content, length as above
        write.write_all(&self.content)?;

        // content_gzip, optional
        // size as u32, should be enough
        let content_bytes_gzip_length: u32 = match self.content_gzip.as_ref() {
            Some(content_gzip) => content_gzip.len().try_into()?,
            None => 0,
        };
        write.write_all(&content_bytes_gzip_length.to_ne_bytes())?;
        // content_gzip, if available
        if let Some(ref content_gzip) = self.content_gzip {
            write.write_all(content_gzip)?;
        }

        // All done
        Ok(())
    }
}

pub struct Pack {
    file_descriptors: LinkedList<FileDescriptor>,
}
impl Pack {
    pub fn new() -> Self {
        Pack {
            file_descriptors: LinkedList::new(),
        }
    }
    pub fn file_add(&mut self, fs_path: PathBuf, pack_path: String) -> Result<(), Error> {
        log::info!(
            "Packing file {} -> {}",
            fs_path.as_path().to_string_lossy(),
            pack_path
        );

        // content
        let content = Bytes::from(fs::read(&fs_path)?);

        // content_gzip
        let mut content_gzip = gzip::Encoder::new(Vec::new())?;
        content_gzip.write_all(&content)?;
        let content_gzip = Bytes::from(content_gzip.finish().into_result()?);
        let content_gzip = if content_gzip.len() < content.len() {
            Some(content_gzip)
        } else {
            None
        };

        // content_type
        let mut content_type = mime_guess::from_path(&fs_path)
            .first_or_octet_stream()
            .as_ref()
            .to_owned();
        if content_type.starts_with("text/") {
            content_type.push_str("; charset=UTF-8");
        }

        // etag
        let mut etag = Sha3_256::new();
        etag.input(&content);
        let etag = etag.result();
        let etag = format!("\"{:x}\"", &etag); // ETag as "quoted" hex sha3

        // Info
        log::info!(
            "Packed {}: content_type={}, etag={}, content.len={}, content_gzip.len={:?}",
            pack_path,
            content_type,
            etag,
            content.len(),
            content_gzip.as_ref().map(|content_gzip| content_gzip.len())
        );

        // FileDescriptor
        self.file_descriptors.push_back(FileDescriptor {
            pack_path,
            content_type,
            etag,
            content,
            content_gzip,
        });

        Ok(())
    }
    pub fn directory_add(&mut self, fs_path: &Path, pack_path_prefix: &Path) -> Result<(), Error> {
        let walk_dir = WalkDir::new(fs_path).follow_links(true);
        for entry in walk_dir {
            let entry = entry?;

            // Strip directories
            if entry.file_type().is_dir() {
                continue;
            }

            // Extract path from record
            let entry_path = entry.into_path();

            // Strip fs_path from entry path (make it relative to fs_path)
            // Add pack_path_prefix
            let relative_path = pack_path_prefix.join(entry_path.strip_prefix(fs_path)?);

            // Convert directory notation to linux-like
            let pack_path_components = relative_path
                .components()
                .filter(|component| match component {
                    Component::RootDir => false,
                    _ => true,
                })
                .map(|component| {
                    component
                        .as_os_str()
                        .to_str()
                        .ok_or(err_msg("Cannot convert path component to string"))
                })
                .collect::<Result<Vec<_>, _>>()?;
            let pack_path = itertools::join([""].iter().chain(pack_path_components.iter()), "/");

            // Add file to pack
            self.file_add(entry_path, pack_path)?;
        }
        Ok(())
    }
    pub fn store(&mut self, path: &Path) -> Result<(), Error> {
        let mut file = File::create(&path)?;
        for file_descriptor in self.file_descriptors.iter_mut() {
            file_descriptor.serialize_into(&mut file)?;
        }
        Ok(())
    }
}
