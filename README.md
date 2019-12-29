# web-static-pack
Embed static resources (GUI, assets, images, styles, html) within executable.
Serve with hyper or any server of your choice.

[![docs.rs](https://docs.rs/web-static-pack/badge.svg)](https://docs.rs/web-static-pack)
  
## Usage scenario:
- Combines given directory tree into single, fast, binary-based single-file representation, called `pack`. Use simple CLI tool `examples/packer` to create a pack.
- Pack could be embedded into your application using `std::include_bytes` single macro.
- Super-fast, zero-copy `loader` provides by-name access to files.
- Easy-to-use `hyper_loader` allows super-quick integration with hyper-based server.
  
## Features:
- Super fast, low overhead
- 100% 'static access, zero data copy
- 100% pack-time calculated `Content-Type`, `ETag` (using sha3)
- Almost no external dependencies

## Limitations:
- By default all files with guesses text/ content type are treated as utf-8
- Packs are not guaranteed to be portable across versions / architectures
  
## Future goals:
- 100% pack-time gzip / deflate / other compression
  
## Non-Goals:
- Directory listings
- automatic index.html resolving
- Uploads
  
## Example:
1. Create a pack from `cargo doc`:
```
$ cargo doc --no-deps
$ cargo run --example packer ./target/doc/ docs.pack
```
  
2. Serve docs.pack from your web-application (see `examples/docs`)
```
use failure::Error;
use hyper::service::{make_service_fn, service_fn};
use hyper::{Body, Request, Response, Server};
use lazy_static::lazy_static;
use std::convert::Infallible;
use std::net::SocketAddr;
use web_static_pack::hyper_loader::{Responder, StaticBody};
use web_static_pack::loader::Loader;
  
#[tokio::main]
async fn main() -> () {
    simple_logger::init_with_level(log::Level::Info).unwrap();
    main_result().await.unwrap()
}
  
async fn service(request: Request<Body>) -> Result<Response<StaticBody>, Infallible> {
    lazy_static! {
        static ref PACK: &'static [u8] = std::include_bytes!("docs.pack");
        static ref LOADER: Loader = Loader::new(&PACK).unwrap();
        static ref RESPONDER: Responder<'static> = Responder::new(&LOADER);
    }
    
    Ok(RESPONDER.respond(&request))
}
  
async fn main_result() -> Result<(), Error> {
    let address = SocketAddr::from(([127, 0, 0, 1], 8080));
    let server = Server::bind(&address).serve(make_service_fn(|_connection| async {
        Ok::<_, Infallible>(service_fn(service))
    }));
    
    log::info!("Server listening on {:?}", address);
    Ok(server.await?)
}
```