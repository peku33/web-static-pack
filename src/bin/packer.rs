use failure::{err_msg, Error};
use std::path::Path;
use web_static_pack::packer::Pack;

/// Main packer executable.
/// Run with `packer <path> <output_file> [root_path]`
/// This scans `<path>`, packs all found files to <output_file> pack
/// If no `[root_path]` is given, pack paths are relative to `<path>`, otherwise `[root_path]` is used.
fn main() -> Result<(), Error> {
    let matches = clap::App::new("web-static-pack packer")
    .arg(clap::Arg::with_name("path").help("the directory to pack").required(true))
    .arg(clap::Arg::with_name("output_file").help("name of the build pack").required(true))
    .arg(clap::Arg::with_name("root_path").help("relative path to build pack paths with. use the same as `path` to have all paths in pack root").required(false))
    .get_matches();

    let path = Path::new(matches.value_of("path").unwrap());
    if !path.is_dir() {
        return Err(err_msg("path is not a directory"));
    }

    let root_path = match matches.value_of("root_path") {
        Some(root_path) => Path::new(root_path),
        None => path,
    };
    if !root_path.is_dir() {
        return Err(err_msg("root_path is not a directory"));
    }

    let output_file = Path::new(matches.value_of("output_file").unwrap());

    let mut pack = Pack::new();
    pack.directory_add(path, root_path)?;
    pack.store(output_file)?;
    Ok(())
}
