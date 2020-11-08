# web-static-pack
Embed static resources (GUI, assets, images, styles, html) within executable.
Serve with hyper or any server of your choice.

[![docs.rs](https://docs.rs/web-static-pack/badge.svg)](https://docs.rs/web-static-pack)
  
## Usage scenario:
- Combines given directory tree into single, fast, binary-based single-file representation, called `pack`. Use simple CLI tool `web-static-pack-packer` to create a pack.
- Pack could be embedded into your application using `include_bytes!` single macro.
- Super-fast, zero-copy `loader` provides by-name access to files.
- Easy-to-use `hyper_loader` allows super-quick integration with hyper-based server.
  
## Features:
- Super fast, low overhead
- 100% 'static access, zero data copy
- 100% pack-time calculated `Content-Type`, `ETag` (using sha3)
- 100% pack-time calculated gzip-compressed files
- Almost no external dependencies

## Limitations:
- By default all files with guesses text/ content type are treated as utf-8
- Packs are not guaranteed to be portable across versions / architectures
  
## Future goals:
- You tell me
  
## Non-Goals:
- Directory listings
- automatic index.html resolving
- Uploads
  
## Example:
1. Create a pack from `cargo doc`:
```bash
$ cargo doc --no-deps
$ cargo run ./target/doc/ docs.pack
```
  
2. Serve docs.pack from your web-application (see `examples/docs`)
```rust
use anyhow::{Context, Error};
use hyper::{
    service::{make_service_fn, service_fn},
    Body, Request, Response, Server,
};
use lazy_static::lazy_static;
use log::LevelFilter;
use simple_logger::SimpleLogger;
use std::{convert::Infallible, include_bytes, net::SocketAddr};
use web_static_pack::{hyper_loader::Responder, loader::Loader};

#[tokio::main]
async fn main() -> () {
    SimpleLogger::from_env()
        .with_level(LevelFilter::Info)
        .init()
        .unwrap();
    main_result().await.unwrap()
}

async fn service(request: Request<Body>) -> Result<Response<Body>, Infallible> {
    lazy_static! {
        static ref PACK: &'static [u8] = include_bytes!("docs.pack");
        static ref LOADER: Loader = Loader::new(&PACK).unwrap();
        static ref RESPONDER: Responder<'static> = Responder::new(&LOADER);
    }

    Ok(RESPONDER.request_respond(&request))
}

async fn main_result() -> Result<(), Error> {
    let address = SocketAddr::from(([0, 0, 0, 0], 8080));
    let server = Server::bind(&address).serve(make_service_fn(|_connection| async {
        Ok::<_, Infallible>(service_fn(service))
    }));

    log::info!("Server listening on {:?}", address);
    Ok(server.await.context("server")?)
}
```
