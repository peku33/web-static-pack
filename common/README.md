# web-static-pack-common

Common crate, containing types shared between
[web-static-pack](https://crates.io/crates/web-static-pack) and
[web-static-pack-packer](https://crates.io/crates/web-static-pack-packer).

For a project documentation, examples, etc. see
[web-static-pack](https://github.com/peku33/web-static-pack).

The root type of this crate is [pack::Pack]. It's a collection (a hashmap)
of files [file::File] distinguished by [pack_path::PackPath] (a custom type
for path including some sanity checks).

web-static-pack uses [rkyv] for serialization. Each module provides a rust
native type, used during `pack` building, ex. [pack::Pack] and [rkyv]
macro-generated zero-copy loadable (aka. mmapable) representation, eg.
[pack::PackArchived], used by loader.

#### Note

There are also things called `Resolver` (eg. [pack::PackResolver]), that are
needed internally by [rkyv], but are not used directly in this project. They
should be hidden from docs.

License: MIT
