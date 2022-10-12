//! Serves this package docs using tokio + hyper + web_static_pack.
//!
//! 1. Run local or install packer executable: `cargo run` or `cargo install web-static-pack-packer`
//! 2. Create docs in `target/doc` directory: `cargo doc --no-deps`.
//! 3. Run packer `web-static-pack-packer ./target/doc/ ./loader/examples/docs/docs.pack`.
//! 4. Build & run this example `cargo run --example docs`.
//! 5. Open http://localhost:8080/ in your browser.

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
async fn main() {
    SimpleLogger::new()
        .env()
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
    server.await.context("server")?;

    Ok(())
}
