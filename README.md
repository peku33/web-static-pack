# web-static-pack
web-static-pack is a set of tools for embedding static resources (GUI, assets, images, styles, html) inside your app, to be later served with a http server of your choice (like `hyper`).

It consists of two parts:
- [web-static-pack-packer](https://crates.io/crates/web-static-pack-packer) (aka "packer") - a standalone application (can be used as a library) used to serialize your assets into single file, called `pack`. It will usually be used before you build your target application (eg. in build script / CI / build.rs). During creation of a `pack` all heavy computations are done, eg. calculating `ETag`, compressed (`gzip`, `brotli`) versions, mime guessing etc. As a result a `pack` file is created, to be used by the next part.
- [web-static-pack](https://crates.io/crates/web-static-pack) (aka "loader") - a library to include in your target application that will read the `pack` (preferably included in the application with <https://docs.rs/include_bytes_aligned/latest/include_bytes_aligned/>). Then `pack` can be used to form a `http` `service` (a function taking a request (parts) and returning response) serving files from the `pack`.

## Features
- Precomputed (in "packer") `ETag` (using `sha3`), compressed bodies (in `gzip` and `brotli` formats), `content-type`, etc. This reduces both runtime overhead and dependencies of your target application.
- Zero-copy deserialization of a `pack` thanks to [rkyv](https://crates.io/crates/rkyv), allows the pack to be read directly from program memory, without allocating additional ram for pack contents.
- `GET`/`HEAD` http methods support, `ETag`/`if-none-match` support, `accept-encoding`/`content-encoding` negotiation, `cache-control`, `content-length` etc.

### Non goals
- Directory listings.
- index.html resolving.

### Limitations
- `pack` is not portable across crate versions / architectures.
- Text files `text/*` are assumed to be utf-8 encoded.

## Examples
For this example lets assume you are building api + gui application in rust. Your gui is pre-built (like with `npm build` or similar) in `./vcard-personal-portfolio` directory (available for real in `tests/data/`) and you want to serve it from `/` of your app.

### Packing your assets
Refer to [web-static-pack-packer](https://crates.io/crates/web-static-pack-packer) for full documentation.

To pack whole `./vcard-personal-portfolio` directory into `./vcard-personal-portfolio.pack` execute the following command:
```bash
$ web-static-pack-packer \
    directory-single \
    ./vcard-personal-portfolio \
    ./vcard-personal-portfolio.pack
```

This will create a `./vcard-personal-portfolio.pack` file, containing all your files combined, ready to be included in your target application.

### Serving from your target application
Refer to [web-static-pack](https://crates.io/crates/web-static-pack) for full example.

You will need to include the pack in your executable with <https://docs.rs/include_bytes_aligned/latest/include_bytes_aligned/>. Then pack needs to be loaded from this binary slice. At the end we construct a http service, that will serve our requests.

```rust
use include_bytes_aligned::include_bytes_aligned;

static PACK_ARCHIVED_SERIALIZED: &[u8] = 
    include_bytes_aligned!(16, "vcard-personal-portfolio.pack");

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), Error> {
    // load (map / cast) [common::pack::PackArchived] from included bytes
    let pack_archived = unsafe { load(PACK_ARCHIVED_SERIALIZED).unwrap() };

    // create a responder (http service) from `pack`
    let responder = Responder::new(pack_archived);

    // hyper requires service to be static
    // we use graceful, no connections will outlive server function
    let responder = unsafe {
        transmute::<
            &Responder<'_, _>,
            &Responder<'static, _>,
        >(&responder)
    };

    // make hyper service function
    let service_fn = service_fn(|request: Request<Incoming>| async {
        // you can probably filter your /api requests here
        let (parts, _body) = request.into_parts();
        
        let response = responder.respond_flatten(
            &parts.method,
            parts.uri.path(),
            &parts.headers,
        );
        
        Ok::<_, Infallible>(response)
    });
    
    // run hyper server using service_fn, as in:
    // https://hyper.rs/guides/1/server/graceful-shutdown/
    todo!();

    Ok(())
}
```

## Migrating from 0.4.x to 0.5.x
The 0.5.0 is almost a complete rewrite, however the general idea remains the same.
- We still have two parts - packer and loader. There is also a `common` crate and `tests` crate, however they are not meant to be used directly.
- Lots of internals were changed, including [rkyv](https://crates.io/crates/rkyv) for serialization / zero-copy deserialization. This of course makes packs built with previous versions incompatible with current loader and vice versa.
- We are now built around [http](https://crates.io/crates/http) crate, which makes web-static-pack compatible with hyper 1.0 without depending on it directly.

### BREAKING CHANGES
- Packer is now built around subcommands. The previous behavior was moved to `directory-single` subcommand, and `root_path` parameter was dropped. See examples.
- Since we no longer depend on `hyper` in any way (the `http` crate is common interface), `hyper_loader` feature is no longer present in loader.
- `let loader = loader::Loader::new(...)` is now `let pack_archived = loader::load(...)`. This value is still used for `Responder::new`.
- `hyper_loader::Responder` is now just `responder::Responder`, and it's now built around `http` crate, compatible with `hyper` 1.0.
- `Responder` was rewritten. It now accepts (method, path, headers) not whole request. `request_respond_or_error` and `parts_respond_or_error` are squashed to `respond`. `request_respond` and `parts_respond` are squashed to `respond_flatten`.

### New features and improvements
- True zero-copy deserialization with `rkyv`.
- `brotli` compression support.
- `cache-control` support.
- Packer is now a lib + bin, making it usable in build.rs. Multiple useful methods were exposed.
- Good test coverage, including integration tests.
- Lots of internal improvements.
