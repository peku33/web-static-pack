//! web-static-pack-packer is the "builder" (1st stage) part of the
//! [web-static-pack](https://github.com/peku33/web-static-pack)
//! project. See project page for a general idea how two parts cooperate.
//!
//! The goal of the packer part is to collect your directories / files / memory
//! slices, precalculate things like `ETag`, compressed versions (`gzip`,
//! `brotli`) and store them as a single file (called `pack`). Your target
//! application will include (ex. with
//! <https://docs.rs/include_bytes_aligned/latest/include_bytes_aligned/>
//! ) and "load" / "parse" `pack` during runtime using
//! [web-static-pack](https://crates.io/crates/web-static-pack)
//! (the loader part) and (possibly) serve it with a web server of your choice.
//!
//! This crate is usually used in build script / CI / build.rs stage, not in
//! your target application. It's used to create a `pack` from list of files
//! (like your GUI app / images / other assets) to be later loaded by your app
//! and served with a web server.
//!
//! This crate can be used in two ways:
//! - As a standalone application, installed with `cargo install`, this is the
//!   preferred way if you are using build scripts, CI pipeline etc.
//! - As a library, imported to your project, this is a way to go if you want to
//!   use it in build.rs of your target application or go with some really
//!   custom approach
//!
//! # Using as a standalone application
//!
//! ## Install (or update to matching version)
//! - Either install it with `$ cargo install web-static-pack-packer` and use
//!   shell command `$ web-static-pack-packer [PARAMS]...`
//! - Or clone repo, go into `packer` directory, `$ cargo run --release --
//!   [PARAMS]...`. (please note `--` which marks end of arguments for cargo run
//!   and beginning of arguments for the application).
//!
//! For the purpose of this example, the first option is assumed, with
//! `web-static-pack-packer` command available.
//!
//! ## Create a `pack`
//! `web-static-pack-packer` provides up to date documentation with `$
//! web-static-pack-packer --help`. Application is built around subcommands to
//! cover basic scenarios:
//! - `directory-single [OPTIONS] <INPUT_DIRECTORY_PATH> <OUTPUT_FILE_PATH>`
//!   will create a `pack` from a single directory. This is the most common
//!   scenario, for example when you have a web application built into
//!   `./gui/build` directory and you want to have it served with your app.
//! - `files-cmd [OPTIONS] <OUTPUT_FILE_PATH> <INPUT_BASE_DIRECTORY_PATH>
//!   [INPUT_FILE_PATHS]...` lets you specify all files from command line in
//!   `xargs` style. base directory path is used as a root for building relative
//!   paths inside a `pack`.
//! - `files-stdin [OPTIONS] <INPUT_BASE_DIRECTORY_PATH> <OUTPUT_FILE_PATH>`
//!   lets you provide list of files from stdin.
//!
//! ### Examples
//! Let's say you have a `vcard-personal-portfolio` directory containing your
//! web project (available in tests/data/ in repository). Directory structure
//! looks like:
//! ```text
//! vcard-personal-portfolio
//! |   index.html
//! |   index.txt
//! +---assets
//! |   +---css
//! |   |       style.css
//! |   +---images
//! |   |       <some files>.png
//! |   \---js
//! |           script.js
//! \---website-demo-image
//!         desktop.png
//!         mobile.png
//! ```
//! By running:
//! ```text
//! $ web-static-pack-packer \
//!     directory-single \
//!     ./vcard-personal-portfolio \
//!     ./vcard-personal-portfolio.pack
//! ```
//! a new file `vcard-personal-portfolio.pack` will be created, containing all
//! files, so that `GET /index.html` or `GET /assets/css/tyle.css` or `GET
//! /website-demo-image/mobile.png` will be correctly resolved.
//!
//! In the next step, the `vcard-personal-portfolio.pack` should be used by
//! [web-static-pack](https://crates.io/crates/web-static-pack)
//! (the loader part) to serve it from your app.
//!
//! # Using as a library
//! When using as a library, you are most likely willing to create a loadable
//! (by the loader) `pack`, by using [pack::Builder].
//!
//! You will need to add [file_pack_path::FilePackPath] (file + path) objects to
//! the builder, which you can obtain by:
//! - Manually constructing the object from [common::pack_path::PackPath] and
//!   [common::file::File] (obtained from fs [file::build_from_path] or memory
//!   slice [file::build_from_content]).
//! - Reading single file with [file_pack_path::FilePackPath::build_from_path].
//! - Automatic search through fs with [directory::search].
//!
//! When all files are added to the builder, you will need to finalize it and
//! either write to fs (to have it included in your target application) with
//! [pack::store_file] or (mostly for test purposes) serialize to memory with
//! [pack::store_memory].
//!
//! ### Examples
//! This example will do exactly the same as one for application scenario:
//! ```no_run
//! # use anyhow::Error;
//! # use std::path::PathBuf;
//! # use web_static_pack_packer::{
//! #     directory::{search, SearchOptions},
//! #     file::BuildFromPathOptions,
//! #     pack::{store_file, Builder},
//! # };
//!
//! # fn main() -> Result<(), Error> {
//! // start with empty pack builder
//! let mut pack = Builder::new();
//!
//! // add files with directory search and default options
//! pack.file_pack_paths_add(search(
//!     &PathBuf::from("vcard-personal-portfolio"),
//!     &SearchOptions::default(),
//!     &BuildFromPathOptions::default(),
//! )?)?;
//!
//! // finalize the builder, obtain pack
//! let pack = pack.finalize();
//!
//! // store (serialize `pack` to the fs) to be included in the target app
//! store_file(&pack, &PathBuf::from("vcard-personal-portfolio.pack"))?;
//! # Ok(())
//! # }
//! ```
//!
//! For more examples browse through modules of this crate.

#![allow(clippy::new_without_default)]
#![warn(missing_docs)]

pub use web_static_pack_common as common;

pub mod directory;
pub mod file;
pub mod file_pack_path;
pub mod pack;
pub mod pack_path;
