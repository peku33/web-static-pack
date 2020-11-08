//! # web-static-pack-packer
//! Executable to build packs for web-static-pack crate
//! See main crate for details
//!
//! [![docs.rs](https://docs.rs/web-static-pack-packer/badge.svg)](https://docs.rs/web-static-pack-packer)
//!
//! ## Usage
//! 1. Install (`cargo install web-static-pack-packer`) or run locally (`cargo run`)
//! 2. Provide positional arguments:
//!  - `<path>` - the directory to pack
//!  - `<output_file>` - name of the build pack
//!  - `[root_pach]` - relative path to build pack paths with. use the same as `path` to have all paths in pack root
//! 3. Use `<output_path>` file with `web-static-pack` (loader)

mod packer;

use anyhow::{Context, Error};
use log::LevelFilter;
use packer::Pack;
use simple_logger::SimpleLogger;
use std::path::Path;

fn main() -> Result<(), Error> {
    SimpleLogger::from_env()
        .with_level(LevelFilter::Info)
        .init()
        .unwrap();

    let matches = clap::App::new("web-static-pack packer")
        .arg(
            clap::Arg::with_name("path")
                .help("the directory to pack")
                .required(true),
        )
        .arg(
            clap::Arg::with_name("output_file")
                .help("name of the build pack")
                .required(true),
        )
        .arg(
            clap::Arg::with_name("root_path")
                .help("relative path to build pack paths with. use the same as `path` to have all paths in pack root")
                .required(false),
        )
        .get_matches();

    let path = Path::new(matches.value_of("path").unwrap());
    let root_path = match matches.value_of("root_path") {
        Some(root_path) => Path::new(root_path),
        None => Path::new(""),
    };
    let output_file = Path::new(matches.value_of("output_file").unwrap());

    let mut pack = Pack::new();
    pack.directory_add(path, root_path)
        .context("directory_add")?;
    pack.store(output_file).context("store")?;
    Ok(())
}
