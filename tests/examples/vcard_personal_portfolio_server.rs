//! Spins up a server listening on [BIND] serving contents of
//! `data/vcard-personal-portfolio.pack`. Pack should be created upfront with
//! builder example.

#![feature(async_closure)]

use anyhow::Error;
use futures::{channel::oneshot, try_join};
use include_bytes_aligned::include_bytes_aligned;
use simple_logger::SimpleLogger;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use tokio::signal::ctrl_c;
use web_static_pack_tests::serve_pack;

static PACK_ARCHIVED_SERIALIZED: &[u8] =
    include_bytes_aligned!(16, "../data/vcard-personal-portfolio.pack");

const BIND: SocketAddr = SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), 3000);

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), Error> {
    SimpleLogger::new().init().unwrap();

    // load `pack` from prebuild version
    log::trace!("loading pack");
    let pack_archived = unsafe { web_static_pack::loader::load(PACK_ARCHIVED_SERIALIZED).unwrap() };

    // ctrl_c handler
    let (shutdown_sender, shutdown_receiver) = oneshot::channel::<()>();
    let ctrl_c_runner = async move {
        ctrl_c().await?;
        shutdown_sender.send(()).unwrap();
        Ok(())
    };

    // server
    log::trace!("running server, go to http://{BIND}/index.html");
    let server_runner = serve_pack(pack_archived, Some(BIND), None, shutdown_receiver);

    // combine and run
    try_join!(ctrl_c_runner, server_runner)?;

    Ok(())
}
