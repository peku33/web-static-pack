//! Packer executable, builds packs to be loaded with the library of this crate.
//! To run: `cargo run --example packer /path/to/your/directory/ my_pack.pack`.

mod packer;

use failure::Error;
use packer::Pack;
use std::path::Path;

fn main() -> Result<(), Error> {
    simple_logger::init().unwrap();

    let matches = clap::App::new("web-static-pack packer")
    .arg(clap::Arg::with_name("path").help("the directory to pack").required(true))
    .arg(clap::Arg::with_name("output_file").help("name of the build pack").required(true))
    .arg(clap::Arg::with_name("root_path").help("relative path to build pack paths with. use the same as `path` to have all paths in pack root").required(false))
    .get_matches();

    let path = Path::new(matches.value_of("path").unwrap());
    let root_path = match matches.value_of("root_path") {
        Some(root_path) => Path::new(root_path),
        None => Path::new(""),
    };
    let output_file = Path::new(matches.value_of("output_file").unwrap());

    let mut pack = Pack::new();
    pack.directory_add(path, root_path)?;
    pack.store(output_file)?;
    Ok(())
}
