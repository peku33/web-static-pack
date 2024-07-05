//! Pack helpers. Contains [Builder], builder for [Pack].

use crate::{
    common::{file::File, pack::Pack, pack_path::PackPath, PACK_FILE_MAGIC, PACK_FILE_VERSION},
    file_pack_path::FilePackPath,
};
use anyhow::{bail, Error};
use rkyv::{
    ser::{
        serializers::{AllocScratch, CompositeSerializer, WriteSerializer},
        Serializer,
    },
    AlignedVec, Infallible,
};
use std::{
    collections::{hash_map, HashMap},
    fs, io,
    path::Path,
};

/// Main builder for `pack`. Inside it keeps list of [File] under respective
/// [PackPath].
#[derive(Debug)]
pub struct Builder {
    files_by_pack_path: HashMap<PackPath, File>,
}
impl Builder {
    /// Creates empty [self] to be filled with files.
    pub fn new() -> Self {
        let files_by_pack_path = HashMap::<PackPath, File>::new();

        Self { files_by_pack_path }
    }

    /// Adds file to the `pack`.
    pub fn file_pack_path_add(
        &mut self,
        file_pack_path: FilePackPath,
    ) -> Result<(), Error> {
        let entry = match self.files_by_pack_path.entry(file_pack_path.pack_path) {
            hash_map::Entry::Occupied(_entry) => {
                bail!("file on specified path already exist");
            }
            hash_map::Entry::Vacant(entry) => entry,
        };

        entry.insert(file_pack_path.file);

        Ok(())
    }

    /// Adds collection of files to the `pack`.
    pub fn file_pack_paths_add(
        &mut self,
        file_pack_paths: impl IntoIterator<Item = FilePackPath>,
    ) -> Result<(), Error> {
        file_pack_paths
            .into_iter()
            .try_for_each(|file_pack_path| self.file_pack_path_add(file_pack_path))?;

        Ok(())
    }

    /// Finalizes to builder, returning built [Pack].
    pub fn finalize(self) -> Pack {
        Pack {
            files_by_path: self.files_by_pack_path,
        }
    }
}

fn store(
    pack: &Pack,
    mut writer: impl io::Write,
) -> Result<(), Error> {
    // NOTE: we rely on `pack` being 16-aligned two u64 at the beginning keeps
    // alignment unchanged, but adding anything here will break it.
    writer.write_all(&PACK_FILE_MAGIC.to_ne_bytes())?;
    writer.write_all(&PACK_FILE_VERSION.to_ne_bytes())?;

    let mut inner_serializer = CompositeSerializer::new(
        WriteSerializer::new(&mut writer),
        AllocScratch::new(),
        Infallible,
    );
    inner_serializer.serialize_value(pack)?;

    Ok(())
}

/// Serializes `pack` to [AlignedVec]. Serialized data can be used with `load`
/// method of loader.
pub fn store_memory(pack: &Pack) -> Result<AlignedVec, Error> {
    let mut buffer = AlignedVec::new();
    store(pack, &mut buffer)?;
    Ok(buffer)
}

/// Serializes `pack` to given file path Serialized data can be used with `load`
/// method of loader.
pub fn store_file(
    pack: &Pack,
    path: &Path,
) -> Result<(), Error> {
    let mut file = fs::File::create(path)?;

    store(pack, &mut file)?;

    file.sync_all()?;
    drop(file);

    Ok(())
}
