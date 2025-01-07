//! Module containing [load] function used to convert (map) serialized `pack`
//! into [PackArchived] object.

use crate::common::{pack::PackArchived, PACK_FILE_MAGIC, PACK_FILE_VERSION};
use anyhow::{ensure, Error};
use rkyv::access_unchecked;

/// Alignment value for `serialized` in [load].
pub const ALIGN_BYTES: usize = 16;

/// Loads [PackArchived] from serialized bytes created by
/// [web-static-pack-packer](https://crates.io/crates/web-static-pack-packer).
///
/// This method is lightweight, as it does some pre-checks and then casts
/// (zero-copy deserialization) [rkyv] archived [PackArchived] to the output.
///
/// In a typical scenario the [Pack] will be created with packer in build
/// pipeline (or build.rs), then serialized to a file and stored in the working
/// / build directory.
///
/// Target application will have serialized `pack` embedded with
/// <https://docs.rs/include_bytes_aligned/latest/include_bytes_aligned/>
/// (or alternatively read from fs) and then passed to this function, that will
/// "map" it to [PackArchived].
///
/// Loaded [PackArchived] will be typically used to create
/// [crate::responder::Responder].
///
/// Please note that `serialized` must be aligned to [ALIGN_BYTES], either by
/// `include_bytes_aligned` crate (if embedding into executable) or
/// [rkyv::util::AlignedVec] (if loading from fs in runtime).
///
/// # Examples
///
/// ```ignore
/// // from workspace root:
/// // $ cargo run -- directory-single ./tests/data/vcard-personal-portfolio/ vcard-personal-portfolio.pack
///
/// // then in your application
/// static PACK_ARCHIVED_SERIALIZED: &[u8] = include_bytes_aligned!(
///     16, // == web_static_pack::loader::ALIGN_BYTES,
///     "vcard-personal-portfolio.pack"
/// );
///
/// fn main() {
///     let pack = unsafe { web_static_pack::loader::load(PACK_ARCHIVED_SERIALIZED).unwrap() };
///     // create responder from pack
///     // pass http requests to responder
///     // see crate documentation for full example
/// }
/// ```
///
/// # Safety
/// `serialized` must point to valid `pack` created with matching version of
/// packer. Underlying loader (rkyv) relies on correct file content. If invalid
/// content is provided it is going to cause undefined behavior.
pub unsafe fn load(serialized: &[u8]) -> Result<&PackArchived, Error> {
    ensure!(
        serialized.as_ptr() as usize % ALIGN_BYTES == 0,
        "invalid alignment, serialized must be aligned to {ALIGN_BYTES} bytes"
    );
    ensure!(serialized.len() > 16, "premature file end");

    // check file magic
    let file_magic_bytes: [u8; 8] = serialized[0..8].try_into()?;
    let file_magic = u64::from_ne_bytes(file_magic_bytes);
    ensure!(
        file_magic == PACK_FILE_MAGIC,
        "file magic mismatch, probably not a pack file"
    );

    // check file version
    let file_version_bytes: [u8; 8] = serialized[8..16].try_into()?;
    let file_version = u64::from_ne_bytes(file_version_bytes);
    ensure!(
        file_version == PACK_FILE_VERSION,
        "file version mismatch (got {file_version}, expected: {PACK_FILE_VERSION}) (probably pack created with different version)"
    );

    // deserialize content
    // NOTE: value passed to [access_unchecked] must be 16-aligned
    let pack = unsafe { access_unchecked::<PackArchived>(&serialized[16..]) };

    Ok(pack)
}
