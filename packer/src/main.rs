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
use std::path::PathBuf;

fn main() -> Result<(), Error> {
    SimpleLogger::new()
        .env()
        .with_level(LevelFilter::Info)
        .init()
        .unwrap();

    let matches = clap::builder::Command::new("web-static-pack packer")
        .arg(
            clap::builder::Arg::new("path")
                .help("the directory to pack")
                .required(true)
                .value_parser(clap::builder::PathBufValueParser::new()),
        )
        .arg(
            clap::builder::Arg::new("output_file")
                .help("name of the build pack")
                .required(true)
                .value_parser(clap::builder::PathBufValueParser::new())
                ,
        )
        .arg(
            clap::builder::Arg::new("root_path")
                .help("relative path to build pack paths with. use the same as `path` to have all paths in pack root")
                .required(false)
                .value_parser(clap::builder::PathBufValueParser::new())
        )
        .get_matches();

    let path = matches.get_one::<PathBuf>("path").cloned().unwrap();
    let output_file = matches.get_one::<PathBuf>("output_file").cloned().unwrap();
    let root_path = matches
        .get_one::<PathBuf>("root_path")
        .cloned()
        .unwrap_or_else(PathBuf::new);

    let mut pack = Pack::new();
    pack.directory_add(&path, &root_path)
        .context("directory_add")?;
    pack.store(&output_file).context("store")?;
    Ok(())
}
