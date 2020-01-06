//! Serves this package docs using tokio + hyper + web_static_pack.
//!
//! 1. Create docs in `target/doc` directory: `cargo doc --no-deps`.
//! 2. Run packer `cargo run --example packer ./target/doc/ ./examples/docs/docs.pack`.
//! 3. Build & run this example `cargo run --example docs`.
//! 4. Open http://localhost:8080/ in your browser.

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

    Ok(RESPONDER.request_respond(&request))
}

async fn main_result() -> Result<(), Error> {
    let address = SocketAddr::from(([0, 0, 0, 0], 8080));
    let server = Server::bind(&address).serve(make_service_fn(|_connection| async {
        Ok::<_, Infallible>(service_fn(service))
    }));

    log::info!("Server listening on {:?}", address);
    Ok(server.await?)
}
