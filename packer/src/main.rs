//! Main packer executable, to be used as cli tool. For help run this command
//! with `-h`.

#![warn(missing_docs)]

use anyhow::{Context, Error};
use clap::{Args, Parser, Subcommand};
use std::{io::stdin, path::PathBuf};
use web_static_pack_packer::{directory, file, file_pack_path, pack};

#[derive(Parser, Debug)]
#[command(version, about)]
struct Arguments {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Args, Debug)]
struct FileGlobalOptions {
    /// Add gzipped version of file to the `pack`. If not set, uses sane
    /// defaults.
    #[arg(long)]
    pub use_gzip: Option<bool>,
    /// Add brotli compressed version of file to the `pack`. If not set, uses
    /// sane defaults.
    #[arg(long)]
    pub use_brotli: Option<bool>,
}
impl FileGlobalOptions {
    pub fn into_file_build_from_path_options(self) -> file::BuildFromPathOptions {
        let mut file_build_from_path_options = file::BuildFromPathOptions::default();

        if let Some(use_gzip) = self.use_gzip {
            file_build_from_path_options.use_gzip = use_gzip;
        }

        if let Some(use_brotli) = self.use_brotli {
            file_build_from_path_options.use_brotli = use_brotli;
        }

        file_build_from_path_options
    }
}

#[derive(Subcommand, Debug)]
enum Command {
    /// Creates a single `pack` from recursively searching through single
    /// directory.
    ///
    /// Please note that all found files are added, including hidden files
    /// (starting with `.` on unix and with certain flags on windows).
    DirectorySingle {
        #[command(flatten)]
        file_global_options: FileGlobalOptions,

        /// Whether to follow links while traversing directories. If not set,
        /// uses sane defaults.
        #[arg(long)]
        follow_links: Option<bool>,

        /// The directory to be added to the `pack`.
        input_directory_path: PathBuf,

        /// Output `pack` path.
        output_file_path: PathBuf,
    },
    /// Creates `pack` from options and list of files supplied through command
    /// line.
    FilesCmd {
        #[command(flatten)]
        file_global_options: FileGlobalOptions,

        /// Output `pack` path.
        output_file_path: PathBuf,

        /// Base directory path, used to resolve relative for file inside
        /// `pack`. All added files must be inside this directory.
        input_base_directory_path: PathBuf,

        /// List of files to be added to the `pack`.
        input_file_paths: Vec<PathBuf>,
    },
    /// Creates `pack` from options supplied through command line and list of
    /// files from stdin.
    FilesStdin {
        #[command(flatten)]
        file_global_options: FileGlobalOptions,

        /// Base directory path, used to resolve relative for file inside
        /// `pack`. All added files must be inside this directory.
        input_base_directory_path: PathBuf,

        /// Output `pack` path.
        output_file_path: PathBuf,
    },
}

fn main() -> Result<(), Error> {
    let arguments = Arguments::parse();

    match arguments.command {
        Command::DirectorySingle {
            file_global_options,
            follow_links,
            input_directory_path,
            output_file_path,
        } => {
            let mut directory_search_options = directory::SearchOptions::default();
            if let Some(follow_links) = follow_links {
                directory_search_options.follow_links = follow_links;
            }

            let file_build_from_path_options =
                file_global_options.into_file_build_from_path_options();

            let mut pack_builder = pack::Builder::new();
            for file_pack_path in directory::search(
                &input_directory_path,
                &directory_search_options,
                &file_build_from_path_options,
            )? {
                // TODO: provide information which file produced error
                pack_builder.file_pack_path_add(file_pack_path)?
            }

            let pack = pack_builder.finalize();
            pack::store_file(&pack, &output_file_path)?;
        }
        Command::FilesCmd {
            file_global_options,
            output_file_path,
            input_base_directory_path,
            input_file_paths,
        } => {
            let file_build_from_path_options =
                file_global_options.into_file_build_from_path_options();

            let mut pack_builder = pack::Builder::new();
            for input_file_path in input_file_paths {
                // TODO: move this into try block with shared context
                let input_file_error_context = || input_file_path.to_string_lossy().into_owned();

                let file_pack_path = file_pack_path::FilePackPath::build_from_path(
                    &input_file_path,
                    &input_base_directory_path,
                    &file_build_from_path_options,
                )
                .with_context(input_file_error_context)?;

                pack_builder
                    .file_pack_path_add(file_pack_path)
                    .with_context(input_file_error_context)?;
            }

            let pack = pack_builder.finalize();
            pack::store_file(&pack, &output_file_path)?;
        }
        Command::FilesStdin {
            file_global_options,
            input_base_directory_path,
            output_file_path,
        } => {
            let file_build_from_path_options =
                file_global_options.into_file_build_from_path_options();

            let mut pack_builder = pack::Builder::new();
            for input_file_path in stdin().lines() {
                let input_file_path = PathBuf::from(input_file_path?);

                // TODO: move this into try block with shared context
                let input_file_error_context = || input_file_path.to_string_lossy().into_owned();

                let file_pack_path = file_pack_path::FilePackPath::build_from_path(
                    &input_file_path,
                    &input_base_directory_path,
                    &file_build_from_path_options,
                )
                .with_context(input_file_error_context)?;

                pack_builder
                    .file_pack_path_add(file_pack_path)
                    .with_context(|| input_file_path.to_string_lossy().into_owned())?;
            }

            let pack = pack_builder.finalize();
            pack::store_file(&pack, &output_file_path)?;
        }
    }

    Ok(())
}
