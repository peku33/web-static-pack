#![feature(async_closure)]

use futures::{channel::oneshot, try_join};
use std::net::SocketAddr;
use web_static_pack_tests::{
    download_verify_pack, load_vcard_personal_portfolio_cached, serve_pack,
};

#[tokio::test(flavor = "current_thread")]
async fn client_downloads_verifies_served_by_server() {
    // `pack` to run both server and client on
    let pack_archived = load_vcard_personal_portfolio_cached();

    // all paths from `pack` used for test
    let pack_paths = pack_archived
        .files_by_path
        .keys()
        .map(|pack_path_archived| &**pack_path_archived)
        .collect::<Vec<_>>();

    // makes client wait for server to become ready and turns it off when completed
    let (bind_ready_sender, bind_ready_receiver) = oneshot::channel::<SocketAddr>();
    let (shutdown_sender, shutdown_receiver) = oneshot::channel::<()>();

    // server
    let server = serve_pack(
        pack_archived,
        None,
        Some(bind_ready_sender),
        shutdown_receiver,
    );

    // client
    let download_verifier = async move {
        let bind = bind_ready_receiver.await?;

        download_verify_pack(pack_archived, &pack_paths, bind).await?;

        shutdown_sender.send(()).unwrap();
        Ok(())
    };

    // wait for all
    try_join!(server, download_verifier).unwrap();
}
