# web-static-pack-packer
Executable to build packs for web-static-pack crate
See main crate for details

[![docs.rs](https://docs.rs/web-static-pack-packer/badge.svg)](https://docs.rs/web-static-pack-packer)

## Usage
1. Install (`cargo install web-static-pack-packer`) or run locally (`cargo run`)
2. Provide positional arguments:
    - `<path>` - the directory to pack
    - `<output_file>` - name of the build pack
    - `[root_pach]` - relative path to build pack paths with. use the same as `path` to have all paths in pack root
3. Use `<output_path>` file with `web-static-pack` (loader)
