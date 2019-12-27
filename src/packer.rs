use failure::{err_msg, format_err, Error};
use mime_guess::Mime;
use std::collections::LinkedList;
use std::convert::TryInto;
use std::fs::File;
use std::io;
use std::io::{Seek, SeekFrom, Write};
use std::path::{Component, Path, PathBuf};
use walkdir::WalkDir;

struct FileDescriptor {
    pack_path: String,
    mime: Mime,
    file: File,
}
impl FileDescriptor {
    fn serialize_into<W: Write>(&mut self, write: &mut W) -> Result<(), Error> {
        // file metadata
        let file_metadata = self.file.metadata()?;
        if !file_metadata.is_file() {
            return Err(err_msg("File should be regular file, but is not..."));
        }

        // pack_path
        let pack_path_bytes = self.pack_path.as_bytes();
        // pack_path length, u16, 2 bytes
        let pack_path_bytes_length: u16 = pack_path_bytes.len().try_into()?;
        write.write(&pack_path_bytes_length.to_ne_bytes())?;
        // pack_path, length as above
        write.write(pack_path_bytes)?;

        // mime
        let mime_bytes = self.mime.as_ref().as_bytes();
        // mime length, u8, 1 byte
        let mime_bytes_length: u8 = mime_bytes.len().try_into()?;
        write.write(&mime_bytes_length.to_ne_bytes())?;
        // mime, length as above
        write.write(mime_bytes)?;

        // file
        // size as u32, should be enough
        let file_bytes_length: u32 = file_metadata.len().try_into()?;
        write.write(&file_bytes_length.to_ne_bytes())?;
        // file contents
        self.file.seek(SeekFrom::Start(0))?;
        let file_bytes_length_copied = io::copy(&mut self.file, write)?;
        if file_bytes_length_copied != file_bytes_length as u64 {
            return Err(format_err!(
                "Copy returned different number of bytes ({}) than file metadata ({})",
                file_bytes_length_copied,
                file_bytes_length
            ));
        }

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
        let file = File::open(&fs_path)?;
        let mime = mime_guess::from_path(&fs_path).first_or_octet_stream();
        self.file_descriptors.push_back(FileDescriptor {
            pack_path,
            mime,
            file,
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
